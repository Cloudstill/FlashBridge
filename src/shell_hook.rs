use crate::{config::Config, processor::Processor, toast, win, Result};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

const HSHELL_REDRAW: usize = 6;
const HSHELL_FLASH: usize = HSHELL_REDRAW | 0x8000;
const CONFIG_RELOAD_TIMER_ID: usize = 1;
const CONFIG_RELOAD_INTERVAL_MS: u32 = 1_000;

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

struct Runtime {
    shell_message: u32,
    processor: Processor,
}

pub fn run(config_path: PathBuf, config: Config, minimized: bool) -> Result<()> {
    let hwnd = create_hidden_window()?;

    let shell_message =
        unsafe { win::RegisterWindowMessageW(win::wide_null("SHELLHOOK").as_ptr()) };
    if shell_message == 0 {
        return Err("RegisterWindowMessageW(\"SHELLHOOK\") returned 0".into());
    }

    let processor = Processor::new(config_path, config, hwnd, minimized)?;
    RUNTIME
        .set(Mutex::new(Runtime {
            shell_message,
            processor,
        }))
        .map_err(|_| "runtime was already initialized")?;

    let registered = unsafe { win::RegisterShellHookWindow(hwnd.0) };
    if registered == 0 {
        return Err("RegisterShellHookWindow failed".into());
    }
    win::set_timer(hwnd, CONFIG_RELOAD_TIMER_ID, CONFIG_RELOAD_INTERVAL_MS)?;

    message_loop()
}

fn create_hidden_window() -> Result<win::Hwnd> {
    unsafe {
        let instance = win::GetModuleHandleW(std::ptr::null());
        if instance == 0 {
            return Err("GetModuleHandleW failed".into());
        }

        let class_name = win::wide_null("FlashBridgeShellHookWindow");
        let title = win::wide_null("FlashBridge");

        let wnd_class = win::WndClassW {
            style: win::CS_HREDRAW | win::CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };

        let atom = win::RegisterClassW(&wnd_class);
        if atom == 0 {
            return Err("RegisterClassW failed".into());
        }

        let hwnd = win::CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            instance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            return Err("CreateWindowExW failed".into());
        }

        Ok(win::Hwnd(hwnd))
    }
}

fn message_loop() -> Result<()> {
    unsafe {
        let mut msg = win::Msg::default();
        loop {
            let result = win::GetMessageW(&mut msg, 0, 0, 0);
            if result == -1 {
                return Err("GetMessageW failed".into());
            }
            if result == 0 {
                break;
            }

            let _ = win::TranslateMessage(&msg);
            win::DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe extern "system" fn wnd_proc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize {
    if msg == win::WM_DESTROY {
        unsafe {
            win::PostQuitMessage(0);
        }
        return 0;
    }

    if let Some(runtime) = RUNTIME.get() {
        let mut runtime = match runtime.lock() {
            Ok(runtime) => runtime,
            Err(poisoned) => poisoned.into_inner(),
        };

        if msg == runtime.shell_message {
            if wparam == HSHELL_FLASH {
                runtime.processor.handle_flash(win::Hwnd(lparam));
                return 0;
            }

            if wparam == HSHELL_REDRAW && lparam != 0 {
                runtime.processor.handle_redraw_flash(win::Hwnd(lparam));
                return 0;
            }
        }

        if msg == toast::TRAY_CALLBACK_MESSAGE {
            if runtime.processor.handle_notification_callback(lparam) {
                win::PostQuitMessage(0);
            }
            return 0;
        }

        if msg == win::NATIVE_CMD_MESSAGE {
            runtime.processor.handle_native_command(wparam);
            return 0;
        }

        if msg == win::WM_TIMER && wparam == CONFIG_RELOAD_TIMER_ID {
            runtime.processor.tick();
            return 0;
        }
    }

    unsafe { win::DefWindowProcW(hwnd, msg, wparam, lparam) }
}
