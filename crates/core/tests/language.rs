//! The interface language: on first start the app follows the system
//! language, falling back to English; once the user chooses German or English
//! in the settings, that choice holds, also after a restart.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nicelibrary_core::{
    Autostart, Clock, Core, CoverCache, Downloader, ExecutablePicker, Icon, IconExtractor, ImagePicker, Language,
    OpenError, Opener, Ports, ProcessList, ProcessStarter, SecretStore, SteamLocator, Store, WebAnswer, WebApi, Wakeup,
};

/// A computer these tests need nothing from: no Steam, no network, no secrets, no file dialog, no
/// autostart, no processes, no icons.
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
fn on_first_start_the_language_follows_the_system() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::System);
}

#[test]
fn the_chosen_language_is_the_language() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());

    core.choose_language(Language::German).unwrap();
    assert_eq!(core.language().unwrap(), Language::German);

    core.choose_language(Language::English).unwrap();
    assert_eq!(core.language().unwrap(), Language::English);
}

#[test]
fn the_chosen_language_survives_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_language(Language::German).unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::German);
}

#[test]
fn choosing_the_system_again_follows_it_again_after_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_language(Language::English).unwrap();
    start(profile.path()).choose_language(Language::System).unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::System);
}

#[test]
fn the_language_reaches_the_interface_and_comes_back_in_the_shape_the_interface_uses() {
    for (language, text) in [(Language::System, "system"), (Language::German, "de"), (Language::English, "en")] {
        assert_eq!(serde_json::to_value(language).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<Language>(serde_json::json!(text)).unwrap(), language);
    }
}
