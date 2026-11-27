//! A Local Game is launched by starting its executable, directly and not
//! through a shell, with the Launch Arguments and in the Working Directory
//! the user set for it; without a Working Directory, in the executable's
//! folder. When the executable or the Working Directory is missing, nothing
//! is started and the outcome says which.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, ExecutablePicker, Launch, LaunchOptions, OpenError, Opener, Ports,
    ProcessStarter, SteamLocator, Store,
};
use tempfile::TempDir;

/// One process the app started, as the operating system was asked to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Started {
    executable: PathBuf,
    arguments: Vec<String>,
    working_directory: PathBuf,
}

/// A substitute for the operating system's process start: records every
/// process the app starts, and can be made to fail.
#[derive(Clone, Default)]
struct RecordingStarter {
    started: Arc<Mutex<Vec<Started>>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl RecordingStarter {
    fn started(&self) -> Vec<Started> {
        self.started.lock().unwrap().clone()
    }

    fn fail_with(&self, reason: &str) {
        *self.failure.lock().unwrap() = Some(reason.to_owned());
    }
}

impl ProcessStarter for RecordingStarter {
    fn start(&self, executable: &Path, arguments: &[String], working_directory: &Path) -> Result<(), String> {
        if let Some(reason) = self.failure.lock().unwrap().clone() {
            return Err(reason);
        }
        self.started.lock().unwrap().push(Started {
            executable: executable.to_path_buf(),
            arguments: arguments.to_vec(),
            working_directory: working_directory.to_path_buf(),
        });
        Ok(())
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

/// A computer that opens no URL: a Local Game is never launched through one.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
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

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A user who chooses no executable; these tests add Local Games directly.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// One running app with a disk of its own that games can be put on.
struct App {
    core: Core,
    processes: RecordingStarter,
    profile: TempDir,
    disk: TempDir,
}

impl App {
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let processes = RecordingStarter::default();
        let core = Self::core_in(profile.path(), &processes);
        Self { core, processes, profile, disk: tempfile::tempdir().unwrap() }
    }

    fn core_in(profile: &Path, processes: &RecordingStarter) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(processes.clone()),
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
        let core = Self::core_in(self.profile.path(), &self.processes);
        Self { core, ..self }
    }

    /// A folder at `relative` (parts separated by `/`) on the disk, created.
    fn folder(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// Puts an executable at `relative` on the disk and adds it to the
    /// Library as the Local Game `title`; answers its id and path.
    fn add(&self, relative: &str, title: &str) -> (String, PathBuf) {
        let (folder, name) = relative.rsplit_once('/').unwrap();
        let path = self.folder(folder).join(name);
        fs::write(&path, b"MZ").unwrap();
        match self.core.add_local_game(path.to_str().unwrap(), title).unwrap() {
            AddLocalGame::Added { game } => (game.id, path),
            other => panic!("expected the game to be added, got {other:?}"),
        }
    }
}

fn options(arguments: &str, working_directory: Option<&Path>) -> LaunchOptions {
    LaunchOptions {
        arguments: arguments.to_owned(),
        working_directory: working_directory.map(|folder| folder.to_str().unwrap().to_owned()),
    }
}

#[test]
fn a_local_game_starts_its_executable_without_arguments_in_the_executables_folder() {
    let app = App::start();
    let (celeste, executable) = app.add("Games/Celeste/Celeste.exe", "Celeste");

    assert_eq!(app.core.launch(&celeste).unwrap(), Launch::Launched);

    assert_eq!(
        app.processes.started(),
        [Started { executable: executable.clone(), arguments: vec![], working_directory: app.folder("Games/Celeste") }]
    );
}

#[test]
fn launch_arguments_reach_the_executable_as_separate_arguments() {
    let app = App::start();
    let (game, _) = app.add("Games/Doom/gzdoom.exe", "Doom");
    let typed = r#"  -iwad "D:\My WADs\doom2.wad"   +map MAP01 --name="Doom Guy" "a & b" "" "#;

    app.core.set_launch_options(&game, &options(typed, None)).unwrap();
    app.core.launch(&game).unwrap();

    let expected = ["-iwad", r"D:\My WADs\doom2.wad", "+map", "MAP01", "--name=Doom Guy", "a & b", ""];
    assert_eq!(app.processes.started()[0].arguments, expected);
}

#[test]
fn a_working_directory_the_user_set_is_used_instead_of_the_executables_folder() {
    let app = App::start();
    let (game, _) = app.add("Games/Doom/bin/gzdoom.exe", "Doom");
    let data = app.folder("Games/Doom/data");

    app.core.set_launch_options(&game, &options("", Some(&data))).unwrap();
    app.core.launch(&game).unwrap();

    assert_eq!(app.processes.started()[0].working_directory, data);
}

#[test]
fn a_working_directory_of_only_spaces_is_none() {
    let app = App::start();
    let (game, _) = app.add("Games/Doom/gzdoom.exe", "Doom");

    app.core
        .set_launch_options(&game, &LaunchOptions { arguments: "  -fast ".into(), working_directory: Some("  ".into()) })
        .unwrap();
    app.core.launch(&game).unwrap();

    assert_eq!(app.core.launch_options(&game).unwrap(), Some(options("-fast", None)));
    assert_eq!(app.processes.started()[0].working_directory, app.folder("Games/Doom"));
}

#[test]
fn a_new_local_game_has_no_launch_arguments_and_no_working_directory() {
    let app = App::start();
    let (game, _) = app.add("Games/Celeste/Celeste.exe", "Celeste");

    assert_eq!(app.core.launch_options(&game).unwrap(), Some(LaunchOptions::default()));
}

#[test]
fn launch_options_are_kept_after_a_restart() {
    let app = App::start();
    let (game, _) = app.add("Games/Doom/gzdoom.exe", "Doom");
    let data = app.folder("Games/Doom/data");
    app.core.set_launch_options(&game, &options("-fast", Some(&data))).unwrap();

    let app = app.restart();

    assert_eq!(app.core.launch_options(&game).unwrap(), Some(options("-fast", Some(&data))));
    app.core.launch(&game).unwrap();
    assert_eq!(app.processes.started()[0].arguments, ["-fast"]);
    assert_eq!(app.processes.started()[0].working_directory, data);
}

#[test]
fn each_local_game_keeps_its_own_launch_options() {
    let app = App::start();
    let (doom, _) = app.add("Games/Doom/gzdoom.exe", "Doom");
    let (celeste, _) = app.add("Games/Celeste/Celeste.exe", "Celeste");

    app.core.set_launch_options(&doom, &options("-fast", None)).unwrap();

    assert_eq!(app.core.launch_options(&celeste).unwrap(), Some(LaunchOptions::default()));
}

#[test]
fn a_missing_executable_starts_nothing_and_says_so() {
    let app = App::start();
    let (game, executable) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    fs::remove_file(&executable).unwrap();

    assert_eq!(app.core.launch(&game).unwrap(), Launch::ExecutableNotFound);
    assert!(app.processes.started().is_empty());
}

#[test]
fn a_folder_where_the_executable_was_starts_nothing_and_says_the_executable_is_missing() {
    let app = App::start();
    let (game, executable) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    fs::remove_file(&executable).unwrap();
    fs::create_dir(&executable).unwrap();

    assert_eq!(app.core.launch(&game).unwrap(), Launch::ExecutableNotFound);
    assert!(app.processes.started().is_empty());
}

#[test]
fn a_missing_working_directory_starts_nothing_and_says_so() {
    let app = App::start();
    let (game, _) = app.add("Games/Doom/gzdoom.exe", "Doom");
    let data = app.folder("Games/Doom/data");
    app.core.set_launch_options(&game, &options("", Some(&data))).unwrap();
    fs::remove_dir(&data).unwrap();

    assert_eq!(app.core.launch(&game).unwrap(), Launch::WorkingDirectoryNotFound);
    assert!(app.processes.started().is_empty());
}

#[test]
fn a_process_that_cannot_be_started_is_an_error_naming_the_game() {
    let app = App::start();
    let (game, _) = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.processes.fail_with("access denied");

    let error = app.core.launch(&game).unwrap_err();

    assert!(error.to_string().contains("Celeste"), "{error}");
}

#[test]
fn launching_a_local_game_opens_no_url() {
    // `NoUrls` fails every URL; launching through it would be an error.
    let app = App::start();
    let (game, _) = app.add("Games/Celeste/Celeste.exe", "Celeste");

    assert_eq!(app.core.launch(&game).unwrap(), Launch::Launched);
}

#[test]
fn launch_options_are_refused_for_anything_but_a_local_game_in_the_library() {
    let app = App::start();

    for game in ["local:999", "steam:620", "nonsense"] {
        assert!(app.core.set_launch_options(game, &options("-fast", None)).is_err(), "{game}");
        assert_eq!(app.core.launch_options(game).unwrap(), None, "{game}");
    }
}

#[test]
fn launching_a_local_game_not_in_the_library_is_an_error() {
    let app = App::start();

    assert!(app.core.launch("local:999").is_err());
    assert!(app.processes.started().is_empty());
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

/// No export is made here: the user cancels every "Save as" dialog.
struct NoExportFile;

impl nicelibrary_core::ExportFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
