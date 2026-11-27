//! The Log: errors and notable events, written to log files in a folder on
//! this computer, for the user to attach to a bug report themselves.
//!
//! Nothing in the Log is ever sent anywhere. Before an entry is written, the
//! Steam Web API key, full SteamIDs and the name of the user's profile
//! folder are taken out of it, and the files are rotated so that together
//! they never exceed a fixed size.

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

/// What stands in the Log in place of the user's profile folder, such as
/// `C:\Users\<name>` or `/home/<name>`.
const PROFILE_REDACTED: &str = "%USERPROFILE%";

/// The folders that hold one folder per user, named after the user, as a
/// path writes them after the drive or the root. Matched regardless of case.
const PROFILE_PARENTS: [&str; 2] = ["users", "home"];

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
    /// The user's home folder, as the environment names it, taken out of every entry.
    home: Option<String>,
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
        Self { folder: Some(folder), file_bytes, files: files.max(1), home: home_folder(), writing: Mutex::new(()) }
    }

    /// No Log: entries go nowhere.
    pub(crate) fn none() -> Self {
        Self { folder: None, file_bytes: FILE_BYTES, files: FILES, home: None, writing: Mutex::new(()) }
    }

    /// The folder of the log files, created if it is not there yet.
    pub(crate) fn folder(&self) -> Result<&Path, Error> {
        let folder = self.folder.as_deref().ok_or_else(|| Error::new("this app keeps no log"))?;
        fs::create_dir_all(folder).map_err(|error| Error::new(format!("log folder: {error}")))?;
        Ok(folder)
    }

    /// Writes `message` at `at`, without the key, full SteamIDs or the name
    /// of the user's profile folder. A Log that cannot be written fails
    /// nothing: the entry is lost.
    pub(crate) fn write(&self, at: SystemTime, level: Level, message: &str) {
        if self.folder.is_none() {
            return;
        }
        let line = cut_to_fit(
            format!("{} {} {}", timestamp(seconds_since_epoch(at)), level.name(), redact(&redact_profiles(message, self.home.as_deref()))),
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

/// The user's home folder as the environment names it, if it names one that
/// is more than a drive or the root.
fn home_folder() -> Option<String> {
    ["USERPROFILE", "HOME"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|home| home.trim_end_matches(['\\', '/']).to_owned())
        .find(|home| home.len() > 3)
}

/// `message` with the user's profile folder replaced: `home` wherever it
/// stands, and every `<drive>:\Users\<name>`, `/home/<name>` and
/// `/Users/<name>`, with either slash and in any case, so the user's name
/// does not reach the Log. A name ends at the next slash, quote or colon.
fn redact_profiles(message: &str, home: Option<&str>) -> String {
    let mut message = match home {
        Some(home) => replace_ignoring_case(message, home, PROFILE_REDACTED),
        None => message.to_owned(),
    };
    for parent in PROFILE_PARENTS {
        message = redact_user_folders(&message, parent);
    }
    message
}

/// `text` with every `needle`, in any ASCII case, replaced by `with`.
fn replace_ignoring_case(text: &str, needle: &str, with: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut replaced = String::with_capacity(text.len());
    let mut done = 0;
    while let Some(found) = lower[done..].find(&needle) {
        replaced.push_str(&text[done..done + found]);
        replaced.push_str(with);
        done += found + needle.len();
    }
    replaced.push_str(&text[done..]);
    replaced
}

/// `message` with every user folder under the folder `parent` replaced,
/// such as `C:\Users\Kevin` for `users`, but not `C:\Program Files\Users\x`:
/// `parent` must follow a drive or start a path.
fn redact_user_folders(message: &str, parent: &str) -> String {
    let lower = message.to_ascii_lowercase();
    let is_slash = |b: u8| b == b'\\' || b == b'/';
    let mut redacted = String::with_capacity(message.len());
    let mut done = 0;
    let mut from = 0;
    while let Some(found) = lower[from..].find(parent) {
        let start = from + found;
        let end = start + parent.len();
        from = end;
        let bytes = lower.as_bytes();
        // The slash before `parent`, and before it a drive (`C:`) or the start of a path.
        let Some(slash) = start.checked_sub(1).filter(|&slash| is_slash(bytes[slash])) else {
            continue;
        };
        let after_drive = slash >= 2 && bytes[slash - 1] == b':' && bytes[slash - 2].is_ascii_alphabetic();
        let starts_path = slash == 0 || !(bytes[slash - 1].is_ascii_alphanumeric() || matches!(bytes[slash - 1], b'.' | b'_' | b'-'));
        if !(after_drive || starts_path) || !bytes.get(end).copied().is_some_and(is_slash) {
            continue;
        }
        let name = end + 1;
        let name_end = message[name..]
            .find(|c: char| matches!(c, '\\' | '/' | '"' | '\'' | ':' | '\n' | '\r'))
            .map_or(message.len(), |at| name + at);
        if name_end == name {
            continue;
        }
        let prefix = if after_drive { slash - 2 } else { slash };
        redacted.push_str(&message[done..prefix]);
        redacted.push_str(PROFILE_REDACTED);
        done = name_end;
        from = name_end;
    }
    redacted.push_str(&message[done..]);
    redacted
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
