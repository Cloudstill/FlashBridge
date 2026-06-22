use crate::Result;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Logger {
    path: PathBuf,
    file: File,
}

impl Logger {
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path, file })
    }

    pub fn set_path(&mut self, path: PathBuf) -> Result<()> {
        if self.path == path {
            return Ok(());
        }

        *self = Self::new(path)?;
        Ok(())
    }

    pub fn info(&mut self, message: impl AsRef<str>) {
        self.write("INFO", message.as_ref());
    }

    pub fn warn(&mut self, message: impl AsRef<str>) {
        self.write("WARN", message.as_ref());
    }

    pub fn error(&mut self, message: impl AsRef<str>) {
        self.write("ERROR", message.as_ref());
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write(&mut self, level: &str, message: &str) {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let line = format!("{seconds} {level} {message}\n");

        if let Err(error) = self.file.write_all(line.as_bytes()) {
            eprintln!("failed to write log {}: {error}", self.path.display());
        }
    }
}
