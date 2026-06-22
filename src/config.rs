use crate::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    pub debounce_ms: u64,
    pub sound: bool,
    pub autostart: bool,
    pub hot_reload: bool,
    pub listen_redraw_flash: bool,
    pub ignore_foreground_process: bool,
    pub deduplicate_same_title: bool,
    pub max_per_minute: u32,
    pub respect_quiet_hours: bool,
    pub log_path: Option<PathBuf>,
    pub history_path: Option<PathBuf>,
    pub history_limit: usize,
    pub web_ui: bool,
    pub web_ui_port: u16,
    pub apps: Vec<AppRule>,
    pub ignore: Vec<AppRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Whitelist,
    Blacklist,
}

#[derive(Debug, Clone)]
pub struct AppRule {
    pub process: String,
    pub display_name: Option<String>,
    pub icon: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Whitelist,
            debounce_ms: 500,
            sound: true,
            autostart: false,
            hot_reload: true,
            listen_redraw_flash: true,
            ignore_foreground_process: true,
            deduplicate_same_title: true,
            max_per_minute: 20,
            respect_quiet_hours: true,
            log_path: None,
            history_path: None,
            history_limit: 500,
            web_ui: false,
            web_ui_port: 47621,
            apps: vec![
                AppRule::new("WeChat.exe", "WeChat"),
                AppRule::new("Weixin.exe", "WeChat"),
                AppRule::new("WeChatAppEx.exe", "WeChat"),
                AppRule::new("DingTalk.exe", "DingTalk"),
                AppRule::new("Feishu.exe", "Feishu"),
            ],
            ignore: Vec::new(),
        }
    }
}

impl AppRule {
    fn new(process: &str, display_name: &str) -> Self {
        Self {
            process: process.to_string(),
            display_name: Some(display_name.to_string()),
            icon: None,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(path)?;
        parse_config(&text)
    }

    pub fn is_allowed(&self, process_name: &str) -> bool {
        match self.mode {
            Mode::Whitelist => self
                .apps
                .iter()
                .any(|rule| process_eq(&rule.process, process_name)),
            Mode::Blacklist => !self
                .ignore
                .iter()
                .any(|rule| process_eq(&rule.process, process_name)),
        }
    }

    pub fn rule_for(&self, process_name: &str) -> Option<&AppRule> {
        self.apps
            .iter()
            .find(|rule| process_eq(&rule.process, process_name))
    }

    pub fn display_name_for(&self, process_name: &str) -> String {
        self.rule_for(process_name)
            .and_then(|rule| rule.display_name.as_ref())
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| process_name.to_string())
    }

    pub fn to_toml(&self) -> String {
        let mut text = String::new();
        text.push_str("# Global settings\n");
        text.push_str(&format!(
            "mode = \"{}\"\n",
            match self.mode {
                Mode::Whitelist => "whitelist",
                Mode::Blacklist => "blacklist",
            }
        ));
        text.push_str(&format!("debounce_ms = {}\n", self.debounce_ms));
        text.push_str(&format!("sound = {}\n", self.sound));
        text.push_str(&format!("autostart = {}\n", self.autostart));
        text.push_str(&format!("hot_reload = {}\n", self.hot_reload));
        text.push_str(&format!(
            "listen_redraw_flash = {}\n",
            self.listen_redraw_flash
        ));
        text.push_str(&format!(
            "ignore_foreground_process = {}\n",
            self.ignore_foreground_process
        ));
        text.push_str(&format!(
            "deduplicate_same_title = {}\n",
            self.deduplicate_same_title
        ));
        text.push_str(&format!("max_per_minute = {}\n", self.max_per_minute));
        text.push_str(&format!(
            "respect_quiet_hours = {}\n",
            self.respect_quiet_hours
        ));
        text.push_str(&format!(
            "log_path = \"{}\"\n",
            self.log_path
                .as_ref()
                .map(|path| toml_escape(&path.to_string_lossy()))
                .unwrap_or_default()
        ));
        text.push_str(&format!(
            "history_path = \"{}\"\n",
            self.history_path
                .as_ref()
                .map(|path| toml_escape(&path.to_string_lossy()))
                .unwrap_or_default()
        ));
        text.push_str(&format!("history_limit = {}\n", self.history_limit));
        text.push_str(&format!("web_ui = {}\n", self.web_ui));
        text.push_str(&format!("web_ui_port = {}\n", self.web_ui_port));
        text.push('\n');
        for rule in &self.apps {
            text.push_str(&rule_to_toml("apps", rule));
        }
        for rule in &self.ignore {
            text.push_str(&rule_to_toml("ignore", rule));
        }
        text
    }
}

pub fn default_config_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("FlashBridge")
            .join("config.toml");
    }

    PathBuf::from("config.toml")
}

pub fn default_log_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("FlashBridge").join("flash.log");
    }

    PathBuf::from("flash.log")
}

pub fn default_history_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("FlashBridge")
            .join("history.tsv");
    }

    PathBuf::from("history.tsv")
}

pub fn default_config_text() -> &'static str {
    include_str!("../config.toml.example")
}

pub fn ensure_config_file(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, default_config_text())?;
    Ok(())
}

pub fn ensure_absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(std::env::current_dir()?.join(path))
}

fn parse_config(text: &str) -> Result<Config> {
    let mut config = Config {
        apps: Vec::new(),
        ignore: Vec::new(),
        ..Config::default()
    };

    let mut section = Section::None;
    let mut current: Option<AppRule> = None;

    for raw_line in text.lines() {
        let line = raw_line
            .split_once('#')
            .map(|(value, _)| value)
            .unwrap_or(raw_line)
            .trim();

        if line.is_empty() {
            continue;
        }

        match line {
            "[[apps]]" => {
                push_rule(&mut config, section, current.take());
                section = Section::Apps;
                current = Some(empty_rule());
                continue;
            }
            "[[ignore]]" => {
                push_rule(&mut config, section, current.take());
                section = Section::Ignore;
                current = Some(empty_rule());
                continue;
            }
            _ => {}
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = unquote(value.trim());

        match (section, key) {
            (Section::None, "mode") => {
                config.mode = match value.as_str() {
                    "whitelist" => Mode::Whitelist,
                    "blacklist" => Mode::Blacklist,
                    other => return Err(format!("unsupported mode: {other}").into()),
                };
            }
            (Section::None, "debounce_ms") => {
                config.debounce_ms = value.parse()?;
            }
            (Section::None, "sound") => {
                config.sound = parse_bool(&value);
            }
            (Section::None, "autostart") => {
                config.autostart = parse_bool(&value);
            }
            (Section::None, "hot_reload") => {
                config.hot_reload = parse_bool(&value);
            }
            (Section::None, "listen_redraw_flash") => {
                config.listen_redraw_flash = parse_bool(&value);
            }
            (Section::None, "ignore_foreground_process") => {
                config.ignore_foreground_process = parse_bool(&value);
            }
            (Section::None, "deduplicate_same_title") => {
                config.deduplicate_same_title = parse_bool(&value);
            }
            (Section::None, "max_per_minute") => {
                config.max_per_minute = value.parse()?;
            }
            (Section::None, "respect_quiet_hours") => {
                config.respect_quiet_hours = parse_bool(&value);
            }
            (Section::None, "log_path") => {
                if value.trim().is_empty() {
                    config.log_path = None;
                } else {
                    config.log_path = Some(PathBuf::from(value));
                }
            }
            (Section::None, "history_path") => {
                if value.trim().is_empty() {
                    config.history_path = None;
                } else {
                    config.history_path = Some(PathBuf::from(value));
                }
            }
            (Section::None, "history_limit") => {
                config.history_limit = value.parse()?;
            }
            (Section::None, "web_ui") => {
                config.web_ui = parse_bool(&value);
            }
            (Section::None, "web_ui_port") => {
                config.web_ui_port = value.parse()?;
            }
            (Section::Apps | Section::Ignore, "process") => {
                if let Some(rule) = current.as_mut() {
                    rule.process = value;
                }
            }
            (Section::Apps | Section::Ignore, "display_name") => {
                if let Some(rule) = current.as_mut() {
                    rule.display_name = Some(value);
                }
            }
            (Section::Apps | Section::Ignore, "icon") => {
                if let Some(rule) = current.as_mut() {
                    rule.icon = if value.trim().is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(value))
                    };
                }
            }
            _ => {}
        }
    }

    push_rule(&mut config, section, current);
    Ok(config)
}

#[derive(Debug, Clone, Copy)]
enum Section {
    None,
    Apps,
    Ignore,
}

fn empty_rule() -> AppRule {
    AppRule {
        process: String::new(),
        display_name: None,
        icon: None,
    }
}

fn push_rule(config: &mut Config, section: Section, rule: Option<AppRule>) {
    let Some(rule) = rule else {
        return;
    };

    if rule.process.trim().is_empty() {
        return;
    }

    match section {
        Section::Apps => config.apps.push(rule),
        Section::Ignore => config.ignore.push(rule),
        Section::None => {}
    }
}

fn process_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn parse_bool(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "on")
}

fn unquote(value: &str) -> String {
    let value = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();

    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => output.push('\\'),
                Some('"') => output.push('"'),
                Some('n') => output.push('\n'),
                Some('t') => output.push('\t'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rule_to_toml(section: &str, rule: &AppRule) -> String {
    let mut text = format!(
        "[[{section}]]\nprocess = \"{}\"\n",
        toml_escape(&rule.process)
    );
    if let Some(display) = &rule.display_name {
        if !display.trim().is_empty() {
            text.push_str(&format!("display_name = \"{}\"\n", toml_escape(display)));
        }
    }
    if let Some(icon) = &rule.icon {
        text.push_str(&format!(
            "icon = \"{}\"\n",
            toml_escape(&icon.to_string_lossy())
        ));
    }
    text.push('\n');
    text
}
