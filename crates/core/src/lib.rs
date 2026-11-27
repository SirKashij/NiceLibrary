//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

mod about;
mod clock;
mod collections;
mod covers;
pub mod keyvalues;
mod launch;
mod local_games;
mod library_filter;
mod status;
mod steam;
mod steam_app_list;
mod steam_web_api;
mod store;
mod sync;
mod theme;
mod window;

use std::fmt;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize, Serializer};

pub use about::{About, AboutPage};
pub use clock::{Clock, Wakeup};
pub use collections::{Collection, CollectionNaming};
pub use covers::{Cover, CoverCache, Downloader};
pub use launch::{Launch, OpenError, Opener};
pub use library_filter::{LibraryFilter, LibrarySort};
pub use local_games::{AddLocalGame, ExecutableChoice, ExecutablePicker};
pub use status::Status;
pub use steam::{SteamAccount, SteamAccountChoice, SteamInstallation, SteamLocator};
pub use steam_app_list::{AutomaticSteamAppListFetch, SteamAppList, SteamAppListFetch, SteamTitle};
pub use steam_web_api::{SecretStore, SteamWebApiKeyCheck, WebAnswer, WebApi};
pub use store::Store;
pub use sync::{AutomaticSync, SyncOutcome};
pub use theme::Theme;
pub use window::WindowClose;

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
    /// Unique across Sources: `steam:<app id>` for a Steam Game,
    /// `local:<number>` for a Local Game.
    pub id: String,
    pub title: String,
    pub source: Source,
    pub installed: bool,
    /// Playtime in minutes; for a Steam Game Steam's figure, for a Local
    /// Game the sum of its Sessions. `None` while it is not known, before
    /// the first Sync.
    pub playtime_minutes: Option<u32>,
    /// Last Played, in seconds since the Unix epoch; `None` for never or
    /// not known.
    pub last_played: Option<u64>,
    /// The person's own judgement of where they stand with the Game;
    /// Unplayed until they set it. No Sync changes it.
    pub status: Status,
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
            status: Status::Unplayed,
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
    pub executable_picker: Box<dyn ExecutablePicker>,
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
    /// Held for the whole of a fetch of the Steam App List, so two never run at the same time.
    fetching_steam_app_list: Mutex<()>,
    /// Rung when a key is stored, so the automatic fetches of the Steam App List do not wait for their next round.
    steam_app_list_soon: Wakeup,
    /// The names lookups search, loaded from the store at the first lookup after a fetch.
    steam_title_index: Mutex<Option<Arc<steam_app_list::TitleIndex>>>,
}

impl Core {
    pub fn new(ports: Ports, store: Store, covers: CoverCache) -> Self {
        Self {
            ports,
            store,
            covers,
            syncing: Mutex::new(()),
            sync_soon: Wakeup::new(),
            fetching_steam_app_list: Mutex::new(()),
            steam_app_list_soon: Wakeup::new(),
            steam_title_index: Mutex::new(None),
        }
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
        games.extend(self.store.local_games()?.into_iter().map(|game| Game::local(game.id, game.title)));
        let statuses = self.store.statuses()?;
        for game in &mut games {
            game.status = statuses.get(&game.id).copied().unwrap_or_default();
        }
        games.sort_by(library_filter::by_title);
        Ok(games)
    }

    /// The Games in the Library that match `filter`, as last stored, in the
    /// order `filter` asks for.
    pub fn filter_library(&self, filter: &LibraryFilter) -> Result<Vec<Game>, Error> {
        let mut games = self.library()?;
        games.retain(|game| filter.matches(game));
        filter.sort.sort(&mut games);
        Ok(games)
    }

    /// The order the user chose for the Library; recently played until they
    /// choose.
    pub fn library_sort(&self) -> Result<LibrarySort, Error> {
        Ok(self.store.library_sort()?.and_then(|name| LibrarySort::from_stored_name(&name)).unwrap_or_default())
    }

    /// Makes `sort` the order of the Library, from now on and after every
    /// restart.
    pub fn choose_library_sort(&self, sort: LibrarySort) -> Result<(), Error> {
        self.store.choose_library_sort(sort.stored_name())
    }

    /// The Game `game_id` as the Library shows it, for its detail page;
    /// `None` when it is not in the Library, for instance after a Sync
    /// took it out.
    pub fn game(&self, game_id: &str) -> Result<Option<Game>, Error> {
        Ok(self.library()?.into_iter().find(|game| game.id == game_id))
    }

    /// Sets the Status of the Game `game_id` to `status`, from now on and
    /// after every restart, and answers the Game as the Library now shows
    /// it. Nothing else changes it, no Sync either. A Game that is not in
    /// the Library is refused.
    pub fn set_status(&self, game_id: &str, status: Status) -> Result<Game, Error> {
        let not_in_library = || Error::new(format!("no such game in the library: {game_id}"));
        if self.game(game_id)?.is_none() {
            return Err(not_in_library());
        }
        self.store.set_status(game_id, status)?;
        self.game(game_id)?.ok_or_else(not_in_library)
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
            self.steam_app_list_soon.ring();
        }
        Ok(check)
    }

    /// Removes the stored Steam Web API key, if there is one.
    pub fn remove_steam_web_api_key(&self) -> Result<(), Error> {
        self.ports.secrets.delete(steam_web_api::KEY_SECRET).map_err(secret_store_error)
    }

    /// Asks the user to choose the executable of a Local Game to add and
    /// proposes its title (see [`Self::add_local_game`]). An executable
    /// already in the Library is answered with the title it has there.
    pub fn choose_local_game_executable(&self) -> Result<ExecutableChoice, Error> {
        let chosen = self
            .ports
            .executable_picker
            .choose_executable()
            .map_err(|reason| Error::new(format!("file dialog: {reason}")))?;
        let Some(path) = chosen else {
            return Ok(ExecutableChoice::Cancelled);
        };
        let Some(executable) = path.to_str() else {
            return Err(Error::new(format!("not a path that can be stored: {}", path.display())));
        };
        if let Some(game) = local_games::already_added(&self.store.local_games()?, executable) {
            return Ok(ExecutableChoice::AlreadyAdded { title: game.title.clone() });
        }
        Ok(ExecutableChoice::Chosen { executable: executable.to_owned(), title: local_games::proposed_title(&path) })
    }

    /// Adds the executable at `executable` to the Library as a Local Game
    /// titled `title`, which the user may have corrected. The path is kept as
    /// given. An empty title, a path with no file, and an executable already
    /// in the Library by any spelling of its path are refused.
    pub fn add_local_game(&self, executable: &str, title: &str) -> Result<AddLocalGame, Error> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(AddLocalGame::NoTitle);
        }
        if !std::path::Path::new(executable).is_file() {
            return Ok(AddLocalGame::NotFound);
        }
        if let Some(game) = local_games::already_added(&self.store.local_games()?, executable) {
            return Ok(AddLocalGame::AlreadyAdded { title: game.title.clone() });
        }
        let id = self.store.add_local_game(title, executable)?;
        Ok(AddLocalGame::Added { game: Game::local(id, title.to_owned()) })
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

    /// Every Collection with how many Games of the Library are in it,
    /// ordered by name whatever its case.
    pub fn collections(&self) -> Result<Vec<Collection>, Error> {
        let library: std::collections::HashSet<String> = self.library()?.into_iter().map(|game| game.id).collect();
        let members = self.store.collection_games()?;
        let mut collections: Vec<Collection> = self
            .store
            .collections()?
            .into_iter()
            .map(|(id, name)| {
                let in_library = members.iter().filter(|(of, game)| *of == id && library.contains(game)).count();
                Collection { id, name, game_count: u32::try_from(in_library).unwrap_or(u32::MAX) }
            })
            .collect();
        collections.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
        Ok(collections)
    }

    /// Creates a Collection named `name`, without the spaces around it. An
    /// empty name, or one another Collection has in any case, is refused and
    /// creates nothing.
    pub fn create_collection(&self, name: &str) -> Result<CollectionNaming, Error> {
        match collections::name_in(name) {
            Some(name) => self.store.name_collection(None, name),
            None => Ok(CollectionNaming::EmptyName),
        }
    }

    /// Renames the Collection `id` to `name`, without the spaces around it,
    /// under the same rules as [`Self::create_collection`]. Fails when there
    /// is no such Collection.
    pub fn rename_collection(&self, id: i64, name: &str) -> Result<CollectionNaming, Error> {
        match collections::name_in(name) {
            Some(name) => self.store.name_collection(Some(id), name),
            None => Ok(CollectionNaming::EmptyName),
        }
    }

    /// Deletes the Collection `id`. Its Games stay in the Library. Deleting
    /// a Collection that is gone already changes nothing.
    pub fn delete_collection(&self, id: i64) -> Result<(), Error> {
        self.store.delete_collection(id)
    }

    /// Whether closing the window keeps the app running in the Tray, so
    /// tracking goes on. On until the user turns it off.
    pub fn close_to_tray(&self) -> Result<bool, Error> {
        window::close_to_tray(&self.store)
    }

    /// Turns close-to-tray on or off, from now on and after every restart.
    pub fn set_close_to_tray(&self, on: bool) -> Result<(), Error> {
        window::set_close_to_tray(&self.store, on)
    }

    /// What closing the window does: with close-to-tray on the app keeps
    /// running in the Tray, and the very first time the window explains this
    /// instead; with close-to-tray off the app quits.
    pub fn close_window(&self) -> Result<WindowClose, Error> {
        window::close_window(&self.store)
    }

    /// What is stored of the Steam App List: how many titles, and when it
    /// was last fetched to its end.
    pub fn steam_app_list(&self) -> Result<SteamAppList, Error> {
        self.store.steam_app_list()
    }

    /// Fetches the Steam App List with the stored Steam Web API key, page by
    /// page, and stores each page as it arrives. A fetch that stopped before
    /// its end, in this run of the app or an earlier one, is resumed after
    /// its last stored page; otherwise the whole list is fetched anew, and
    /// the list stored before stays usable meanwhile. A fetch asked for while
    /// another runs begins once that one ends.
    pub fn fetch_steam_app_list(&self) -> Result<SteamAppListFetch, Error> {
        let _fetching = self.wait_until_no_steam_app_list_fetch_runs();
        self.fetch_steam_app_list_now()
    }

    fn wait_until_no_steam_app_list_fetch_runs(&self) -> MutexGuard<'_, ()> {
        // The guard holds no data; what a panicking fetch stored is whole pages.
        self.fetching_steam_app_list.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One fetch; the caller holds [`Self::wait_until_no_steam_app_list_fetch_runs`].
    fn fetch_steam_app_list_now(&self) -> Result<SteamAppListFetch, Error> {
        let Some(key) = self.ports.secrets.read(steam_web_api::KEY_SECRET).map_err(secret_store_error)? else {
            return Ok(SteamAppListFetch::NoKey);
        };
        let fetched = steam_app_list::fetch(self.ports.web_api.as_ref(), &self.store, &key, || {
            clock::seconds_since_epoch(self.ports.clock.now())
        });
        // Pages may be stored whatever the outcome. Loaded here, not at the next keystroke.
        let index = self.store.searchable_steam_titles().map(|titles| Arc::new(steam_app_list::TitleIndex::new(titles)));
        // Unloaded, the next lookup tries again and tells what failed.
        *self.steam_title_index.lock().unwrap_or_else(PoisonError::into_inner) = index.ok();
        fetched
    }

    /// Fetches the Steam App List on its own while the app runs: a minute
    /// after the start, then every hour, whenever it was never fetched to its
    /// end, a fetch stopped before its end, or the last one ended a week ago
    /// or longer. A round without a stored key fetches nothing; storing a key
    /// starts the next round right away. `report` hears each round as it
    /// happens; the rounds go on until it answers `Break`.
    ///
    /// Before the first round, the stored list is made ready for lookups, so
    /// the first one answers as fast as the others.
    pub fn fetch_steam_app_list_automatically(
        &self,
        mut report: impl FnMut(AutomaticSteamAppListFetch) -> ControlFlow<()>,
    ) {
        // A failure here shows at the first lookup, which tries again.
        let _ = self.steam_title_index();
        self.ports.clock.sleep(steam_app_list::FIRST_AUTOMATIC_FETCH_AFTER, &self.steam_app_list_soon);
        loop {
            if self.automatic_steam_app_list_round(&mut report).is_break() {
                return;
            }
            self.ports.clock.sleep(steam_app_list::AUTOMATIC_FETCH_CHECK_EVERY, &self.steam_app_list_soon);
        }
    }

    fn automatic_steam_app_list_round(
        &self,
        report: &mut impl FnMut(AutomaticSteamAppListFetch) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        match self.steam_web_api_key_stored() {
            Ok(false) => return report(AutomaticSteamAppListFetch::Skipped),
            Ok(true) => {}
            Err(reason) => return report(AutomaticSteamAppListFetch::Failed { reason }),
        }
        let _fetching = self.wait_until_no_steam_app_list_fetch_runs();
        let now = clock::seconds_since_epoch(self.ports.clock.now());
        match self.store.steam_app_list() {
            Ok(list) if !steam_app_list::due(&list, now) => return report(AutomaticSteamAppListFetch::Skipped),
            Ok(_) => {}
            Err(reason) => return report(AutomaticSteamAppListFetch::Failed { reason }),
        }
        report(AutomaticSteamAppListFetch::Started)?;
        report(match self.fetch_steam_app_list_now() {
            Ok(outcome) => AutomaticSteamAppListFetch::Finished(outcome),
            Err(reason) => AutomaticSteamAppListFetch::Failed { reason },
        })
    }

    /// Up to twenty titles of the stored Steam App List whose name contains
    /// `typed`, whatever its case and punctuation: the same name first, then
    /// names that begin with it, then names with a word that does, then the
    /// rest. Answers from the store alone, fast enough to run as the user
    /// types.
    pub fn find_steam_titles(&self, typed: &str) -> Result<Vec<SteamTitle>, Error> {
        let search = steam_app_list::searchable(typed);
        if search.is_empty() {
            return Ok(Vec::new());
        }
        let app_ids = self.steam_title_index()?.find(&search, steam_app_list::LOOKUP_LIMIT);
        self.store.steam_titles(&app_ids)
    }

    /// The names lookups search, loaded from the store unless already held.
    fn steam_title_index(&self) -> Result<Arc<steam_app_list::TitleIndex>, Error> {
        let mut held = self.steam_title_index.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = &*held {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(steam_app_list::TitleIndex::new(self.store.searchable_steam_titles()?));
        *held = Some(Arc::clone(&index));
        Ok(index)
    }

    /// The version of the app that runs and its license, for the About dialog.
    pub fn about(&self) -> About {
        about::about()
    }

    /// Opens `page`, the project page or the license, in the browser.
    pub fn open_about_page(&self, page: AboutPage) -> Result<(), Error> {
        let url = page.url();
        self.ports.opener.open_url(url).map_err(|error| Error::new(format!("{url} could not be opened: {error}")))
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
