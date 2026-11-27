//! Starting with Windows: a setting makes the app start when the user signs
//! in to Windows, hidden in the Tray. It is off by default. The setting is
//! the autostart entry itself, so it shows what Windows will really do, also
//! after the entry was removed outside the app.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use nicelibrary_core::{
    AUTOSTART_ARGUMENT, Autostart, Clock, Core, CoverCache, Downloader, OpenError, Opener, Ports, SecretStore,
    SteamLocator, Store, Wakeup, WebAnswer, WebApi, started_with_windows,
};
use tempfile::TempDir;

/// The current user's autostart entry for the app, as Windows keeps it: the
/// arguments the app starts with, once there is one. Shared with the test,
/// which plays the user changing it outside the app.
#[derive(Clone, Default)]
struct FakeAutostart {
    entry: Arc<Mutex<Option<Vec<String>>>>,
    /// Every call fails with this reason, as when the registry refuses access.
    broken: Option<&'static str>,
}

impl FakeAutostart {
    fn entry(&self) -> Option<Vec<String>> {
        self.entry.lock().unwrap().clone()
    }

    /// The user removes the entry outside the app, for instance in the
    /// Task Manager's list of startup apps.
    fn remove_outside_the_app(&self) {
        *self.entry.lock().unwrap() = None;
    }

    fn check(&self) -> Result<(), String> {
        match self.broken {
            Some(reason) => Err(reason.into()),
            None => Ok(()),
        }
    }
}

impl Autostart for FakeAutostart {
    fn is_on(&self) -> Result<bool, String> {
        self.check()?;
        Ok(self.entry().is_some())
    }

    fn turn_on(&self, arguments: &[&str]) -> Result<(), String> {
        self.check()?;
        *self.entry.lock().unwrap() = Some(arguments.iter().map(|argument| (*argument).to_owned()).collect());
        Ok(())
    }

    fn turn_off(&self) -> Result<(), String> {
        self.check()?;
        *self.entry.lock().unwrap() = None;
        Ok(())
    }
}

/// A computer without Steam, secrets, network or browser; starting with
/// Windows needs none of them.
struct Nothing;

impl SteamLocator for Nothing {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

impl Downloader for Nothing {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

impl Opener for Nothing {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

impl SecretStore for Nothing {
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

impl WebApi for Nothing {
    fn get(&self, _url: &str) -> Result<WebAnswer, String> {
        Err("this test asks no Web API".into())
    }
}

impl Clock for Nothing {
    fn now(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

impl nicelibrary_core::ExecutablePicker for Nothing {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// The app's data folder, kept across the starts of one test.
struct Profile(TempDir);

impl Profile {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    /// Starts the app on this profile, on a computer whose autostart
    /// entries are `autostart`.
    fn start(&self, autostart: &FakeAutostart) -> Core {
        let ports = Ports {
            steam_locator: Box::new(Nothing),
            downloader: Box::new(Nothing),
            opener: Box::new(Nothing),
            secrets: Box::new(Nothing),
            web_api: Box::new(Nothing),
            clock: Box::new(Nothing),
            executable_picker: Box::new(Nothing),
            icon_extractor: Box::new(NoIconsNoImages),
            image_picker: Box::new(NoIconsNoImages),
            export_file_picker: Box::new(NoExportFile),
            autostart: Box::new(autostart.clone()),
            processes: Box::new(NothingRuns),
            process_starter: Box::new(NothingStarted),
        };
        let store = Store::open(&self.path().join("nicelibrary.db")).unwrap();
        Core::new(ports, store, CoverCache::new(self.path().join("covers")))
    }
}

#[test]
fn the_app_does_not_start_with_windows_by_default() {
    let autostart = FakeAutostart::default();
    let core = Profile::new().start(&autostart);

    assert_eq!(core.starts_with_windows(), Ok(false));
    assert_eq!(autostart.entry(), None);
}

#[test]
fn turning_it_on_adds_an_autostart_entry_that_starts_the_app_in_the_tray() {
    let autostart = FakeAutostart::default();
    let core = Profile::new().start(&autostart);

    core.set_start_with_windows(true).unwrap();

    assert_eq!(core.starts_with_windows(), Ok(true));
    let arguments = autostart.entry().expect("an autostart entry");
    assert!(started_with_windows(&arguments));
}

#[test]
fn turning_it_off_removes_the_autostart_entry() {
    let autostart = FakeAutostart::default();
    let core = Profile::new().start(&autostart);
    core.set_start_with_windows(true).unwrap();

    core.set_start_with_windows(false).unwrap();

    assert_eq!(core.starts_with_windows(), Ok(false));
    assert_eq!(autostart.entry(), None);
}

#[test]
fn turning_it_off_when_it_is_off_changes_nothing() {
    let autostart = FakeAutostart::default();
    let core = Profile::new().start(&autostart);

    core.set_start_with_windows(false).unwrap();

    assert_eq!(core.starts_with_windows(), Ok(false));
}

#[test]
fn the_setting_shows_an_entry_removed_outside_the_app_as_off() {
    let autostart = FakeAutostart::default();
    let core = Profile::new().start(&autostart);
    core.set_start_with_windows(true).unwrap();

    autostart.remove_outside_the_app();

    assert_eq!(core.starts_with_windows(), Ok(false));
}

#[test]
fn after_a_restart_the_setting_is_what_the_autostart_entry_says() {
    let profile = Profile::new();
    let autostart = FakeAutostart::default();
    profile.start(&autostart).set_start_with_windows(true).unwrap();
    assert_eq!(profile.start(&autostart).starts_with_windows(), Ok(true));

    autostart.remove_outside_the_app();

    assert_eq!(profile.start(&autostart).starts_with_windows(), Ok(false));
}

#[test]
fn an_autostart_entry_that_cannot_be_read_or_changed_is_an_error() {
    let autostart = FakeAutostart { broken: Some("access denied"), ..FakeAutostart::default() };
    let core = Profile::new().start(&autostart);

    let read = core.starts_with_windows().unwrap_err().to_string();
    let changed = core.set_start_with_windows(true).unwrap_err().to_string();

    assert!(read.contains("access denied"), "{read}");
    assert!(changed.contains("access denied"), "{changed}");
}

#[test]
fn only_a_start_with_the_autostart_argument_is_a_start_with_windows() {
    assert!(started_with_windows([AUTOSTART_ARGUMENT]));
    assert!(started_with_windows(["--other", AUTOSTART_ARGUMENT]));
    assert!(!started_with_windows(Vec::<String>::new()));
    assert!(!started_with_windows(["--other"]));
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
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
