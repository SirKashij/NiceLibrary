//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

mod about;
mod achievements;
mod autostart;
mod clock;
mod collections;
mod cover_hue;
mod cover_match;
mod covers;
mod executables_in_folder;
mod export;
mod favorites;
mod import;
pub mod keyvalues;
mod language;
mod launch;
mod local_covers;
mod limited_mode;
mod local_games;
mod library_filter;
mod progress;
mod library_layout;
mod log;
mod playtime_adjustment;
mod sessions;
mod statistics;
mod status;
mod steam;
mod steam_app_list;
mod steam_web_api;
mod store;
mod sync;
mod theme;
mod window;

use std::collections::BTreeSet;
use std::fmt;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize, Serializer};

use log::Level;
use sessions::Tracked;

pub use about::{About, AboutPage};
pub use achievements::{Achievement, GameAchievements};
pub use autostart::{AUTOSTART_ARGUMENT, Autostart, started_with_windows};
pub use clock::{Clock, Wakeup};
pub use collections::{Collection, CollectionNaming};
pub use cover_hue::CoverHue;
pub use covers::{Cover, CoverCache, Downloader};
pub use executables_in_folder::{FolderSuggestions, SuggestedExecutable};
pub use export::{DataExport, ExportFilePicker};
pub use import::DataImport;
pub use language::Language;
pub use launch::{Install, Launch, LaunchOptions, OpenError, Opener, ProcessStarter};
pub use limited_mode::LimitedMode;
pub use library_filter::{LibraryFilter, LibrarySort};
pub use local_covers::{CoverImageChoice, Icon, IconExtractor, ImagePicker};
pub use local_games::{AddLocalGame, EditLocalGame, ExecutableChoice, ExecutablePicker, LocalGame};
pub use progress::ManualProgress;
pub use log::Log;
pub use sessions::{ProcessList, Session, Tracking};
pub use statistics::{GamesByStatus, PlayedWeek, Statistics};
pub use status::Status;
pub use library_layout::LibraryLayout;
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
    /// Progress in percent, from 0 to 100, as its Source works it out (see
    /// `progress`): for a Local Game its Manual Progress, for a Steam Game
    /// the share of its unlocked Achievements. `None` when the Game has none,
    /// such as a Steam Game without Achievements.
    pub progress: Option<u8>,
    /// The person's own judgement of where they stand with the Game;
    /// Unplayed until they set it. No Sync changes it.
    pub status: Status,
    /// Whether the user marked the Game as a Favorite.
    pub favorite: bool,
}

impl Game {
    /// The Steam Game with Steam's app id `app_id`, without Playtime, Last
    /// Played and Progress.
    pub(crate) fn steam(app_id: u32, title: String, installed: bool) -> Self {
        Self {
            id: format!("steam:{app_id}"),
            title,
            source: Source::Steam,
            installed,
            playtime_minutes: None,
            last_played: None,
            progress: None,
            status: Status::Unplayed,
            favorite: false,
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
    pub autostart: Box<dyn Autostart>,
    pub executable_picker: Box<dyn ExecutablePicker>,
    pub processes: Box<dyn ProcessList>,
    pub process_starter: Box<dyn ProcessStarter>,
    pub icon_extractor: Box<dyn IconExtractor>,
    pub image_picker: Box<dyn ImagePicker>,
    pub export_file_picker: Box<dyn ExportFilePicker>,
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
    /// Where errors and notable events are written.
    log: Log,
    /// The stored Sessions whose process the tracking found at its last
    /// look, by their id; empty while the tracking does not run.
    running_sessions: Mutex<std::collections::BTreeSet<(Tracked, i64)>>,
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
            log: Log::none(),
            running_sessions: Mutex::default(),
        }
    }

    /// This core, writing errors and notable events to `log`; without it,
    /// the core keeps no Log.
    pub fn with_log(mut self, log: Log) -> Self {
        self.log = log;
        self
    }

    /// Writes the error of `operation` to the Log, if it failed, and answers `result`.
    fn logged<T>(&self, operation: &str, result: Result<T, Error>) -> Result<T, Error> {
        if let Err(error) = &result {
            self.log.write(self.ports.clock.now(), Level::Error, &format!("{operation}: {error}"));
        }
        result
    }

    /// Like [`Self::logged`], and writes the outcome as a notable event when nothing failed.
    fn logged_outcome<T: Serialize>(&self, operation: &str, result: Result<T, Error>) -> Result<T, Error> {
        if let Ok(outcome) = &result {
            self.note(&format!("{operation}: {}", outcome_name(outcome)));
        }
        self.logged(operation, result)
    }

    /// Writes a notable event to the Log.
    fn note(&self, event: &str) {
        self.log.write(self.ports.clock.now(), Level::Info, event);
    }

    /// Shows the folder of the Log in the file manager, so the user can
    /// attach a log file to a bug report themselves. Nothing in the Log is
    /// ever sent by the app.
    pub fn open_log_folder(&self) -> Result<(), Error> {
        let opened = self.log.folder().and_then(|folder| {
            self.ports
                .opener
                .open_folder(folder)
                .map_err(|error| Error::new(format!("the log folder could not be opened: {error}")))
        });
        self.logged("open_log_folder", opened)
    }

    /// All Games in the Library, as last stored, ordered by title; a Hidden
    /// Game is not among them. Answers from the store alone, so a start
    /// shows the Library at once.
    pub fn library(&self) -> Result<Vec<Game>, Error> {
        self.logged("library", self.stored_library())
    }

    /// [`Self::library`], without writing to the Log. Every view built on
    /// the Library (the filtered Library, a Game's detail page, setting its
    /// Status, its Launch) leaves the Hidden Games out through it.
    fn stored_library(&self) -> Result<Vec<Game>, Error> {
        let hidden = self.store.hidden_game_ids()?;
        let mut games = self.every_game()?;
        games.retain(|game| !hidden.contains(&game.id));
        Ok(games)
    }

    /// Every Game of both Sources, Hidden Games too, ordered by title.
    fn every_game(&self) -> Result<Vec<Game>, Error> {
        let mut games = match self.library_steam_account()? {
            Some(account) => {
                let mut games = self.store.steam_games(&account)?;
                let achievements = self.store.achievement_counts(&account)?;
                for game in &mut games {
                    game.progress = steam_app_id(&game.id)
                        .and_then(|app_id| achievements.get(&app_id))
                        .and_then(|&(unlocked, total)| progress::of_steam_game(unlocked, total));
                }
                games
            }
            None => Vec::new(),
        };
        let played = self.store.local_games_played()?;
        let adjustments = self.store.playtime_adjustments()?;
        games.extend(self.store.local_games()?.into_iter().map(|game| {
            let id = game.id;
            let unplayed = Game::from_stored_local(game);
            let played = match played.get(&id) {
                Some(played) => played.into_game(unplayed),
                None => unplayed,
            };
            match adjustments.get(&id) {
                Some(&minutes) => playtime_adjustment::adjusted(played, minutes),
                None => played,
            }
        }));
        let statuses = self.store.statuses()?;
        for game in &mut games {
            game.status = statuses.get(&game.id).copied().unwrap_or_default();
        }
        favorites::mark(&mut games, &self.store.favorites()?);
        games.sort_by(library_filter::by_title);
        Ok(games)
    }

    /// The Steam Account whose Steam Games the Library holds: the chosen
    /// one, and until the user chooses, the account read last.
    fn library_steam_account(&self) -> Result<Option<String>, Error> {
        self.chosen_steam_account_or(|| self.store.last_read_steam_account())
    }

    /// The Games in the Library that match `filter`, as last stored, in the
    /// order `filter` asks for.
    pub fn filter_library(&self, filter: &LibraryFilter) -> Result<Vec<Game>, Error> {
        let games = self.stored_library().and_then(|mut games| {
            games.retain(|game| filter.matches(game));
            if let Some(collection) = filter.collection {
                let members = self.store.games_in_collection(collection)?;
                games.retain(|game| members.contains(&game.id));
            }
            filter.sort.sort(&mut games);
            Ok(games)
        });
        self.logged("filter_library", games)
    }

    /// The order the user chose for the Library; recently played until they
    /// choose.
    pub fn library_sort(&self) -> Result<LibrarySort, Error> {
        let sort = self.store.library_sort().map(|stored| {
            stored.and_then(|name| LibrarySort::from_stored_name(&name)).unwrap_or_default()
        });
        self.logged("library_sort", sort)
    }

    /// Makes `sort` the order of the Library, from now on and after every
    /// restart.
    pub fn choose_library_sort(&self, sort: LibrarySort) -> Result<(), Error> {
        self.logged("choose_library_sort", self.store.choose_library_sort(sort.stored_name()))
    }

    /// The Game `game_id` as the Library shows it, for its detail page;
    /// `None` when it is not in the Library, for instance after a Sync
    /// took it out.
    pub fn game(&self, game_id: &str) -> Result<Option<Game>, Error> {
        let game = self.stored_library().map(|games| games.into_iter().find(|game| game.id == game_id));
        self.logged("game", game)
    }

    /// Sets the Status of the Game `game_id` to `status`, from now on and
    /// after every restart, and answers the Game as the Library now shows
    /// it. Nothing else changes it, no Sync either. A Game that is not in
    /// the Library is refused.
    pub fn set_status(&self, game_id: &str, status: Status) -> Result<Game, Error> {
        self.logged("set_status", self.record_status(game_id, status))
    }

    fn record_status(&self, game_id: &str, status: Status) -> Result<Game, Error> {
        let not_in_library = || Error::new(format!("no such game in the library: {game_id}"));
        let in_library = |library: Vec<Game>| library.into_iter().find(|game| game.id == game_id);
        if in_library(self.stored_library()?).is_none() {
            return Err(not_in_library());
        }
        self.store.set_status(game_id, status)?;
        in_library(self.stored_library()?).ok_or_else(not_in_library)
    }

    /// Reads which Steam Games are Installed from Steam's library folders,
    /// stores that for the Steam Account the user chose, or until they choose
    /// for the most recently used one, and answers the Library.
    ///
    /// When Steam, a Steam Account or its list of library folders cannot be
    /// found or read, the stored Library is left as it is.
    pub fn read_installed_games(&self) -> Result<Vec<Game>, Error> {
        self.logged("read_installed_games", self.read_and_record_installed_games())
    }

    fn read_and_record_installed_games(&self) -> Result<Vec<Game>, Error> {
        if let Some(folder) = self.ports.steam_locator.steam_folder()
            && let Some(account) = self.chosen_steam_account_or(|| Ok(most_recent_account(&folder)))?
            && let Some(apps) = steam::installed_apps(&folder)
        {
            let local = steam::local_playtimes(&folder, &account);
            self.store.record_installed_steam_games(&account, &apps, &local)?;
        }
        self.stored_library()
    }

    /// The SteamID of the Steam Account the app works with: the one the user
    /// chose; until then the one that logged in most recently on this
    /// computer, or else the one whose Games were read last.
    pub fn steam_account(&self) -> Result<Option<String>, Error> {
        self.logged("steam_account", self.current_steam_account())
    }

    /// [`Self::steam_account`], without writing to the Log.
    fn current_steam_account(&self) -> Result<Option<String>, Error> {
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
        let choice = match steam::personal_steam_id(steam_id) {
            Some(steam_id) => self.store.choose_steam_account(steam_id).map(|()| SteamAccountChoice::Chosen),
            None => Ok(SteamAccountChoice::InvalidSteamId),
        };
        self.logged_outcome("choose_steam_account", choice)
    }

    /// The Steam installation on this computer and its Steam Accounts.
    pub fn steam_installation(&self) -> SteamInstallation {
        steam::read_installation(self.ports.steam_locator.as_ref())
    }

    /// The Cover of the Game `game_id`: a Steam Game's from Steam, a Local
    /// Game's the image the user chose, else the Steam Cover of its Cover
    /// Match (see [`Self::set_cover_match`]), else its executable's icon on a
    /// neutral background. `None` when it has none and the interface shows a
    /// placeholder instead.
    pub fn cover(&self, game_id: &str) -> Result<Option<Cover>, Error> {
        if let Some(app_id) = steam_app_id(game_id) {
            return self.covers.steam_cover(self.ports.downloader.as_ref(), app_id);
        }
        match self.stored_local_game(game_id)? {
            Some(game) => {
                let matched = self.store.cover_match(game.id)?.map(|title| title.app_id);
                local_covers::cover(
                    &self.covers,
                    self.ports.icon_extractor.as_ref(),
                    self.ports.downloader.as_ref(),
                    &game,
                    matched,
                )
            }
            None => Ok(None),
        }
    }

    /// Launches the Game `game_id`. A Steam Game is launched through Steam, by
    /// opening its `steam://rungameid/` URL; the app never starts a Steam
    /// Game's executable itself (ADR-0002). A Local Game is launched by
    /// starting its executable, not through a shell, with its Launch
    /// Arguments in its Working Directory, or else in the executable's
    /// folder (see [`Self::set_launch_options`]).
    pub fn launch(&self, game_id: &str) -> Result<Launch, Error> {
        self.logged_outcome("launch", self.launch_game(game_id))
    }

    fn launch_game(&self, game_id: &str) -> Result<Launch, Error> {
        if let Some(id) = local_game_id(game_id) {
            return match self.store.local_game(id)? {
                Some(game) => launch::launch_local_game(self.ports.process_starter.as_ref(), &game),
                None => Err(Error::new(format!("no such game in the library: {game_id}"))),
            };
        }
        let Some(game) = self.stored_library()?.into_iter().find(|game| game.id == game_id) else {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
        };
        if !game.installed {
            return Err(Error::new(format!("not installed: {}", game.title)));
        }
        let Some(app_id) = steam_app_id(&game.id) else {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
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

    /// Starts the installation of the Steam Game `game_id`, which is owned
    /// but not Installed, by opening its `steam://install/` URL: Steam shows
    /// its install dialog and installs the Game. The next reading of the
    /// Installed Games finds it once Steam is done.
    pub fn install(&self, game_id: &str) -> Result<Install, Error> {
        self.logged_outcome("install", self.install_game(game_id))
    }

    fn install_game(&self, game_id: &str) -> Result<Install, Error> {
        let Some(game) = self.stored_library()?.into_iter().find(|game| game.id == game_id) else {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
        };
        if game.installed {
            return Err(Error::new(format!("already installed: {}", game.title)));
        }
        let Some(app_id) = steam_app_id(&game.id) else {
            return Err(Error::new(format!("only Steam Games can be installed: {}", game.title)));
        };
        if self.ports.steam_locator.steam_folder().is_none() {
            return Ok(Install::SteamNotFound);
        }
        match self.ports.opener.open_url(&launch::steam_install_url(app_id)) {
            Ok(()) => Ok(Install::Started),
            Err(OpenError::NoProgram) => Ok(Install::SteamNotFound),
            Err(error) => Err(Error::new(format!("the installation of {} could not be started: {error}", game.title))),
        }
    }

    /// Syncs: fetches every Steam Game the Steam Account owns, Installed or
    /// not, with Steam's Playtime and Last Played, and merges it into the
    /// store. A Steam Game the account no longer owns leaves the Library
    /// unless it is Installed. Then fetches the Achievements of the Steam
    /// Games played since they were last fetched, or never fetched, the most
    /// recently played first, as many as this Sync's and this day's share of
    /// Steam's calls allows; the others follow with the next Syncs. When
    /// Steam does not give them, the stored Achievements stay as they were.
    ///
    /// Every outcome but `Synced`, and every error, leaves the stored data as
    /// it was. A Sync asked for while another runs begins once that one ends.
    pub fn sync(&self) -> Result<SyncOutcome, Error> {
        let _syncing = self.wait_until_no_sync_runs();
        self.logged_outcome("sync", self.sync_now())
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
        let Some(account) = self.current_steam_account()?.filter(|id| steam::personal_steam_id(id).is_some()) else {
            return Ok(SyncOutcome::NoSteamAccount);
        };
        let outcome = match sync::owned_games(self.ports.web_api.as_ref(), &key, &account) {
            Ok(games) => {
                let at = clock::seconds_since_epoch(self.ports.clock.now());
                self.store.record_steam_sync(&account, &games, at)?;
                // One call so far: the owned games.
                achievements::fetch(self.ports.web_api.as_ref(), &self.store, &key, &account, at, 1)?;
                SyncOutcome::Synced { at }
            }
            Err(failed) => failed,
        };
        if let Some(reason) = LimitedMode::after_sync(outcome) {
            self.store.record_limited_mode_reason(&account, reason.stored_name())?;
        }
        Ok(outcome)
    }

    /// Whether the app runs in Limited Mode, and why: without a stored Steam
    /// Web API key, or when the last Sync of the Steam Account the app works
    /// with found Steam refusing the key, its game details private, or Steam
    /// unreachable. A successful Sync, or storing a key Steam accepts, ends
    /// it.
    pub fn limited_mode(&self) -> Result<LimitedMode, Error> {
        self.logged("limited_mode", self.current_limited_mode())
    }

    fn current_limited_mode(&self) -> Result<LimitedMode, Error> {
        if !self.key_stored()? {
            return Ok(LimitedMode::NoKey);
        }
        let Some(account) = self.current_steam_account()? else {
            return Ok(LimitedMode::Off);
        };
        Ok(self.store.limited_mode_reason(&account)?.map_or(LimitedMode::Off, |name| LimitedMode::from_stored_name(&name)))
    }

    /// Opens the Steam page where the user makes their game details
    /// public, in the browser.
    pub fn open_steam_privacy_settings(&self) -> Result<(), Error> {
        let page = limited_mode::PRIVACY_SETTINGS_PAGE;
        let opened = self
            .ports
            .opener
            .open_url(page)
            .map_err(|error| Error::new(format!("{page} could not be opened: {error}")));
        self.logged("open_steam_privacy_settings", opened)
    }

    /// Syncs on its own while the app runs: shortly after the start, then
    /// every 30 minutes, each time as long after the previous Sync ended,
    /// waiting on the clock. A round without a stored Steam Web API key
    /// starts no Sync. Storing a key that Steam accepts, and the end of any
    /// Session (see [`Self::track_sessions`]), start the next round right
    /// away, or once the Sync that runs has ended. `report`
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
        match self.logged("automatic_sync", self.key_stored()) {
            Ok(false) => return report(AutomaticSync::Skipped),
            Ok(true) => {}
            Err(reason) => return report(AutomaticSync::Failed { reason }),
        }
        // Started only once a Sync begun elsewhere has ended, so the interface shows the one that runs.
        let _syncing = self.wait_until_no_sync_runs();
        report(AutomaticSync::Started)?;
        report(match self.logged_outcome("automatic_sync", self.sync_now()) {
            Ok(outcome) => AutomaticSync::Finished(outcome),
            Err(reason) => AutomaticSync::Failed { reason },
        })
    }

    /// When the last successful Sync of the Steam Account whose Library is
    /// shown happened, in seconds since the Unix epoch; `None` before the
    /// first.
    pub fn last_sync(&self) -> Result<Option<u64>, Error> {
        let last = self.chosen_steam_account_or(|| self.store.last_read_steam_account()).and_then(|account| {
            match account {
                Some(account) => self.store.last_steam_sync(&account),
                None => Ok(None),
            }
        });
        self.logged("last_sync", last)
    }

    /// Whether a Steam Web API key is stored. The key itself never leaves
    /// the core.
    pub fn steam_web_api_key_stored(&self) -> Result<bool, Error> {
        self.logged("steam_web_api_key_stored", self.key_stored())
    }

    /// [`Self::steam_web_api_key_stored`], without writing to the Log.
    fn key_stored(&self) -> Result<bool, Error> {
        let stored = self.ports.secrets.read(steam_web_api::KEY_SECRET).map_err(secret_store_error)?;
        Ok(stored.is_some())
    }

    /// Checks the Steam Web API key the user pasted with Steam and, when
    /// Steam accepts it, stores it in the operating system's secret store in
    /// place of any earlier one; the automatic Syncs then sync right away
    /// (see [`Self::sync_automatically`]). A key that is refused or cannot
    /// be checked changes nothing.
    pub fn store_steam_web_api_key(&self, key: &str) -> Result<SteamWebApiKeyCheck, Error> {
        self.logged_outcome("store_steam_web_api_key", self.check_and_store_key(key))
    }

    fn check_and_store_key(&self, key: &str) -> Result<SteamWebApiKeyCheck, Error> {
        let Some(key) = steam_web_api::key_in(key) else {
            return Ok(SteamWebApiKeyCheck::Invalid);
        };
        let check = steam_web_api::check_key(self.ports.web_api.as_ref(), &key);
        if check == SteamWebApiKeyCheck::Stored {
            self.ports.secrets.write(steam_web_api::KEY_SECRET, &key).map_err(secret_store_error)?;
            // Steam accepts this key; why an earlier Sync failed is for the next Sync to tell.
            self.store.forget_limited_mode_reasons()?;
            self.sync_soon.ring();
            self.steam_app_list_soon.ring();
        }
        Ok(check)
    }

    /// The Achievements of the Game `game_id` for the Steam Account whose
    /// Library is shown, as the last Syncs fetched them. Answers from the
    /// store alone.
    pub fn achievements(&self, game_id: &str) -> Result<GameAchievements, Error> {
        self.logged("achievements", self.stored_achievements(game_id))
    }

    fn stored_achievements(&self, game_id: &str) -> Result<GameAchievements, Error> {
        let Some(app_id) = steam_app_id(game_id) else {
            return Ok(GameAchievements::LocalGame);
        };
        let Some(account) = self.chosen_steam_account_or(|| self.store.last_read_steam_account())? else {
            return Ok(GameAchievements::NotFetchedYet);
        };
        Ok(match self.store.achievements(&account, app_id)? {
            None => GameAchievements::NotFetchedYet,
            Some(achievements) if achievements.is_empty() => GameAchievements::NoAchievements,
            Some(achievements) => GameAchievements::Listed {
                unlocked: u32::try_from(achievements.iter().filter(|a| a.unlocked).count()).unwrap_or(u32::MAX),
                total: u32::try_from(achievements.len()).unwrap_or(u32::MAX),
                achievements,
            },
        })
    }

    /// The image of the Achievement icon `icon` (see [`Achievement::icon`])
    /// of the Game `game_id`: from the cache of Covers when it is there, else
    /// downloaded from Steam and kept. `None` when Steam has no such image,
    /// or `icon` is not one of Steam's names for an icon.
    pub fn achievement_icon(&self, game_id: &str, icon: &str) -> Result<Option<Cover>, Error> {
        let Some(app_id) = steam_app_id(game_id) else {
            return Ok(None);
        };
        if !achievements::is_icon_name(icon) {
            return Ok(None);
        }
        self.covers.achievement_icon(self.ports.downloader.as_ref(), app_id, icon)
    }

    /// Removes the stored Steam Web API key, if there is one.
    pub fn remove_steam_web_api_key(&self) -> Result<(), Error> {
        let removed = self.ports.secrets.delete(steam_web_api::KEY_SECRET).map_err(secret_store_error);
        if removed.is_ok() {
            self.note("remove_steam_web_api_key: removed");
        }
        self.logged("remove_steam_web_api_key", removed)
    }

    /// Asks the user to choose the executable of a Local Game to add and
    /// proposes its title (see [`Self::add_local_game`]). An executable
    /// already in the Library is answered with the title it has there.
    pub fn choose_local_game_executable(&self) -> Result<ExecutableChoice, Error> {
        self.logged_outcome("choose_local_game_executable", self.choose_executable_except(None))
    }

    /// Asks the user to choose an executable, answered as
    /// [`Self::choose_local_game_executable`] does; the Local Game numbered
    /// `except`, whose executable is being replaced, does not count as
    /// having it already.
    fn choose_executable_except(&self, except: Option<i64>) -> Result<ExecutableChoice, Error> {
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
        if let Some(game) = local_games::already_added(&self.local_games_except(except)?, executable) {
            return Ok(ExecutableChoice::AlreadyAdded { title: game.title.clone() });
        }
        Ok(ExecutableChoice::Chosen { executable: executable.to_owned(), title: local_games::proposed_title(&path) })
    }

    /// Adds the executable at `executable` to the Library as a Local Game
    /// titled `title`, which the user may have corrected. The path is kept as
    /// given. An empty title, a path with no file, and an executable already
    /// in the Library by any spelling of its path are refused.
    pub fn add_local_game(&self, executable: &str, title: &str) -> Result<AddLocalGame, Error> {
        self.logged_outcome("add_local_game", self.record_local_game(executable, title))
    }

    fn record_local_game(&self, executable: &str, title: &str) -> Result<AddLocalGame, Error> {
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

    /// Records Sessions while the app runs: looks at the running processes
    /// every few seconds, waiting on the clock, and finds the Local Games
    /// whose executable runs and the Installed Steam Games with an
    /// executable running from their install folder, however they were
    /// started. A Session begins at the first look that finds its Game and
    /// is stored at once; each later look that finds it again moves its end
    /// there, so it is kept up to the last look even if the app is closed or
    /// crashes. A Game still running when tracking starts again begins a new
    /// Session. Each Session that ends starts a Sync (see
    /// [`Self::sync_automatically`]). A Steam Game's Sessions never change
    /// its Playtime, which stays Steam's figure. `report` hears each
    /// Session that begins or ends and each look that failed; tracking goes
    /// on until it answers `Break`, otherwise for good.
    pub fn track_sessions(&self, mut report: impl FnMut(Tracking) -> ControlFlow<()>) {
        let mut open = sessions::OpenSessions::new();
        // Nothing ends the waits early; the clock alone paces the looks.
        let never = Wakeup::new();
        // Whether the look before failed: a look that keeps failing is written to the Log once, not every few seconds.
        let mut failing = false;
        loop {
            let mut failed = false;
            let flow = self.look_at_processes(&mut open, &mut |event: Tracking| {
                match &event {
                    Tracking::Started { .. } => self.note("track_sessions: started"),
                    Tracking::Ended { .. } => self.note("track_sessions: ended"),
                    Tracking::Failed { reason } => {
                        if !failing {
                            let _ = self.logged::<()>("track_sessions", Err(reason.clone()));
                        }
                        failed = true;
                    }
                }
                report(event)
            });
            failing = failed;
            if flow.is_break() {
                // Nothing looks any more, so no Session is known to run.
                self.show_running(&sessions::OpenSessions::new());
                return;
            }
            self.ports.clock.sleep(sessions::LOOK_EVERY, &never);
        }
    }

    /// One look of [`Self::track_sessions`]: begins, extends and ends the
    /// Sessions in `open` by what runs now.
    fn look_at_processes(
        &self,
        open: &mut sessions::OpenSessions,
        report: &mut impl FnMut(Tracking) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let running = match self.ports.processes.running_executables() {
            Ok(running) => running,
            Err(reason) => return report(Tracking::Failed { reason: Error::new(format!("process list: {reason}")) }),
        };
        let at = clock::seconds_since_epoch(self.ports.clock.now());
        let games = match self.store.local_games() {
            Ok(games) => games,
            Err(reason) => return report(Tracking::Failed { reason }),
        };
        let steam_games = match self.ports.steam_locator.steam_folder() {
            Some(folder) => sessions::running_steam_games(&folder, &running),
            None => BTreeSet::new(),
        };
        let running: BTreeSet<Tracked> = sessions::running_games(&games, &running)
            .into_iter()
            .map(Tracked::Local)
            .chain(steam_games.into_iter().map(Tracked::Steam))
            .collect();
        let ended: Vec<Tracked> = open.keys().copied().filter(|game| !running.contains(game)).collect();
        for game in ended {
            open.remove(&game);
            self.show_running(open);
            // So Playtime and Achievements are current after playing (see `sync_automatically`).
            self.sync_soon.ring();
            report(Tracking::Ended { game_id: game.game_id() })?;
        }
        for game in running {
            let recorded = match (open.get(&game), game) {
                (Some(&session), _) => self.store.extend_session(game, session, at).map(|()| None),
                (None, Tracked::Local(local_game)) => self.store.begin_session(local_game, at).map(Some),
                (None, Tracked::Steam(app_id)) => self
                    .current_steam_account()
                    .and_then(|account| self.store.begin_steam_session(app_id, account.as_deref(), at))
                    .map(Some),
            };
            match recorded {
                Ok(None) => {}
                Ok(Some(session)) => {
                    open.insert(game, session);
                    self.show_running(open);
                    report(Tracking::Started { game_id: game.game_id() })?;
                }
                Err(reason) => report(Tracking::Failed { reason })?,
            }
        }
        ControlFlow::Continue(())
    }

    /// How the Local Game `game_id` is launched; `None` when it is not a
    /// Local Game in the Library.
    pub fn launch_options(&self, game_id: &str) -> Result<Option<LaunchOptions>, Error> {
        let Some(id) = local_game_id(game_id) else {
            return Ok(None);
        };
        self.logged("launch_options", self.store.local_game(id).map(|game| game.map(|game| game.launch_options)))
    }

    /// Sets how the Local Game `game_id` is launched, from now on and after
    /// every restart: its Launch Arguments as typed, split when it is
    /// launched (see [`Self::launch`]), and its Working Directory. A Working
    /// Directory of only spaces is none, so the executable's folder is used.
    /// Anything but a Local Game in the Library is refused.
    pub fn set_launch_options(&self, game_id: &str, options: &LaunchOptions) -> Result<(), Error> {
        let set = self.record_launch_options(game_id, options);
        if set.is_ok() {
            self.note("set_launch_options: set");
        }
        self.logged("set_launch_options", set)
    }

    fn record_launch_options(&self, game_id: &str, options: &LaunchOptions) -> Result<(), Error> {
        let options = LaunchOptions {
            arguments: options.arguments.trim().to_owned(),
            working_directory: options
                .working_directory
                .as_deref()
                .map(str::trim)
                .filter(|folder| !folder.is_empty())
                .map(str::to_owned),
        };
        match local_game_id(game_id) {
            Some(id) if self.store.set_launch_options(id, &options)? => Ok(()),
            _ => Err(Error::new(format!("no such Local Game in the library: {game_id}"))),
        }
    }

    /// Sets the Manual Progress of the Local Game `game_id` to `percent`, a
    /// whole number from 0 to 100, or clears it with `None`; it is then the
    /// Game's Progress. Anything outside 0 to 100, and any Game that is not a
    /// Local Game, is refused and changes nothing. Fails when there is no
    /// such Local Game.
    pub fn set_manual_progress(&self, game_id: &str, percent: Option<i64>) -> Result<ManualProgress, Error> {
        let Some(id) = local_game_id(game_id) else {
            return Ok(ManualProgress::NotALocalGame);
        };
        let manual_progress = match percent {
            Some(percent) => match progress::percent_in(percent) {
                Some(percent) => Some(percent),
                None => return Ok(ManualProgress::OutOfRange),
            },
            None => None,
        };
        if !self.store.set_manual_progress(id, manual_progress)? {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
        }
        match self.game(game_id)? {
            Some(game) => Ok(ManualProgress::Set { game }),
            None => Err(Error::new(format!("no such game in the library: {game_id}"))),
        }
    }

    /// Asks the user to choose a folder of Games and suggests the executables
    /// in it as Local Games, each with a proposed title; those already in the
    /// Library are marked. Adds nothing: the user adds the ones they tick
    /// with [`Self::add_local_game`].
    pub fn suggest_executables_in_folder(&self) -> Result<FolderSuggestions, Error> {
        self.logged_outcome("suggest_executables_in_folder", self.suggest_executables())
    }

    fn suggest_executables(&self) -> Result<FolderSuggestions, Error> {
        let chosen = self
            .ports
            .executable_picker
            .choose_folder()
            .map_err(|reason| Error::new(format!("folder dialog: {reason}")))?;
        let Some(folder) = chosen else {
            return Ok(FolderSuggestions::Cancelled);
        };
        let Some(text) = folder.to_str() else {
            return Err(Error::new(format!("not a path that can be stored: {}", folder.display())));
        };
        let executables = executables_in_folder::suggestions(&folder, &self.store.local_games()?);
        Ok(FolderSuggestions::Suggested { folder: text.to_owned(), executables })
    }

    /// Makes the Sessions in `open` the ones [`Self::sessions`] marks as running.
    fn show_running(&self, open: &sessions::OpenSessions) {
        *self.running_sessions.lock().unwrap_or_else(PoisonError::into_inner) =
            open.iter().map(|(&game, &session)| (game, session)).collect();
    }

    /// The Sessions of the Game `game_id`, newest first, each with when it
    /// began and how long it lasted; one whose process still runs is marked
    /// as running and lasts up to the last look that found it. A Game with
    /// no Sessions, or not in the Library, has none.
    pub fn sessions(&self, game_id: &str) -> Result<Vec<Session>, Error> {
        self.logged("sessions", self.stored_sessions(game_id))
    }

    fn stored_sessions(&self, game_id: &str) -> Result<Vec<Session>, Error> {
        let (game, stored) = if let Some(local_game) = sessions::local_game_id(game_id) {
            (Tracked::Local(local_game), self.store.local_game_sessions(local_game)?)
        } else if let Some(app_id) = steam_app_id(game_id) {
            let account = self.current_steam_account()?;
            (Tracked::Steam(app_id), self.store.steam_game_sessions(app_id, account.as_deref())?)
        } else {
            return Ok(Vec::new());
        };
        let running = self.running_sessions.lock().unwrap_or_else(PoisonError::into_inner).clone();
        Ok(stored
            .into_iter()
            .map(|(id, started_at, ended_at)| Session {
                started_at,
                seconds: ended_at.saturating_sub(started_at),
                running: running.contains(&(game, id)),
            })
            .collect())
    }

    /// Opens the Steam page where the user creates their Steam Web API key,
    /// in the browser.
    pub fn open_steam_web_api_key_page(&self) -> Result<(), Error> {
        let opened = self
            .ports
            .opener
            .open_url(steam_web_api::KEY_PAGE)
            .map_err(|error| Error::new(format!("{} could not be opened: {error}", steam_web_api::KEY_PAGE)));
        self.logged("open_steam_web_api_key_page", opened)
    }

    /// Whether the app starts with Windows, hidden in the Tray, when the
    /// user signs in: what the current user's autostart entry says, also
    /// after it was changed outside the app. Off until the user turns it on.
    pub fn starts_with_windows(&self) -> Result<bool, Error> {
        self.logged("starts_with_windows", autostart::starts_with_windows(self.ports.autostart.as_ref()))
    }

    /// Adds or removes the current user's autostart entry, which starts the
    /// app hidden in the Tray (see [`started_with_windows`]).
    pub fn set_start_with_windows(&self, on: bool) -> Result<(), Error> {
        self.logged("set_start_with_windows", autostart::set_start_with_windows(self.ports.autostart.as_ref(), on))
    }

    /// The theme the user chose; until they choose, [`Theme::System`]: the
    /// interface follows the operating system's dark or light setting.
    pub fn theme(&self) -> Result<Theme, Error> {
        self.logged("theme", self.store.theme())
    }

    /// Makes `theme` the theme of the interface, from now on and after every
    /// restart.
    pub fn choose_theme(&self, theme: Theme) -> Result<(), Error> {
        self.logged("choose_theme", self.store.choose_theme(theme))
    }

    /// How the Library shows its Games; until the user switches,
    /// [`LibraryLayout::CoverGrid`].
    pub fn library_layout(&self) -> Result<LibraryLayout, Error> {
        self.logged("library_layout", self.store.library_layout())
    }

    /// Makes `layout` the Library Layout, from now on and after every restart.
    pub fn choose_library_layout(&self, layout: LibraryLayout) -> Result<(), Error> {
        self.logged("choose_library_layout", self.store.choose_library_layout(layout))
    }

    /// The interface language the user chose; until they choose,
    /// [`Language::System`]: the interface follows the system language, and
    /// English for anything but German.
    pub fn language(&self) -> Result<Language, Error> {
        self.logged("language", self.store.language())
    }

    /// Makes `language` the interface language, from now on and after every
    /// restart.
    pub fn choose_language(&self, language: Language) -> Result<(), Error> {
        self.logged("choose_language", self.store.choose_language(language))
    }

    /// Every Collection with how many Games of the Library are in it,
    /// ordered by name whatever its case.
    pub fn collections(&self) -> Result<Vec<Collection>, Error> {
        self.logged("collections", self.stored_collections())
    }

    /// [`Self::collections`], without writing to the Log.
    fn stored_collections(&self) -> Result<Vec<Collection>, Error> {
        let library: std::collections::HashSet<String> =
            self.stored_library()?.into_iter().map(|game| game.id).collect();
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
        let naming = match collections::name_in(name) {
            Some(name) => self.store.name_collection(None, name),
            None => Ok(CollectionNaming::EmptyName),
        };
        self.logged_outcome("create_collection", naming)
    }

    /// Renames the Collection `id` to `name`, without the spaces around it,
    /// under the same rules as [`Self::create_collection`]. Fails when there
    /// is no such Collection.
    pub fn rename_collection(&self, id: i64, name: &str) -> Result<CollectionNaming, Error> {
        let naming = match collections::name_in(name) {
            Some(name) => self.store.name_collection(Some(id), name),
            None => Ok(CollectionNaming::EmptyName),
        };
        self.logged_outcome("rename_collection", naming)
    }

    /// Deletes the Collection `id`. Its Games stay in the Library. Deleting
    /// a Collection that is gone already changes nothing.
    pub fn delete_collection(&self, id: i64) -> Result<(), Error> {
        let deleted = self.store.delete_collection(id);
        if deleted.is_ok() {
            self.note("delete_collection: deleted");
        }
        self.logged("delete_collection", deleted)
    }

    /// The ids of the Collections the Game `game_id` is in, ordered like
    /// [`Self::collections`]; empty when it is in none.
    pub fn game_collections(&self, game_id: &str) -> Result<Vec<i64>, Error> {
        let ids = self.store.collection_games().and_then(|members| {
            Ok(self
                .stored_collections()?
                .into_iter()
                .map(|collection| collection.id)
                .filter(|id| members.iter().any(|(of, game)| of == id && game == game_id))
                .collect())
        });
        self.logged("game_collections", ids)
    }

    /// Puts the Game `game_id` into the Collection `collection`, besides any
    /// other Collections it is in; a Game already in it stays there once.
    /// Fails when there is no such Collection or the Game is not in the
    /// Library.
    pub fn add_game_to_collection(&self, collection: i64, game_id: &str) -> Result<(), Error> {
        let added = self.stored_library().and_then(|games| {
            if !games.iter().any(|game| game.id == game_id) {
                return Err(Error::new(format!("no such game in the library: {game_id}")));
            }
            self.store.add_game_to_collection(collection, game_id)
        });
        if added.is_ok() {
            self.note("add_game_to_collection: added");
        }
        self.logged("add_game_to_collection", added)
    }

    /// Takes the Game `game_id` out of the Collection `collection`; it stays
    /// in the Library and in its other Collections. Changes nothing when it
    /// is not in that Collection.
    pub fn remove_game_from_collection(&self, collection: i64, game_id: &str) -> Result<(), Error> {
        let removed = self.store.remove_game_from_collection(collection, game_id);
        if removed.is_ok() {
            self.note("remove_game_from_collection: removed");
        }
        self.logged("remove_game_from_collection", removed)
    }

    /// Whether closing the window keeps the app running in the Tray, so
    /// tracking goes on. On until the user turns it off.
    pub fn close_to_tray(&self) -> Result<bool, Error> {
        self.logged("close_to_tray", window::close_to_tray(&self.store))
    }

    /// Turns close-to-tray on or off, from now on and after every restart.
    pub fn set_close_to_tray(&self, on: bool) -> Result<(), Error> {
        self.logged("set_close_to_tray", window::set_close_to_tray(&self.store, on))
    }

    /// What closing the window does: with close-to-tray on the app keeps
    /// running in the Tray, and the very first time the window explains this
    /// instead; with close-to-tray off the app quits.
    pub fn close_window(&self) -> Result<WindowClose, Error> {
        self.logged_outcome("close_window", window::close_window(&self.store))
    }

    /// The Local Game `game_id` with its title and executable, for editing;
    /// `None` for a Steam Game or a Game that is not in the Library.
    pub fn local_game(&self, game_id: &str) -> Result<Option<LocalGame>, Error> {
        let Some(number) = local_games::local_number(game_id) else {
            return Ok(None);
        };
        let game = self.store.local_game(number).map(|game| {
            game.map(|game| LocalGame { id: game_id.to_owned(), title: game.title, executable: game.executable })
        });
        self.logged("local_game", game)
    }

    /// Asks the user to choose a new executable for the Local Game `game_id`
    /// and proposes its title, as [`Self::choose_local_game_executable`]
    /// does; the Game's own executable is not already in the Library. Fails
    /// for a Steam Game.
    pub fn choose_executable_for(&self, game_id: &str) -> Result<ExecutableChoice, Error> {
        let choice = local_number_or_refuse(game_id).and_then(|number| self.choose_executable_except(Some(number)));
        self.logged_outcome("choose_executable_for", choice)
    }

    /// Gives the Local Game `game_id` the title `title`, without the spaces
    /// around it, and the executable at `executable`, kept as given; its id,
    /// and so everything kept about it, stays. Refused as in
    /// [`Self::add_local_game`]: an empty title, a new executable with no
    /// file, an executable of another Local Game. The unchanged executable
    /// may be missing, so a Game whose files moved can still be renamed.
    /// Fails for a Steam Game and for a Local Game that is gone.
    pub fn edit_local_game(&self, game_id: &str, title: &str, executable: &str) -> Result<EditLocalGame, Error> {
        self.logged_outcome("edit_local_game", self.record_local_game_edit(game_id, title, executable))
    }

    fn record_local_game_edit(&self, game_id: &str, title: &str, executable: &str) -> Result<EditLocalGame, Error> {
        let number = local_number_or_refuse(game_id)?;
        let Some(stored) = self.store.local_game(number)? else {
            return Err(Error::new(format!("no such game in the library: {game_id}")));
        };
        let title = title.trim();
        if title.is_empty() {
            return Ok(EditLocalGame::NoTitle);
        }
        if executable != stored.executable && !std::path::Path::new(executable).is_file() {
            return Ok(EditLocalGame::NotFound);
        }
        if let Some(other) = local_games::already_added(&self.local_games_except(Some(number))?, executable) {
            return Ok(EditLocalGame::AlreadyAdded { title: other.title.clone() });
        }
        self.store.edit_local_game(number, title, executable)?;
        match self.game(game_id)? {
            Some(game) => Ok(EditLocalGame::Edited { game }),
            None => Err(Error::new(format!("no such game in the library: {game_id}"))),
        }
    }

    /// Removes the Local Game `game_id` from the Library with everything the
    /// app keeps about it, such as its Collection memberships and Cover. The
    /// Game's files on disk stay as they are. Removing a Local Game that is
    /// gone already changes nothing; fails for a Steam Game.
    pub fn remove_local_game(&self, game_id: &str) -> Result<(), Error> {
        let removed = local_number_or_refuse(game_id).and_then(|number| self.store.remove_local_game(number));
        if removed.is_ok() {
            // Its kept Cover and the image the user chose as its Cover.
            self.covers.forget(game_id);
            self.note("remove_local_game: removed");
        }
        self.logged("remove_local_game", removed)
    }

    /// Every stored Local Game but the one numbered `except`.
    fn local_games_except(&self, except: Option<i64>) -> Result<Vec<local_games::StoredLocalGame>, Error> {
        let mut games = self.store.local_games()?;
        games.retain(|game| Some(game.id) != except);
        Ok(games)
    }

    /// What is stored of the Steam App List: how many titles, and when it
    /// was last fetched to its end.
    pub fn steam_app_list(&self) -> Result<SteamAppList, Error> {
        self.logged("steam_app_list", self.store.steam_app_list())
    }

    /// Fetches the Steam App List with the stored Steam Web API key, page by
    /// page, and stores each page as it arrives. A fetch that stopped before
    /// its end, in this run of the app or an earlier one, is resumed after
    /// its last stored page; otherwise the whole list is fetched anew, and
    /// the list stored before stays usable meanwhile. A fetch asked for while
    /// another runs begins once that one ends.
    pub fn fetch_steam_app_list(&self) -> Result<SteamAppListFetch, Error> {
        let _fetching = self.wait_until_no_steam_app_list_fetch_runs();
        self.logged_outcome("fetch_steam_app_list", self.fetch_steam_app_list_now())
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
        match self.logged("automatic_steam_app_list_fetch", self.key_stored()) {
            Ok(false) => return report(AutomaticSteamAppListFetch::Skipped),
            Ok(true) => {}
            Err(reason) => return report(AutomaticSteamAppListFetch::Failed { reason }),
        }
        let _fetching = self.wait_until_no_steam_app_list_fetch_runs();
        let now = clock::seconds_since_epoch(self.ports.clock.now());
        match self.logged("automatic_steam_app_list_fetch", self.store.steam_app_list()) {
            Ok(list) if !steam_app_list::due(&list, now) => return report(AutomaticSteamAppListFetch::Skipped),
            Ok(_) => {}
            Err(reason) => return report(AutomaticSteamAppListFetch::Failed { reason }),
        }
        report(AutomaticSteamAppListFetch::Started)?;
        report(match self.logged_outcome("automatic_steam_app_list_fetch", self.fetch_steam_app_list_now()) {
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
        let titles = self
            .steam_title_index()
            .and_then(|index| self.store.steam_titles(&index.find(&search, steam_app_list::LOOKUP_LIMIT)));
        self.logged("find_steam_titles", titles)
    }

    /// Asks the user to choose an image and makes it the Cover of the Local
    /// Game `game_id`, in place of its executable's icon or an image chosen
    /// before. The image is copied into the app's data folder, so it stays
    /// the Cover when the original is moved or deleted. A file that is not a
    /// PNG or JPEG image is refused and changes nothing. Fails for a Game
    /// that is not a Local Game in the Library.
    pub fn choose_cover_image(&self, game_id: &str) -> Result<CoverImageChoice, Error> {
        self.logged_outcome("choose_cover_image", self.pick_cover_image(game_id))
    }

    fn pick_cover_image(&self, game_id: &str) -> Result<CoverImageChoice, Error> {
        let Some(game) = self.stored_local_game(game_id)? else {
            return Err(Error::new(format!("not a Local Game in the library: {game_id}")));
        };
        let chosen = self
            .ports
            .image_picker
            .choose_image()
            .map_err(|reason| Error::new(format!("file dialog: {reason}")))?;
        match chosen {
            Some(image) => local_covers::keep_chosen_image(&self.covers, game.id, &image),
            None => Ok(CoverImageChoice::Cancelled),
        }
    }

    /// Whether the user chose an image as the Cover of the Local Game
    /// `game_id` (see [`Self::choose_cover_image`]).
    pub fn cover_image_chosen(&self, game_id: &str) -> Result<bool, Error> {
        let chosen = self.stored_local_game(game_id).and_then(|game| match game {
            Some(game) => Ok(local_covers::chosen_image(&self.covers, game.id)?.is_some()),
            None => Ok(false),
        });
        self.logged("cover_image_chosen", chosen)
    }

    /// Forgets the image the user chose as the Cover of the Local Game
    /// `game_id`: the Game shows the Cover the app finds for it again, its
    /// executable's icon. Without a chosen image nothing changes.
    pub fn remove_cover_image(&self, game_id: &str) -> Result<(), Error> {
        let removed = self.stored_local_game(game_id).and_then(|game| match game {
            Some(game) => local_covers::remove_chosen_image(&self.covers, game.id),
            None => Ok(()),
        });
        self.logged("remove_cover_image", removed)
    }

    /// The title of the stored Steam App List the app proposes as the Cover
    /// Match of a Local Game titled `title`, before it is added: the title of
    /// the same name, whatever its case and punctuation, else the shortest
    /// that begins with all of its words. `None` when no title matches, or no
    /// Steam App List is stored, as without a Steam Web API key; the Game
    /// then keeps its executable's icon.
    pub fn propose_cover_match(&self, title: &str) -> Result<Option<SteamTitle>, Error> {
        let search = steam_app_list::searchable(title);
        if search.is_empty() {
            return Ok(None);
        }
        let proposal = self
            .steam_title_index()
            .and_then(|index| self.store.steam_titles(&index.find(&search, steam_app_list::LOOKUP_LIMIT)))
            .map(|found| cover_match::proposal(&search, found));
        self.logged("propose_cover_match", proposal)
    }

    /// The Steam title the Local Game `game_id` borrows its Cover from;
    /// `None` without a Cover Match, for a Steam Game and for a Game that is
    /// not in the Library.
    pub fn cover_match(&self, game_id: &str) -> Result<Option<SteamTitle>, Error> {
        let matched = self.stored_local_game(game_id).and_then(|game| match game {
            Some(game) => self.store.cover_match(game.id),
            None => Ok(None),
        });
        self.logged("cover_match", matched)
    }

    /// Gives the Local Game `game_id` the Cover Match `app_id`, a title of
    /// the stored Steam App List, in place of any before; `None` removes its
    /// Cover Match, and the Game shows its executable's icon again. An image
    /// the user chose stays the Cover either way. The match stays when the
    /// key is removed or the Steam App List is fetched anew. Fails for a Game
    /// that is not a Local Game in the Library, and for an app id that is not
    /// in the stored Steam App List.
    pub fn set_cover_match(&self, game_id: &str, app_id: Option<u32>) -> Result<(), Error> {
        self.logged("set_cover_match", self.record_cover_match(game_id, app_id))
    }

    fn record_cover_match(&self, game_id: &str, app_id: Option<u32>) -> Result<(), Error> {
        let Some(game) = self.stored_local_game(game_id)? else {
            return Err(Error::new(format!("not a Local Game in the library: {game_id}")));
        };
        let title = match app_id {
            Some(app_id) => match self.store.steam_titles(&[app_id])?.pop() {
                Some(title) => Some(title),
                None => return Err(Error::new(format!("not in the Steam App List: {app_id}"))),
            },
            None => None,
        };
        self.store.set_cover_match(game.id, title.as_ref())
    }

    /// The Local Game `game_id` as the store keeps it; `None` when the id is
    /// not a Local Game's or the Game is not in the Library.
    fn stored_local_game(&self, game_id: &str) -> Result<Option<local_games::StoredLocalGame>, Error> {
        let Some(id) = game_id.strip_prefix("local:").and_then(|id| id.parse::<i64>().ok()) else {
            return Ok(None);
        };
        Ok(self.store.local_games()?.into_iter().find(|game| game.id == id))
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

    /// The Hidden Games, ordered by title like [`Self::library`]. A Hidden
    /// Game stays hidden through Syncs and restarts until it is unhidden.
    pub fn hidden_games(&self) -> Result<Vec<Game>, Error> {
        let games = self.store.hidden_game_ids().and_then(|hidden| {
            let mut games = self.every_game()?;
            games.retain(|game| hidden.contains(&game.id));
            Ok(games)
        });
        self.logged("hidden_games", games)
    }

    /// Hides the Game `game_id`: it leaves the Library and every view built
    /// on it without being deleted. Fails when there is no such Game.
    pub fn hide_game(&self, game_id: &str) -> Result<(), Error> {
        let hidden = self.every_game().and_then(|games| {
            if !games.iter().any(|game| game.id == game_id) {
                return Err(Error::new(format!("no such game in the library: {game_id}")));
            }
            self.store.hide_game(game_id)
        });
        if hidden.is_ok() {
            self.note("hide_game: hidden");
        }
        self.logged("hide_game", hidden)
    }

    /// Brings the Hidden Game `game_id` back into the Library; unhiding a
    /// Game that is not hidden changes nothing.
    pub fn unhide_game(&self, game_id: &str) -> Result<(), Error> {
        let unhidden = self.store.unhide_game(game_id);
        if unhidden.is_ok() {
            self.note("unhide_game: unhidden");
        }
        self.logged("unhide_game", unhidden)
    }

    /// The Statistics of the Library as it shows: total Playtime, unlocked
    /// Achievements of all and their share, Games at 100 % Progress, and
    /// Games per Status, the Playtime of each of the last eight weeks from
    /// the recorded Sessions, and the five most played Games. Hidden Games
    /// count in none of them.
    pub fn statistics(&self) -> Result<Statistics, Error> {
        let statistics = self.stored_library().and_then(|games| {
            let achievements = match self.library_steam_account()? {
                Some(account) => self.store.achievement_counts(&account)?,
                None => Default::default(),
            };
            let now = clock::seconds_since_epoch(self.ports.clock.now());
            let sessions = self.store.sessions_ended_after(statistics::first_week_start(now))?;
            Ok(statistics::of(
                &games,
                |game| steam_app_id(&game.id).and_then(|app_id| achievements.get(&app_id).copied()),
                &sessions,
                now,
            ))
        });
        self.logged("statistics", statistics)
    }

    /// Marks the Game `game_id` as a Favorite, or with `favorite` false
    /// unmarks it. The mark stays through Syncs and restarts, also while a
    /// Steam Game is out of the Library. Fails when the Game is not in the
    /// Library; a Hidden Game is not.
    pub fn mark_favorite(&self, game_id: &str, favorite: bool) -> Result<(), Error> {
        let marked = self.stored_library().and_then(|games| {
            if !games.iter().any(|game| game.id == game_id) {
                return Err(Error::new(format!("no such game in the library: {game_id}")));
            }
            self.store.mark_favorite(game_id, favorite)
        });
        if marked.is_ok() {
            self.note(if favorite { "mark_favorite: marked" } else { "mark_favorite: unmarked" });
        }
        self.logged("mark_favorite", marked)
    }

    /// How many Games of the Library are Favorites; a Hidden Game does not count.
    pub fn favorite_count(&self) -> Result<u32, Error> {
        let count = self
            .stored_library()
            .map(|games| u32::try_from(games.iter().filter(|game| game.favorite).count()).unwrap_or(u32::MAX));
        self.logged("favorite_count", count)
    }

    /// The version of the app that runs and its license, for the About dialog.
    pub fn about(&self) -> About {
        about::about()
    }

    /// Opens `page`, the project page or the license, in the browser.
    pub fn open_about_page(&self, page: AboutPage) -> Result<(), Error> {
        let url = page.url();
        let opened =
            self.ports.opener.open_url(url).map_err(|error| Error::new(format!("{url} could not be opened: {error}")));
        self.logged("open_about_page", opened)
    }

    /// The Playtime Adjustment of the Game `game_id`, in minutes, negative
    /// when time was removed; `None` when it has none. Steam Games never
    /// have one.
    pub fn playtime_adjustment(&self, game_id: &str) -> Result<Option<i32>, Error> {
        let adjustment = match playtime_adjustment::local_game_number(game_id) {
            Some(number) => self.store.playtime_adjustments().map(|adjustments| adjustments.get(&number).copied()),
            None => Ok(None),
        };
        self.logged("playtime_adjustment", adjustment)
    }

    /// Makes `minutes` the Playtime Adjustment of the Local Game `game_id`,
    /// from now on and after every restart, and answers the Game as the
    /// Library now shows it: its Playtime is the sum of its Sessions plus
    /// the adjustment, never below zero. `None` or zero clears it. A Steam
    /// Game, or a Game that is not in the Library, is refused.
    pub fn set_playtime_adjustment(&self, game_id: &str, minutes: Option<i32>) -> Result<Game, Error> {
        self.logged("set_playtime_adjustment", self.record_playtime_adjustment(game_id, minutes))
    }

    fn record_playtime_adjustment(&self, game_id: &str, minutes: Option<i32>) -> Result<Game, Error> {
        let not_in_library = || Error::new(format!("no such local game in the library: {game_id}"));
        let number = playtime_adjustment::local_game_number(game_id).ok_or_else(not_in_library)?;
        let in_library = |library: Vec<Game>| library.into_iter().find(|game| game.id == game_id);
        if in_library(self.stored_library()?).is_none() {
            return Err(not_in_library());
        }
        self.store.set_playtime_adjustment(number, playtime_adjustment::stored(minutes))?;
        in_library(self.stored_library()?).ok_or_else(not_in_library)
    }

    /// Exports everything that is the person's own into one Export File
    /// they choose in a "Save as" dialog, which offers a name with today's
    /// date: Local Games with how they are launched and the Cover images
    /// they chose, Sessions, Playtime Adjustments, Manual Progress,
    /// Statuses, Favorites, Hidden Games, Collections and the settings
    /// (docs/export-format.md). The Steam Web API key is never in it. A file
    /// already there is replaced; cancelling writes nothing.
    pub fn export_data(&self) -> Result<DataExport, Error> {
        self.logged_outcome("export_data", self.write_export())
    }

    fn write_export(&self) -> Result<DataExport, Error> {
        let now = clock::seconds_since_epoch(self.ports.clock.now());
        let chosen = self
            .ports
            .export_file_picker
            .choose_export_file(&export::suggested_name(now))
            .map_err(|reason| Error::new(format!("file dialog: {reason}")))?;
        let Some(file) = chosen else {
            return Ok(DataExport::Cancelled);
        };
        let contents = export::contents(&self.store, &self.covers, self.ports.autostart.as_ref(), now)?;
        export::write(&file, &contents)?;
        Ok(DataExport::Exported { file: file.display().to_string() })
    }

    /// Imports an Export File the user chooses in an "Open" dialog: its
    /// Local Games with how they are launched and the Cover images chosen
    /// for them, Sessions, Playtime Adjustments, Manual Progress, Statuses,
    /// Favorites, Hidden Games, Collections and settings replace those here,
    /// all at once. Steam Games stay, and the Steam Web API key is never
    /// touched. Each Local Game gets a new number here. A file of a newer
    /// version, or one that is damaged or no Export File, is refused and
    /// changes nothing; so does cancelling.
    pub fn import_data(&self) -> Result<DataImport, Error> {
        self.logged_outcome("import_data", self.read_import())
    }

    fn read_import(&self) -> Result<DataImport, Error> {
        let chosen = self
            .ports
            .export_file_picker
            .choose_import_file()
            .map_err(|reason| Error::new(format!("file dialog: {reason}")))?;
        let Some(file) = chosen else {
            return Ok(DataImport::Cancelled);
        };
        let bytes =
            std::fs::read(&file).map_err(|error| Error::new(format!("cannot read {}: {error}", file.display())))?;
        let checked = match import::check(&bytes) {
            Ok(checked) => checked,
            Err(refused) => return Ok(refused),
        };
        import::replace(&self.store, &self.covers, self.ports.autostart.as_ref(), &checked)?;
        let local_games = u32::try_from(checked.file.local_games.len()).unwrap_or(u32::MAX);
        Ok(DataImport::Imported { file: file.display().to_string(), local_games })
    }

    /// The dominant hue of the Game `game_id`'s Cover, from which its Detail
    /// Page takes its Tint; `None` when the Game has no Cover or a Cover
    /// without colour to speak of. Reads the cached Cover, downloading it
    /// first like [`Core::cover`] when it is not cached yet.
    pub fn cover_hue(&self, game_id: &str) -> Result<Option<CoverHue>, Error> {
        let hue = self.cover(game_id).map(|cover| cover.as_ref().and_then(cover_hue::dominant_hue));
        self.logged("cover_hue", hue)
    }
}

/// The name of an outcome the interface branches on (its `outcome` tag),
/// for the Log.
fn outcome_name(outcome: &impl Serialize) -> String {
    match serde_json::to_value(outcome) {
        Ok(serde_json::Value::Object(fields)) => match fields.get("outcome") {
            Some(serde_json::Value::String(name)) => name.clone(),
            _ => "done".to_owned(),
        },
        _ => "done".to_owned(),
    }
}

/// The number in the Local Game id `game_id`; an error for any other Game,
/// since only Local Games are edited and removed by hand.
fn local_number_or_refuse(game_id: &str) -> Result<i64, Error> {
    local_games::local_number(game_id).ok_or_else(|| Error::new(format!("not a Local Game: {game_id}")))
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

/// The store's number in a Local Game's id (`local:<number>`).
fn local_game_id(game_id: &str) -> Option<i64> {
    game_id.strip_prefix("local:")?.parse().ok()
}

/// Steam's app id in a Steam Game's id (`steam:<app id>`).
fn steam_app_id(game_id: &str) -> Option<u32> {
    game_id.strip_prefix("steam:")?.parse().ok()
}
