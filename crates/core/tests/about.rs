//! About NiceLibrary: the app tells which version runs and under which
//! license, and opens the project page and the license in the browser.

use std::path::Path;
use std::sync::{Arc, Mutex};

use nicelibrary_core::{About, AboutPage, Core, CoverCache, OpenError, Opener, Ports, Store};
use nicelibrary_core::stand_ins;

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
            opener: Box::new(browser),
            ..stand_ins::ports()
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
