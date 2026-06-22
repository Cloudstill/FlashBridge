use crate::{win, Result};

pub struct Notification {
    pub title: String,
    pub body: String,
    pub hwnd: Option<win::Hwnd>,
    pub sound: bool,
    pub icon: Option<win::IconHandle>,
}

pub enum TrayRequest {
    ShowMenu,
    LeftClick,
}

pub enum TrayCommand {
    TogglePause,
    OpenSettings,
    OpenDashboard,
    OpenConfig,
    TestNotification,
    Exit,
}

pub const TRAY_CALLBACK_MESSAGE: u32 = win::WM_APP + 1;

const WM_LBUTTONUP: u32 = 0x0202;
const WM_LBUTTONDBLCLK: u32 = 0x0203;

const NIM_ADD: u32 = 0x0000_0000;
const NIM_MODIFY: u32 = 0x0000_0001;
const NIM_DELETE: u32 = 0x0000_0002;
const NIM_SETVERSION: u32 = 0x0000_0004;

const NIF_MESSAGE: u32 = 0x0000_0001;
const NIF_ICON: u32 = 0x0000_0002;
const NIF_TIP: u32 = 0x0000_0004;
const NIF_INFO: u32 = 0x0000_0010;

const NIIF_INFO: u32 = 0x0000_0001;
const NIIF_USER: u32 = 0x0000_0004;
const NIIF_NOSOUND: u32 = 0x0000_0010;
const NOTIFYICON_VERSION_4: u32 = 4;
const NIN_BALLOONUSERCLICK: u32 = win::WM_USER + 5;
const IDI_APPLICATION: usize = 32512;

const MENU_TOGGLE_PAUSE: usize = 1001;
const MENU_OPEN_DASHBOARD: usize = 1002;
const MENU_OPEN_CONFIG: usize = 1003;
const MENU_TEST_NOTIFICATION: usize = 1004;
const MENU_EXIT: usize = 1005;
const MENU_OPEN_SETTINGS: usize = 1006;

pub struct ToastDispatcher {
    owner_hwnd: win::Hwnd,
    last_target: Option<win::Hwnd>,
    last_click: win::Point,
}

impl ToastDispatcher {
    pub fn new(owner_hwnd: win::Hwnd) -> Result<Self> {
        let mut dispatcher = Self {
            owner_hwnd,
            last_target: None,
            last_click: win::Point::default(),
        };
        dispatcher.add_icon()?;
        Ok(dispatcher)
    }

    pub fn show(&mut self, notification: Notification) -> Result<()> {
        self.last_target = notification.hwnd;

        let mut data = self.base_data();
        data.uFlags = NIF_INFO;
        copy_wide(&mut data.szInfoTitle, &notification.title);
        copy_wide(&mut data.szInfo, &notification.body);

        if let Some(icon) = notification.icon.as_ref() {
            data.dwInfoFlags = NIIF_USER;
            data.hBalloonIcon = icon.0;
        } else {
            data.dwInfoFlags = NIIF_INFO;
        }

        if !notification.sound {
            data.dwInfoFlags |= NIIF_NOSOUND;
        }

        let ok = unsafe { win::Shell_NotifyIconW(NIM_MODIFY, &mut data) };
        if ok == 0 {
            return Err("Shell_NotifyIconW(NIM_MODIFY) failed".into());
        }

        Ok(())
    }

    pub fn handle_callback(&mut self, lparam: isize) -> Option<TrayRequest> {
        let event = lparam as i32 as u32 & 0xffff;
        let x = (lparam >> 16) as i16 as i32;
        let y = (lparam >> 32) as i16 as i32;
        self.last_click = win::Point { x, y };

        match event {
            NIN_BALLOONUSERCLICK => {
                if let Some(hwnd) = self.last_target {
                    win::focus_window(hwnd);
                }
                None
            }
            // Under NOTIFYICON_VERSION_4 the right-button click arrives as
            // WM_RBUTTONUP. Left click opens settings directly.
            win::WM_RBUTTONUP | win::WM_CONTEXTMENU => Some(TrayRequest::ShowMenu),
            WM_LBUTTONUP | WM_LBUTTONDBLCLK => Some(TrayRequest::LeftClick),
            _ => None,
        }
    }

    pub fn show_context_menu(&self, paused: bool) -> Option<TrayCommand> {
        unsafe {
            let menu = win::CreatePopupMenu();
            if menu == 0 {
                return None;
            }

            let toggle = if paused { "Resume" } else { "Pause" };
            let labels = [
                win::wide_null(toggle),
                win::wide_null("Open settings"),
                win::wide_null("Open dashboard"),
                win::wide_null("Open config"),
                win::wide_null("Test notification"),
                win::wide_null("Exit"),
            ];

            win::AppendMenuW(menu, win::MF_STRING, MENU_TOGGLE_PAUSE, labels[0].as_ptr());
            win::AppendMenuW(menu, win::MF_STRING, MENU_OPEN_SETTINGS, labels[1].as_ptr());
            win::AppendMenuW(
                menu,
                win::MF_STRING,
                MENU_OPEN_DASHBOARD,
                labels[2].as_ptr(),
            );
            win::AppendMenuW(menu, win::MF_STRING, MENU_OPEN_CONFIG, labels[3].as_ptr());
            win::AppendMenuW(
                menu,
                win::MF_STRING,
                MENU_TEST_NOTIFICATION,
                labels[4].as_ptr(),
            );
            win::AppendMenuW(menu, win::MF_SEPARATOR, 0, std::ptr::null());
            win::AppendMenuW(menu, win::MF_STRING, MENU_EXIT, labels[5].as_ptr());

            // Use the captured click coordinates so the menu appears where the
            // user clicked even though the message loop has already drained.
            let point = self.last_click;
            win::focus_window(self.owner_hwnd);
            let command = win::TrackPopupMenu(
                menu,
                win::TPM_RIGHTBUTTON | win::TPM_RETURNCMD,
                point.x,
                point.y,
                0,
                self.owner_hwnd.0,
                std::ptr::null(),
            );
            win::DestroyMenu(menu);

            match command as usize {
                MENU_TOGGLE_PAUSE => Some(TrayCommand::TogglePause),
                MENU_OPEN_SETTINGS => Some(TrayCommand::OpenSettings),
                MENU_OPEN_DASHBOARD => Some(TrayCommand::OpenDashboard),
                MENU_OPEN_CONFIG => Some(TrayCommand::OpenConfig),
                MENU_TEST_NOTIFICATION => Some(TrayCommand::TestNotification),
                MENU_EXIT => Some(TrayCommand::Exit),
                _ => None,
            }
        }
    }

    // MAKEINTRESOURCE idiom: a resource id encoded as a low-value pointer;
    // `ptr::dangling` would be semantically wrong (low WORD must carry the id).
    #[allow(clippy::manual_dangling_ptr)]
    fn add_icon(&mut self) -> Result<()> {
        let mut data = self.base_data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_CALLBACK_MESSAGE;
        // Use the embedded application icon (resource id 1) when available;
        // fall back to the system application icon if loading fails.
        let instance = unsafe { win::GetModuleHandleW(std::ptr::null()) };
        let app_icon = if instance != 0 {
            let icon = unsafe { win::LoadIconW(instance, 1 as *const u16) };
            if icon != 0 {
                icon
            } else {
                unsafe { win::LoadIconW(0, IDI_APPLICATION as *const u16) }
            }
        } else {
            unsafe { win::LoadIconW(0, IDI_APPLICATION as *const u16) }
        };
        data.hIcon = app_icon;
        copy_wide(&mut data.szTip, "FlashBridge");

        let ok = unsafe { win::Shell_NotifyIconW(NIM_ADD, &mut data) };
        if ok == 0 {
            return Err("Shell_NotifyIconW(NIM_ADD) failed".into());
        }

        data.uTimeoutOrVersion = NOTIFYICON_VERSION_4;
        let ok = unsafe { win::Shell_NotifyIconW(NIM_SETVERSION, &mut data) };
        if ok == 0 {
            return Err("Shell_NotifyIconW(NIM_SETVERSION) failed".into());
        }

        Ok(())
    }

    fn base_data(&self) -> win::NotifyIconDataW {
        win::NotifyIconDataW {
            cbSize: std::mem::size_of::<win::NotifyIconDataW>() as u32,
            hWnd: self.owner_hwnd.0,
            uID: 1,
            ..Default::default()
        }
    }
}

impl Drop for ToastDispatcher {
    fn drop(&mut self) {
        let mut data = self.base_data();
        unsafe {
            win::Shell_NotifyIconW(NIM_DELETE, &mut data);
        }
    }
}

fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
    target.fill(0);
    for (slot, code_unit) in target
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(value.encode_utf16())
    {
        *slot = code_unit;
    }
}
