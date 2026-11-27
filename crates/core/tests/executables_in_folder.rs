//! Several Local Games are added at once by pointing at a folder. The app
//! looks a few folders deep for executables and suggests each with a proposed
//! title, passing over installers, uninstallers, crash handlers and
//! redistributables. Nothing is added until the user adds the ones they
//! ticked; executables already in the Library are marked as such.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, ExecutablePicker, FolderSuggestions, OpenError, Opener, Ports,
    SteamLocator, Store, SuggestedExecutable,
};
use tempfile::TempDir;

/// A substitute for the operating system's folder dialog: answers the folder
/// the test says the user chose, or that they cancelled.
#[derive(Clone, Default)]
struct UserChooses(Arc<Mutex<Option<PathBuf>>>);

impl UserChooses {
    fn folder(&self, path: &Path) {
        *self.0.lock().unwrap() = Some(path.to_path_buf());
    }
}

impl ExecutablePicker for UserChooses {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Err("these tests choose folders, not executables".into())
    }

    fn choose_folder(&self) -> Result<Option<PathBuf>, String> {
        Ok(self.0.lock().unwrap().clone())
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

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(
        &self,
        executable: &std::path::Path,
        _arguments: &[String],
        _working_directory: &std::path::Path,
    ) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// One running app with a disk of its own that games can be put on.
struct App {
    core: Core,
    user: UserChooses,
    _profile: TempDir,
    disk: TempDir,
}

impl App {
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let user = UserChooses::default();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(user.clone()),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                export_file_picker: Box::new(NoExportFile),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        );
        Self { core, user, _profile: profile, disk: tempfile::tempdir().unwrap() }
    }

    /// The path of `relative` (folders separated by `/`) on the disk.
    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part))
    }

    /// Puts an executable at `relative` on the disk.
    fn put(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// What the app suggests when the user chooses the folder at `relative`.
    fn suggestions(&self, relative: &str) -> Vec<SuggestedExecutable> {
        let folder = self.path(relative);
        fs::create_dir_all(&folder).unwrap();
        self.user.folder(&folder);
        match self.core.suggest_executables_in_folder().unwrap() {
            FolderSuggestions::Suggested { folder: answered, executables } => {
                assert_eq!(answered, path_text(&folder));
                executables
            }
            other => panic!("expected suggestions, got {other:?}"),
        }
    }

    /// The titles the app suggests for the folder at `relative`.
    fn suggested_titles(&self, relative: &str) -> Vec<String> {
        self.suggestions(relative).into_iter().map(|suggested| suggested.title).collect()
    }
}

fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

#[test]
fn the_executables_in_a_folder_are_suggested_with_proposed_titles() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    let hades = app.put("Games/Hades/x64/Hades.exe");

    assert_eq!(
        app.suggestions("Games"),
        [
            SuggestedExecutable { executable: path_text(&celeste), title: "Celeste".into(), already_added: false },
            SuggestedExecutable { executable: path_text(&hades), title: "Hades".into(), already_added: false },
        ]
    );
}

#[test]
fn executables_are_found_up_to_four_folders_below_the_chosen_one_and_no_deeper() {
    let app = App::start();
    app.put("Games/Deep/Deep/Binaries/Win64/Deep Game.exe");
    app.put("Games/Deeper/Deeper/Deeper/Binaries/Win64/Deeper Game.exe");

    assert_eq!(app.suggested_titles("Games"), ["Deep Game"]);
}

#[test]
fn installers_uninstallers_crash_handlers_and_redistributables_are_not_suggested() {
    let app = App::start();
    for no_game in [
        "Celeste/unins000.exe",
        "Celeste/Uninstall.exe",
        "Celeste/setup.exe",
        "Celeste/CelesteInstaller.exe",
        "Celeste/UnityCrashHandler64.exe",
        "Celeste/crashpad_handler.exe",
        "Celeste/CrashReportClient.exe",
        "Celeste/vc_redist.x64.exe",
        "Celeste/VCREDIST_x86.exe",
        "Celeste/DXSETUP.exe",
        "Celeste/_CommonRedist/DirectX/Jun2010/DXSETUP.exe",
        "Celeste/_CommonRedist/vcredist/2019/VC_redist.x64.exe",
        "Celeste/Redist/dotNetFx40_Full_setup.exe",
        "Celeste/Engine/Extras/Redist/en-us/UE4PrereqSetup_x64.exe",
    ] {
        app.put(no_game);
    }
    app.put("Celeste/Celeste.exe");

    assert_eq!(app.suggested_titles("Celeste"), ["Celeste"]);
}

#[test]
fn only_executables_are_suggested_whatever_the_case_of_their_extension() {
    let app = App::start();
    app.put("Games/Celeste/CELESTE.EXE");
    app.put("Games/Celeste/Celeste.dll");
    app.put("Games/Celeste/readme.txt");

    assert_eq!(app.suggested_titles("Games"), ["CELESTE"]);
}

#[test]
fn an_executable_already_in_the_library_is_marked_with_the_title_it_has_there() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    let hades = app.put("Games/Hades/Hades.exe");
    app.core.add_local_game(&path_text(&celeste), "Celeste (Steam-free)").unwrap();

    assert_eq!(
        app.suggestions("Games"),
        [
            SuggestedExecutable {
                executable: path_text(&celeste),
                title: "Celeste (Steam-free)".into(),
                already_added: true,
            },
            SuggestedExecutable { executable: path_text(&hades), title: "Hades".into(), already_added: false },
        ]
    );
}

#[test]
fn suggesting_adds_nothing_and_the_ticked_executables_are_added_one_by_one() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    app.put("Games/Hades/Hades.exe");

    let suggested = app.suggestions("Games");
    assert_eq!(app.core.library().unwrap(), []);

    let ticked = &suggested[0];
    assert_eq!(ticked.executable, path_text(&celeste));
    let AddLocalGame::Added { game } = app.core.add_local_game(&ticked.executable, &ticked.title).unwrap() else {
        panic!("expected the ticked executable to be added");
    };
    assert_eq!(app.core.library().unwrap(), [game]);
    assert_eq!(
        app.suggestions("Games").iter().map(|suggested| suggested.already_added).collect::<Vec<_>>(),
        [true, false]
    );
}

#[test]
fn a_folder_without_executables_suggests_nothing() {
    let app = App::start();

    assert_eq!(app.suggestions("Empty"), []);
}

#[test]
fn cancelling_the_folder_dialog_suggests_nothing() {
    let app = App::start();

    assert_eq!(app.core.suggest_executables_in_folder().unwrap(), FolderSuggestions::Cancelled);
}

#[test]
fn a_folder_dialog_that_cannot_be_shown_is_a_failure() {
    struct BrokenDialog;

    impl ExecutablePicker for BrokenDialog {
        fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
            Err("no desktop".into())
        }

        fn choose_folder(&self) -> Result<Option<PathBuf>, String> {
            Err("no desktop".into())
        }
    }

    let profile = tempfile::tempdir().unwrap();
    let core = Core::new(
        Ports {
            steam_locator: Box::new(NoSteam),
            downloader: Box::new(Offline),
            opener: Box::new(NoUrls),
            secrets: Box::new(NoSecretsNoWebApi),
            web_api: Box::new(NoSecretsNoWebApi),
            clock: Box::new(StoppedClock),
            executable_picker: Box::new(BrokenDialog),
            autostart: Box::new(NoAutostart),
            processes: Box::new(NothingRuns),
            process_starter: Box::new(NothingStarted),
            icon_extractor: Box::new(NoIconsNoImages),
            image_picker: Box::new(NoIconsNoImages),
            export_file_picker: Box::new(NoExportFile),
        },
        Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.path().join("covers")),
    );

    let error = core.suggest_executables_in_folder().unwrap_err();
    assert!(error.to_string().contains("no desktop"), "{error}");
}

/// No export is made here: the user cancels every "Save as" dialog.
struct NoExportFile;

impl nicelibrary_core::ExportFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
