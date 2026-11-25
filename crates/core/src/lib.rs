//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

mod covers;
pub mod keyvalues;
mod launch;
mod steam;
mod store;

use std::fmt;

use serde::{Serialize, Serializer};

pub use covers::{Cover, CoverCache, Downloader};
pub use launch::{Launch, OpenError, Opener};
pub use steam::{SteamAccount, SteamAccountChoice, SteamInstallation, SteamLocator};
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
    pub opener: Box<dyn Opener>,
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
        // Until the user chooses, the account read last: it holds the Games.
        let account = self.chosen_steam_account_or(|| self.store.last_read_steam_account())?;
        let mut games = match account {
            Some(account) => self.store.steam_games(&account)?,
            None => Vec::new(),
        };
        games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
        Ok(games)
    }

    /// Reads which Steam Games are Installed from Steam's library folders,
    /// stores that for the Steam Account the user chose, or until they choose
    /// for the most recently used one, and answers the Library.
    ///
    /// When Steam, a Steam Account or its list of library folders cannot be
    /// found or read, the stored Library is left as it is.
    pub fn read_installed_games(&self) -> Result<Vec<Game>, Error> {
        if let Some(folder) = self.ports.steam_locator.steam_folder()
            && let Some(account) = self.chosen_steam_account_or(|| Ok(most_recent_account(&folder)))?
            && let Some(apps) = steam::installed_apps(&folder)
        {
            self.store.record_installed_steam_games(&account, &apps)?;
        }
        self.library()
    }

    /// The SteamID of the Steam Account the app works with: the one the user
    /// chose; until then the one that logged in most recently on this
    /// computer, or else the one whose Games were read last.
    pub fn steam_account(&self) -> Result<Option<String>, Error> {
        self.chosen_steam_account_or(|| {
            match self.ports.steam_locator.steam_folder().and_then(|folder| most_recent_account(&folder)) {
                Some(detected) => Ok(Some(detected)),
                None => self.store.last_read_steam_account(),
            }
        })
    }

    /// The Steam Account the user chose; until they choose, the one `default`
    /// answers.
    fn chosen_steam_account_or(
        &self,
        default: impl FnOnce() -> Result<Option<String>, Error>,
    ) -> Result<Option<String>, Error> {
        match self.store.chosen_steam_account()? {
            Some(chosen) => Ok(Some(chosen)),
            None => default(),
        }
    }

    /// Makes `steam_id` the Steam Account the app works with, from now on and
    /// after every restart: one of the accounts on this computer, or a
    /// SteamID64 the user entered by hand. Anything that is not a SteamID64
    /// of a personal account is refused and changes nothing.
    pub fn choose_steam_account(&self, steam_id: &str) -> Result<SteamAccountChoice, Error> {
        let Some(steam_id) = steam::personal_steam_id(steam_id) else {
            return Ok(SteamAccountChoice::InvalidSteamId);
        };
        self.store.choose_steam_account(steam_id)?;
        Ok(SteamAccountChoice::Chosen)
    }

    /// The Steam installation on this computer and its Steam Accounts.
    pub fn steam_installation(&self) -> SteamInstallation {
        steam::read_installation(self.ports.steam_locator.as_ref())
    }

    /// The Cover of the Game `game_id`; `None` when it has none and the
    /// interface shows a placeholder instead.
    pub fn cover(&self, game_id: &str) -> Result<Option<Cover>, Error> {
        let Some(app_id) = steam_app_id(game_id) else {
            return Ok(None);
        };
        self.covers.steam_cover(self.ports.downloader.as_ref(), app_id)
    }

    /// Launches the Game `game_id`. A Steam Game is launched through Steam, by
    /// opening its `steam://rungameid/` URL; the app never starts a Steam
    /// Game's executable itself (ADR-0002).
    pub fn launch(&self, game_id: &str) -> Result<Launch, Error> {
        let Some(game) = self.library()?.into_iter().find(|game| game.id == game_id) else {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
        };
        if !game.installed {
            return Err(Error::new(format!("not installed: {}", game.title)));
        }
        let Some(app_id) = steam_app_id(&game.id) else {
            return Err(Error::new(format!("only Steam Games can be launched so far: {}", game.title)));
        };
        if self.ports.steam_locator.steam_folder().is_none() {
            return Ok(Launch::SteamNotFound);
        }
        match self.ports.opener.open_url(&launch::steam_launch_url(app_id)) {
            Ok(()) => Ok(Launch::Launched),
            Err(OpenError::NoProgram) => Ok(Launch::SteamNotFound),
            Err(error) => Err(Error::new(format!("{} could not be launched: {error}", game.title))),
        }
    }
}

/// The SteamID of the account that logged in most recently to the Steam
/// installation in `folder`.
fn most_recent_account(folder: &std::path::Path) -> Option<String> {
    steam::current_account(folder).map(|account| account.steam_id)
}

/// Steam's app id in a Steam Game's id (`steam:<app id>`).
fn steam_app_id(game_id: &str) -> Option<u32> {
    game_id.strip_prefix("steam:")?.parse().ok()
}
