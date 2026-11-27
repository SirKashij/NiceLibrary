//! Limited Mode: when the app cannot read the Steam Account's data from
//! Steam, it says why: no key, key rejected, game details private, or Steam
//! unreachable. The reason holds until a Sync succeeds. Meanwhile Installed
//! Steam Games show the fallback Playtime from Steam's local files.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, ExecutablePicker, LimitedMode, OpenError, Opener, Ports, SecretStore,
    SteamLocator, SteamWebApiKeyCheck, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The most recently used Steam Account on the test computer.
const PLAYER_ONE: &str = "76561197960287930";
/// The other Steam Account on the test computer.
const PLAYER_TWO: &str = "76561197960287931";
/// PLAYER_ONE's account number, the folder name Steam uses under `userdata`.
const PLAYER_ONE_FOLDER: &str = "22202";

/// 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch.
const NOW: u64 = 1_795_608_000;
/// 2026-11-20 20:15:00 UTC.
const LAST_WEEK: u64 = 1_795_205_700;

/// How the fake Steam Web API behaves.
#[derive(Clone, Copy)]
enum Network {
    /// Steam answers; PLAYER_ONE owns Portal 2 with 90 minutes, played last week.
    Online,
    /// Steam refuses the key.
    KeyRejected,
    /// The account's game details are private: Steam answers with an empty response.
    GameDetailsPrivate,
    /// No answer arrives at all.
    Offline,
}

/// A fake of the Steam Web API in the shape Steam answers with, which can
/// change between Syncs.
#[derive(Clone)]
struct SteamWebApi(Arc<Mutex<Network>>);

impl SteamWebApi {
    fn new(network: Network) -> Self {
        Self(Arc::new(Mutex::new(network)))
    }

    fn becomes(&self, network: Network) {
        *self.0.lock().unwrap() = network;
    }
}

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        let answer = |status, text: &str| Ok(WebAnswer { status, body: text.as_bytes().to_vec() });
        match *self.0.lock().unwrap() {
            Network::Offline => Err("no connection to the server".into()),
            Network::KeyRejected => answer(403, "<html><body>Forbidden</body></html>"),
            Network::GameDetailsPrivate => answer(200, r#"{"response":{}}"#),
            Network::Online if url.contains("IStoreService/GetAppList") => {
                answer(200, r#"{"response":{"apps":[{"appid":10,"name":"Counter-Strike"}]}}"#)
            }
            Network::Online if url.contains(&format!("steamid={PLAYER_ONE}")) => answer(
                200,
                &format!(
                    r#"{{"response":{{"game_count":1,"games":[{{"appid":620,"name":"Portal 2","playtime_forever":90,"rtime_last_played":{LAST_WEEK}}}]}}}}"#
                ),
            ),
            Network::Online => answer(200, r#"{"response":{"game_count":0}}"#),
        }
    }
}

/// A substitute for the operating system's secret store, kept in memory.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<HashMap<String, String>>>);

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

/// A clock that stands at NOW.
struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW)
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// A substitute for the registry lookup: Steam is in the fixture.
struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// No Covers are needed here.
struct NoDownloads;

impl Downloader for NoDownloads {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A computer that remembers every URL it was asked to open.
#[derive(Clone, Default)]
struct Browser(Arc<Mutex<Vec<String>>>);

impl Opener for Browser {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        self.0.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// These tests add no Local Game.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// These tests do not start the app with Windows.
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

/// The computer the app runs on: Steam with PLAYER_ONE and PLAYER_TWO,
/// Portal 2 Installed, and the user profile.
struct Computer {
    steam: TempDir,
    profile: TempDir,
    secrets: Secrets,
    browser: Browser,
}

impl Computer {
    fn new() -> Self {
        let computer = Self {
            steam: tempfile::tempdir().unwrap(),
            profile: tempfile::tempdir().unwrap(),
            secrets: Secrets::default(),
            browser: Browser::default(),
        };
        let steam = computer.steam.path();
        fs::create_dir_all(steam.join("config")).unwrap();
        fs::write(
            steam.join("config").join("loginusers.vdf"),
            format!(
                r#""users" {{ "{PLAYER_ONE}" {{ "PersonaName" "One" "MostRecent" "1" }} "{PLAYER_TWO}" {{ "PersonaName" "Two" "MostRecent" "0" }} }}"#
            ),
        )
        .unwrap();
        fs::create_dir_all(steam.join("steamapps")).unwrap();
        for (app_id, name) in [(620, "Portal 2"), (400, "Portal")] {
            fs::write(
                steam.join("steamapps").join(format!("appmanifest_{app_id}.acf")),
                format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n}}\n"),
            )
            .unwrap();
        }
        computer
    }

    fn with_key(self) -> Self {
        self.secrets.write("steam-web-api-key", KEY).unwrap();
        self
    }

    /// Writes PLAYER_ONE's `localconfig.vdf` as Steam does, with `text`
    /// as the block of apps.
    fn local_config(&self, apps: &str) {
        let config = self.steam.path().join("userdata").join(PLAYER_ONE_FOLDER).join("config");
        fs::create_dir_all(&config).unwrap();
        fs::write(
            config.join("localconfig.vdf"),
            format!(
                "\"UserLocalConfigStore\"\n{{\n\t\"Software\"\n\t{{\n\t\t\"Valve\"\n\t\t{{\n\t\t\t\"Steam\"\n\t\t\t{{\n\t\t\t\t\"apps\"\n\t\t\t\t{{\n{apps}\n\t\t\t\t}}\n\t\t\t}}\n\t\t}}\n\t}}\n}}\n"
            ),
        )
        .unwrap();
    }

    fn start(&self, api: &SteamWebApi) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(self.steam.path().to_path_buf())),
                downloader: Box::new(NoDownloads),
                opener: Box::new(self.browser.clone()),
                secrets: Box::new(self.secrets.clone()),
                web_api: Box::new(api.clone()),
                clock: Box::new(StoppedClock),
                autostart: Box::new(NoAutostart),
                executable_picker: Box::new(NoExecutableChosen),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        )
    }

    fn opened(&self) -> Vec<String> {
        self.browser.0.lock().unwrap().clone()
    }
}

/// Playtime and Last Played of the Game `game_id` as the Library shows it.
fn played(core: &Core, game_id: &str) -> (Option<u32>, Option<u64>) {
    let game = core.library().unwrap().into_iter().find(|game| game.id == game_id).expect("game in the library");
    (game.playtime_minutes, game.last_played)
}

#[test]
fn without_a_key_the_app_runs_in_limited_mode_for_want_of_a_key() {
    let computer = Computer::new();
    let core = computer.start(&SteamWebApi::new(Network::Online));

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::NoKey);
}

#[test]
fn with_a_key_and_no_failed_sync_the_app_does_not_run_in_limited_mode() {
    let computer = Computer::new().with_key();
    let core = computer.start(&SteamWebApi::new(Network::Online));

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::Off);
}

#[test]
fn a_sync_that_steam_refuses_puts_the_app_in_limited_mode_with_that_reason() {
    for (network, reason) in [
        (Network::KeyRejected, LimitedMode::KeyRejected),
        (Network::GameDetailsPrivate, LimitedMode::GameDetailsPrivate),
        (Network::Offline, LimitedMode::SteamUnreachable),
    ] {
        let computer = Computer::new().with_key();
        let core = computer.start(&SteamWebApi::new(network));

        core.sync().unwrap();

        assert_eq!(core.limited_mode().unwrap(), reason);
    }
}

#[test]
fn limited_mode_ends_once_a_sync_succeeds() {
    let api = SteamWebApi::new(Network::GameDetailsPrivate);
    let computer = Computer::new().with_key();
    let core = computer.start(&api);
    core.sync().unwrap();

    api.becomes(Network::Online);
    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW });

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::Off);
}

#[test]
fn the_reason_survives_a_restart() {
    let computer = Computer::new().with_key();
    let api = SteamWebApi::new(Network::KeyRejected);
    computer.start(&api).sync().unwrap();

    let core = computer.start(&api);

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::KeyRejected);
}

#[test]
fn removing_the_key_names_the_missing_key_rather_than_the_last_failed_sync() {
    let computer = Computer::new().with_key();
    let core = computer.start(&SteamWebApi::new(Network::Offline));
    core.sync().unwrap();

    core.remove_steam_web_api_key().unwrap();

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::NoKey);
}

#[test]
fn storing_a_key_that_steam_accepts_ends_a_rejected_keys_limited_mode() {
    let api = SteamWebApi::new(Network::KeyRejected);
    let computer = Computer::new().with_key();
    let core = computer.start(&api);
    core.sync().unwrap();

    api.becomes(Network::Online);
    assert_eq!(core.store_steam_web_api_key(KEY).unwrap(), SteamWebApiKeyCheck::Stored);

    assert_eq!(core.limited_mode().unwrap(), LimitedMode::Off);
}

#[test]
fn the_reason_belongs_to_the_steam_account_whose_sync_failed() {
    let computer = Computer::new().with_key();
    let core = computer.start(&SteamWebApi::new(Network::GameDetailsPrivate));
    core.sync().unwrap();

    core.choose_steam_account(PLAYER_TWO).unwrap();
    assert_eq!(core.limited_mode().unwrap(), LimitedMode::Off);

    core.choose_steam_account(PLAYER_ONE).unwrap();
    assert_eq!(core.limited_mode().unwrap(), LimitedMode::GameDetailsPrivate);
}

#[test]
fn installed_steam_games_show_the_fallback_playtime_from_steams_local_files() {
    let computer = Computer::new();
    computer.local_config(&format!(
        "\"620\" {{ \"LastPlayed\" \"{LAST_WEEK}\" \"Playtime\" \"75\" \"cloud\" {{ \"last_sync_state\" \"synchronized\" }} }}\n\"400\" {{ \"Playtime\" \"0\" }}\n\"10\" {{ \"Playtime\" \"5\" }}"
    ));
    let core = computer.start(&SteamWebApi::new(Network::Online));

    core.read_installed_games().unwrap();

    assert_eq!(played(&core, "steam:620"), (Some(75), Some(LAST_WEEK)));
    assert_eq!(played(&core, "steam:400"), (Some(0), None));
    // Only Installed Steam Games come from the local files.
    assert!(core.library().unwrap().iter().all(|game| game.id != "steam:10"));
}

#[test]
fn keys_in_steams_local_files_are_read_regardless_of_case() {
    let computer = Computer::new();
    let config = computer.steam.path().join("userdata").join(PLAYER_ONE_FOLDER).join("config");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("localconfig.vdf"),
        r#""UserLocalConfigStore" { "Software" { "valve" { "Steam" { "Apps" { "620" { "playtime" "42" } } } } } }"#,
    )
    .unwrap();
    let core = computer.start(&SteamWebApi::new(Network::Online));

    core.read_installed_games().unwrap();

    assert_eq!(played(&core, "steam:620"), (Some(42), None));
}

#[test]
fn without_local_files_the_playtime_of_an_installed_steam_game_stays_unknown() {
    let computer = Computer::new();
    let core = computer.start(&SteamWebApi::new(Network::Online));

    core.read_installed_games().unwrap();

    assert_eq!(played(&core, "steam:620"), (None, None));
}

#[test]
fn steams_figures_from_a_sync_take_the_place_of_the_fallback_playtime() {
    let computer = Computer::new().with_key();
    computer.local_config("\"620\" { \"Playtime\" \"75\" }");
    let core = computer.start(&SteamWebApi::new(Network::Online));
    core.read_installed_games().unwrap();

    core.sync().unwrap();

    assert_eq!(played(&core, "steam:620"), (Some(90), Some(LAST_WEEK)));
}

#[test]
fn opens_the_privacy_settings_of_the_steam_profile_in_the_browser() {
    let computer = Computer::new();
    let core = computer.start(&SteamWebApi::new(Network::Online));

    core.open_steam_privacy_settings().unwrap();

    assert_eq!(computer.opened(), ["https://steamcommunity.com/my/edit/settings"]);
}
