//! Filtering the Library: by Source, to Installed Games only, and by a search
//! in the title. The filters combine with each other and with the search; the
//! core answers them, not the interface.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, LibraryFilter, OpenError, Opener, Ports, SecretStore, Source, SteamLocator,
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
                processes: Box::new(NothingRuns),
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

/// The titles of the Games `core` answers for `filter`, in the order it answers them.
fn titles(core: &Core, filter: LibraryFilter) -> Vec<String> {
    core.filter_library(&filter).unwrap().into_iter().map(|game| game.title).collect()
}

#[test]
fn a_filter_that_filters_nothing_answers_the_whole_library_ordered_by_title() {
    let core = Computer::new().start();

    assert_eq!(titles(&core, LibraryFilter::default()), ["Hades", "Half-Life 2", "Portal 2"]);
}

#[test]
fn filtering_by_source_answers_only_the_games_of_that_source() {
    let core = Computer::new().start();

    let steam = LibraryFilter { source: Some(Source::Steam), ..Default::default() };
    assert_eq!(titles(&core, steam), ["Hades", "Half-Life 2", "Portal 2"]);
    // There are no Local Games yet.
    let local = LibraryFilter { source: Some(Source::Local), ..Default::default() };
    assert!(titles(&core, local).is_empty());
}

#[test]
fn filtering_to_installed_games_leaves_out_the_games_that_are_not_installed() {
    let core = Computer::new().start();

    assert_eq!(titles(&core, LibraryFilter { installed_only: true, ..Default::default() }), ["Hades", "Portal 2"]);
}

#[test]
fn the_filters_combine_with_each_other_and_with_the_search() {
    let core = Computer::new().start();
    let filter = |search: &str, source, installed_only| LibraryFilter { search: search.into(), source, installed_only };

    // "ha" is in Hades and Half-Life 2; only Hades is Installed.
    assert_eq!(titles(&core, filter("HA", None, false)), ["Hades", "Half-Life 2"]);
    assert_eq!(titles(&core, filter("ha", Some(Source::Steam), true)), ["Hades"]);
    assert!(titles(&core, filter("ha", Some(Source::Local), true)).is_empty());
    assert!(titles(&core, filter("half", None, true)).is_empty());
}

#[test]
fn a_filter_arrives_from_the_interface_in_the_shape_the_interface_sends() {
    let sent = serde_json::json!({ "search": "ha", "source": "local", "installedOnly": true });
    let filter: LibraryFilter = serde_json::from_value(sent).unwrap();
    assert_eq!(filter, LibraryFilter { search: "ha".into(), source: Some(Source::Local), installed_only: true });

    let for_both_sources = serde_json::json!({ "search": "", "source": null, "installedOnly": false });
    assert_eq!(serde_json::from_value::<LibraryFilter>(for_both_sources).unwrap(), LibraryFilter::default());
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}
