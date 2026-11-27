//! Closing the window: the app keeps running in the Tray so tracking goes
//! on, and the first time it says so. A setting turns this off, and closing
//! the window then quits the app.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, OpenError, Opener, Ports, SecretStore, SteamLocator, Store, Wakeup,
    WebAnswer, WebApi, WindowClose,
};
use tempfile::TempDir;

/// A computer without Steam, secrets, network or browser; closing the
/// window needs none of them.
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

    /// Starts the app on this profile.
    fn start(&self) -> Core {
        let ports = Ports {
            steam_locator: Box::new(Nothing),
            downloader: Box::new(Nothing),
            opener: Box::new(Nothing),
            secrets: Box::new(Nothing),
            web_api: Box::new(Nothing),
            clock: Box::new(Nothing),
            executable_picker: Box::new(Nothing),
            autostart: Box::new(NoAutostart),
        };
        let store = Store::open(&self.path().join("nicelibrary.db")).unwrap();
        Core::new(ports, store, CoverCache::new(self.path().join("covers")))
    }
}

#[test]
fn closing_keeps_the_app_running_in_the_tray_by_default() {
    let profile = Profile::new();
    let core = profile.start();

    assert!(core.close_to_tray().unwrap());
    assert_ne!(core.close_window().unwrap(), WindowClose::Quit);
}

#[test]
fn the_first_close_shows_the_tray_notice_and_every_later_one_only_keeps_running() {
    let profile = Profile::new();
    let core = profile.start();

    assert_eq!(core.close_window().unwrap(), WindowClose::ShowTrayNotice);
    assert_eq!(core.close_window().unwrap(), WindowClose::KeepRunningInTray);
    assert_eq!(core.close_window().unwrap(), WindowClose::KeepRunningInTray);
}

#[test]
fn the_tray_notice_is_not_shown_again_after_a_restart() {
    let profile = Profile::new();
    assert_eq!(profile.start().close_window().unwrap(), WindowClose::ShowTrayNotice);

    let restarted = profile.start();

    assert_eq!(restarted.close_window().unwrap(), WindowClose::KeepRunningInTray);
}

#[test]
fn with_close_to_tray_off_closing_quits_the_app() {
    let profile = Profile::new();
    let core = profile.start();

    core.set_close_to_tray(false).unwrap();

    assert!(!core.close_to_tray().unwrap());
    assert_eq!(core.close_window().unwrap(), WindowClose::Quit);
}

#[test]
fn quitting_on_close_shows_no_tray_notice_and_leaves_it_for_when_close_to_tray_is_on_again() {
    let profile = Profile::new();
    let core = profile.start();
    core.set_close_to_tray(false).unwrap();
    assert_eq!(core.close_window().unwrap(), WindowClose::Quit);

    core.set_close_to_tray(true).unwrap();

    assert_eq!(core.close_window().unwrap(), WindowClose::ShowTrayNotice);
}

#[test]
fn the_close_to_tray_setting_is_kept_after_a_restart() {
    let profile = Profile::new();
    profile.start().set_close_to_tray(false).unwrap();

    let restarted = profile.start();

    assert!(!restarted.close_to_tray().unwrap());
    assert_eq!(restarted.close_window().unwrap(), WindowClose::Quit);
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
