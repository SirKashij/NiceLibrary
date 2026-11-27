//! Manual Progress: the user sets a Progress percentage by hand on a Local
//! Game, from 0 to 100, and may clear it again. A Local Game's Progress is its
//! Manual Progress or nothing. Steam Games offer no Manual Progress.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, ExecutablePicker, Game, ManualProgress, OpenError, Opener, Ports,
    SteamLocator, Store,
};
use tempfile::TempDir;

/// A file dialog that answers the executable the test put on the disk.
struct UserChooses(PathBuf);

impl ExecutablePicker for UserChooses {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(Some(self.0.clone()))
    }
}

/// A computer whose secret store holds nothing and whose network reaches no
/// Web API; these tests need neither.
struct NoSecretsNoWebApi;

impl nicelibrary_core::SecretStore for NoSecretsNoWebApi {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}

impl nicelibrary_core::WebApi for NoSecretsNoWebApi {
    fn get(&self, _url: &str) -> Result<nicelibrary_core::WebAnswer, String> {
        Err("this test asks no Web API".into())
    }
}

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A substitute for the registry lookup: Steam is in the fixture folder.
struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A computer where the app does not start with Windows; these tests do not change that.
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
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// One running app with Portal 2 Installed through Steam and the Local Game
/// Celeste added.
struct App {
    core: Core,
    profile: TempDir,
    steam: TempDir,
    _disk: TempDir,
    celeste: PathBuf,
}

impl App {
    fn start() -> Self {
        let steam = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam.path().join("config")).unwrap();
        fs::write(
            steam.path().join("config").join("loginusers.vdf"),
            r#""users" { "76561197960287930" { "PersonaName" "Player" "MostRecent" "1" } }"#,
        )
        .unwrap();
        fs::create_dir_all(steam.path().join("steamapps")).unwrap();
        fs::write(
            steam.path().join("steamapps").join("appmanifest_620.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"name\"\t\t\"Portal 2\"\n\t\"StateFlags\"\t\t\"4\"\n}\n",
        )
        .unwrap();
        let disk = tempfile::tempdir().unwrap();
        let celeste = disk.path().join("Celeste").join("Celeste.exe");
        fs::create_dir_all(celeste.parent().unwrap()).unwrap();
        fs::write(&celeste, b"MZ").unwrap();
        let profile = tempfile::tempdir().unwrap();
        let core = Self::core_in(profile.path(), steam.path(), &celeste);
        core.read_installed_games().unwrap();
        let AddLocalGame::Added { .. } = core.add_local_game(celeste.to_str().unwrap(), "Celeste").unwrap() else {
            panic!("expected Celeste to be added");
        };
        Self { core, profile, steam, _disk: disk, celeste }
    }

    fn core_in(profile: &Path, steam: &Path, celeste: &Path) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(UserChooses(celeste.to_path_buf())),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")),
        )
    }

    /// The app started again over the same profile.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), self.steam.path(), &self.celeste);
        Self { core, ..self }
    }

    fn game(&self, title: &str) -> Game {
        self.core.library().unwrap().into_iter().find(|game| game.title == title).unwrap()
    }
}

#[test]
fn a_local_game_without_manual_progress_has_no_progress() {
    let app = App::start();

    assert_eq!(app.game("Celeste").progress, None);
}

#[test]
fn manual_progress_becomes_the_progress_of_the_local_game_everywhere() {
    let app = App::start();
    let celeste = app.game("Celeste");

    let set = app.core.set_manual_progress(&celeste.id, Some(42)).unwrap();

    let ManualProgress::Set { game } = set else { panic!("expected Manual Progress to be set, got {set:?}") };
    assert_eq!(game.progress, Some(42));
    assert_eq!(app.core.game(&celeste.id).unwrap().unwrap().progress, Some(42));
    assert_eq!(app.game("Celeste").progress, Some(42));
}

#[test]
fn manual_progress_from_0_to_100_is_accepted() {
    let app = App::start();
    let celeste = app.game("Celeste");

    for percent in [0, 1, 99, 100] {
        let set = app.core.set_manual_progress(&celeste.id, Some(percent)).unwrap();
        assert!(matches!(set, ManualProgress::Set { .. }), "{percent}: {set:?}");
        assert_eq!(app.game("Celeste").progress, Some(u8::try_from(percent).unwrap()));
    }
}

#[test]
fn manual_progress_outside_0_to_100_is_refused_and_changes_nothing() {
    let app = App::start();
    let celeste = app.game("Celeste");
    app.core.set_manual_progress(&celeste.id, Some(30)).unwrap();

    for percent in [101, 250, -1, i64::MIN, i64::MAX] {
        assert_eq!(app.core.set_manual_progress(&celeste.id, Some(percent)), Ok(ManualProgress::OutOfRange));
    }
    assert_eq!(app.game("Celeste").progress, Some(30));
}

#[test]
fn manual_progress_can_be_cleared_again() {
    let app = App::start();
    let celeste = app.game("Celeste");
    app.core.set_manual_progress(&celeste.id, Some(80)).unwrap();

    let cleared = app.core.set_manual_progress(&celeste.id, None).unwrap();

    let ManualProgress::Set { game } = cleared else {
        panic!("expected Manual Progress to be cleared, got {cleared:?}")
    };
    assert_eq!(game.progress, None);
    assert_eq!(app.game("Celeste").progress, None);
}

#[test]
fn manual_progress_is_kept_after_a_restart() {
    let app = App::start();
    let celeste = app.game("Celeste");
    app.core.set_manual_progress(&celeste.id, Some(65)).unwrap();

    let app = app.restart();

    assert_eq!(app.game("Celeste").progress, Some(65));
}

#[test]
fn steam_games_offer_no_manual_progress() {
    let app = App::start();

    assert_eq!(app.core.set_manual_progress("steam:620", Some(50)), Ok(ManualProgress::NotALocalGame));
    assert_eq!(app.core.set_manual_progress("steam:620", None), Ok(ManualProgress::NotALocalGame));
    assert_eq!(app.game("Portal 2").progress, None);
}

#[test]
fn manual_progress_of_a_game_that_is_not_in_the_library_fails() {
    let app = App::start();

    assert!(app.core.set_manual_progress("local:999", Some(50)).is_err());
}

#[test]
fn the_outcome_reaches_the_interface_in_the_shape_the_interface_expects() {
    let app = App::start();
    let celeste = app.game("Celeste");

    let set = serde_json::to_value(app.core.set_manual_progress(&celeste.id, Some(7)).unwrap()).unwrap();

    assert_eq!(set["outcome"], "set");
    assert_eq!(set["game"]["id"], celeste.id.as_str());
    assert_eq!(set["game"]["progress"], 7);
    assert_eq!(serde_json::to_value(ManualProgress::OutOfRange).unwrap(), serde_json::json!({ "outcome": "outOfRange" }));
    assert_eq!(
        serde_json::to_value(ManualProgress::NotALocalGame).unwrap(),
        serde_json::json!({ "outcome": "notALocalGame" })
    );
}
