//! The Playtime Adjustment of a Local Game: time the person adds to or
//! removes from its Playtime by hand, for example for time played before the
//! app was installed. The Playtime shown is the sum of the Sessions plus the
//! adjustment, never below zero. The adjustment can be changed or cleared, it
//! survives a restart, and Steam Games, whose Playtime is Steam's figure,
//! have none.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, Downloader, ExecutablePicker, Game, LibraryFilter, LibrarySort, OpenError,
    Opener, Ports, ProcessList, SteamLocator, Store, Tracking, Wakeup,
};
use tempfile::TempDir;

/// The time of day on the test's day, in seconds since the Unix epoch.
fn at(hours: u64, minutes: u64) -> u64 {
    // 2027-01-15 00:00:00 UTC.
    1_800_000_000 - 1_800_000_000 % 86_400 + hours * 3600 + minutes * 60
}

#[test]
fn a_local_game_has_no_playtime_adjustment_until_the_person_sets_one() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");

    assert_eq!(app.core.playtime_adjustment(&id), Ok(None));
    assert_eq!(app.game(&id).playtime_minutes, Some(0));
}

#[test]
fn the_adjustment_is_added_to_the_sessions() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.play(&id, at(10, 0), at(10, 45));

    let game = app.core.set_playtime_adjustment(&id, Some(600)).unwrap();

    assert_eq!(game.playtime_minutes, Some(645));
    assert_eq!(app.game(&id), game);
    assert_eq!(app.core.playtime_adjustment(&id), Ok(Some(600)));
}

#[test]
fn an_adjustment_on_a_game_never_played_is_its_whole_playtime_but_no_last_played() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");

    let game = app.core.set_playtime_adjustment(&id, Some(90)).unwrap();

    assert_eq!(game.playtime_minutes, Some(90));
    assert_eq!(game.last_played, None);
}

#[test]
fn a_negative_adjustment_removes_time_but_the_playtime_never_drops_below_zero() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.play(&id, at(10, 0), at(11, 0));

    assert_eq!(app.core.set_playtime_adjustment(&id, Some(-20)).unwrap().playtime_minutes, Some(40));
    assert_eq!(app.core.set_playtime_adjustment(&id, Some(-500)).unwrap().playtime_minutes, Some(0));
    assert_eq!(app.core.playtime_adjustment(&id), Ok(Some(-500)), "the adjustment is kept as entered");
}

#[test]
fn the_adjustment_can_be_changed_and_cleared() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.play(&id, at(10, 0), at(10, 30));
    app.core.set_playtime_adjustment(&id, Some(100)).unwrap();

    let changed = app.core.set_playtime_adjustment(&id, Some(200)).unwrap();
    assert_eq!(changed.playtime_minutes, Some(230));
    assert_eq!(app.core.playtime_adjustment(&id), Ok(Some(200)));

    let cleared = app.core.set_playtime_adjustment(&id, None).unwrap();
    assert_eq!(cleared.playtime_minutes, Some(30));
    assert_eq!(app.core.playtime_adjustment(&id), Ok(None));
}

#[test]
fn an_adjustment_of_zero_clears_it() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.core.set_playtime_adjustment(&id, Some(100)).unwrap();

    app.core.set_playtime_adjustment(&id, Some(0)).unwrap();

    assert_eq!(app.core.playtime_adjustment(&id), Ok(None));
}

#[test]
fn the_adjustment_survives_a_restart_and_counts_with_later_sessions() {
    let app = App::start();
    let id = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.core.set_playtime_adjustment(&id, Some(120)).unwrap();

    let app = app.restart();
    app.play(&id, at(10, 0), at(10, 15));

    assert_eq!(app.core.playtime_adjustment(&id), Ok(Some(120)));
    assert_eq!(app.game(&id).playtime_minutes, Some(135));
}

#[test]
fn the_adjustment_belongs_to_one_game_only() {
    let app = App::start();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.add("Games/Hades/Hades.exe", "Hades");

    app.core.set_playtime_adjustment(&celeste, Some(60)).unwrap();

    assert_eq!(app.game(&hades).playtime_minutes, Some(0));
    assert_eq!(app.core.playtime_adjustment(&hades), Ok(None));
}

#[test]
fn the_adjusted_playtime_orders_the_library_by_playtime() {
    let app = App::start();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.add("Games/Hades/Hades.exe", "Hades");
    app.play(&celeste, at(10, 0), at(11, 0));
    app.core.set_playtime_adjustment(&hades, Some(90)).unwrap();

    let games = app.core.filter_library(&LibraryFilter { sort: LibrarySort::Playtime, ..Default::default() }).unwrap();

    let order: Vec<_> = games.into_iter().map(|game| game.id).collect();
    assert_eq!(order, [hades, celeste]);
}

#[test]
fn steam_games_offer_no_adjustment() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start_with(Some(&steam));
    app.core.read_installed_games().unwrap();

    assert_eq!(app.core.playtime_adjustment("steam:620"), Ok(None));
    assert!(app.core.set_playtime_adjustment("steam:620", Some(60)).is_err());
    assert_eq!(app.game("steam:620").playtime_minutes, None);
}

#[test]
fn a_game_that_is_not_in_the_library_cannot_be_adjusted() {
    let app = App::start();

    assert!(app.core.set_playtime_adjustment("local:7", Some(60)).is_err());
    assert!(app.core.library().unwrap().is_empty());
}

/// A clock the process list sets: each look happens at the time the script says.
#[derive(Clone)]
struct ScriptedClock(Arc<Mutex<SystemTime>>);

impl Clock for ScriptedClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// A substitute for the operating system's process list: answers each look
/// as scripted, at the scripted time. Once the script is over, it fails.
#[derive(Clone)]
struct ScriptedProcesses {
    looks: Arc<Mutex<VecDeque<(u64, Vec<PathBuf>)>>>,
    clock: ScriptedClock,
}

impl ProcessList for ScriptedProcesses {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        let Some((at, running)) = self.looks.lock().unwrap().pop_front() else {
            return Err("the script is over".into());
        };
        *self.clock.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(at);
        Ok(running)
    }
}

/// Steam on disk, as the test sets it up, at the path it names.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// Steam on disk with one Steam Account that has logged in.
struct SteamFixture(TempDir);

impl SteamFixture {
    fn new() -> Self {
        let steam = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam.path().join("config")).unwrap();
        fs::write(
            steam.path().join("config").join("loginusers.vdf"),
            r#""users" { "76561197960287930" { "PersonaName" "Player" "MostRecent" "1" } }"#,
        )
        .unwrap();
        Self(steam)
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        let steamapps = self.0.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
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

/// No file dialog is needed here; Games are added by path.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
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

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// A computer whose executables show no icon and whose user chooses no
/// image; these tests need neither.
struct NoIconsNoImages;

impl nicelibrary_core::IconExtractor for NoIconsNoImages {
    fn largest_icon(&self, _executable: &Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}

impl nicelibrary_core::ImagePicker for NoIconsNoImages {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// One running app with a disk of its own that games can be put on.
struct App {
    core: Core,
    processes: ScriptedProcesses,
    steam: Option<PathBuf>,
    profile: TempDir,
    disk: TempDir,
    /// The executable of each Local Game added, by its id.
    executables: RefCell<HashMap<String, PathBuf>>,
}

impl App {
    fn start() -> Self {
        Self::start_with(None)
    }

    fn start_with(steam: Option<&SteamFixture>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let processes = ScriptedProcesses {
            looks: Arc::default(),
            clock: ScriptedClock(Arc::new(Mutex::new(UNIX_EPOCH))),
        };
        let steam = steam.map(|steam| steam.0.path().to_path_buf());
        let core = Self::core_in(profile.path(), &processes, steam.clone());
        Self { core, processes, steam, profile, disk: tempfile::tempdir().unwrap(), executables: RefCell::default() }
    }

    fn core_in(profile: &Path, processes: &ScriptedProcesses, steam: Option<PathBuf>) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam)),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(processes.clone()),
                autostart: Box::new(NoAutostart),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                export_file_picker: Box::new(NoExportFile),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")),
        )
    }

    /// The app started again over the same profile.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.processes, self.steam.clone());
        Self { core, ..self }
    }

    /// Puts an executable at `relative` (folders separated by `/`) on the
    /// disk, adds it as a Local Game titled `title` and answers its id.
    fn add(&self, relative: &str, title: &str) -> String {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        let AddLocalGame::Added { game } = self.core.add_local_game(path.to_str().unwrap(), title).unwrap() else {
            panic!("expected {title} to be added");
        };
        self.executables.borrow_mut().insert(game.id.clone(), path);
        game.id
    }

    /// Records one Session of the Local Game `id` from `from` to `to`: its
    /// process is seen at both times and gone right after.
    fn play(&self, id: &str, from: u64, to: u64) {
        let executable = self.executables.borrow()[id].clone();
        {
            let mut looks = self.processes.looks.lock().unwrap();
            looks.push_back((from, vec![executable.clone()]));
            looks.push_back((to, vec![executable]));
            looks.push_back((to + 1, Vec::new()));
        }
        self.core.track_sessions(|event| {
            if matches!(event, Tracking::Failed { .. }) {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
    }

    fn game(&self, id: &str) -> Game {
        self.core.game(id).unwrap().expect("the game is in the library")
    }
}

/// No export or import is made here: the user cancels every file dialog.
struct NoExportFile;

impl nicelibrary_core::ExportFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }

    fn choose_import_file(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
