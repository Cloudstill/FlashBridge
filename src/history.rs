use crate::Result;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp: u64,
    pub process: String,
    pub title: String,
    pub body: String,
}

pub struct HistoryStore {
    path: PathBuf,
    limit: usize,
}

impl HistoryStore {
    pub fn new(path: PathBuf, limit: usize) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path, limit })
    }

    pub fn set_target(&mut self, path: PathBuf, limit: usize) -> Result<()> {
        if self.path == path && self.limit == limit {
            return Ok(());
        }

        *self = Self::new(path, limit)?;
        Ok(())
    }

    pub fn record(&mut self, process: &str, title: &str, body: &str) -> Result<()> {
        let entry = HistoryEntry {
            timestamp: now_seconds(),
            process: process.to_string(),
            title: title.to_string(),
            body: body.to_string(),
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            entry.timestamp,
            clean_field(&entry.process),
            clean_field(&entry.title),
            clean_field(&entry.body)
        )?;

        self.trim()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn trim(&self) -> Result<()> {
        if self.limit == 0 {
            return Ok(());
        }

        let text = fs::read_to_string(&self.path).unwrap_or_default();
        let mut lines: Vec<&str> = text.lines().collect();
        if lines.len() <= self.limit {
            return Ok(());
        }

        let keep_from = lines.len() - self.limit;
        lines.drain(0..keep_from);
        fs::write(&self.path, format!("{}\n", lines.join("\n")))?;
        Ok(())
    }
}

pub fn read_tail(path: &Path, limit: usize) -> Vec<HistoryEntry> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut entries: Vec<HistoryEntry> = text.lines().filter_map(parse_entry).collect();
    if entries.len() > limit {
        let keep_from = entries.len() - limit;
        entries.drain(0..keep_from);
    }
    entries
}

fn parse_entry(line: &str) -> Option<HistoryEntry> {
    let mut parts = line.splitn(4, '\t');
    Some(HistoryEntry {
        timestamp: parts.next()?.trim_start_matches('\u{feff}').parse().ok()?,
        process: parts.next()?.to_string(),
        title: parts.next()?.to_string(),
        body: parts.next()?.to_string(),
    })
}

fn clean_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
