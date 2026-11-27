//! Sessions are recorded for Installed Steam Games too: a process whose
//! executable lies in a Steam Game's install folder, however it was started,
//! is that Game running. Steam's figure stays the Playtime of a Steam Game;
//! its Sessions never change it. When any Session ends, a Sync runs, so
//! Playtime and Achievements are current.

use std::collections::VecDeque;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, AutomaticSync, Clock, Core, CoverCache, Downloader, ExecutablePicker, Game, Log, OpenError, Opener,
    Ports, ProcessList, SecretStore, SteamLocator, Store, SyncOutcome, Tracking, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account logged in on the test computer.
const PLAYER: &str = "76561197960287930";
/// Portal 2's app id.
const PORTAL_2: u32 = 620;
/// Portal 2's Playtime as Steam reports it, in minutes.
const STEAMS_PLAYTIME: u32 = 95;
/// Portal 2's Last Played as Steam reports it: 2026-11-20 20:15:00 UTC.
const STEAMS_LAST_PLAYED: u64 = 1_795_205_700;

/// The time of day on the test's day, in seconds since the Unix epoch.
fn at(hours: u64, minutes: u64, seconds: u64) -> u64 {
    // 2027-01-15 00:00:00 UTC.
    1_800_000_000 - 1_800_000_000 % 86_400 + hours * 3600 + minutes * 60 + seconds
}

/// A clock whose time passes at once when it sleeps, unless the wakeup has
/// rung; the process list sets it to the time of each scripted look.
#[derive(Clone)]
struct TestClock(Arc<Mutex<SystemTime>>);

impl TestClock {
    fn seconds(&self) -> u64 {
        self.now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn set(&self, seconds: u64) {
        *self.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(seconds);
    }
}

impl Clock for TestClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    fn sleep(&self, duration: Duration, wakeup: &Wakeup) {
        if !wakeup.take() {
            *self.0.lock().unwrap() += duration;
        }
    }
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
    clock: TestClock,
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
        self.clock.set(look.at);
        Ok(look.running)
    }
}

/// Steam on the test computer, in the folder the test made.
struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// A substitute for the operating system's secret store, kept in memory.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<Option<String>>>);

impl SecretStore for Secrets {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn write(&self, _name: &str, secret: &str) -> Result<(), String> {
        *self.0.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

/// Steam's Web API, where the player owns Portal 2 with Steam's figures.
struct SteamWebApi;

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        if !url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") || !url.contains(KEY) {
            return Ok(WebAnswer { status: 404, body: b"<html><body>Not Found</body></html>".to_vec() });
        }
        let body = serde_json::json!({ "response": { "game_count": 1, "games": [{
            "appid": PORTAL_2,
            "name": "Portal 2",
            "playtime_forever": STEAMS_PLAYTIME,
            "rtime_last_played": STEAMS_LAST_PLAYED,
        }] } });
        Ok(WebAnswer { status: 200, body: body.to_string().into_bytes() })
    }
}

struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

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

/// The app on a computer with Steam, the player logged in, and a disk the
/// test puts Games on.
struct App {
    core: Core,
    processes: ScriptedProcesses,
    secrets: Secrets,
    profile: TempDir,
    steam: TempDir,
}

impl App {
    /// The app without a Steam Web API key.
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let steam = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam.path().join("config")).unwrap();
        fs::write(
            steam.path().join("config").join("loginusers.vdf"),
            format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
        )
        .unwrap();
        fs::create_dir_all(steam.path().join("steamapps")).unwrap();
        let processes = ScriptedProcesses {
            looks: Arc::default(),
            clock: TestClock(Arc::new(Mutex::new(UNIX_EPOCH + Duration::from_secs(at(9, 0, 0))))),
        };
        let secrets = Secrets::default();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.path().to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(secrets.clone()),
                web_api: Box::new(SteamWebApi),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(processes.clone()),
                autostart: Box::new(NoAutostart),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        )
        .with_log(Log::new(profile.path().join("logs")));
        Self { core, processes, secrets, profile, steam }
    }

    /// The app with the Steam Web API key stored.
    fn with_key() -> Self {
        let app = Self::start();
        *app.secrets.0.lock().unwrap() = Some(KEY.into());
        app
    }

    fn clock(&self) -> &TestClock {
        &self.processes.clock
    }

    /// The current log file's text; empty when there is none yet.
    fn log(&self) -> String {
        fs::read_to_string(self.profile.path().join("logs").join("nicelibrary.log")).unwrap_or_default()
    }

    /// Writes Steam's list of library folders as Steam does: the Steam
    /// folder and the folders in `extra`.
    fn library_folders(&self, extra: &[&Path]) {
        let entries: String = std::iter::once(self.steam.path())
            .chain(extra.iter().copied())
            .enumerate()
            .map(|(n, path)| {
                let path = path.display().to_string().replace('\\', "\\\\");
                format!("\t\"{n}\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t}}\n")
            })
            .collect();
        fs::write(
            self.steam.path().join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n{entries}}}\n"),
        )
        .unwrap();
    }

    /// Installs the Steam app `app_id` into `installdir` of the library folder
    /// `library` with Steam's `flags`, and puts its executable at
    /// `executable` (folders separated by `/`) inside its install folder.
    fn install_in(&self, library: &Path, app_id: u32, name: &str, installdir: &str, flags: u32, executable: &str) -> PathBuf {
        fs::create_dir_all(library.join("steamapps")).unwrap();
        fs::write(
            library.join("steamapps").join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"{flags}\"\n\t\"installdir\"\t\t\"{installdir}\"\n}}\n"
            ),
        )
        .unwrap();
        let path = executable
            .split('/')
            .fold(library.join("steamapps").join("common").join(installdir), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// Fully installs the Steam app `app_id` in the Steam folder; see [`Self::install_in`].
    fn install(&self, app_id: u32, name: &str, installdir: &str, executable: &str) -> PathBuf {
        self.install_in(self.steam.path(), app_id, name, installdir, 4, executable)
    }

    /// Puts an executable at `relative` inside the Steam folder.
    fn put_in_steam(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.steam.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// Scripts the next look at the running processes.
    fn look(&self, at: u64, running: &[&Path]) {
        let running = running.iter().map(|path| path.to_path_buf()).collect();
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

fn started(game_id: &str) -> Tracking {
    Tracking::Started { game_id: game_id.into() }
}

fn ended(game_id: &str) -> Tracking {
    Tracking::Ended { game_id: game_id.into() }
}

#[test]
fn a_running_steam_game_is_recognised_by_its_install_folder() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "bin/win64/portal2.exe");
    app.core.read_installed_games().unwrap();
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(10, 30, 0), &[&portal]);
    app.look(at(10, 30, 5), &[]);

    assert_eq!(app.track(), [started("steam:620"), ended("steam:620")]);
}

/// Runs the automatic Syncs; between the first and the second the app
/// tracks the scripted looks. Answers the second Sync and when the tracking
/// stopped, in seconds since the Unix epoch.
fn sync_after_tracking(app: &App) -> (AutomaticSync, u64) {
    let mut finished = Vec::new();
    let mut tracked_until = 0;
    app.core.sync_automatically(|event| {
        if matches!(event, AutomaticSync::Started) {
            return ControlFlow::Continue(());
        }
        finished.push(event);
        if finished.len() == 1 {
            app.track();
            tracked_until = app.clock().seconds();
            return ControlFlow::Continue(());
        }
        ControlFlow::Break(())
    });
    (finished.pop().unwrap(), tracked_until)
}

#[test]
fn the_end_of_a_session_of_a_steam_game_starts_a_sync_right_away() {
    let app = App::with_key();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(10, 30, 0), &[&portal]);
    app.look(at(10, 30, 5), &[]);

    let (sync, tracked_until) = sync_after_tracking(&app);

    assert_eq!(sync, AutomaticSync::Finished(SyncOutcome::Synced { at: tracked_until }));
}

#[test]
fn the_end_of_a_session_of_a_local_game_starts_a_sync_right_away() {
    let app = App::with_key();
    let celeste = app.put_in_steam("Games/Celeste/Celeste.exe");
    let AddLocalGame::Added { .. } = app.core.add_local_game(celeste.to_str().unwrap(), "Celeste").unwrap() else {
        panic!("expected Celeste to be added");
    };
    app.look(at(10, 0, 0), &[&celeste]);
    app.look(at(10, 30, 0), &[]);

    let (sync, tracked_until) = sync_after_tracking(&app);

    assert_eq!(sync, AutomaticSync::Finished(SyncOutcome::Synced { at: tracked_until }));
}

#[test]
fn a_session_that_goes_on_starts_no_sync_before_the_next_round() {
    let app = App::with_key();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(10, 30, 0), &[&portal]);

    let (sync, tracked_until) = sync_after_tracking(&app);

    assert_eq!(sync, AutomaticSync::Finished(SyncOutcome::Synced { at: tracked_until + 30 * 60 }));
}

#[test]
fn recorded_sessions_never_change_a_steam_games_playtime() {
    let app = App::with_key();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    app.core.read_installed_games().unwrap();
    assert_eq!(app.core.sync(), Ok(SyncOutcome::Synced { at: at(9, 0, 0) }));
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(12, 0, 0), &[&portal]);
    app.look(at(12, 0, 5), &[]);

    app.track();

    let game = app.game("steam:620");
    assert_eq!(game.playtime_minutes, Some(STEAMS_PLAYTIME));
    assert_eq!(game.last_played, Some(STEAMS_LAST_PLAYED));
}

#[test]
fn a_steam_games_playtime_stays_unknown_without_a_sync_however_long_it_ran() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    app.core.read_installed_games().unwrap();
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(12, 0, 0), &[&portal]);
    app.look(at(12, 0, 5), &[]);

    app.track();

    let game = app.game("steam:620");
    assert_eq!(game.playtime_minutes, None);
    assert_eq!(game.last_played, None);
}

#[test]
fn a_steam_game_in_another_library_folder_is_recognised_too() {
    let app = App::start();
    let drive = tempfile::tempdir().unwrap();
    app.library_folders(&[drive.path()]);
    let hades = app.install_in(drive.path(), 1145360, "Hades", "Hades", 4, "x64/Hades.exe");
    app.look(at(10, 0, 0), &[&hades]);
    app.look(at(10, 0, 5), &[]);

    assert_eq!(app.track(), [started("steam:1145360"), ended("steam:1145360")]);
}

#[test]
fn the_install_folder_is_recognised_by_any_spelling_of_the_path() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "bin/portal2.exe");
    let spelled_otherwise = portal.parent().unwrap().join("..").join("bin").join(".").join("portal2.exe");
    app.look(at(10, 0, 0), &[&spelled_otherwise]);
    app.look(at(10, 0, 5), &[]);

    assert_eq!(app.track(), [started("steam:620"), ended("steam:620")]);
}

#[cfg(windows)]
#[test]
fn the_install_folder_is_recognised_whatever_the_case_of_its_path_on_windows() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    let upper = PathBuf::from(portal.to_str().unwrap().to_uppercase());
    app.look(at(10, 0, 0), &[&upper]);
    app.look(at(10, 0, 5), &[]);

    assert_eq!(app.track(), [started("steam:620"), ended("steam:620")]);
}

#[test]
fn processes_outside_every_install_folder_record_no_session() {
    let app = App::start();
    app.library_folders(&[]);
    app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    let steam_itself = app.put_in_steam("steam.exe");
    let next_door = app.put_in_steam("steamapps/common/Portal 2 Tools/portal2.exe");
    app.look(at(10, 0, 0), &[&steam_itself, &next_door]);
    app.look(at(10, 0, 5), &[]);

    assert_eq!(app.track(), []);
}

#[test]
fn a_steam_app_that_is_not_fully_installed_or_one_of_steams_tools_records_no_session() {
    let app = App::start();
    app.library_folders(&[]);
    let downloading = app.install_in(app.steam.path(), PORTAL_2, "Portal 2", "Portal 2", 1026, "portal2.exe");
    let redistributables =
        app.install(228980, "Steamworks Common Redistributables", "Steamworks Shared", "_CommonRedist/setup.exe");
    app.look(at(10, 0, 0), &[&downloading, &redistributables]);
    app.look(at(10, 0, 5), &[]);

    assert_eq!(app.track(), []);
}

#[test]
fn several_processes_in_one_install_folder_make_one_session() {
    let app = App::start();
    app.library_folders(&[]);
    let launcher = app.install(PORTAL_2, "Portal 2", "Portal 2", "launcher.exe");
    let game = app.put_in_steam("steamapps/common/Portal 2/bin/portal2.exe");
    app.look(at(10, 0, 0), &[&launcher, &game]);
    app.look(at(10, 0, 5), &[&game]);
    app.look(at(10, 0, 10), &[]);

    assert_eq!(app.track(), [started("steam:620"), ended("steam:620")]);
}

#[test]
fn a_steam_game_and_a_local_game_running_at_the_same_time_each_have_a_session() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    let celeste = app.put_in_steam("Games/Celeste/Celeste.exe");
    let AddLocalGame::Added { game } = app.core.add_local_game(celeste.to_str().unwrap(), "Celeste").unwrap() else {
        panic!("expected Celeste to be added");
    };
    app.look(at(10, 0, 0), &[&portal, &celeste]);
    app.look(at(10, 30, 0), &[&celeste]);
    app.look(at(10, 30, 5), &[]);

    assert_eq!(app.track(), [started(&game.id), started("steam:620"), ended("steam:620"), ended(&game.id)]);
    assert_eq!(app.game(&game.id).playtime_minutes, Some(30));
}

#[test]
fn sessions_of_steam_games_are_written_to_the_log_without_their_title() {
    let app = App::start();
    app.library_folders(&[]);
    let portal = app.install(PORTAL_2, "Portal 2", "Portal 2", "portal2.exe");
    app.look(at(10, 0, 0), &[&portal]);
    app.look(at(10, 0, 5), &[]);

    app.track();

    let log = app.log();
    assert!(log.contains("INFO track_sessions: started"), "{log}");
    assert!(log.contains("INFO track_sessions: ended"), "{log}");
    assert!(!log.contains("Portal"), "the Log names no title or path: {log}");
}
