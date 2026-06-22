#![windows_subsystem = "windows"]

mod autostart;
mod config;
mod history;
mod logger;
mod native_ui;
mod processor;
mod self_test;
mod shell_hook;
mod toast;
mod web_ui;
mod win;

use std::ffi::OsStr;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    if args.first().map(|value| value.as_os_str()) == Some(OsStr::new("--flash-test-window")) {
        let hold_seconds = args
            .get(1)
            .and_then(|value| value.to_string_lossy().parse::<u64>().ok())
            .unwrap_or(4);
        return self_test::run_flash_test_window(hold_seconds);
    }

    let mut minimized = false;
    let mut config_path: Option<PathBuf> = None;
    for arg in &args {
        if arg == OsStr::new("--minimized") {
            minimized = true;
        } else if config_path.is_none() {
            config_path = Some(PathBuf::from(arg));
        }
    }

    let config_path = config_path.unwrap_or_else(config::default_config_path);
    config::ensure_config_file(&config_path)?;
    let config = config::Config::load(&config_path)?;

    shell_hook::run(config_path, config, minimized)
}
