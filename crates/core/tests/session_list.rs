//! The detail page lists a Game's Sessions, newest first, each with when it
//! began and how long it lasted. A Session whose process still runs is marked
//! as running; once the process is gone, or the app stopped tracking, it no
//! longer is.

use std::collections::VecDeque;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{AddLocalGame, Clock, Core, CoverCache, Ports, ProcessList, Session, Store, Tracking, Wakeup};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

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
    running: Vec<PathBuf>,
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
        Ok(look.running)
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
                clock: Box::new(processes.clock.clone()),
                processes: Box::new(processes.clone()),
                ..stand_ins::ports()
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

    /// Puts an executable at `relative` (folders separated by `/`) on the disk
    /// and adds it as a Local Game titled `title`.
    fn add(&self, relative: &str, title: &str) -> (PathBuf, String) {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        let AddLocalGame::Added { game } = self.core.add_local_game(path.to_str().unwrap(), title).unwrap() else {
            panic!("expected {title} to be added");
        };
        (path, game.id)
    }

    /// Scripts the next look at the running processes.
    fn look(&self, at: u64, running: &[&Path]) {
        let running = running.iter().map(|path| path.to_path_buf()).collect();
        self.processes.looks.lock().unwrap().push_back(Look { at, running });
    }

    /// Tracks the running processes until the script is over, as if the app
    /// were closed then.
    fn track(&self) {
        self.track_and(|_| {});
    }

    /// Like [`Self::track`], and lets `heard` look at the app after each
    /// Session that begins or ends.
    fn track_and(&self, mut heard: impl FnMut(&Tracking)) {
        self.core.track_sessions(|event| {
            if matches!(event, Tracking::Failed { .. }) && self.processes.is_over() {
                return ControlFlow::Break(());
            }
            heard(&event);
            ControlFlow::Continue(())
        });
    }

    fn sessions(&self, game_id: &str) -> Vec<Session> {
        self.core.sessions(game_id).unwrap()
    }
}

fn ended(started_at: u64, seconds: u64) -> Session {
    Session { started_at, seconds, running: false }
}

#[test]
fn a_game_never_played_has_no_sessions() {
    let app = App::start();
    let (_, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");

    assert_eq!(app.sessions(&id), []);
}

#[test]
fn the_sessions_of_a_game_are_listed_newest_first_with_their_start_and_length() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[&celeste]);
    app.look(at(10, 40, 0), &[]);
    app.look(at(12, 0, 0), &[&celeste]);
    app.look(at(13, 25, 30), &[&celeste]);
    app.look(at(13, 30, 0), &[]);

    app.track();

    assert_eq!(app.sessions(&id), [ended(at(12, 0, 0), 85 * 60 + 30), ended(at(10, 0, 0), 30 * 60)]);
}

#[test]
fn only_the_sessions_of_the_game_asked_for_are_listed() {
    let app = App::start();
    let (celeste, celeste_id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let (hades, hades_id) = app.add("Games/Hades/Hades.exe", "Hades");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 10, 0), &[&hades]);
    app.look(at(10, 30, 0), &[&hades]);
    app.look(at(10, 31, 0), &[]);

    app.track();

    assert_eq!(app.sessions(&celeste_id), [ended(at(10, 0, 0), 0)]);
    assert_eq!(app.sessions(&hades_id), [ended(at(10, 10, 0), 20 * 60)]);
}

#[test]
fn a_session_whose_process_still_runs_is_marked_as_running_until_it_ends() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(11, 0, 0), &[&celeste]);
    app.look(at(12, 0, 0), &[&celeste]);
    app.look(at(12, 5, 0), &[]);
    app.look(at(12, 6, 0), &[&celeste]);
    let mut seen = Vec::new();

    app.track_and(|event| seen.push((event.clone(), app.sessions(&id))));

    assert_eq!(
        seen,
        [
            (Tracking::Started { game_id: id.clone() }, vec![Session { started_at: at(10, 0, 0), seconds: 0, running: true }]),
            (Tracking::Ended { game_id: id.clone() }, vec![ended(at(10, 0, 0), 2 * 3600)]),
            (
                Tracking::Started { game_id: id.clone() },
                vec![Session { started_at: at(12, 6, 0), seconds: 0, running: true }, ended(at(10, 0, 0), 2 * 3600)]
            ),
        ]
    );
}

#[test]
fn a_running_session_grows_with_each_look_that_finds_its_process() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 45, 0), &[&celeste]);
    let mut lengths = Vec::new();

    app.core.track_sessions(|event| {
        if matches!(event, Tracking::Failed { .. }) {
            // The script is over: the look before found Celeste still running.
            lengths.push(app.sessions(&id));
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    });

    assert_eq!(lengths, [vec![Session { started_at: at(10, 0, 0), seconds: 45 * 60, running: true }]]);
}

#[test]
fn no_session_is_running_once_the_app_stopped_tracking() {
    let app = App::start();
    let (celeste, id) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[&celeste]);

    app.track();

    assert_eq!(app.sessions(&id), [ended(at(10, 0, 0), 30 * 60)]);
    let app = app.restart();
    assert_eq!(app.sessions(&id), [ended(at(10, 0, 0), 30 * 60)]);
}

#[test]
fn a_game_not_in_the_library_has_no_sessions() {
    let app = App::start();

    assert_eq!(app.sessions("local:42"), []);
    assert_eq!(app.sessions("steam:620"), []);
    assert_eq!(app.sessions("nonsense"), []);
}
