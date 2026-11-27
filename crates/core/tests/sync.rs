//! Sync: with a Steam Web API key, one round of fetching every Steam Game the
//! Steam Account owns, Installed or not, with Steam's Playtime and Last
//! Played, merged into the store. A failed Sync changes nothing.

use std::collections::HashMap;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AutomaticSync, Clock, Core, CoverCache, Downloader, Game, OpenError, Opener, Ports, SecretStore, Source, Status, SteamLocator,
    SteamWebApiKeyCheck, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The most recently used Steam Account on the test computer.
const PLAYER_ONE: &str = "76561197960287930";
/// Another Steam Account.
const PLAYER_TWO: &str = "76561197960287931";

/// 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch.
const NOW: u64 = 1_795_608_000;
/// 2026-11-20 20:15:00 UTC.
const LAST_WEEK: u64 = 1_795_205_700;

/// One game in Steam's answer to `GetOwnedGames`.
#[derive(Clone)]
struct Owned {
    app_id: u32,
    name: &'static str,
    minutes: u32,
    /// Seconds since the Unix epoch; 0 for never, as Steam writes it.
    last_played: u64,
}

fn owned(app_id: u32, name: &'static str, minutes: u32, last_played: u64) -> Owned {
    Owned { app_id, name, minutes, last_played }
}

/// How the fake Steam Web API behaves.
#[derive(Clone)]
enum Network {
    /// Steam answers as it does; each account owns what the map says.
    Online(HashMap<&'static str, Vec<Owned>>),
    /// Steam refuses the key.
    KeyRejected,
    /// The account's game details are private: Steam answers with an empty response.
    GameDetailsPrivate,
    /// No answer arrives at all.
    Offline,
    /// Steam answers, but with a server error.
    SteamDown,
    /// Something in between, such as a network's sign-in page, answers.
    SignInPage,
}

/// A fake of the Steam Web API in the shape Steam answers with. It
/// remembers every URL it was asked for and can be changed between Syncs.
#[derive(Clone)]
struct SteamWebApi {
    network: Arc<Mutex<Network>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl SteamWebApi {
    fn new(network: Network) -> Self {
        Self { network: Arc::new(Mutex::new(network)), asked: Arc::default() }
    }

    /// Steam online, where PLAYER_ONE owns `games`.
    fn owning(games: Vec<Owned>) -> Self {
        Self::new(Network::Online(HashMap::from([(PLAYER_ONE, games)])))
    }

    fn becomes(&self, network: Network) {
        *self.network.lock().unwrap() = network;
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

/// The value of `name` in the query of `url`.
fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        self.asked.lock().unwrap().push(url.to_owned());
        let answer = |status, text: String| Ok(WebAnswer { status, body: text.into_bytes() });
        match &*self.network.lock().unwrap() {
            Network::Offline => Err("no connection to the server".into()),
            Network::SteamDown => answer(503, "<html><body>Service Unavailable</body></html>".into()),
            Network::SignInPage => answer(200, "<!doctype html><title>Sign in to the Wi-Fi</title>".into()),
            Network::KeyRejected => answer(403, "<html><head><title>Forbidden</title></head><body><h1>Forbidden</h1>Access is denied. Retrying will not help. Please verify your <pre>key=</pre> parameter.</body></html>".into()),
            Network::GameDetailsPrivate => answer(200, r#"{"response":{}}"#.into()),
            Network::Online(accounts) => {
                if query_value(url, "key") != Some(KEY) {
                    return answer(403, "<html><body>Forbidden</body></html>".into());
                }
                // The smallest request that needs a key, with which the app checks a key.
                if url.starts_with("https://api.steampowered.com/IStoreService/GetAppList/v1/?") {
                    return answer(200, r#"{"response":{"apps":[{"appid":10,"name":"Counter-Strike"}],"have_more_results":true,"last_appid":10}}"#.into());
                }
                if !url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") {
                    return answer(404, "<html><body>Not Found</body></html>".into());
                }
                let games = query_value(url, "steamid").and_then(|id| accounts.get(id)).cloned().unwrap_or_default();
                let games: Vec<_> = games
                    .iter()
                    .map(|g| {
                        serde_json::json!({
                            "appid": g.app_id,
                            "name": g.name,
                            "playtime_forever": g.minutes,
                            "img_icon_url": "0123456789abcdef0123456789abcdef01234567",
                            "has_community_visible_stats": true,
                            "playtime_windows_forever": g.minutes,
                            "playtime_mac_forever": 0,
                            "playtime_linux_forever": 0,
                            "playtime_deck_forever": 0,
                            "rtime_last_played": g.last_played,
                            "playtime_disconnected": 0
                        })
                    })
                    .collect();
                let response = if games.is_empty() {
                    serde_json::json!({ "response": { "game_count": 0 } })
                } else {
                    serde_json::json!({ "response": { "game_count": games.len(), "games": games } })
                };
                answer(200, response.to_string())
            }
        }
    }
}

/// A substitute for the operating system's secret store, kept in memory.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<HashMap<String, String>>>);

impl Secrets {
    fn holding_the_key() -> Self {
        let secrets = Self::default();
        secrets.0.lock().unwrap().insert("steam-web-api-key".into(), KEY.into());
        secrets
    }
}

impl SecretStore for Secrets {
    fn read(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

/// A clock the test sets.
#[derive(Clone)]
struct TestClock(Arc<Mutex<SystemTime>>);

impl TestClock {
    fn at(seconds: u64) -> Self {
        Self(Arc::new(Mutex::new(UNIX_EPOCH + Duration::from_secs(seconds))))
    }

    fn set(&self, seconds: u64) {
        *self.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(seconds);
    }
}

impl Clock for TestClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    /// The time passes at once, unless the wakeup has rung.
    fn sleep(&self, duration: Duration, wakeup: &Wakeup) {
        if !wakeup.take() {
            *self.0.lock().unwrap() += duration;
        }
    }
}

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// No Covers are needed here.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// These tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A Steam installation on disk with one library folder: the Steam folder.
struct SteamFixture(TempDir);

impl SteamFixture {
    /// Steam with PLAYER_ONE logged in most recently.
    fn new() -> Self {
        let fixture = Self(tempfile::tempdir().unwrap());
        let path = fixture.path();
        fs::create_dir_all(path.join("config")).unwrap();
        fs::write(
            path.join("config").join("loginusers.vdf"),
            format!(
                r#""users" {{ "{PLAYER_ONE}" {{ "PersonaName" "One" "MostRecent" "1" }} "{PLAYER_TWO}" {{ "PersonaName" "Two" "MostRecent" "0" }} }}"#
            ),
        )
        .unwrap();
        let library = path.display().to_string().replace('\\', "\\\\");
        fs::create_dir_all(path.join("steamapps")).unwrap();
        fs::write(
            path.join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{library}\"\n\t}}\n}}\n"),
        )
        .unwrap();
        fixture
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        fs::write(
            self.path().join("steamapps").join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }
}

/// The user profile of one test, and the computer the app runs on.
struct Computer {
    profile: TempDir,
    steam: SteamFixture,
    secrets: Secrets,
    clock: TestClock,
}

impl Computer {
    /// A computer with Steam, a stored key, and the clock at NOW.
    fn new() -> Self {
        Self {
            profile: tempfile::tempdir().unwrap(),
            steam: SteamFixture::new(),
            secrets: Secrets::holding_the_key(),
            clock: TestClock::at(NOW),
        }
    }

    fn without_key(mut self) -> Self {
        self.secrets = Secrets::default();
        self
    }

    /// Starts the app with Steam's Web API behaving as `api` does.
    fn start(&self, api: &SteamWebApi) -> Core {
        self.start_with_steam(api, Some(self.steam.path()))
    }

    fn start_with_steam(&self, api: &SteamWebApi, steam: Option<&Path>) -> Core {
        self.start_with(Box::new(api.clone()), steam)
    }

    /// Starts the app with Steam's Web API answering through `api`.
    fn start_with_web_api(&self, api: Box<dyn WebApi>) -> Core {
        self.start_with(api, Some(self.steam.path()))
    }

    fn start_with(&self, api: Box<dyn WebApi>, steam: Option<&Path>) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.map(Path::to_path_buf))),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(self.secrets.clone()),
                web_api: api,
                clock: Box::new(self.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        )
    }
}

fn steam_game(app_id: u32, title: &str, installed: bool, minutes: u32, last_played: Option<u64>) -> Game {
    Game {
        id: format!("steam:{app_id}"),
        title: title.into(),
        source: Source::Steam,
        installed,
        playtime_minutes: Some(minutes),
        last_played,
        status: Status::Unplayed,
    }
}

#[test]
fn a_sync_brings_every_owned_game_into_the_library_with_steams_playtime_and_last_played() {
    let computer = Computer::new();
    computer.steam.install(620, "Portal 2");
    let steam = SteamWebApi::owning(vec![
        owned(400, "Portal", 95, LAST_WEEK),
        owned(620, "Portal 2", 1_234, NOW - 3_600),
        owned(70, "Half-Life", 0, 0),
    ]);
    let core = computer.start(&steam);
    core.read_installed_games().unwrap();

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW });

    assert_eq!(
        core.library().unwrap(),
        [
            steam_game(70, "Half-Life", false, 0, None),
            steam_game(400, "Portal", false, 95, Some(LAST_WEEK)),
            steam_game(620, "Portal 2", true, 1_234, Some(NOW - 3_600)),
        ]
    );
}

#[test]
fn the_time_of_the_last_successful_sync_is_kept_after_a_restart() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    assert_eq!(core.last_sync().unwrap(), None);

    core.sync().unwrap();

    assert_eq!(core.last_sync().unwrap(), Some(NOW));
    assert_eq!(computer.start(&steam).last_sync().unwrap(), Some(NOW));
}

#[test]
fn a_failed_sync_leaves_the_library_and_the_last_sync_as_they_were_and_says_why() {
    for (network, why) in [
        (Network::KeyRejected, SyncOutcome::KeyRejected),
        (Network::GameDetailsPrivate, SyncOutcome::GameDetailsPrivate),
        (Network::Offline, SyncOutcome::SteamUnreachable),
        (Network::SteamDown, SyncOutcome::SteamUnreachable),
        (Network::SignInPage, SyncOutcome::SteamUnreachable),
    ] {
        let computer = Computer::new();
        computer.steam.install(620, "Portal 2");
        let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK), owned(620, "Portal 2", 1_234, NOW)]);
        let core = computer.start(&steam);
        core.read_installed_games().unwrap();
        core.sync().unwrap();
        let library = core.library().unwrap();
        steam.becomes(network);
        computer.clock.set(NOW + 3_600);

        assert_eq!(core.sync().unwrap(), why);

        assert_eq!(core.library().unwrap(), library);
        assert_eq!(core.last_sync().unwrap(), Some(NOW));
    }
}

#[test]
fn without_a_key_there_is_no_sync_and_steam_is_not_asked() {
    let computer = Computer::new().without_key();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);

    assert_eq!(core.sync().unwrap(), SyncOutcome::NoKey);

    assert_eq!(steam.asked(), Vec::<String>::new());
    assert_eq!(core.library().unwrap(), []);
    assert_eq!(core.last_sync().unwrap(), None);
}

#[test]
fn without_a_steam_account_there_is_no_sync() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start_with_steam(&steam, None);

    assert_eq!(core.sync().unwrap(), SyncOutcome::NoSteamAccount);

    assert_eq!(steam.asked(), Vec::<String>::new());
}

#[test]
fn the_sync_asks_steam_over_https_for_the_chosen_steam_account() {
    let computer = Computer::new();
    let steam = SteamWebApi::new(Network::Online(HashMap::from([
        (PLAYER_ONE, vec![owned(400, "Portal", 95, LAST_WEEK)]),
        (PLAYER_TWO, vec![owned(70, "Half-Life", 30, LAST_WEEK)]),
    ])));
    let core = computer.start(&steam);
    core.choose_steam_account(PLAYER_TWO).unwrap();

    core.sync().unwrap();

    assert_eq!(core.library().unwrap(), [steam_game(70, "Half-Life", false, 30, Some(LAST_WEEK))]);
    let asked = steam.asked();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].starts_with("https://api.steampowered.com/"), "{}", asked[0]);
}

#[test]
fn until_the_user_chooses_the_sync_reads_the_most_recently_used_steam_account() {
    let computer = Computer::new();
    let steam = SteamWebApi::new(Network::Online(HashMap::from([
        (PLAYER_ONE, vec![owned(400, "Portal", 95, LAST_WEEK)]),
        (PLAYER_TWO, vec![owned(70, "Half-Life", 30, LAST_WEEK)]),
    ])));
    let core = computer.start(&steam);

    core.sync().unwrap();

    assert_eq!(core.library().unwrap(), [steam_game(400, "Portal", false, 95, Some(LAST_WEEK))]);
}

#[test]
fn a_later_sync_brings_steams_new_figures() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    steam.becomes(Network::Online(HashMap::from([(PLAYER_ONE, vec![owned(400, "Portal", 155, NOW - 60)])])));
    computer.clock.set(NOW + 1_800);

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW + 1_800 });

    assert_eq!(core.library().unwrap(), [steam_game(400, "Portal", false, 155, Some(NOW - 60))]);
    assert_eq!(core.last_sync().unwrap(), Some(NOW + 1_800));
}

#[test]
fn a_game_the_account_no_longer_owns_leaves_the_library_unless_it_is_installed() {
    let computer = Computer::new();
    computer.steam.install(620, "Portal 2");
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK), owned(620, "Portal 2", 1_234, NOW)]);
    let core = computer.start(&steam);
    core.read_installed_games().unwrap();
    core.sync().unwrap();
    steam.becomes(Network::Online(HashMap::from([(PLAYER_ONE, vec![])])));

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW });

    let titles: Vec<_> = core.library().unwrap().into_iter().map(|game| game.title).collect();
    assert_eq!(titles, ["Portal 2"]);
}

#[test]
fn reading_the_installed_games_again_keeps_steams_figures() {
    let computer = Computer::new();
    computer.steam.install(620, "Portal 2");
    let steam = SteamWebApi::owning(vec![owned(620, "Portal 2", 1_234, LAST_WEEK)]);
    let core = computer.start(&steam);
    core.sync().unwrap();

    core.read_installed_games().unwrap();

    assert_eq!(core.library().unwrap(), [steam_game(620, "Portal 2", true, 1_234, Some(LAST_WEEK))]);
}

#[test]
fn an_installed_game_shows_no_playtime_before_the_first_sync() {
    let computer = Computer::new();
    computer.steam.install(620, "Portal 2");
    let core = computer.start(&SteamWebApi::owning(vec![]));

    let library = core.read_installed_games().unwrap();

    assert_eq!(library[0].playtime_minutes, None);
    assert_eq!(library[0].last_played, None);
}

#[test]
fn the_games_of_the_previous_version_are_kept_and_synced() {
    let computer = Computer::new();
    // The database as the previous version of the app left it: schema 1,
    // with one Installed Steam Game.
    let connection = rusqlite::Connection::open(computer.profile.path().join("nicelibrary.db")).unwrap();
    connection
        .execute_batch(&format!(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE steam_games (
                 steam_account TEXT NOT NULL, app_id INTEGER NOT NULL, title TEXT NOT NULL,
                 installed INTEGER NOT NULL, PRIMARY KEY (steam_account, app_id));
             INSERT INTO settings VALUES ('steam_account', '{PLAYER_ONE}');
             INSERT INTO steam_games VALUES ('{PLAYER_ONE}', 620, 'Portal 2', 1);
             PRAGMA user_version = 1;"
        ))
        .unwrap();
    drop(connection);
    let core = computer.start_with_steam(&SteamWebApi::owning(vec![owned(620, "Portal 2", 1_234, LAST_WEEK)]), None);
    assert_eq!(core.library().unwrap(), [Game { playtime_minutes: None, last_played: None, ..steam_game(620, "Portal 2", true, 0, None) }]);

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW });

    assert_eq!(core.library().unwrap(), [steam_game(620, "Portal 2", true, 1_234, Some(LAST_WEEK))]);
}

#[test]
fn a_sync_outcome_reaches_the_interface_in_the_shape_the_interface_expects() {
    for (outcome, json) in [
        (SyncOutcome::Synced { at: NOW }, serde_json::json!({ "outcome": "synced", "at": NOW })),
        (SyncOutcome::NoKey, serde_json::json!({ "outcome": "noKey" })),
        (SyncOutcome::NoSteamAccount, serde_json::json!({ "outcome": "noSteamAccount" })),
        (SyncOutcome::KeyRejected, serde_json::json!({ "outcome": "keyRejected" })),
        (SyncOutcome::GameDetailsPrivate, serde_json::json!({ "outcome": "gameDetailsPrivate" })),
        (SyncOutcome::SteamUnreachable, serde_json::json!({ "outcome": "steamUnreachable" })),
    ] {
        assert_eq!(serde_json::to_value(outcome).unwrap(), json);
    }
}

/// Runs the automatic Syncs until `rounds` of them have ended, and answers
/// how each ended.
fn automatic_syncs(core: &Core, rounds: usize) -> Vec<AutomaticSync> {
    let mut ended = Vec::new();
    core.sync_automatically(|event| {
        if !matches!(event, AutomaticSync::Started) {
            ended.push(event);
        }
        if ended.len() == rounds { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });
    ended
}

#[test]
fn the_app_syncs_shortly_after_it_starts_and_then_every_30_minutes() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);

    let ended = automatic_syncs(&core, 3);

    assert_eq!(
        ended,
        [
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 }),
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 + 1_800 }),
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 + 3_600 }),
        ]
    );
    assert_eq!(core.library().unwrap(), [steam_game(400, "Portal", false, 95, Some(LAST_WEEK))]);
}

#[test]
fn without_a_key_no_automatic_sync_starts_until_one_is_stored() {
    let computer = Computer::new().without_key();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    let mut ended = Vec::new();

    core.sync_automatically(|event| {
        if matches!(event, AutomaticSync::Started) {
            return ControlFlow::Continue(());
        }
        ended.push(event);
        if ended.len() == 2 {
            assert_eq!(steam.asked(), Vec::<String>::new());
            the_user_stores_the_key(&computer);
        }
        if ended.len() == 3 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });

    assert_eq!(
        ended,
        [
            AutomaticSync::Skipped,
            AutomaticSync::Skipped,
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 + 1_800 + 1_800 }),
        ]
    );
}

/// The user stores the key while the app runs.
fn the_user_stores_the_key(computer: &Computer) {
    computer.secrets.write("steam-web-api-key", KEY).unwrap();
}

/// The time on `clock`, in seconds since the Unix epoch.
fn seconds(clock: &TestClock) -> u64 {
    clock.now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[test]
fn storing_a_key_that_steam_accepts_starts_a_sync_right_away() {
    let computer = Computer::new().without_key();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    let mut ended = Vec::new();

    core.sync_automatically(|event| {
        if matches!(event, AutomaticSync::Started) {
            return ControlFlow::Continue(());
        }
        ended.push(event);
        if ended.len() == 1 {
            // Between two rounds, as the user does it.
            assert_eq!(core.store_steam_web_api_key(KEY), Ok(SteamWebApiKeyCheck::Stored));
        }
        if ended.len() == 3 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });

    assert_eq!(
        ended,
        [
            AutomaticSync::Skipped,
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 }),
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 + 1_800 }),
        ]
    );
    assert_eq!(core.library().unwrap(), [steam_game(400, "Portal", false, 95, Some(LAST_WEEK))]);
}

#[test]
fn a_key_that_is_refused_or_cannot_be_checked_starts_no_sync() {
    for (network, check) in [
        (Network::Online(HashMap::new()), SteamWebApiKeyCheck::Invalid),
        (Network::KeyRejected, SteamWebApiKeyCheck::Invalid),
        (Network::Offline, SteamWebApiKeyCheck::SteamUnreachable),
        (Network::SignInPage, SteamWebApiKeyCheck::SteamUnreachable),
    ] {
        let computer = Computer::new().without_key();
        let steam = SteamWebApi::new(network);
        let core = computer.start(&steam);
        let mut ended = Vec::new();

        core.sync_automatically(|event| {
            ended.push((event, seconds(&computer.clock)));
            if ended.len() == 1 {
                // A key Steam does not know, or one it cannot be asked about.
                assert_eq!(core.store_steam_web_api_key("FEDCBA9876543210FEDCBA9876543210"), Ok(check));
            }
            if ended.len() == 2 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
        });

        assert_eq!(ended, [(AutomaticSync::Skipped, NOW + 5), (AutomaticSync::Skipped, NOW + 5 + 1_800)], "{check:?}");
        assert!(steam.asked().iter().all(|url| !url.contains("GetOwnedGames")), "{check:?}");
    }
}

#[test]
fn a_key_stored_while_a_sync_runs_starts_another_once_it_ends() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    let mut events = Vec::new();

    core.sync_automatically(|event| {
        if events.is_empty() {
            // The user replaces the key while the first Sync runs.
            assert_eq!(core.store_steam_web_api_key(KEY), Ok(SteamWebApiKeyCheck::Stored));
        }
        events.push(event);
        if events.len() == 4 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });

    assert_eq!(
        events,
        [
            AutomaticSync::Started,
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 }),
            AutomaticSync::Started,
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 }),
        ]
    );
}

#[test]
fn a_failed_automatic_sync_changes_nothing_and_the_next_one_follows_30_minutes_later() {
    let computer = Computer::new();
    let steam = SteamWebApi::new(Network::Offline);
    let core = computer.start(&steam);
    let mut ended = Vec::new();

    core.sync_automatically(|event| {
        if matches!(event, AutomaticSync::Started) {
            return ControlFlow::Continue(());
        }
        ended.push(event);
        steam.becomes(Network::Online(HashMap::from([(PLAYER_ONE, vec![owned(400, "Portal", 95, LAST_WEEK)])])));
        if ended.len() == 2 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });

    assert_eq!(
        ended,
        [
            AutomaticSync::Finished(SyncOutcome::SteamUnreachable),
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW + 5 + 1_800 }),
        ]
    );
}

#[test]
fn an_automatic_sync_reports_that_it_starts_before_it_asks_steam() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let core = computer.start(&steam);
    let mut asked_when_started = None;

    core.sync_automatically(|event| match event {
        AutomaticSync::Started => {
            asked_when_started = Some(steam.asked().len());
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Break(()),
    });

    assert_eq!(asked_when_started, Some(0));
    assert_eq!(steam.asked().len(), 1);
}

#[test]
fn two_syncs_never_run_at_the_same_time() {
    let computer = Computer::new();
    let steam = SteamWebApi::owning(vec![owned(400, "Portal", 95, LAST_WEEK)]);
    let slow = SlowSteam { steam: steam.clone(), running: Arc::default(), most_at_once: Arc::default() };
    let core = computer.start_with_web_api(Box::new(slow.clone()));
    let together = std::sync::Barrier::new(3);

    std::thread::scope(|threads| {
        for _ in 0..3 {
            threads.spawn(|| {
                together.wait();
                core.sync().unwrap()
            });
        }
    });

    assert_eq!(steam.asked().len(), 3);
    assert_eq!(*slow.most_at_once.lock().unwrap(), 1);
}

#[test]
fn what_happens_in_an_automatic_sync_reaches_the_interface_in_the_shape_the_interface_expects() {
    for (event, json) in [
        (AutomaticSync::Skipped, serde_json::json!({ "state": "skipped" })),
        (AutomaticSync::Started, serde_json::json!({ "state": "started" })),
        (
            AutomaticSync::Finished(SyncOutcome::Synced { at: NOW }),
            serde_json::json!({ "state": "finished", "outcome": "synced", "at": NOW }),
        ),
        (
            AutomaticSync::Finished(SyncOutcome::KeyRejected),
            serde_json::json!({ "state": "finished", "outcome": "keyRejected" }),
        ),
    ] {
        assert_eq!(serde_json::to_value(event).unwrap(), json);
    }
}

#[test]
fn an_automatic_sync_that_fails_in_the_app_reaches_the_interface_with_its_reason() {
    let computer = Computer::new();
    let core = Core::new(
        Ports {
            steam_locator: Box::new(SteamAt(Some(computer.steam.path().to_path_buf()))),
            downloader: Box::new(Offline),
            opener: Box::new(NoUrls),
            secrets: Box::new(LockedSecrets),
            web_api: Box::new(SteamWebApi::owning(vec![])),
            clock: Box::new(computer.clock.clone()),
            executable_picker: Box::new(NoExecutableChosen),
            autostart: Box::new(NoAutostart),
            processes: Box::new(NothingRuns),
        },
        Store::open(&computer.profile.path().join("nicelibrary.db")).unwrap(),
        CoverCache::new(computer.profile.path().join("covers")),
    );
    let mut ended = None;

    core.sync_automatically(|event| {
        ended = Some(event);
        ControlFlow::Break(())
    });

    assert_eq!(
        serde_json::to_value(ended.unwrap()).unwrap(),
        serde_json::json!({ "state": "failed", "reason": "secret store: locked" })
    );
}

/// A secret store that cannot be read.
struct LockedSecrets;

impl SecretStore for LockedSecrets {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Err("locked".into())
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("locked".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Err("locked".into())
    }
}

/// Steam as `steam`, but each answer takes a while; remembers how many
/// questions were open at the same time at most.
#[derive(Clone)]
struct SlowSteam {
    steam: SteamWebApi,
    running: Arc<Mutex<u32>>,
    most_at_once: Arc<Mutex<u32>>,
}

impl WebApi for SlowSteam {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        {
            let mut running = self.running.lock().unwrap();
            *running += 1;
            let mut most = self.most_at_once.lock().unwrap();
            *most = (*most).max(*running);
        }
        std::thread::sleep(Duration::from_millis(50));
        let answer = self.steam.get(url);
        *self.running.lock().unwrap() -= 1;
        answer
    }
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// A computer where the app does not start with Windows; these tests do not turn it on.
struct NoAutostart;

impl nicelibrary_core::Autostart for NoAutostart {
    fn is_on(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn turn_on(&self, _arguments: &[&str]) -> Result<(), String> {
        Err("this test does not start the app with Windows".into())
    }

    fn turn_off(&self) -> Result<(), String> {
        Ok(())
    }
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}
