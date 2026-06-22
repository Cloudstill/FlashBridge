use crate::{win, Result};
use std::{thread, time::Duration};

pub fn run_flash_test_window(hold_seconds: u64) -> Result<()> {
    let hwnd = create_test_window()?;
    win::show_window(hwnd, win::SW_SHOWMINNOACTIVE);
    win::update_window(hwnd);

    thread::sleep(Duration::from_millis(500));
    win::flash_window(hwnd)?;

    thread::sleep(Duration::from_secs(hold_seconds));
    win::destroy_window(hwnd);
    Ok(())
}

fn create_test_window() -> Result<win::Hwnd> {
    unsafe {
        let instance = win::GetModuleHandleW(std::ptr::null());
        if instance == 0 {
            return Err("GetModuleHandleW failed".into());
        }

        let class_name = win::wide_null("FlashBridgeSelfTestWindow");
        let title = win::wide_null("FlashBridge Self Test");
        let wnd_class = win::WndClassW {
            style: win::CS_HREDRAW | win::CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };

        let atom = win::RegisterClassW(&wnd_class);
        if atom == 0 {
            return Err("RegisterClassW failed for self-test window".into());
        }

        let hwnd = win::CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            win::WS_OVERLAPPEDWINDOW,
            100,
            100,
            420,
            180,
            0,
            0,
            instance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            return Err("CreateWindowExW failed for self-test window".into());
        }

        Ok(win::Hwnd(hwnd))
    }
}

unsafe extern "system" fn wnd_proc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize {
    if msg == win::WM_DESTROY {
        win::post_quit_message(0);
        return 0;
    }

    win::DefWindowProcW(hwnd, msg, wparam, lparam)
}
