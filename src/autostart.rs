use crate::{config, win, Result};
use std::path::Path;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "FlashBridge";

pub fn apply(enabled: bool, config_path: &Path) -> Result<()> {
    if enabled {
        enable(config_path)
    } else {
        disable()
    }
}

fn enable(config_path: &Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let command = format!(
        "\"{}\" \"{}\" --minimized",
        exe.display(),
        config::ensure_absolute(config_path)?.display()
    );
    win::set_run_value(RUN_KEY, VALUE_NAME, &command)
}

fn disable() -> Result<()> {
    win::delete_run_value(RUN_KEY, VALUE_NAME)
}
