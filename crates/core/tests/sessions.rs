//! While the app runs, it looks at the running processes again and again.
//! When a Local Game's executable runs, however it was started, a Session
//! begins; it lasts for as long as the process is seen, and it is kept up to
//! the last moment the process was seen, so a Session survives the app being
//! closed or crashing while the Game runs. A Local Game's Playtime is the sum
//! of its Sessions, its Last Played the end of the latest one.

use std::collections::VecDeque;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, Downloader, ExecutablePicker, Game, OpenError, Opener, Ports, ProcessList,
    SteamLocator, Store, Tracking, Wakeup,
};
use tempfile::TempDir;

/// The time of day on the test's day, in seconds since the Unix epoch.
fn at(hours: u64, minutes: u64, seconds: u64) -> u64 {
    // 2027-01-15 00:00:00 UTC.
    1_800_000_000 - 1_800_000_000 % 86_400 + hours * 3600 + minutes * 60 + seconds
}

/// A clock the process list sets: each look happens at the time the script says.
#[derive(Clone)]
struct ScriptedClock(Arc<Mutex<SystemTime>>);

impl Clock for ScriptedClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    /// The script decides when the next look happens.
    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// One look at the running processes, as the test scripts it.
struct Look {
    at: u64,
    running: Result<Vec<PathBuf>, String>,
}

/// A substitute for the operating system's process list: answers each look
/// as scripted, at the scripted time. Once the script is over, it fails.
#[derive(Clone)]
struct ScriptedProcesses {
    looks: Arc<Mutex<VecDeque<Look>>>,
    clock: ScriptedClock,
}

impl ScriptedProcesses {
    fn is_over(&self) -> bool {
        self.looks.lock().unwrap().is_empty()
    }
}

impl ProcessList for ScriptedProcesses {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        let Some(look) = self.looks.lock().unwrap().pop_front() else {
            return Err("the script is over".into());
        };
        *self.clock.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(look.at);
        look.running
    }
}

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
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

/// One running app with a disk of its own that games can be put on.
struct App {
    core: Core,
    processes: ScriptedProcesses,
    profile: TempDir,
    disk: TempDir,
}

impl App {
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let processes = ScriptedProcesses {
            looks: Arc::default(),
            clock: ScriptedClock(Arc::new(Mutex::new(UNIX_EPOCH))),
        };
        let core = Self::core_in(profile.path(), &processes);
        Self { core, processes, profile, disk: tempfile::tempdir().unwrap() }
    }

    fn core_in(profile: &Path, processes: &ScriptedProcesses) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(processes.clone()),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")),
        )
    }

    /// The app started again over the same profile, after it was closed or crashed.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.processes);
        Self { core, ..self }
    }

    /// Puts an executable at `relative` (folders separated by `/`) on the disk.
    fn put(&self, relative: &str) -> PathBuf {
        let path = self.on_disk(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// The path `relative` (folders separated by `/`) on the disk, spelled as given.
    fn on_disk(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part))
    }

    /// Puts an executable at `relative` and adds it as a Local Game titled `title`.
    fn add(&self, relative: &str, title: &str) -> (PathBuf, String) {
        let path = self.put(relative);
        let AddLocalGame::Added { game } = self.core.add_local_game(path.to_str().unwrap(), title).unwrap() else {
            panic!("expected {title} to be added");
        };
        (path, game.id)
    }

    /// Scripts the next look at the running processes.
    fn look(&self, at: u64, running: &[&Path]) {
        self.looks(at, Ok(running.iter().map(|path| path.to_path_buf()).collect()));
    }

    fn looks(&self, at: u64, running: Result<Vec<PathBuf>, String>) {
        self.processes.looks.lock().unwrap().push_back(Look { at, running });
    }

    /// Tracks the running processes until the script is over, as if the app
    /// were closed then, and answers what the tracking reported.
    fn track(&self) -> Vec<Tracking> {
        let mut reported = Vec::new();
        self.core.track_sessions(|event| {
            if matches!(event, Tracking::Failed { .. }) && self.processes.is_over() {
                return ControlFlow::Break(());
            }
            reported.push(event);
            ControlFlow::Continue(())
        });
        reported
    }

    fn game(&self, id: &str) -> Game {
        self.core.game(id).unwrap().expect("the game is in the library")
    }
}

#[test]
fn a_session_spans_the_whole_time_the_process_runs() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 20, 0), &[&celeste]);
    app.look(at(10, 45, 0), &[&celeste]);
    app.look(at(11, 0, 0), &[]);

    let reported = app.track();

    assert_eq!(reported, [Tracking::Started { game_id: id.clone() }, Tracking::Ended { game_id: id.clone() }]);
    let game = app.game(&id);
    assert_eq!(game.playtime_minutes, Some(45));
    assert_eq!(game.last_played, Some(at(10, 45, 0)));
}

#[test]
fn playtime_is_the_sum_of_the_sessions_and_last_played_the_end_of_the_latest() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[&celeste]);
    app.look(at(10, 40, 0), &[]);
    app.look(at(12, 0, 0), &[&celeste]);
    app.look(at(12, 15, 0), &[&celeste]);
    app.look(at(12, 20, 0), &[]);

    let reported = app.track();

    assert_eq!(reported.len(), 4, "two Sessions start and end: {reported:?}");
    let game = app.game(&id);
    assert_eq!(game.playtime_minutes, Some(45));
    assert_eq!(game.last_played, Some(at(12, 15, 0)));
    assert_eq!(app.core.library().unwrap(), [game]);
}

#[test]
fn playtime_sums_the_seconds_of_the_sessions_before_counting_whole_minutes() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 0, 40), &[&celeste]);
    app.look(at(10, 1, 0), &[]);
    app.look(at(11, 0, 0), &[&celeste]);
    app.look(at(11, 0, 40), &[&celeste]);
    app.look(at(11, 1, 0), &[]);

    app.track();

    assert_eq!(app.game(&id).playtime_minutes, Some(1));
}

#[test]
fn a_session_is_kept_up_to_the_last_look_when_the_app_stops_while_the_game_runs() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[&celeste]);

    let reported = app.track();
    let app = app.restart();

    assert_eq!(reported, [Tracking::Started { game_id: id.clone() }]);
    let game = app.game(&id);
    assert_eq!(game.playtime_minutes, Some(30));
    assert_eq!(game.last_played, Some(at(10, 30, 0)));
}

#[test]
fn a_game_still_running_after_a_restart_starts_a_new_session() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[&celeste]);
    app.track();
    let app = app.restart();
    app.look(at(11, 0, 0), &[&celeste]);
    app.look(at(11, 10, 0), &[&celeste]);
    app.look(at(11, 11, 0), &[]);

    let reported = app.track();

    assert_eq!(reported, [Tracking::Started { game_id: id.clone() }, Tracking::Ended { game_id: id.clone() }]);
    let game = app.game(&id);
    assert_eq!(game.playtime_minutes, Some(40));
    assert_eq!(game.last_played, Some(at(11, 10, 0)));
}

#[test]
fn the_process_is_recognised_by_any_spelling_of_its_executable_path() {
    let app = App::start();
    let (_, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let spelled_otherwise = app.on_disk("Games/Celeste/../Celeste/./Celeste.exe");
    app.look(at(10, 0, 0), &[&spelled_otherwise]);
    app.look(at(10, 5, 0), &[&spelled_otherwise]);
    app.look(at(10, 6, 0), &[]);

    app.track();

    assert_eq!(app.game(&id).playtime_minutes, Some(5));
}

#[test]
fn other_processes_record_no_session_even_with_the_same_file_name() {
    let app = App::start();
    let (_, id) = app.add("Games/Celeste/game.exe", "Celeste");
    let other = app.put("Games/Other/game.exe");
    app.look(at(10, 0, 0), &[&other]);
    app.look(at(10, 30, 0), &[&other]);
    app.look(at(10, 31, 0), &[]);

    let reported = app.track();

    assert_eq!(reported, []);
    let game = app.game(&id);
    assert_eq!(game.playtime_minutes, Some(0));
    assert_eq!(game.last_played, None);
}

#[test]
fn several_processes_of_one_game_make_one_session() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste, &celeste]);
    app.look(at(10, 30, 0), &[&celeste, &celeste]);
    app.look(at(10, 31, 0), &[]);

    let reported = app.track();

    assert_eq!(reported.len(), 2, "{reported:?}");
    assert_eq!(app.game(&id).playtime_minutes, Some(30));
}

#[test]
fn games_running_at_the_same_time_each_have_a_session() {
    let app = App::start();
    let (celeste, celeste_id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let (hades, hades_id) = app.add("Games/Hades/Hades.exe", "Hades");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 10, 0), &[&celeste, &hades]);
    app.look(at(10, 30, 0), &[&hades]);
    app.look(at(10, 50, 0), &[]);

    app.track();

    assert_eq!(app.game(&celeste_id).playtime_minutes, Some(10));
    assert_eq!(app.game(&celeste_id).last_played, Some(at(10, 10, 0)));
    assert_eq!(app.game(&hades_id).playtime_minutes, Some(20));
    assert_eq!(app.game(&hades_id).last_played, Some(at(10, 30, 0)));
}

#[test]
fn a_game_added_while_the_app_runs_is_tracked_from_the_next_look_on() {
    let app = App::start();
    let (celeste, celeste_id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.put("Games/Hades/Hades.exe");
    app.look(at(10, 0, 0), &[&celeste, &hades]);
    app.look(at(10, 10, 0), &[&hades]);
    app.look(at(10, 30, 0), &[&hades]);
    app.look(at(10, 40, 0), &[]);
    let mut hades_id = None;

    app.core.track_sessions(|event| match event {
        // Hades is added while the first look's Session of Celeste begins.
        Tracking::Started { game_id } if game_id == celeste_id => {
            let added = app.core.add_local_game(hades.to_str().unwrap(), "Hades").unwrap();
            let AddLocalGame::Added { game } = added else { panic!("expected Hades to be added") };
            hades_id = Some(game.id);
            ControlFlow::Continue(())
        }
        Tracking::Failed { .. } if app.processes.is_over() => ControlFlow::Break(()),
        _ => ControlFlow::Continue(()),
    });

    let hades = app.game(&hades_id.expect("Hades was added"));
    assert_eq!(hades.playtime_minutes, Some(20));
    assert_eq!(hades.last_played, Some(at(10, 30, 0)));
}

#[test]
fn a_failed_look_at_the_processes_ends_no_session() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.looks(at(10, 10, 0), Err("access denied".into()));
    app.look(at(10, 20, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[]);

    let reported = app.track();

    assert_eq!(reported.len(), 3, "{reported:?}");
    assert_eq!(reported[0], Tracking::Started { game_id: id.clone() });
    assert!(matches!(&reported[1], Tracking::Failed { reason } if reason.to_string().contains("access denied")));
    assert_eq!(reported[2], Tracking::Ended { game_id: id.clone() });
    assert_eq!(app.game(&id).playtime_minutes, Some(20));
}
