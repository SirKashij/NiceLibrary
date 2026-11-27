//! Stand-ins for every port that do nothing, for tests (feature `test-support`).
//!
//! A test takes [`ports`] and replaces only the ports it cares about:
//!
//! ```ignore
//! Ports { downloader: Box::new(steam.clone()), ..stand_ins::ports() }
//! ```
//!
//! A new port gets its stand-in here, so the tests that do not use it stay
//! as they are.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    Autostart, Clock, DataFilePicker, Downloader, ExecutablePicker, Icon, IconExtractor, ImagePicker, OpenError,
    Opener, Ports, ProcessList, ProcessStarter, SecretStore, SteamLocator, Wakeup, WebAnswer, WebApi,
};

/// A computer that gives the core nothing: no Steam, offline, no secrets,
/// opens nothing, a stopped clock, cancels every dialog, does not start the
/// app with Windows, runs and starts no process and shows no icon.
pub fn ports() -> Ports {
    Ports {
        steam_locator: Box::new(NoSteam),
        downloader: Box::new(Offline),
        opener: Box::new(NoUrls),
        secrets: Box::new(NoSecrets),
        web_api: Box::new(NoWebApi),
        clock: Box::new(StoppedClock),
        autostart: Box::new(NoAutostart),
        executable_picker: Box::new(NoExecutableChosen),
        processes: Box::new(NothingRuns),
        process_starter: Box::new(NothingStarted),
        icon_extractor: Box::new(NoIcons),
        image_picker: Box::new(NoImageChosen),
        data_file_picker: Box::new(NoDataFileChosen),
        update_downloader: Box::new(Offline),
    }
}

/// A computer without Steam.
pub struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

/// A computer without a network connection.
pub struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A computer that opens no URL and shows no folder.
pub struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A secret store that holds nothing and stores nothing.
pub struct NoSecrets;

impl SecretStore for NoSecrets {
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

/// A network that reaches no Web API.
pub struct NoWebApi;

impl WebApi for NoWebApi {
    fn get(&self, _url: &str) -> Result<WebAnswer, String> {
        Err("this test asks no Web API".into())
    }
}

/// A clock that stands still at the Unix epoch and does not sleep.
pub struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// A computer where the app does not start with Windows and cannot be made to.
pub struct NoAutostart;

impl Autostart for NoAutostart {
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

/// A user who chooses no executable; no folder dialog can be shown.
pub struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// A computer on which no process runs.
pub struct NothingRuns;

impl ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// A computer that starts no process.
pub struct NothingStarted;

impl ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// Executables that show no icon.
pub struct NoIcons;

impl IconExtractor for NoIcons {
    fn largest_icon(&self, _executable: &Path) -> Result<Option<Icon>, String> {
        Ok(None)
    }
}

/// A user who chooses no image.
pub struct NoImageChosen;

impl ImagePicker for NoImageChosen {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// A user who cancels every export and import file dialog.
pub struct NoDataFileChosen;

impl DataFilePicker for NoDataFileChosen {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    fn choose_import_file(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}
