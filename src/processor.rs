use crate::{
    autostart,
    config::{self, Config},
    history::HistoryStore,
    logger::Logger,
    native_ui::NativeUi,
    toast::{Notification, ToastDispatcher, TrayCommand, TrayRequest},
    web_ui::WebUiHandle,
    win::{self, WindowInfo},
    Result,
};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

const RATE_WINDOW: Duration = Duration::from_secs(60);

pub struct Processor {
    config_path: PathBuf,
    config_modified: Option<SystemTime>,
    config: Config,
    logger: Logger,
    history: HistoryStore,
    web_ui: Option<WebUiHandle>,
    native_ui: Option<NativeUi>,
    toast: ToastDispatcher,
    paused: bool,
    last_by_window: HashMap<isize, Instant>,
    last_title_by_window: HashMap<isize, String>,
    rate_by_process: HashMap<String, VecDeque<Instant>>,
}

impl Processor {
    pub fn new(
        config_path: PathBuf,
        config: Config,
        owner_hwnd: win::Hwnd,
        minimized: bool,
    ) -> Result<Self> {
        let log_path = config
            .log_path
            .clone()
            .unwrap_or_else(config::default_log_path);
        let mut logger = Logger::new(log_path)?;
        let history_path = config
            .history_path
            .clone()
            .unwrap_or_else(config::default_history_path);
        let history = HistoryStore::new(history_path.clone(), config.history_limit)?;

        if let Err(error) = autostart::apply(config.autostart, &config_path) {
            logger.warn(format!("failed to apply autostart setting: {error}"));
        }

        logger.info(format!("log file: {}", logger.path().display()));
        logger.info(format!("history file: {}", history.path().display()));
        let log_path_for_ui = logger.path().to_path_buf();
        let history_path_for_ui = history.path().to_path_buf();
        let web_ui = start_web_ui(
            &mut logger,
            &config_path,
            &log_path_for_ui,
            &history_path_for_ui,
            config.history_limit,
            &config,
        );

        let native_ui = match NativeUi::start(
            owner_hwnd,
            config_path.clone(),
            log_path_for_ui.clone(),
            history_path_for_ui.clone(),
            config.history_limit,
        ) {
            Ok(ui) => {
                logger.info("native settings window ready");
                Some(ui)
            }
            Err(error) => {
                logger.warn(format!("failed to start native settings window: {error}"));
                None
            }
        };

        let started_minimized = minimized;
        let processor = Self {
            config_modified: modified_time(&config_path),
            config_path,
            config,
            logger,
            history,
            web_ui,
            native_ui,
            toast: ToastDispatcher::new(owner_hwnd)?,
            paused: false,
            last_by_window: HashMap::new(),
            last_title_by_window: HashMap::new(),
            rate_by_process: HashMap::new(),
        };

        if let Some(ui) = processor.native_ui.as_ref() {
            ui.open_initial(!started_minimized);
        }

        Ok(processor)
    }

    pub fn test_notification(&mut self) {
        let _ = self.toast.show(Notification {
            title: "FlashBridge".to_string(),
            body: "Test notification".to_string(),
            hwnd: None,
            sound: self.config.sound,
            icon: None,
        });
        if let Err(error) = self
            .history
            .record("FlashBridge", "FlashBridge", "Test notification")
        {
            self.logger
                .warn(format!("failed to record test notification: {error}"));
        }
    }

    pub fn handle_native_command(&mut self, command: usize) {
        if command == win::NATIVE_CMD_TEST {
            self.test_notification();
        }
    }

    pub fn handle_flash(&mut self, hwnd: win::Hwnd) {
        match self.try_handle_flash(hwnd) {
            Ok(()) => {}
            Err(error) => self.logger.error(format!(
                "failed to handle flash event for {:?}: {error}",
                hwnd
            )),
        }
    }

    pub fn handle_redraw_flash(&mut self, hwnd: win::Hwnd) {
        if !self.config.listen_redraw_flash {
            return;
        }

        match self.try_handle_flash(hwnd) {
            Ok(()) => {}
            Err(error) => self.logger.error(format!(
                "failed to handle redraw flash event for {:?}: {error}",
                hwnd
            )),
        }
    }

    pub fn handle_notification_callback(&mut self, lparam: isize) -> bool {
        let Some(request) = self.toast.handle_callback(lparam) else {
            return false;
        };

        match request {
            TrayRequest::ShowMenu => {
                let Some(command) = self.toast.show_context_menu(self.paused) else {
                    return false;
                };
                self.handle_tray_command(command)
            }
            TrayRequest::LeftClick => {
                if let Some(ui) = self.native_ui.as_ref() {
                    ui.show_settings();
                }
                false
            }
        }
    }

    pub fn tick(&mut self) {
        if let Err(error) = self.reload_config_if_changed() {
            self.logger
                .error(format!("failed to reload config: {error}"));
        }
    }

    fn try_handle_flash(&mut self, hwnd: win::Hwnd) -> Result<()> {
        if self.paused {
            return Ok(());
        }

        if !win::is_window(hwnd) {
            return Ok(());
        }

        let info = WindowInfo::from_hwnd(hwnd)?;
        if !self.config.is_allowed(&info.process_name) {
            return Ok(());
        }

        if self.is_foreground_app_event(&info) {
            self.logger.info(format!(
                "suppressed foreground app event for {} - {}",
                info.process_name, info.title
            ));
            return Ok(());
        }

        if self.is_debounced(hwnd) {
            return Ok(());
        }

        let title = self.config.display_name_for(&info.process_name);
        let body = if info.title.trim().is_empty() {
            "You have a new message".to_string()
        } else {
            info.title.clone()
        };

        if self.is_duplicate_title(hwnd, &body) {
            self.logger.info(format!(
                "suppressed duplicate title for {}",
                info.process_name
            ));
            return Ok(());
        }

        if self.is_rate_limited(&info.process_name) {
            self.logger
                .warn(format!("rate limited {}", info.process_name));
            return Ok(());
        }

        if self.config.respect_quiet_hours && win::should_suppress_notifications() {
            self.logger.info(format!(
                "suppressed notification for {} because Windows is busy or quiet",
                info.process_name
            ));
            return Ok(());
        }

        let icon = self.icon_for(&info);

        self.logger
            .info(format!("flash: {} - {}", info.process_name, body));
        self.toast.show(Notification {
            title,
            body: body.clone(),
            hwnd: Some(hwnd),
            sound: self.config.sound,
            icon,
        })?;
        if let Err(error) = self.history.record(&info.process_name, &info.title, &body) {
            self.logger
                .warn(format!("failed to record notification history: {error}"));
        }

        Ok(())
    }

    fn handle_tray_command(&mut self, command: TrayCommand) -> bool {
        match command {
            TrayCommand::TogglePause => {
                self.paused = !self.paused;
                let state = if self.paused { "paused" } else { "resumed" };
                self.logger.info(format!("service {state}"));
                let _ = self.toast.show(Notification {
                    title: "FlashBridge".to_string(),
                    body: format!("FlashBridge {state}"),
                    hwnd: None,
                    sound: false,
                    icon: None,
                });
                false
            }
            TrayCommand::OpenSettings => {
                if let Some(ui) = self.native_ui.as_ref() {
                    ui.show_settings();
                } else {
                    let _ = self.toast.show(Notification {
                        title: "FlashBridge".to_string(),
                        body: "Settings window is unavailable".to_string(),
                        hwnd: None,
                        sound: false,
                        icon: None,
                    });
                }
                false
            }
            TrayCommand::OpenDashboard => {
                if let Some(web_ui) = self.web_ui.as_ref() {
                    if let Err(error) = win::open_target(web_ui.url()) {
                        self.logger
                            .error(format!("failed to open dashboard: {error}"));
                    }
                } else {
                    let _ = self.toast.show(Notification {
                        title: "FlashBridge".to_string(),
                        body: "Dashboard is disabled or failed to start".to_string(),
                        hwnd: None,
                        sound: false,
                        icon: None,
                    });
                }
                false
            }
            TrayCommand::OpenConfig => {
                if let Err(error) = config::ensure_config_file(&self.config_path)
                    .and_then(|_| win::open_path(&self.config_path))
                {
                    self.logger
                        .error(format!("failed to open config file: {error}"));
                }
                false
            }
            TrayCommand::TestNotification => {
                self.test_notification();
                false
            }
            TrayCommand::Exit => {
                self.logger.info("exit requested from tray menu");
                true
            }
        }
    }

    fn reload_config_if_changed(&mut self) -> Result<()> {
        if !self.config.hot_reload {
            return Ok(());
        }

        let modified = modified_time(&self.config_path);
        if modified == self.config_modified {
            return Ok(());
        }

        let config = Config::load(&self.config_path)?;
        self.config_modified = modified;

        let log_path = config
            .log_path
            .clone()
            .unwrap_or_else(config::default_log_path);
        self.logger.set_path(log_path)?;
        let history_path = config
            .history_path
            .clone()
            .unwrap_or_else(config::default_history_path);
        self.history
            .set_target(history_path.clone(), config.history_limit)?;

        if let Err(error) = autostart::apply(config.autostart, &self.config_path) {
            self.logger
                .warn(format!("failed to apply autostart setting: {error}"));
        }

        if let Some(web_ui) = self.web_ui.as_ref() {
            web_ui.update_paths(
                self.logger.path().to_path_buf(),
                history_path.clone(),
                config.history_limit,
            );
        } else {
            let log_path_for_web = self.logger.path().to_path_buf();
            self.web_ui = start_web_ui(
                &mut self.logger,
                &self.config_path,
                &log_path_for_web,
                &history_path,
                config.history_limit,
                &config,
            );
        }

        if let Some(native_ui) = self.native_ui.as_ref() {
            native_ui.update_paths(
                self.logger.path().to_path_buf(),
                history_path.clone(),
                config.history_limit,
            );
        }

        self.config = config;
        self.logger.info("config reloaded");
        Ok(())
    }

    fn is_debounced(&mut self, hwnd: win::Hwnd) -> bool {
        let now = Instant::now();
        let debounce = Duration::from_millis(self.config.debounce_ms);
        let key = hwnd.0;

        if let Some(last_seen) = self.last_by_window.get(&key) {
            if now.duration_since(*last_seen) < debounce {
                return true;
            }
        }

        self.last_by_window.insert(key, now);
        false
    }

    fn is_foreground_app_event(&self, info: &WindowInfo) -> bool {
        if !self.config.ignore_foreground_process {
            return false;
        }

        let Some(foreground) = win::foreground_window_info() else {
            return false;
        };

        if foreground.process_id == info.process_id {
            return true;
        }

        if !self.config.is_allowed(&foreground.process_name) {
            return false;
        }

        self.config.display_name_for(&foreground.process_name)
            == self.config.display_name_for(&info.process_name)
    }

    fn is_duplicate_title(&mut self, hwnd: win::Hwnd, body: &str) -> bool {
        if !self.config.deduplicate_same_title {
            return false;
        }

        let key = hwnd.0;
        if self
            .last_title_by_window
            .get(&key)
            .is_some_and(|last| last == body)
        {
            return true;
        }

        self.last_title_by_window.insert(key, body.to_string());
        false
    }

    fn is_rate_limited(&mut self, process_name: &str) -> bool {
        if self.config.max_per_minute == 0 {
            return false;
        }

        let now = Instant::now();
        let timestamps = self
            .rate_by_process
            .entry(process_name.to_ascii_lowercase())
            .or_default();
        while timestamps
            .front()
            .is_some_and(|seen| now.duration_since(*seen) > RATE_WINDOW)
        {
            timestamps.pop_front();
        }

        if timestamps.len() >= self.config.max_per_minute as usize {
            return true;
        }

        timestamps.push_back(now);
        false
    }

    fn icon_for(&self, info: &WindowInfo) -> Option<win::IconHandle> {
        let configured = self
            .config
            .rule_for(&info.process_name)
            .and_then(|rule| rule.icon.as_ref())
            .map(|path| resolve_relative(&self.config_path, path));

        let path = configured.unwrap_or_else(|| info.process_path.clone());
        win::extract_icon(&path)
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
}

fn resolve_relative(config_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(path)
}

fn start_web_ui(
    logger: &mut Logger,
    config_path: &Path,
    log_path: &Path,
    history_path: &Path,
    history_limit: usize,
    config: &Config,
) -> Option<WebUiHandle> {
    if !config.web_ui {
        return None;
    }

    match WebUiHandle::start(
        config_path.to_path_buf(),
        log_path.to_path_buf(),
        history_path.to_path_buf(),
        history_limit,
        config.web_ui_port,
    ) {
        Ok(handle) => {
            logger.info(format!("web dashboard: {}", handle.url()));
            Some(handle)
        }
        Err(error) => {
            logger.warn(format!("failed to start web dashboard: {error}"));
            None
        }
    }
}
