//! Unreadable Database: the database could not be opened, and the app runs
//! without it. The reason and the file's path are all it has to tell; the
//! file itself is never changed, moved or deleted.

use std::fmt;
use std::path::PathBuf;

use serde::Serialize;

use crate::log::Level;
use crate::{Error, Log, Ports};

/// Why the database could not be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnreadableReason {
    /// A newer version of the app wrote it; this version does not change it.
    NewerVersion,
    /// The file is damaged or is not a SQLite database.
    Damaged,
    /// Its folder cannot be created, or the database cannot be created or written there.
    FolderNotWritable,
}

/// A database that could not be opened: why, where, and what failed, in
/// words for the Log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableDatabase {
    pub reason: UnreadableReason,
    /// The database file.
    pub path: PathBuf,
    /// What failed, for the Log and a developer; the interface goes by [`Self::reason`].
    pub detail: String,
}

impl UnreadableDatabase {
    pub(crate) fn new(reason: UnreadableReason, path: PathBuf, detail: impl Into<String>) -> Self {
        Self { reason, path, detail: detail.into() }
    }
}

impl fmt::Display for UnreadableDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the database {} cannot be opened: {}", self.path.display(), self.detail)
    }
}

impl std::error::Error for UnreadableDatabase {}

impl From<UnreadableDatabase> for Error {
    fn from(unreadable: UnreadableDatabase) -> Self {
        Self::new(unreadable.to_string())
    }
}

/// Whether the store is open, as the interface asks at start to choose
/// between the Library and the page of the Unreadable Database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Database {
    Open,
    Unreadable {
        reason: UnreadableReason,
        /// The database file, in full.
        path: String,
    },
}

/// The app in the Unreadable Database state: it can only say why, show the
/// database's folder, and quit. It holds no store, so nothing here reads or
/// writes the database.
pub struct UnreadableCore {
    ports: Ports,
    database: UnreadableDatabase,
}

impl UnreadableCore {
    /// Writes the failure to `log` (or does without, when the Log cannot be
    /// written) and keeps what the interface needs to tell.
    pub fn new(ports: Ports, database: UnreadableDatabase, log: Log) -> Self {
        log.write(ports.clock.now(), Level::Error, &format!("open database: {database}"));
        Self { ports, database }
    }

    /// The reason and the path of the database that could not be opened.
    pub fn database(&self) -> Database {
        Database::Unreadable { reason: self.database.reason, path: self.database.path.display().to_string() }
    }

    /// Shows the database's folder in the file manager.
    pub fn open_database_folder(&self) -> Result<(), Error> {
        let folder = self
            .database
            .path
            .parent()
            .ok_or_else(|| Error::new(format!("{} lies in no folder", self.database.path.display())))?;
        self.ports
            .opener
            .open_folder(folder)
            .map_err(|error| Error::new(format!("the database folder could not be opened: {error}")))
    }
}
