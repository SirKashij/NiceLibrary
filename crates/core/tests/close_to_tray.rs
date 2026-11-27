//! Closing the window: the app keeps running in the Tray so tracking goes
//! on, and the first time it says so. A setting turns this off, and closing
//! the window then quits the app.

use std::path::Path;

use nicelibrary_core::{Core, CoverCache, Store, WindowClose};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

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
        let ports = stand_ins::ports();
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
