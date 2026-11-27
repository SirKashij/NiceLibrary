//! Hidden Games: a Game the user hides leaves the Library, its search
//! results and every view built on it, without being deleted. The Hidden
//! Games are listed on their own and can be unhidden; hiding survives Syncs
//! and restarts.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, Game, LibraryFilter, OpenError, Opener, Ports, SecretStore, SteamLocator,
    Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account logged in on the test computer.
const PLAYER: &str = "76561197960287930";

/// A Steam Web API where PLAYER owns Portal 2, Half-Life 2 and Hades, and a
/// secret store that holds the key.
struct SteamOwning;

impl WebApi for SteamOwning {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        assert!(url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?"), "asked for {url}");
        let body = serde_json::json!({ "response": { "game_count": 3, "games": [
            { "appid": 620, "name": "Portal 2", "playtime_forever": 600, "rtime_last_played": 0 },
            { "appid": 220, "name": "Half-Life 2", "playtime_forever": 0, "rtime_last_played": 0 },
            { "appid": 1145360, "name": "Hades", "playtime_forever": 90, "rtime_last_played": 0 },
        ] } });
        Ok(WebAnswer { status: 200, body: body.to_string().into_bytes() })
    }
}

impl SecretStore for SteamOwning {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(Some(KEY.into()))
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Err("this test removes no secret".into())
    }
}

struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
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

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &Wakeup) {}
}

/// The test computer: Steam with PLAYER logged in, Portal 2 and Hades
/// Installed, Half-Life 2 owned but not Installed.
struct Computer {
    profile: TempDir,
    steam: TempDir,
}

impl Computer {
    fn new() -> Self {
        let steam = tempfile::tempdir().unwrap();
        let path = steam.path();
        fs::create_dir_all(path.join("config")).unwrap();
        fs::write(
            path.join("config").join("loginusers.vdf"),
            format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "One" "MostRecent" "1" }} }}"#),
        )
        .unwrap();
        fs::create_dir_all(path.join("steamapps")).unwrap();
        let library = path.display().to_string().replace('\\', "\\\\");
        fs::write(
            path.join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{library}\"\n\t}}\n}}\n"),
        )
        .unwrap();
        for (app_id, name) in [(620, "Portal 2"), (1145360, "Hades")] {
            install(path, app_id, name);
        }
        Self { profile: tempfile::tempdir().unwrap(), steam }
    }

    /// Starts the app, reads what is Installed and syncs, so the Library holds all three Games.
    fn start(&self) -> Core {
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(self.steam.path().to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(SteamOwning),
                web_api: Box::new(SteamOwning),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        );
        core.read_installed_games().unwrap();
        assert!(matches!(core.sync().unwrap(), SyncOutcome::Synced { .. }));
        core
    }
}

/// Writes an app manifest as Steam does for a fully installed app.
fn install(steam: &Path, app_id: u32, name: &str) {
    fs::write(
        steam.join("steamapps").join(format!("appmanifest_{app_id}.acf")),
        format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
        ),
    )
    .unwrap();
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// The titles of the Games in `games`, in the order they are answered.
fn titles(games: Vec<Game>) -> Vec<String> {
    games.into_iter().map(|game| game.title).collect()
}

#[test]
fn a_fresh_library_has_no_hidden_games() {
    let core = Computer::new().start();

    assert!(core.hidden_games().unwrap().is_empty());
}

#[test]
fn a_hidden_game_leaves_the_library_and_is_listed_among_the_hidden_games() {
    let core = Computer::new().start();

    core.hide_game("steam:220").unwrap();

    assert_eq!(titles(core.library().unwrap()), ["Hades", "Portal 2"]);
    assert_eq!(titles(core.hidden_games().unwrap()), ["Half-Life 2"]);
}

#[test]
fn a_hidden_game_is_absent_from_search_results_and_has_no_detail_page() {
    let core = Computer::new().start();

    core.hide_game("steam:220").unwrap();

    let search = LibraryFilter { search: "ha".into(), ..Default::default() };
    assert_eq!(titles(core.filter_library(&search).unwrap()), ["Hades"]);
    assert_eq!(core.game("steam:220").unwrap(), None);
}

#[test]
fn the_hidden_games_are_ordered_by_title() {
    let core = Computer::new().start();

    core.hide_game("steam:620").unwrap();
    core.hide_game("steam:1145360").unwrap();

    assert_eq!(titles(core.hidden_games().unwrap()), ["Hades", "Portal 2"]);
    assert_eq!(titles(core.library().unwrap()), ["Half-Life 2"]);
}

#[test]
fn an_unhidden_game_is_back_in_the_library_as_it_was() {
    let core = Computer::new().start();
    let before = core.library().unwrap();

    core.hide_game("steam:620").unwrap();
    core.unhide_game("steam:620").unwrap();

    assert_eq!(core.library().unwrap(), before);
    assert!(core.hidden_games().unwrap().is_empty());
}

#[test]
fn hiding_a_hidden_game_again_or_unhiding_a_game_that_is_not_hidden_changes_nothing() {
    let core = Computer::new().start();

    core.hide_game("steam:220").unwrap();
    core.hide_game("steam:220").unwrap();
    core.unhide_game("steam:620").unwrap();

    assert_eq!(titles(core.hidden_games().unwrap()), ["Half-Life 2"]);
    assert_eq!(titles(core.library().unwrap()), ["Hades", "Portal 2"]);
}

#[test]
fn a_game_that_is_not_in_the_library_cannot_be_hidden() {
    let core = Computer::new().start();

    assert!(core.hide_game("steam:4000").is_err());
    assert!(core.hidden_games().unwrap().is_empty());
}

#[test]
fn a_game_stays_hidden_after_a_sync_and_after_reading_the_installed_games() {
    let core = Computer::new().start();

    core.hide_game("steam:620").unwrap();
    assert!(matches!(core.sync().unwrap(), SyncOutcome::Synced { .. }));
    core.read_installed_games().unwrap();

    assert_eq!(titles(core.library().unwrap()), ["Hades", "Half-Life 2"]);
    assert_eq!(titles(core.hidden_games().unwrap()), ["Portal 2"]);
}

#[test]
fn a_game_stays_hidden_after_a_restart() {
    let computer = Computer::new();
    computer.start().hide_game("steam:620").unwrap();

    let restarted = computer.start();

    assert_eq!(titles(restarted.library().unwrap()), ["Hades", "Half-Life 2"]);
    assert_eq!(titles(restarted.hidden_games().unwrap()), ["Portal 2"]);
}
