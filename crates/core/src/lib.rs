//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

mod clock;
mod covers;
pub mod keyvalues;
mod launch;
mod library_filter;
mod steam;
mod steam_web_api;
mod store;
mod sync;
mod theme;

use std::fmt;
use std::ops::ControlFlow;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize, Serializer};

pub use clock::{Clock, Wakeup};
pub use covers::{Cover, CoverCache, Downloader};
pub use launch::{Launch, OpenError, Opener};
pub use library_filter::LibraryFilter;
pub use steam::{SteamAccount, SteamAccountChoice, SteamInstallation, SteamLocator};
pub use steam_web_api::{SecretStore, SteamWebApiKeyCheck, WebAnswer, WebApi};
pub use store::Store;
pub use sync::{AutomaticSync, SyncOutcome};
pub use theme::Theme;

/// The origin a Game is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Playtime in minutes; for a Steam Game Steam's figure. `None` while
    /// it is not known, before the first Sync.
    pub playtime_minutes: Option<u32>,
    /// Last Played, in seconds since the Unix epoch; `None` for never or
    /// not known.
    pub last_played: Option<u64>,
}

impl Game {
    /// The Steam Game with Steam's app id `app_id`, without Playtime and
    /// Last Played.
    pub(crate) fn steam(app_id: u32, title: String, installed: bool) -> Self {
        Self {
            id: format!("steam:{app_id}"),
            title,
            source: Source::Steam,
            installed,
            playtime_minutes: None,
            last_played: None,
        }
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
    pub secrets: Box<dyn SecretStore>,
    pub web_api: Box<dyn WebApi>,
    pub clock: Box<dyn Clock>,
}

/// The application core. Each public method is one operation.
pub struct Core {
    ports: Ports,
    store: Store,
    covers: CoverCache,
    /// Held for the whole of a Sync, so two never run at the same time.
    syncing: Mutex<()>,
    /// Rung when a key is stored, so the automatic Syncs do not wait for their next round.
    sync_soon: Wakeup,
}

impl Core {
    pub fn new(ports: Ports, store: Store, covers: CoverCache) -> Self {
        Self { ports, store, covers, syncing: Mutex::new(()), sync_soon: Wakeup::new() }
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

    /// The Games in the Library that match `filter`, as last stored, ordered
    /// by title like [`Self::library`].
    pub fn filter_library(&self, filter: &LibraryFilter) -> Result<Vec<Game>, Error> {
        let mut games = self.library()?;
        games.retain(|game| filter.matches(game));
        Ok(games)
    }

    /// The Game `game_id` as the Library shows it, for its detail page;
    /// `None` when it is not in the Library, for instance after a Sync
    /// took it out.
    pub fn game(&self, game_id: &str) -> Result<Option<Game>, Error> {
        Ok(self.library()?.into_iter().find(|game| game.id == game_id))
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

    /// Syncs: fetches every Steam Game the Steam Account owns, Installed or
    /// not, with Steam's Playtime and Last Played, and merges it into the
    /// store. A Steam Game the account no longer owns leaves the Library
    /// unless it is Installed.
    ///
    /// Every outcome but `Synced`, and every error, leaves the stored data as
    /// it was. A Sync asked for while another runs begins once that one ends.
    pub fn sync(&self) -> Result<SyncOutcome, Error> {
        let _syncing = self.wait_until_no_sync_runs();
        self.sync_now()
    }

    /// Blocks until no other Sync runs; no other begins until the answer is dropped.
    fn wait_until_no_sync_runs(&self) -> MutexGuard<'_, ()> {
        // Nothing a panicking Sync left half done reaches the store: the guard holds no data.
        self.syncing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One Sync; the caller holds [`Self::wait_until_no_sync_runs`].
    fn sync_now(&self) -> Result<SyncOutcome, Error> {
        let Some(key) = self.ports.secrets.read(steam_web_api::KEY_SECRET).map_err(secret_store_error)? else {
            return Ok(SyncOutcome::NoKey);
        };
        // Only a SteamID64 may reach the URL.
        let Some(account) = self.steam_account()?.filter(|id| steam::personal_steam_id(id).is_some()) else {
            return Ok(SyncOutcome::NoSteamAccount);
        };
        let games = match sync::owned_games(self.ports.web_api.as_ref(), &key, &account) {
            Ok(games) => games,
            Err(failed) => return Ok(failed),
        };
        let at = clock::seconds_since_epoch(self.ports.clock.now());
        self.store.record_steam_sync(&account, &games, at)?;
        Ok(SyncOutcome::Synced { at })
    }

    /// Syncs on its own while the app runs: shortly after the start, then
    /// every 30 minutes, each time as long after the previous Sync ended,
    /// waiting on the clock. A round without a stored Steam Web API key
    /// starts no Sync. Storing a key that Steam accepts starts the next
    /// round right away, or once the Sync that runs has ended. `report`
    /// hears what happens in each round as it happens; the Syncs go on until
    /// it answers `Break`, otherwise for good.
    pub fn sync_automatically(&self, mut report: impl FnMut(AutomaticSync) -> ControlFlow<()>) {
        self.ports.clock.sleep(sync::FIRST_AUTOMATIC_SYNC_AFTER, &self.sync_soon);
        loop {
            if self.automatic_sync_round(&mut report).is_break() {
                return;
            }
            self.ports.clock.sleep(sync::AUTOMATIC_SYNC_EVERY, &self.sync_soon);
        }
    }

    fn automatic_sync_round(&self, report: &mut impl FnMut(AutomaticSync) -> ControlFlow<()>) -> ControlFlow<()> {
        match self.steam_web_api_key_stored() {
            Ok(false) => return report(AutomaticSync::Skipped),
            Ok(true) => {}
            Err(reason) => return report(AutomaticSync::Failed { reason }),
        }
        // Started only once a Sync begun elsewhere has ended, so the interface shows the one that runs.
        let _syncing = self.wait_until_no_sync_runs();
        report(AutomaticSync::Started)?;
        report(match self.sync_now() {
            Ok(outcome) => AutomaticSync::Finished(outcome),
            Err(reason) => AutomaticSync::Failed { reason },
        })
    }

    /// When the last successful Sync of the Steam Account whose Library is
    /// shown happened, in seconds since the Unix epoch; `None` before the
    /// first.
    pub fn last_sync(&self) -> Result<Option<u64>, Error> {
        match self.chosen_steam_account_or(|| self.store.last_read_steam_account())? {
            Some(account) => self.store.last_steam_sync(&account),
            None => Ok(None),
        }
    }

    /// Whether a Steam Web API key is stored. The key itself never leaves
    /// the core.
    pub fn steam_web_api_key_stored(&self) -> Result<bool, Error> {
        let stored = self.ports.secrets.read(steam_web_api::KEY_SECRET).map_err(secret_store_error)?;
        Ok(stored.is_some())
    }

    /// Checks the Steam Web API key the user pasted with Steam and, when
    /// Steam accepts it, stores it in the operating system's secret store in
    /// place of any earlier one; the automatic Syncs then sync right away
    /// (see [`Self::sync_automatically`]). A key that is refused or cannot
    /// be checked changes nothing.
    pub fn store_steam_web_api_key(&self, key: &str) -> Result<SteamWebApiKeyCheck, Error> {
        let Some(key) = steam_web_api::key_in(key) else {
            return Ok(SteamWebApiKeyCheck::Invalid);
        };
        let check = steam_web_api::check_key(self.ports.web_api.as_ref(), &key);
        if check == SteamWebApiKeyCheck::Stored {
            self.ports.secrets.write(steam_web_api::KEY_SECRET, &key).map_err(secret_store_error)?;
            self.sync_soon.ring();
        }
        Ok(check)
    }

    /// Removes the stored Steam Web API key, if there is one.
    pub fn remove_steam_web_api_key(&self) -> Result<(), Error> {
        self.ports.secrets.delete(steam_web_api::KEY_SECRET).map_err(secret_store_error)
    }

    /// Opens the Steam page where the user creates their Steam Web API key,
    /// in the browser.
    pub fn open_steam_web_api_key_page(&self) -> Result<(), Error> {
        self.ports
            .opener
            .open_url(steam_web_api::KEY_PAGE)
            .map_err(|error| Error::new(format!("{} could not be opened: {error}", steam_web_api::KEY_PAGE)))
    }

    /// The theme the user chose; until they choose, [`Theme::System`]: the
    /// interface follows the operating system's dark or light setting.
    pub fn theme(&self) -> Result<Theme, Error> {
        self.store.theme()
    }

    /// Makes `theme` the theme of the interface, from now on and after every
    /// restart.
    pub fn choose_theme(&self, theme: Theme) -> Result<(), Error> {
        self.store.choose_theme(theme)
    }
}

/// The secret store's reason, which never contains the secret (see [`SecretStore`]).
fn secret_store_error(reason: String) -> Error {
    Error::new(format!("secret store: {reason}"))
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
