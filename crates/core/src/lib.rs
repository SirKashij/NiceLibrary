//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

mod covers;
pub mod keyvalues;
mod steam;
mod store;

use std::fmt;

use serde::{Serialize, Serializer};

pub use covers::{Cover, CoverCache, Downloader};
pub use steam::{SteamAccount, SteamInstallation, SteamLocator};
pub use store::Store;

/// The origin a Game is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Steam,
    Local,
}

/// One playable title in the Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// Unique across Sources: `steam:<app id>` for a Steam Game.
    pub id: String,
    pub title: String,
    pub source: Source,
    pub installed: bool,
}

impl Game {
    /// The Steam Game with Steam's app id `app_id`.
    pub(crate) fn steam(app_id: u32, title: String, installed: bool) -> Self {
        Self { id: format!("steam:{app_id}"), title, source: Source::Steam, installed }
    }
}

/// Why an operation failed. The interface receives the message as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
}

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(format!("database: {error}"))
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.message)
    }
}

/// Everything that varies by machine or OS, handed to the core.
pub struct Ports {
    pub steam_locator: Box<dyn SteamLocator>,
    pub downloader: Box<dyn Downloader>,
}

/// The application core. Each public method is one operation.
pub struct Core {
    ports: Ports,
    store: Store,
    covers: CoverCache,
}

impl Core {
    pub fn new(ports: Ports, store: Store, covers: CoverCache) -> Self {
        Self { ports, store, covers }
    }

    /// All Games in the Library, as last stored, ordered by title. Answers
    /// from the store alone, so a start shows the Library at once.
    pub fn library(&self) -> Result<Vec<Game>, Error> {
        let mut games = match self.store.steam_account()? {
            Some(account) => self.store.steam_games(&account)?,
            None => Vec::new(),
        };
        games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
        Ok(games)
    }

    /// Reads which Steam Games are Installed from Steam's library folders,
    /// stores that for the current Steam Account and answers the Library.
    ///
    /// When Steam, its Steam Account or its list of library folders cannot be
    /// found or read, the stored Library is left as it is.
    pub fn read_installed_games(&self) -> Result<Vec<Game>, Error> {
        if let Some(folder) = self.ports.steam_locator.steam_folder()
            && let Some(account) = steam::current_account(&folder)
            && let Some(apps) = steam::installed_apps(&folder)
        {
            self.store.record_installed_steam_games(&account.steam_id, &apps)?;
        }
        self.library()
    }

    /// The Steam installation on this computer and its Steam Accounts.
    pub fn steam_installation(&self) -> SteamInstallation {
        steam::read_installation(self.ports.steam_locator.as_ref())
    }

    /// The Cover of the Game `game_id`; `None` when it has none and the
    /// interface shows a placeholder instead.
    pub fn cover(&self, game_id: &str) -> Result<Option<Cover>, Error> {
        let Some(app_id) = game_id.strip_prefix("steam:").and_then(|id| id.parse().ok()) else {
            return Ok(None);
        };
        self.covers.steam_cover(self.ports.downloader.as_ref(), app_id)
    }
}
