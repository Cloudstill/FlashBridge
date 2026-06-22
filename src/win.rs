use crate::Result;
use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
};

pub const CS_VREDRAW: u32 = 0x0001;
pub const CS_HREDRAW: u32 = 0x0002;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_USER: u32 = 0x0400;
pub const WM_APP: u32 = 0x8000;
pub const MF_STRING: u32 = 0x0000;
pub const MF_SEPARATOR: u32 = 0x0800;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
pub const SW_SHOWMINNOACTIVE: i32 = 7;
pub const SW_HIDE: i32 = 0;
pub const SW_RESTORE: i32 = 9;

pub const WM_CREATE: u32 = 0x0001;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_SETFONT: u32 = 0x0030;

pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;

pub const ES_MULTILINE: u32 = 0x0004;
pub const ES_READONLY: u32 = 0x0800;
pub const ES_AUTOVSCROLL: u32 = 0x0040;
pub const ES_AUTOHSCROLL: u32 = 0x0080;

pub const BS_AUTOCHECKBOX: u32 = 0x0003;
pub const BS_GROUPBOX: u32 = 0x0007;
pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const LBS_NOTIFY: u32 = 0x0001;

pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const LB_ADDSTRING: u32 = 0x0180;
pub const LB_RESETCONTENT: u32 = 0x0184;
pub const BM_GETCHECK: u32 = 0x00F0;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const BST_CHECKED: usize = 1;

pub const COLOR_BTNFACE: isize = 15;
pub const IDC_ARROW: isize = 32512;
pub const DEFAULT_GUI_FONT: i32 = 17;
pub const CW_USEDEFAULT: i32 = i32::MIN;

pub const NATIVE_CMD_MESSAGE: u32 = 0x8002;
pub const NATIVE_CMD_TEST: usize = 1;

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
const INVALID_HANDLE_VALUE: isize = -1;
const HKEY_CURRENT_USER: isize = -2147483647;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_OPTION_NON_VOLATILE: u32 = 0;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const SW_SHOWNORMAL: i32 = 1;
const SHGFI_ICON: u32 = 0x0000_0100;
const SHGFI_SMALLICON: u32 = 0x0000_0001;
const QUNS_ACCEPTS_NOTIFICATIONS: i32 = 5;
const FLASHW_ALL: u32 = 0x0000_0003;
const FLASHW_TIMERNOFG: u32 = 0x0000_000C;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hwnd(pub isize);

pub struct IconHandle(pub isize);

impl Drop for IconHandle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                DestroyIcon(self.0);
            }
        }
    }
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct WndClassW {
    pub style: u32,
    pub lpfnWndProc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: isize,
    pub hIcon: isize,
    pub hCursor: isize,
    pub hbrBackground: isize,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

impl Default for WndClassW {
    fn default() -> Self {
        Self {
            style: 0,
            lpfnWndProc: None,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: std::ptr::null(),
        }
    }
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Default)]
pub struct Msg {
    pub hwnd: isize,
    pub message: u32,
    pub wParam: usize,
    pub lParam: isize,
    pub time: u32,
    pub pt: Point,
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NotifyIconDataW {
    pub cbSize: u32,
    pub hWnd: isize,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: isize,
    pub szTip: [u16; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: Guid,
    pub hBalloonIcon: isize,
}

impl Default for NotifyIconDataW {
    fn default() -> Self {
        Self {
            cbSize: 0,
            hWnd: 0,
            uID: 0,
            uFlags: 0,
            uCallbackMessage: 0,
            hIcon: 0,
            szTip: [0; 128],
            dwState: 0,
            dwStateMask: 0,
            szInfo: [0; 256],
            uTimeoutOrVersion: 0,
            szInfoTitle: [0; 64],
            dwInfoFlags: 0,
            guidItem: Guid::default(),
            hBalloonIcon: 0,
        }
    }
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ShFileInfoW {
    pub hIcon: isize,
    pub iIcon: i32,
    pub dwAttributes: u32,
    pub szDisplayName: [u16; 260],
    pub szTypeName: [u16; 80],
}

impl Default for ShFileInfoW {
    fn default() -> Self {
        Self {
            hIcon: 0,
            iIcon: 0,
            dwAttributes: 0,
            szDisplayName: [0; 260],
            szTypeName: [0; 80],
        }
    }
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct FlashInfoW {
    pub cbSize: u32,
    pub hwnd: isize,
    pub dwFlags: u32,
    pub uCount: u32,
    pub dwTimeout: u32,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct ProcessEntry32W {
    pub dwSize: u32,
    pub cntUsage: u32,
    pub th32ProcessID: u32,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: u32,
    pub cntThreads: u32,
    pub th32ParentProcessID: u32,
    pub pcPriClassBase: i32,
    pub dwFlags: u32,
    pub szExeFile: [u16; 260],
}

impl Default for ProcessEntry32W {
    fn default() -> Self {
        Self {
            dwSize: std::mem::size_of::<Self>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        }
    }
}

#[derive(Debug)]
pub struct WindowInfo {
    pub process_id: u32,
    pub process_name: String,
    pub process_path: PathBuf,
    pub title: String,
}

impl WindowInfo {
    pub fn from_hwnd(hwnd: Hwnd) -> Result<Self> {
        let pid = process_id_for_window(hwnd)?;
        let process_path = process_path(pid)?;
        let process_name = process_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| process_path.to_string_lossy().into_owned());

        Ok(Self {
            process_id: pid,
            process_name,
            process_path,
            title: window_title(hwnd),
        })
    }
}

pub fn is_window(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(hwnd.0) != 0 }
}

pub fn focus_window(hwnd: Hwnd) {
    unsafe {
        SetForegroundWindow(hwnd.0);
    }
}

pub fn foreground_window_info() -> Option<WindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == 0 {
        return None;
    }

    WindowInfo::from_hwnd(Hwnd(hwnd)).ok()
}

pub fn flash_window(hwnd: Hwnd) -> Result<()> {
    let mut info = FlashInfoW {
        cbSize: std::mem::size_of::<FlashInfoW>() as u32,
        hwnd: hwnd.0,
        dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
        uCount: 3,
        dwTimeout: 0,
    };

    let ok = unsafe { FlashWindowEx(&mut info) };
    if ok == 0 {
        return Err("FlashWindowEx failed".into());
    }

    Ok(())
}

pub fn show_window(hwnd: Hwnd, command: i32) {
    unsafe {
        ShowWindow(hwnd.0, command);
    }
}

pub fn update_window(hwnd: Hwnd) {
    unsafe {
        UpdateWindow(hwnd.0);
    }
}

pub fn destroy_window(hwnd: Hwnd) {
    unsafe {
        DestroyWindow(hwnd.0);
    }
}

pub fn post_quit_message(exit_code: i32) {
    unsafe {
        PostQuitMessage(exit_code);
    }
}

pub fn open_path(path: &Path) -> Result<()> {
    open_target(&path.to_string_lossy())
}

pub fn open_target(target: &str) -> Result<()> {
    let operation = wide_null("open");
    let file = wide_null(target);
    let result = unsafe {
        ShellExecuteW(
            0,
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if result <= 32 {
        return Err(format!("ShellExecuteW failed with code {result}").into());
    }

    Ok(())
}

pub fn set_timer(hwnd: Hwnd, id: usize, millis: u32) -> Result<()> {
    let timer = unsafe { SetTimer(hwnd.0, id, millis, None) };
    if timer == 0 {
        return Err("SetTimer failed".into());
    }
    Ok(())
}

pub fn should_suppress_notifications() -> bool {
    let mut state = 0;
    let result = unsafe { SHQueryUserNotificationState(&mut state) };
    result >= 0 && state != QUNS_ACCEPTS_NOTIFICATIONS
}

pub fn extract_icon(path: &Path) -> Option<IconHandle> {
    let wide = wide_null(&path.to_string_lossy());
    let mut info = ShFileInfoW::default();
    let result = unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<ShFileInfoW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        )
    };

    if result == 0 || info.hIcon == 0 {
        None
    } else {
        Some(IconHandle(info.hIcon))
    }
}

pub fn running_process_names() -> Vec<String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = ProcessEntry32W::default();
        let mut names = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|value| *value == 0)
                    .unwrap_or(entry.szExeFile.len());
                if len > 0 {
                    names.push(
                        OsString::from_wide(&entry.szExeFile[..len])
                            .to_string_lossy()
                            .into_owned(),
                    );
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);

        names.sort_by_key(|name| name.to_ascii_lowercase());
        names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        names
    }
}

pub fn set_run_value(subkey: &str, value_name: &str, value: &str) -> Result<()> {
    let subkey = wide_null(subkey);
    let value_name = wide_null(value_name);
    let data = wide_null(value);
    let mut key = 0;

    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };

    if result != ERROR_SUCCESS {
        return Err(format!("RegCreateKeyExW failed with code {result}").into());
    }

    let result = unsafe {
        RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr() as *const u8,
            (data.len() * std::mem::size_of::<u16>()) as u32,
        )
    };
    unsafe {
        RegCloseKey(key);
    }

    if result != ERROR_SUCCESS {
        return Err(format!("RegSetValueExW failed with code {result}").into());
    }

    Ok(())
}

pub fn delete_run_value(subkey: &str, value_name: &str) -> Result<()> {
    let subkey = wide_null(subkey);
    let value_name = wide_null(value_name);
    let mut key = 0;

    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    if result != ERROR_SUCCESS {
        return Err(format!("RegOpenKeyExW failed with code {result}").into());
    }

    let result = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
    unsafe {
        RegCloseKey(key);
    }

    if result != ERROR_SUCCESS && result != ERROR_FILE_NOT_FOUND {
        return Err(format!("RegDeleteValueW failed with code {result}").into());
    }

    Ok(())
}

pub fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn send_message(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize {
    unsafe { SendMessageW(hwnd, msg, wparam, lparam) }
}

pub fn set_window_text(hwnd: isize, text: &str) {
    let wide = wide_null(text);
    unsafe { SetWindowTextW(hwnd, wide.as_ptr()) };
}

pub fn get_window_text(hwnd: isize) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        if copied <= 0 {
            return String::new();
        }
        buffer.truncate(copied as usize);
        OsString::from_wide(&buffer).to_string_lossy().into_owned()
    }
}

pub fn get_dlg_item(parent: isize, id: i32) -> isize {
    unsafe { GetDlgItem(parent, id) }
}

pub fn post_message(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) {
    unsafe {
        PostMessageW(hwnd.0, msg, wparam, lparam);
    }
}

pub fn load_cursor(name: isize) -> isize {
    unsafe { LoadCursorW(0, name as *const u16) }
}

pub fn stock_object(index: i32) -> isize {
    unsafe { GetStockObject(index) }
}

pub fn move_window(hwnd: isize, x: i32, y: i32, width: i32, height: i32) {
    unsafe {
        MoveWindow(hwnd, x, y, width, height, 1);
    }
}

pub fn client_rect(hwnd: isize) -> Rect {
    let mut rect = Rect::default();
    unsafe {
        GetClientRect(hwnd, &mut rect);
    }
    rect
}

fn process_id_for_window(hwnd: Hwnd) -> Result<u32> {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd.0, &mut pid);
    }

    if pid == 0 {
        return Err("window has no process id".into());
    }

    Ok(pid)
}

fn process_path(pid: u32) -> Result<PathBuf> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return Err(format!("OpenProcess failed for pid {pid}").into());
        }

        let mut buffer = vec![0u16; 32_768];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len);
        let close_result = CloseHandle(handle);

        if close_result == 0 {
            eprintln!("warning: CloseHandle failed for pid {pid}");
        }

        if result == 0 {
            return Err(format!("QueryFullProcessImageNameW failed for pid {pid}").into());
        }

        buffer.truncate(len as usize);
        Ok(PathBuf::from(OsString::from_wide(&buffer)))
    }
}

fn window_title(hwnd: Hwnd) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd.0);
        if len <= 0 {
            return String::new();
        }

        let mut buffer = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd.0, buffer.as_mut_ptr(), buffer.len() as i32);
        if copied <= 0 {
            return String::new();
        }

        buffer.truncate(copied as usize);
        OsString::from_wide(&buffer).to_string_lossy().into_owned()
    }
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterWindowMessageW(lpString: *const u16) -> u32;
    pub fn RegisterClassW(lpWndClass: *const WndClassW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: isize,
        hMenu: isize,
        hInstance: isize,
        lpParam: *const std::ffi::c_void,
    ) -> isize;
    pub fn DefWindowProcW(hwnd: isize, msg: u32, wParam: usize, lParam: isize) -> isize;
    pub fn GetMessageW(lpMsg: *mut Msg, hWnd: isize, wMsgFilterMin: u32, wMsgFilterMax: u32)
        -> i32;
    pub fn TranslateMessage(lpMsg: *const Msg) -> i32;
    pub fn DispatchMessageW(lpMsg: *const Msg) -> isize;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn DestroyWindow(hWnd: isize) -> i32;
    pub fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
    pub fn UpdateWindow(hWnd: isize) -> i32;
    pub fn FlashWindowEx(pfwi: *mut FlashInfoW) -> i32;
    pub fn SetTimer(
        hWnd: isize,
        nIDEvent: usize,
        uElapse: u32,
        lpTimerFunc: Option<unsafe extern "system" fn(isize, u32, usize, u32)>,
    ) -> usize;
    pub fn RegisterShellHookWindow(hwnd: isize) -> i32;
    pub fn GetWindowThreadProcessId(hwnd: isize, lpdwProcessId: *mut u32) -> u32;
    pub fn IsWindow(hwnd: isize) -> i32;
    pub fn GetWindowTextLengthW(hwnd: isize) -> i32;
    pub fn GetWindowTextW(hwnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
    pub fn GetForegroundWindow() -> isize;
    pub fn SetForegroundWindow(hwnd: isize) -> i32;
    pub fn LoadIconW(hInstance: isize, lpIconName: *const u16) -> isize;
    pub fn DestroyIcon(hIcon: isize) -> i32;
    pub fn CreatePopupMenu() -> isize;
    pub fn AppendMenuW(hMenu: isize, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> i32;
    pub fn TrackPopupMenu(
        hMenu: isize,
        uFlags: u32,
        x: i32,
        y: i32,
        nReserved: i32,
        hWnd: isize,
        prcRect: *const std::ffi::c_void,
    ) -> i32;
    pub fn DestroyMenu(hMenu: isize) -> i32;
    pub fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    pub fn SetWindowTextW(hwnd: isize, lpString: *const u16) -> i32;
    pub fn LoadCursorW(hInstance: isize, lpCursorName: *const u16) -> isize;
    pub fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    pub fn GetDlgItem(hDlg: isize, nIDDlgItem: i32) -> isize;
    pub fn MoveWindow(hWnd: isize, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: i32)
        -> i32;
    pub fn GetClientRect(hWnd: isize, lpRect: *mut Rect) -> i32;
}

#[allow(non_snake_case)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn QueryFullProcessImageNameW(
        hProcess: isize,
        dwFlags: u32,
        lpExeName: *mut u16,
        lpdwSize: *mut u32,
    ) -> i32;
    fn CloseHandle(hObject: isize) -> i32;
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: u32, lpData: *mut NotifyIconDataW) -> i32;
    fn ShellExecuteW(
        hwnd: isize,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
    fn SHGetFileInfoW(
        pszPath: *const u16,
        dwFileAttributes: u32,
        psfi: *mut ShFileInfoW,
        cbFileInfo: u32,
        uFlags: u32,
    ) -> usize;
    fn SHQueryUserNotificationState(pquns: *mut i32) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn RegCreateKeyExW(
        hKey: isize,
        lpSubKey: *const u16,
        Reserved: u32,
        lpClass: *mut u16,
        dwOptions: u32,
        samDesired: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        phkResult: *mut isize,
        lpdwDisposition: *mut u32,
    ) -> i32;
    fn RegOpenKeyExW(
        hKey: isize,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut isize,
    ) -> i32;
    fn RegSetValueExW(
        hKey: isize,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    fn RegDeleteValueW(hKey: isize, lpValueName: *const u16) -> i32;
    fn RegCloseKey(hKey: isize) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn GetStockObject(nIndex: i32) -> isize;
}
