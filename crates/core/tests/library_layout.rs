//! The Library Layout: on first start the Library shows its Games as a cover
//! grid; once the user switches to the list, or back, that choice holds, also
//! after a restart.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, ExecutablePicker, OpenError, Opener, Ports, SecretStore, SteamLocator, Store, LibraryLayout, Theme, WebAnswer,
    WebApi, Wakeup,
};

/// A computer these tests need nothing from: no Steam, no network, no secrets, no file dialog.
struct Nothing;

impl SteamLocator for Nothing {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

impl Downloader for Nothing {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("this test downloads nothing".into())
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

impl ExecutablePicker for Nothing {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// The app started on the profile in `profile`, as on every start.
fn start(profile: &Path) -> Core {
    Core::new(
        Ports {
            steam_locator: Box::new(Nothing),
            downloader: Box::new(Nothing),
            opener: Box::new(Nothing),
            secrets: Box::new(Nothing),
            web_api: Box::new(Nothing),
            clock: Box::new(Nothing),
            executable_picker: Box::new(Nothing),
        },
        Store::open(&profile.join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.join("covers")),
    )
}

#[test]
fn on_first_start_the_library_shows_the_cover_grid() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_chosen_library_layout_is_the_library_layout() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());

    core.choose_library_layout(LibraryLayout::List).unwrap();
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::List);

    core.choose_library_layout(LibraryLayout::CoverGrid).unwrap();
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_chosen_library_layout_survives_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::List).unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::List);
}

#[test]
fn switching_back_to_the_cover_grid_holds_after_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::List).unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::CoverGrid).unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_library_layout_and_the_theme_are_kept_apart() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());
    core.choose_theme(Theme::Light).unwrap();

    core.choose_library_layout(LibraryLayout::List).unwrap();

    assert_eq!(core.theme().unwrap(), Theme::Light);
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::List);
}

#[test]
fn the_library_layout_reaches_the_interface_and_comes_back_in_the_shape_the_interface_uses() {
    for (layout, text) in [(LibraryLayout::CoverGrid, "coverGrid"), (LibraryLayout::List, "list")] {
        assert_eq!(serde_json::to_value(layout).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<LibraryLayout>(serde_json::json!(text)).unwrap(), layout);
    }
}

