use crate::{
    config::{AppRule, Config, Mode},
    history, win, Result,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

static STATE: OnceLock<Arc<Mutex<NativeUiState>>> = OnceLock::new();
static GUI_FONT: OnceLock<isize> = OnceLock::new();

const SETTINGS_CLASS: &str = "FlashBridgeSettingsWindow";
const TIMER_ID: usize = 2;
const TIMER_INTERVAL_MS: u32 = 2_000;
const LOG_TAIL_LINES: usize = 240;

// Control identifiers.
const IDC_MODE: i32 = 101;
const IDC_DEBOUNCE: i32 = 102;
const IDC_MAXPERMIN: i32 = 103;
const IDC_WEBPORT: i32 = 104;
const IDC_HISTLIMIT: i32 = 105;
const IDC_SOUND: i32 = 110;
const IDC_AUTOSTART: i32 = 111;
const IDC_HOTRELOAD: i32 = 112;
const IDC_LISTENREDRAW: i32 = 113;
const IDC_IGNOREFG: i32 = 114;
const IDC_DEDUP: i32 = 115;
const IDC_QUIET: i32 = 116;
const IDC_WEBUI: i32 = 117;
const IDC_LOGPATH: i32 = 120;
const IDC_HISTPATH: i32 = 121;
const IDC_APPS: i32 = 122;
const IDC_IGNORE: i32 = 123;
const IDC_LOG: i32 = 130;
const IDC_HISTORY: i32 = 131;
const IDC_SAVE: i32 = 200;
const IDC_TEST: i32 = 201;
const IDC_VIEWLOG: i32 = 202;
const IDC_OPENCONFIG: i32 = 203;

// Group boxes (addressed by id for layout).
const IDC_GRP_GENERAL: i32 = 300;
const IDC_GRP_RULES: i32 = 301;
const IDC_GRP_LOG: i32 = 302;
const IDC_GRP_HISTORY: i32 = 303;

// Static labels (addressed by id for layout).
const IDC_LBL_MODE: i32 = 400;
const IDC_LBL_DEBOUNCE: i32 = 401;
const IDC_LBL_MAXPERMIN: i32 = 402;
const IDC_LBL_WEBPORT: i32 = 403;
const IDC_LBL_HISTLIMIT: i32 = 404;
const IDC_LBL_LOGPATH: i32 = 405;
const IDC_LBL_HISTPATH: i32 = 406;
const IDC_LBL_APPS: i32 = 407;
const IDC_LBL_IGNORE: i32 = 408;

#[derive(Clone)]
struct NativeUiState {
    config_path: PathBuf,
    log_path: PathBuf,
    history_path: PathBuf,
    history_limit: usize,
    owner_hwnd: win::Hwnd,
    settings_hwnd: isize,
}

pub struct NativeUi {
    state: Arc<Mutex<NativeUiState>>,
}

impl NativeUi {
    pub fn start(
        owner_hwnd: win::Hwnd,
        config_path: PathBuf,
        log_path: PathBuf,
        history_path: PathBuf,
        history_limit: usize,
    ) -> Result<Self> {
        let state = Arc::new(Mutex::new(NativeUiState {
            config_path,
            log_path,
            history_path,
            history_limit,
            owner_hwnd,
            settings_hwnd: 0,
        }));

        // Register the shared state before creating the window: WM_CREATE runs
        // synchronously inside CreateWindowExW and needs to read it.
        let _ = STATE.set(Arc::clone(&state));
        create_settings_window(&state)?;

        Ok(Self { state })
    }

    pub fn show_settings(&self) {
        let hwnd = self
            .state
            .lock()
            .map(|state| state.settings_hwnd)
            .unwrap_or(0);
        if hwnd != 0 {
            unsafe {
                win::ShowWindow(hwnd, win::SW_RESTORE);
                win::SetForegroundWindow(hwnd);
            }
        }
    }

    pub fn open_initial(&self, show: bool) {
        if show {
            self.show_settings();
        }
    }

    pub fn update_paths(&self, log_path: PathBuf, history_path: PathBuf, history_limit: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.log_path = log_path;
            state.history_path = history_path;
            state.history_limit = history_limit;
        }
    }
}

fn snapshot() -> Option<NativeUiState> {
    STATE.get().and_then(|state| {
        state
            .lock()
            .map(|state| state.clone())
            .ok()
            .or_else(|| state.lock().ok().map(|state| state.clone()))
    })
}

// MAKEINTRESOURCE idiom: resource ids encoded as low-value pointers via
// `N as *const u16`. `ptr::dangling` would be semantically wrong here — the
// low WORD must carry the integer resource id for IS_INTRESOURCE.
#[allow(clippy::manual_dangling_ptr)]
fn create_settings_window(state: &Arc<Mutex<NativeUiState>>) -> Result<()> {
    unsafe {
        let instance = win::GetModuleHandleW(std::ptr::null());
        if instance == 0 {
            return Err("GetModuleHandleW failed".into());
        }

        let class_name = win::wide_null(SETTINGS_CLASS);
        let app_icon = {
            let icon = win::LoadIconW(instance, 1 as *const u16);
            if icon != 0 {
                icon
            } else {
                win::LoadIconW(0, 32512 as *const u16)
            }
        };
        let wnd_class = win::WndClassW {
            style: win::CS_HREDRAW | win::CS_VREDRAW,
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: instance,
            hIcon: app_icon,
            hCursor: win::load_cursor(win::IDC_ARROW),
            hbrBackground: win::COLOR_BTNFACE + 1,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };

        let atom = win::RegisterClassW(&wnd_class);
        if atom == 0 {
            // Class may already be registered from a previous run; ignore.
        }

        let title = win::wide_null("FlashBridge Settings");
        let hwnd = win::CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            win::WS_OVERLAPPEDWINDOW,
            win::CW_USEDEFAULT,
            win::CW_USEDEFAULT,
            860,
            660,
            0,
            0,
            instance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            return Err("CreateWindowExW failed for settings window".into());
        }

        if let Ok(mut state) = state.lock() {
            state.settings_hwnd = hwnd;
        }
    }

    Ok(())
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if msg == win::WM_CREATE {
        create_controls(hwnd);
        if let Some(state) = snapshot() {
            populate_controls(hwnd, &state);
            let _ = win::set_timer(win::Hwnd(hwnd), TIMER_ID, TIMER_INTERVAL_MS);
            refresh_log(hwnd, &state);
            refresh_history(hwnd, &state);
        }
        return 0;
    }

    if msg == win::WM_CLOSE {
        win::ShowWindow(hwnd, win::SW_HIDE);
        return 0;
    }

    if msg == win::WM_SIZE {
        layout(hwnd);
        return 0;
    }

    if msg == win::WM_COMMAND {
        let id = (wparam & 0xffff) as i32;
        let code = (wparam >> 16) as i32;
        if code == 0 {
            if let Some(state) = snapshot() {
                match id {
                    IDC_SAVE => save_config(hwnd, &state),
                    IDC_TEST => win::post_message(
                        state.owner_hwnd,
                        win::NATIVE_CMD_MESSAGE,
                        win::NATIVE_CMD_TEST,
                        0,
                    ),
                    IDC_VIEWLOG => {
                        let _ = win::open_path(&state.log_path);
                    }
                    IDC_OPENCONFIG => {
                        let _ = win::open_path(&state.config_path);
                    }
                    _ => {}
                }
            }
        }
        return 0;
    }

    if msg == win::WM_TIMER && wparam == TIMER_ID {
        if let Some(state) = snapshot() {
            refresh_log(hwnd, &state);
            refresh_history(hwnd, &state);
        }
        return 0;
    }

    win::DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn gui_font() -> usize {
    *GUI_FONT.get_or_init(|| win::stock_object(win::DEFAULT_GUI_FONT)) as usize
}

#[allow(clippy::too_many_arguments)]
fn create_control(
    parent: isize,
    class: &str,
    text: &str,
    style: u32,
    ex_style: u32,
    id: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> isize {
    let class_w = win::wide_null(class);
    let text_w = win::wide_null(text);
    let hwnd = unsafe {
        win::CreateWindowExW(
            ex_style,
            class_w.as_ptr(),
            text_w.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            id as isize,
            win::GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    if hwnd != 0 {
        win::send_message(hwnd, win::WM_SETFONT, gui_font(), 1);
    }
    hwnd
}

// Layout metrics. The left settings column has a fixed width; the right
// monitoring column and the bottom button row reflow on resize.
const MARGIN: i32 = 14;
const COL_GAP: i32 = 12;
const LEFT_COL_WIDTH: i32 = 388;
const ROW: i32 = 22;
const ROW_GAP: i32 = 7;
const LABEL_H: i32 = 16;
const BTN_H: i32 = 30;
const BTN_BAR_H: i32 = BTN_H + MARGIN;
const GROUP_PAD: i32 = 14; // padding inside a group box, below the title
const GROUP_TITLE_H: i32 = 18;

fn create_controls(hwnd: isize) {
    let child = win::WS_CHILD | win::WS_VISIBLE;
    let cb_style = child | win::BS_AUTOCHECKBOX;
    let edit_single = child | win::WS_BORDER | win::ES_AUTOHSCROLL;
    let edit_multi =
        child | win::WS_BORDER | win::ES_MULTILINE | win::ES_AUTOVSCROLL | win::WS_VSCROLL;

    // Left column group boxes.
    create_control(
        hwnd,
        "button",
        "General",
        child | win::BS_GROUPBOX,
        0,
        IDC_GRP_GENERAL,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Process rules",
        child | win::BS_GROUPBOX,
        0,
        IDC_GRP_RULES,
        0,
        0,
        0,
        0,
    );

    // General: mode.
    create_control(hwnd, "static", "Mode", child, 0, IDC_LBL_MODE, 0, 0, 0, 0);
    create_control(
        hwnd,
        "combobox",
        "",
        child | win::CBS_DROPDOWNLIST | win::WS_VSCROLL,
        0,
        IDC_MODE,
        0,
        0,
        0,
        0,
    );

    // Checkboxes.
    create_control(hwnd, "button", "Sound", cb_style, 0, IDC_SOUND, 0, 0, 0, 0);
    create_control(
        hwnd,
        "button",
        "Autostart",
        cb_style,
        0,
        IDC_AUTOSTART,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Hot reload",
        cb_style,
        0,
        IDC_HOTRELOAD,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Listen redraw flash",
        cb_style,
        0,
        IDC_LISTENREDRAW,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Ignore foreground process",
        cb_style,
        0,
        IDC_IGNOREFG,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Deduplicate same title",
        cb_style,
        0,
        IDC_DEDUP,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Respect quiet hours",
        cb_style,
        0,
        IDC_QUIET,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Enable web dashboard",
        cb_style,
        0,
        IDC_WEBUI,
        0,
        0,
        0,
        0,
    );

    // Numeric row.
    create_control(
        hwnd,
        "static",
        "Debounce ms",
        child,
        0,
        IDC_LBL_DEBOUNCE,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_DEBOUNCE,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "static",
        "Max/min",
        child,
        0,
        IDC_LBL_MAXPERMIN,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_MAXPERMIN,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "static",
        "Web port",
        child,
        0,
        IDC_LBL_WEBPORT,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_WEBPORT,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "static",
        "History limit",
        child,
        0,
        IDC_LBL_HISTLIMIT,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_HISTLIMIT,
        0,
        0,
        0,
        0,
    );

    // Path edits.
    create_control(
        hwnd,
        "static",
        "Log path (empty = default)",
        child,
        0,
        IDC_LBL_LOGPATH,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_LOGPATH,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "static",
        "History path (empty = default)",
        child,
        0,
        IDC_LBL_HISTPATH,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_single,
        win::WS_EX_CLIENTEDGE,
        IDC_HISTPATH,
        0,
        0,
        0,
        0,
    );

    // Process rules.
    create_control(
        hwnd,
        "static",
        "Apps (process | display name | icon path)",
        child,
        0,
        IDC_LBL_APPS,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_multi,
        win::WS_EX_CLIENTEDGE,
        IDC_APPS,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "static",
        "Ignore (blacklist mode only)",
        child,
        0,
        IDC_LBL_IGNORE,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_multi,
        win::WS_EX_CLIENTEDGE,
        IDC_IGNORE,
        0,
        0,
        0,
        0,
    );

    // Right column group boxes + contents.
    create_control(
        hwnd,
        "button",
        "Live log",
        child | win::BS_GROUPBOX,
        0,
        IDC_GRP_LOG,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "edit",
        "",
        edit_multi | win::ES_READONLY,
        win::WS_EX_CLIENTEDGE,
        IDC_LOG,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Notification history",
        child | win::BS_GROUPBOX,
        0,
        IDC_GRP_HISTORY,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "listbox",
        "",
        child | win::WS_BORDER | win::LBS_NOTIFY | win::WS_VSCROLL,
        win::WS_EX_CLIENTEDGE,
        IDC_HISTORY,
        0,
        0,
        0,
        0,
    );

    // Bottom button row.
    create_control(hwnd, "button", "Save", child, 0, IDC_SAVE, 0, 0, 0, 0);
    create_control(
        hwnd,
        "button",
        "Test notification",
        child,
        0,
        IDC_TEST,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "View log",
        child,
        0,
        IDC_VIEWLOG,
        0,
        0,
        0,
        0,
    );
    create_control(
        hwnd,
        "button",
        "Open config",
        child,
        0,
        IDC_OPENCONFIG,
        0,
        0,
        0,
        0,
    );

    layout(hwnd);
}

// Reposition every control for the current client area. Left column is fixed
// width with two stacked group boxes; right column and button bar reflow.
fn layout(hwnd: isize) {
    let rect = win::client_rect(hwnd);
    let client_w = rect.right - rect.left;
    let client_h = rect.bottom - rect.top;
    if client_w <= 0 || client_h <= 0 {
        return;
    }

    let left_x = MARGIN;
    let left_w = LEFT_COL_WIDTH;
    let right_x = left_x + left_w + COL_GAP;
    let right_w = client_w - right_x - MARGIN;
    let btn_bar_y = client_h - BTN_BAR_H;
    let content_bottom = btn_bar_y - MARGIN;
    let inner_x = left_x + GROUP_PAD;
    let inner_w = left_w - GROUP_PAD * 2;

    // ---- General group box ----
    let g1_top = MARGIN;
    let mut y = g1_top + GROUP_TITLE_H + 4;

    move_ctl(hwnd, IDC_LBL_MODE, inner_x, y, 80, LABEL_H);
    move_ctl(hwnd, IDC_MODE, inner_x + 90, y - 3, inner_w - 90, 160);
    y += ROW + ROW_GAP;

    let cb_w = (inner_w - COL_GAP) / 2;
    let cb_x2 = inner_x + cb_w + COL_GAP;
    for (left, right) in [
        (IDC_SOUND, IDC_HOTRELOAD),
        (IDC_AUTOSTART, IDC_LISTENREDRAW),
        (IDC_IGNOREFG, IDC_DEDUP),
        (IDC_QUIET, IDC_WEBUI),
    ] {
        move_ctl(hwnd, left, inner_x, y, cb_w, ROW);
        move_ctl(hwnd, right, cb_x2, y, cb_w, ROW);
        y += ROW + ROW_GAP;
    }

    let num_pairs = [
        (IDC_LBL_DEBOUNCE, IDC_DEBOUNCE),
        (IDC_LBL_MAXPERMIN, IDC_MAXPERMIN),
        (IDC_LBL_WEBPORT, IDC_WEBPORT),
        (IDC_LBL_HISTLIMIT, IDC_HISTLIMIT),
    ];
    let pair_w = (inner_w - COL_GAP * 3) / 4;
    for (i, (label, edit)) in num_pairs.iter().enumerate() {
        let px = inner_x + (pair_w + COL_GAP) * i as i32;
        move_ctl(hwnd, *label, px, y, pair_w, LABEL_H);
        move_ctl(hwnd, *edit, px, y + LABEL_H, pair_w, ROW);
    }
    y += LABEL_H + ROW + ROW_GAP;

    move_ctl(hwnd, IDC_LBL_LOGPATH, inner_x, y, inner_w, LABEL_H);
    y += LABEL_H;
    move_ctl(hwnd, IDC_LOGPATH, inner_x, y, inner_w, ROW);
    y += ROW + ROW_GAP;

    move_ctl(hwnd, IDC_LBL_HISTPATH, inner_x, y, inner_w, LABEL_H);
    y += LABEL_H;
    move_ctl(hwnd, IDC_HISTPATH, inner_x, y, inner_w, ROW);
    y += ROW + GROUP_PAD;

    move_ctl(hwnd, IDC_GRP_GENERAL, left_x, g1_top, left_w, y - g1_top);
    y += MARGIN;

    // ---- Process rules group box ----
    let g2_top = y;
    let mut y = g2_top + GROUP_TITLE_H + 4;
    move_ctl(hwnd, IDC_LBL_APPS, inner_x, y, inner_w, LABEL_H);
    y += LABEL_H;
    let apps_h = (content_bottom - y) / 2 - LABEL_H - ROW_GAP;
    move_ctl(hwnd, IDC_APPS, inner_x, y, inner_w, apps_h);
    y += apps_h + ROW_GAP;
    move_ctl(hwnd, IDC_LBL_IGNORE, inner_x, y, inner_w, LABEL_H);
    y += LABEL_H;
    let ignore_h = content_bottom - y;
    move_ctl(hwnd, IDC_IGNORE, inner_x, y, inner_w, ignore_h);
    let g2_h = content_bottom + GROUP_PAD - g2_top;
    move_ctl(hwnd, IDC_GRP_RULES, left_x, g2_top, left_w, g2_h);

    // ---- Right column: live log + history ----
    let avail_h = content_bottom - MARGIN;
    let half_h = avail_h / 2;
    let log_box_top = MARGIN;
    let log_box_h = half_h;
    let hist_box_top = log_box_top + log_box_h + COL_GAP;
    let hist_box_h = content_bottom - hist_box_top;

    move_ctl(hwnd, IDC_GRP_LOG, right_x, log_box_top, right_w, log_box_h);
    move_ctl(
        hwnd,
        IDC_LOG,
        right_x + GROUP_PAD,
        log_box_top + GROUP_TITLE_H + 4,
        right_w - GROUP_PAD * 2,
        log_box_h - GROUP_TITLE_H - 4 - GROUP_PAD,
    );

    move_ctl(
        hwnd,
        IDC_GRP_HISTORY,
        right_x,
        hist_box_top,
        right_w,
        hist_box_h,
    );
    move_ctl(
        hwnd,
        IDC_HISTORY,
        right_x + GROUP_PAD,
        hist_box_top + GROUP_TITLE_H + 4,
        right_w - GROUP_PAD * 2,
        hist_box_h - GROUP_TITLE_H - 4 - GROUP_PAD,
    );

    // ---- Bottom button row (right-aligned) ----
    let btns = [IDC_SAVE, IDC_TEST, IDC_VIEWLOG, IDC_OPENCONFIG];
    let btn_widths = [80, 130, 90, 110];
    let total_btn_w: i32 = btn_widths.iter().sum::<i32>() + COL_GAP * (btns.len() as i32 - 1);
    let mut bx = client_w - MARGIN - total_btn_w;
    for (id, w) in btns.iter().zip(btn_widths.iter()) {
        move_ctl(hwnd, *id, bx, btn_bar_y, *w, BTN_H);
        bx += w + COL_GAP;
    }
}

fn move_ctl(hwnd: isize, id: i32, x: i32, y: i32, w: i32, h: i32) {
    let ctl = win::get_dlg_item(hwnd, id);
    if ctl != 0 {
        win::move_window(ctl, x, y, w, h);
    }
}

fn populate_controls(hwnd: isize, state: &NativeUiState) {
    let config = Config::load(&state.config_path).unwrap_or_default();

    let combo = win::get_dlg_item(hwnd, IDC_MODE);
    if combo != 0 {
        for label in ["Whitelist", "Blacklist"] {
            let wide = win::wide_null(label);
            win::send_message(combo, win::CB_ADDSTRING, 0, wide.as_ptr() as isize);
        }
        let index = if config.mode == Mode::Whitelist { 0 } else { 1 };
        win::send_message(combo, win::CB_SETCURSEL, index, 0);
    }

    set_edit_text(hwnd, IDC_DEBOUNCE, &config.debounce_ms.to_string());
    set_edit_text(hwnd, IDC_MAXPERMIN, &config.max_per_minute.to_string());
    set_edit_text(hwnd, IDC_WEBPORT, &config.web_ui_port.to_string());
    set_edit_text(hwnd, IDC_HISTLIMIT, &config.history_limit.to_string());

    set_check(hwnd, IDC_SOUND, config.sound);
    set_check(hwnd, IDC_AUTOSTART, config.autostart);
    set_check(hwnd, IDC_HOTRELOAD, config.hot_reload);
    set_check(hwnd, IDC_LISTENREDRAW, config.listen_redraw_flash);
    set_check(hwnd, IDC_IGNOREFG, config.ignore_foreground_process);
    set_check(hwnd, IDC_DEDUP, config.deduplicate_same_title);
    set_check(hwnd, IDC_QUIET, config.respect_quiet_hours);
    set_check(hwnd, IDC_WEBUI, config.web_ui);

    set_edit_text(
        hwnd,
        IDC_LOGPATH,
        &config
            .log_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    set_edit_text(
        hwnd,
        IDC_HISTPATH,
        &config
            .history_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    set_edit_text(hwnd, IDC_APPS, &rules_to_text(&config.apps));
    set_edit_text(hwnd, IDC_IGNORE, &rules_to_text(&config.ignore));
}

fn save_config(hwnd: isize, state: &NativeUiState) {
    let config = collect_config(hwnd);
    if let Err(error) = std::fs::write(&state.config_path, config.to_toml()) {
        let _ = error;
    }
    // The processor hot-reloads the file; reload the controls to confirm.
    populate_controls(hwnd, state);
}

fn collect_config(hwnd: isize) -> Config {
    let mut config = Config::default();

    let mode_index = win::send_message(win::get_dlg_item(hwnd, IDC_MODE), win::CB_GETCURSEL, 0, 0);
    config.mode = if mode_index == 1 {
        Mode::Blacklist
    } else {
        Mode::Whitelist
    };

    config.debounce_ms = parse_u64(&get_edit_text(hwnd, IDC_DEBOUNCE), 500);
    config.max_per_minute = parse_u64(&get_edit_text(hwnd, IDC_MAXPERMIN), 20) as u32;
    config.web_ui_port = parse_u64(&get_edit_text(hwnd, IDC_WEBPORT), 47621) as u16;
    config.history_limit = parse_u64(&get_edit_text(hwnd, IDC_HISTLIMIT), 500) as usize;

    config.sound = is_checked(hwnd, IDC_SOUND);
    config.autostart = is_checked(hwnd, IDC_AUTOSTART);
    config.hot_reload = is_checked(hwnd, IDC_HOTRELOAD);
    config.listen_redraw_flash = is_checked(hwnd, IDC_LISTENREDRAW);
    config.ignore_foreground_process = is_checked(hwnd, IDC_IGNOREFG);
    config.deduplicate_same_title = is_checked(hwnd, IDC_DEDUP);
    config.respect_quiet_hours = is_checked(hwnd, IDC_QUIET);
    config.web_ui = is_checked(hwnd, IDC_WEBUI);

    let log_path = get_edit_text(hwnd, IDC_LOGPATH);
    config.log_path = if log_path.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(log_path))
    };
    let history_path = get_edit_text(hwnd, IDC_HISTPATH);
    config.history_path = if history_path.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(history_path))
    };

    config.apps = parse_rules(&get_edit_text(hwnd, IDC_APPS));
    config.ignore = parse_rules(&get_edit_text(hwnd, IDC_IGNORE));
    config
}

fn refresh_log(hwnd: isize, state: &NativeUiState) {
    let ctl = win::get_dlg_item(hwnd, IDC_LOG);
    if ctl == 0 {
        return;
    }
    let text = std::fs::read_to_string(&state.log_path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let tail = if lines.len() > LOG_TAIL_LINES {
        lines[lines.len() - LOG_TAIL_LINES..].join("\n")
    } else {
        text
    };
    win::set_window_text(ctl, &tail);
}

fn refresh_history(hwnd: isize, state: &NativeUiState) {
    let ctl = win::get_dlg_item(hwnd, IDC_HISTORY);
    if ctl == 0 {
        return;
    }
    win::send_message(ctl, win::LB_RESETCONTENT, 0, 0);
    for entry in history::read_tail(&state.history_path, state.history_limit)
        .into_iter()
        .rev()
    {
        let line = format!("{} {} - {}", entry.timestamp, entry.process, entry.title);
        let wide = win::wide_null(&line);
        win::send_message(ctl, win::LB_ADDSTRING, 0, wide.as_ptr() as isize);
    }
}

fn set_edit_text(hwnd: isize, id: i32, text: &str) {
    let ctl = win::get_dlg_item(hwnd, id);
    if ctl != 0 {
        win::set_window_text(ctl, text);
    }
}

fn get_edit_text(hwnd: isize, id: i32) -> String {
    let ctl = win::get_dlg_item(hwnd, id);
    if ctl == 0 {
        return String::new();
    }
    win::get_window_text(ctl)
}

fn set_check(hwnd: isize, id: i32, checked: bool) {
    let ctl = win::get_dlg_item(hwnd, id);
    if ctl != 0 {
        let value = if checked { win::BST_CHECKED } else { 0 };
        win::send_message(ctl, win::BM_SETCHECK, value, 0);
    }
}

fn is_checked(hwnd: isize, id: i32) -> bool {
    let ctl = win::get_dlg_item(hwnd, id);
    if ctl == 0 {
        return false;
    }
    win::send_message(ctl, win::BM_GETCHECK, 0, 0) == win::BST_CHECKED as isize
}

fn parse_u64(text: &str, default: u64) -> u64 {
    text.trim().parse::<u64>().unwrap_or(default)
}

fn parse_rules(text: &str) -> Vec<AppRule> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches('\r');
            if line.is_empty() {
                return None;
            }
            let mut parts = line.split('|').map(str::trim);
            let process = parts.next()?.to_string();
            if process.is_empty() {
                return None;
            }
            let display_name = parts
                .next()
                .filter(|value| !value.is_empty())
                .map(String::from);
            let icon = parts
                .next()
                .filter(|value| !value.is_empty())
                .map(PathBuf::from);
            Some(AppRule {
                process,
                display_name,
                icon,
            })
        })
        .collect()
}

fn rules_to_text(rules: &[AppRule]) -> String {
    rules
        .iter()
        .map(|rule| {
            let display = rule.display_name.clone().unwrap_or_default();
            let icon = rule
                .icon
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("{} | {} | {}", rule.process, display, icon)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
