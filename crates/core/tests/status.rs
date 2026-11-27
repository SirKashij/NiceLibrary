//! The Status of a Game: the person's own judgement of where they stand with
//! it (Unplayed, Playing, Completed or Dropped). A new Game starts as
//! Unplayed; the person sets it by hand, the Library and the detail page show
//! it, and the Library can be filtered by it. Status is independent of
//! Progress and Playtime, and a Sync never changes it.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, Downloader, ExecutablePicker, LibraryFilter, OpenError, Opener, Ports,
    SecretStore, Source, Status, SteamLocator, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
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

/// The user cancels every file dialog; Local Games are added by their path here.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
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

    /// Starts the app on this computer's profile and reads what is Installed.
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
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                autostart: Box::new(NoAutostart),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        );
        core.read_installed_games().unwrap();
        core
    }

    /// Starts the app and syncs, so the Library holds all three Games.
    fn start_synced(&self) -> Core {
        let core = self.start();
        assert!(matches!(core.sync().unwrap(), SyncOutcome::Synced { .. }));
        core
    }
}

impl Computer {
    /// Puts an executable into this computer's profile and adds it as a Local Game titled `title`.
    fn add_local_game(&self, core: &Core, title: &str) -> String {
        let executable = self.profile.path().join(format!("{title}.exe"));
        fs::write(&executable, b"MZ").unwrap();
        let AddLocalGame::Added { game } = core.add_local_game(&executable.display().to_string(), title).unwrap()
        else {
            panic!("{title} was not added");
        };
        game.id
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

/// The Status of the Game `game_id` as the Library shows it.
fn status_in_library(core: &Core, game_id: &str) -> Status {
    core.library().unwrap().into_iter().find(|game| game.id == game_id).expect("in the library").status
}

#[test]
fn a_new_game_starts_as_unplayed() {
    let core = Computer::new().start_synced();

    for game in core.library().unwrap() {
        assert_eq!(game.status, Status::Unplayed, "{}", game.title);
    }
}

#[test]
fn a_status_set_by_hand_shows_in_the_library_and_on_the_game_and_survives_a_restart() {
    let computer = Computer::new();
    let core = computer.start_synced();

    let portal = core.set_status("steam:620", Status::Playing).unwrap();
    assert_eq!((portal.id.as_str(), portal.status), ("steam:620", Status::Playing));
    assert_eq!(status_in_library(&core, "steam:620"), Status::Playing);
    assert_eq!(core.game("steam:620").unwrap().unwrap().status, Status::Playing);
    // The other Games keep theirs.
    assert_eq!(status_in_library(&core, "steam:220"), Status::Unplayed);

    core.set_status("steam:620", Status::Completed).unwrap();
    drop(core);
    let core = computer.start();
    assert_eq!(status_in_library(&core, "steam:620"), Status::Completed);
}

#[test]
fn every_status_can_be_set_also_back_to_unplayed() {
    let core = Computer::new().start_synced();

    for status in [Status::Playing, Status::Completed, Status::Dropped, Status::Unplayed] {
        core.set_status("steam:1145360", status).unwrap();
        assert_eq!(status_in_library(&core, "steam:1145360"), status);
    }
}

#[test]
fn a_status_can_be_set_only_on_a_game_in_the_library() {
    let core = Computer::new().start_synced();

    assert!(core.set_status("steam:400", Status::Playing).is_err());
    assert!(core.library().unwrap().iter().all(|game| game.status == Status::Unplayed));
}

#[test]
fn a_sync_never_changes_a_status() {
    let core = Computer::new().start_synced();
    core.set_status("steam:620", Status::Dropped).unwrap();
    // Half-Life 2 has never been played, Hades has been: neither changes what the person set.
    core.set_status("steam:220", Status::Completed).unwrap();

    assert!(matches!(core.sync().unwrap(), SyncOutcome::Synced { .. }));
    core.read_installed_games().unwrap();

    assert_eq!(status_in_library(&core, "steam:620"), Status::Dropped);
    assert_eq!(status_in_library(&core, "steam:220"), Status::Completed);
    assert_eq!(status_in_library(&core, "steam:1145360"), Status::Unplayed);
}

/// The titles of the Games `core` answers for `filter`, in the order it answers them.
fn titles(core: &Core, filter: LibraryFilter) -> Vec<String> {
    core.filter_library(&filter).unwrap().into_iter().map(|game| game.title).collect()
}

#[test]
fn filtering_by_status_answers_only_the_games_with_that_status() {
    let core = Computer::new().start_synced();
    core.set_status("steam:620", Status::Playing).unwrap();
    core.set_status("steam:1145360", Status::Playing).unwrap();
    core.set_status("steam:220", Status::Dropped).unwrap();

    let by = |status| LibraryFilter { status: Some(status), ..Default::default() };
    assert_eq!(titles(&core, by(Status::Playing)), ["Hades", "Portal 2"]);
    assert_eq!(titles(&core, by(Status::Dropped)), ["Half-Life 2"]);
    assert!(titles(&core, by(Status::Unplayed)).is_empty());
    assert!(titles(&core, by(Status::Completed)).is_empty());
    assert_eq!(titles(&core, LibraryFilter::default()), ["Hades", "Half-Life 2", "Portal 2"]);
}

#[test]
fn the_status_filter_combines_with_the_other_filters_and_the_search() {
    let core = Computer::new().start_synced();
    // Hades and Half-Life 2 are Playing; only Hades is Installed.
    core.set_status("steam:1145360", Status::Playing).unwrap();
    core.set_status("steam:220", Status::Playing).unwrap();
    let playing = |search: &str, source, installed_only| LibraryFilter {
        search: search.into(),
        source,
        installed_only,
        status: Some(Status::Playing),
        ..Default::default()
    };

    assert_eq!(titles(&core, playing("", None, false)), ["Hades", "Half-Life 2"]);
    assert_eq!(titles(&core, playing("", None, true)), ["Hades"]);
    assert_eq!(titles(&core, playing("half", Some(Source::Steam), false)), ["Half-Life 2"]);
    assert!(titles(&core, playing("portal", None, false)).is_empty());
    assert!(titles(&core, playing("", Some(Source::Local), false)).is_empty());
}

#[test]
fn a_status_and_its_filter_cross_the_seam_in_the_shape_the_interface_uses() {
    let sent = serde_json::json!({ "search": "", "source": null, "installedOnly": false, "status": "completed" });
    let filter: LibraryFilter = serde_json::from_value(sent).unwrap();
    assert_eq!(filter, LibraryFilter { status: Some(Status::Completed), ..Default::default() });
    // An interface that sends no Status filters by none.
    let without = serde_json::json!({ "search": "", "source": null, "installedOnly": false });
    assert_eq!(serde_json::from_value::<LibraryFilter>(without).unwrap(), LibraryFilter::default());

    for (status, text) in [
        (Status::Unplayed, "unplayed"),
        (Status::Playing, "playing"),
        (Status::Completed, "completed"),
        (Status::Dropped, "dropped"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<Status>(serde_json::json!(text)).unwrap(), status);
    }
    let core = Computer::new().start_synced();
    let game = serde_json::to_value(core.game("steam:620").unwrap().unwrap()).unwrap();
    assert_eq!(game["status"], "unplayed");
}

#[test]
fn a_local_game_starts_as_unplayed_and_takes_a_status_like_a_steam_game() {
    let computer = Computer::new();
    let core = computer.start_synced();
    let celeste = computer.add_local_game(&core, "Celeste");
    assert_eq!(status_in_library(&core, &celeste), Status::Unplayed);

    core.set_status(&celeste, Status::Completed).unwrap();
    core.set_status("steam:620", Status::Completed).unwrap();

    assert_eq!(status_in_library(&core, &celeste), Status::Completed);
    let completed = LibraryFilter { status: Some(Status::Completed), ..Default::default() };
    assert_eq!(titles(&core, completed), ["Celeste", "Portal 2"]);
    let local_and_completed =
        LibraryFilter { source: Some(Source::Local), status: Some(Status::Completed), ..Default::default() };
    assert_eq!(titles(&core, local_and_completed), ["Celeste"]);
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

/// A computer whose executables show no icon and whose user chooses no
/// image; these tests need neither.
struct NoIconsNoImages;

impl nicelibrary_core::IconExtractor for NoIconsNoImages {
    fn largest_icon(&self, _executable: &std::path::Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}

impl nicelibrary_core::ImagePicker for NoIconsNoImages {
    fn choose_image(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
