//! The Log: errors and notable events, written to log files in a folder on
//! this computer, for the user to attach to a bug report themselves.
//!
//! Nothing in the Log is ever sent anywhere. Before an entry is written, the
//! Steam Web API key and full SteamIDs are taken out of it, and the files are
//! rotated so that together they never exceed a fixed size.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use crate::Error;
use crate::clock::seconds_since_epoch;

/// The current log file; older ones are `nicelibrary.1.log`, `nicelibrary.2.log`, ...
const CURRENT: &str = "nicelibrary.log";

/// How big one log file grows before it is rotated, in bytes.
const FILE_BYTES: u64 = 1024 * 1024;

/// How many log files are kept, the current one included.
const FILES: usize = 3;

/// What stands in the Log in place of a Steam Web API key.
const KEY_REDACTED: &str = "[key]";

/// What stands in the Log in place of a full SteamID.
const STEAM_ID_REDACTED: &str = "[SteamID]";

/// How serious a Log entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// A notable event, such as a Sync.
    Info,
    /// An operation failed.
    Error,
}

impl Level {
    fn name(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Error => "ERROR",
        }
    }
}

/// The log files in one folder on this computer.
pub struct Log {
    /// `None` for an app that keeps no Log.
    folder: Option<PathBuf>,
    file_bytes: u64,
    files: usize,
    /// Held while an entry is written, so entries and rotations never interleave.
    writing: Mutex<()>,
}

impl Log {
    /// The Log in `folder`: at most three files of 1 MiB each. The folder is
    /// created when the first entry is written or the folder is opened.
    pub fn new(folder: PathBuf) -> Self {
        Self::with_limits(folder, FILE_BYTES, FILES)
    }

    /// The Log in `folder`, keeping at most `files` files (at least one) of
    /// at most `file_bytes` bytes each.
    pub fn with_limits(folder: PathBuf, file_bytes: u64, files: usize) -> Self {
        Self { folder: Some(folder), file_bytes, files: files.max(1), writing: Mutex::new(()) }
    }

    /// No Log: entries go nowhere.
    pub(crate) fn none() -> Self {
        Self { folder: None, file_bytes: FILE_BYTES, files: FILES, writing: Mutex::new(()) }
    }

    /// The folder of the log files, created if it is not there yet.
    pub(crate) fn folder(&self) -> Result<&Path, Error> {
        let folder = self.folder.as_deref().ok_or_else(|| Error::new("this app keeps no log"))?;
        fs::create_dir_all(folder).map_err(|error| Error::new(format!("log folder: {error}")))?;
        Ok(folder)
    }

    /// Writes `message` at `at`, without the key or full SteamIDs. A Log
    /// that cannot be written fails nothing: the entry is lost.
    pub(crate) fn write(&self, at: SystemTime, level: Level, message: &str) {
        if self.folder.is_none() {
            return;
        }
        let line = cut_to_fit(
            format!("{} {} {}", timestamp(seconds_since_epoch(at)), level.name(), redact(message)),
            self.file_bytes,
        );
        let _writing = self.writing.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = self.append(&line);
    }

    fn append(&self, line: &str) -> std::io::Result<()> {
        let folder = self.folder().map_err(|error| std::io::Error::other(error.to_string()))?;
        let current = folder.join(CURRENT);
        let size = fs::metadata(&current).map_or(0, |metadata| metadata.len());
        if size > 0 && size + line.len() as u64 > self.file_bytes {
            self.rotate(folder)?;
        }
        OpenOptions::new().create(true).append(true).open(current)?.write_all(line.as_bytes())
    }

    /// Moves every log file one place older and drops the oldest.
    fn rotate(&self, folder: &Path) -> std::io::Result<()> {
        let file = |index: usize| match index {
            0 => folder.join(CURRENT),
            index => folder.join(format!("nicelibrary.{index}.log")),
        };
        let oldest = file(self.files - 1);
        if oldest.exists() {
            fs::remove_file(oldest)?;
        }
        for index in (0..self.files - 1).rev() {
            if file(index).exists() {
                fs::rename(file(index), file(index + 1))?;
            }
        }
        Ok(())
    }
}

/// `line` with a line break, cut so that it fits into a file of `bytes` bytes.
fn cut_to_fit(mut line: String, bytes: u64) -> String {
    let room = usize::try_from(bytes).unwrap_or(usize::MAX).saturating_sub(1);
    let line_breaks = |c: char| c == '\n' || c == '\r';
    // One entry, one line.
    if line.contains(line_breaks) {
        line = line.replace(line_breaks, " ");
    }
    if line.len() > room {
        let mut end = room;
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        line.truncate(end);
    }
    line.push('\n');
    line
}

/// `message` with every Steam Web API key (32 hexadecimal digits) and every
/// full SteamID (a SteamID64: 17 digits) replaced. Longer runs of hex digits
/// and of digits go too, in case one hides in them.
fn redact(message: &str) -> String {
    let mut redacted = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find(|c: char| c.is_ascii_hexdigit()) {
        redacted.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(rest.len());
        let run = &rest[..end];
        let digits = run.find(|c: char| !c.is_ascii_digit()).is_none();
        if run.len() >= 32 {
            redacted.push_str(KEY_REDACTED);
        } else if digits && run.len() >= 17 {
            redacted.push_str(STEAM_ID_REDACTED);
        } else {
            redacted.push_str(&redact_steam_ids(run));
        }
        rest = &rest[end..];
    }
    redacted.push_str(rest);
    redacted
}

/// `run` of hex digits with every run of 17 or more decimal digits in it replaced.
fn redact_steam_ids(run: &str) -> String {
    let mut redacted = String::with_capacity(run.len());
    let mut rest = run;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        redacted.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        redacted.push_str(if end >= 17 { STEAM_ID_REDACTED } else { &rest[..end] });
        rest = &rest[end..];
    }
    redacted.push_str(rest);
    redacted
}

/// `seconds` since the Unix epoch as a UTC time, such as `2026-11-25T12:00:00Z`.
fn timestamp(seconds: u64) -> String {
    let (days, time) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil_date(days);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", time / 3600, time % 3600 / 60, time % 60)
}

/// The calendar date `days` days after 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}
