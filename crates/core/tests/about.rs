//! About NiceLibrary: the app tells which version runs and under which
//! license, and opens the project page and the license in the browser.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use nicelibrary_core::{
    About, AboutPage, Autostart, Clock, Core, CoverCache, Downloader, ExecutablePicker, Icon, IconExtractor, ImagePicker,
    OpenError, Opener, Ports, ProcessList, ProcessStarter, SecretStore, SteamLocator, Store, WebAnswer, WebApi, Wakeup,
};

/// A computer these tests need nothing from but a browser: no Steam, no network, no secrets, no file
/// dialog, no autostart, no processes, no icons.
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

impl Autostart for Nothing {
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

impl ProcessList for Nothing {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

impl ProcessStarter for Nothing {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

impl IconExtractor for Nothing {
    fn largest_icon(&self, _executable: &Path) -> Result<Option<Icon>, String> {
        Ok(None)
    }
}

impl ImagePicker for Nothing {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// A browser that records each URL it is asked to open, or fails to open any.
#[derive(Clone, Default)]
struct Browser {
    opened: Arc<Mutex<Vec<String>>>,
    broken: bool,
}

impl Opener for Browser {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        if self.broken {
            return Err(OpenError::Failed("the browser crashed".into()));
        }
        self.opened.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

fn start(profile: &Path, browser: Browser) -> Core {
    Core::new(
        Ports {
            steam_locator: Box::new(Nothing),
            downloader: Box::new(Nothing),
            opener: Box::new(browser),
            secrets: Box::new(Nothing),
            web_api: Box::new(Nothing),
            clock: Box::new(Nothing),
            executable_picker: Box::new(Nothing),
            autostart: Box::new(Nothing),
            processes: Box::new(Nothing),
            process_starter: Box::new(Nothing),
            icon_extractor: Box::new(Nothing),
            image_picker: Box::new(Nothing),
        },
        Store::open(&profile.join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.join("covers")),
    )
}

#[test]
fn about_tells_the_version_of_the_app_and_its_license() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(
        start(profile.path(), Browser::default()).about(),
        About { version: "0.1.0".into(), license: "MIT".into() }
    );
}

#[test]
fn the_project_page_opens_in_the_browser() {
    let profile = tempfile::tempdir().unwrap();
    let browser = Browser::default();

    start(profile.path(), browser.clone()).open_about_page(AboutPage::Project).unwrap();

    assert_eq!(*browser.opened.lock().unwrap(), ["https://github.com/SirKashij/NiceLibrary"]);
}

#[test]
fn the_license_opens_in_the_browser() {
    let profile = tempfile::tempdir().unwrap();
    let browser = Browser::default();

    start(profile.path(), browser.clone()).open_about_page(AboutPage::License).unwrap();

    assert_eq!(*browser.opened.lock().unwrap(), ["https://github.com/SirKashij/NiceLibrary/blob/main/LICENSE"]);
}

#[test]
fn a_page_the_browser_cannot_open_is_an_error_that_names_it() {
    let profile = tempfile::tempdir().unwrap();
    let browser = Browser { broken: true, ..Browser::default() };

    let error = start(profile.path(), browser).open_about_page(AboutPage::License).unwrap_err();

    assert_eq!(
        error.to_string(),
        "https://github.com/SirKashij/NiceLibrary/blob/main/LICENSE could not be opened: the browser crashed"
    );
}
