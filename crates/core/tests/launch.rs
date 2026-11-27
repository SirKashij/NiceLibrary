//! An Installed Steam Game is launched through Steam itself, by opening its
//! `steam://rungameid/<app id>` URL, never by starting its executable
//! (ADR-0002). Steam keeps handling overlay, cloud saves and its own Playtime.
//! A Steam Game that is owned but not Installed is installed through Steam
//! too: its `steam://install/<app id>` URL opens Steam's install dialog.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{Core, CoverCache, Downloader, Install, Launch, OpenError, Opener, Ports, SteamLocator, Store};
use tempfile::TempDir;

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

/// A substitute for the registry lookup: Steam is where the test says.
#[derive(Clone, Default)]
struct SteamAt(Arc<Mutex<Option<PathBuf>>>);

impl SteamAt {
    fn uninstall_steam(&self) {
        *self.0.lock().unwrap() = None;
    }
}

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A substitute for the operating system's URL handling: records every URL
/// the app opens, and can be made to fail.
#[derive(Clone, Default)]
struct RecordingOpener {
    opened: Arc<Mutex<Vec<String>>>,
    failure: Arc<Mutex<Option<OpenError>>>,
}

impl RecordingOpener {
    fn opened(&self) -> Vec<String> {
        self.opened.lock().unwrap().clone()
    }

    fn fail_with(&self, error: OpenError) {
        *self.failure.lock().unwrap() = Some(error);
    }
}

impl Opener for RecordingOpener {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        if let Some(error) = self.failure.lock().unwrap().clone() {
            return Err(error);
        }
        self.opened.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// Steam on disk with one Steam Account that has logged in.
struct SteamFixture(TempDir);

impl SteamFixture {
    fn new() -> Self {
        let steam = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam.path().join("config")).unwrap();
        fs::write(
            steam.path().join("config").join("loginusers.vdf"),
            r#""users" { "76561197960287930" { "PersonaName" "Player" "MostRecent" "1" } }"#,
        )
        .unwrap();
        Self(steam)
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        let steamapps = self.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }

    fn uninstall(&self, app_id: u32) {
        fs::remove_file(self.path().join("steamapps").join(format!("appmanifest_{app_id}.acf"))).unwrap();
    }
}

/// One running app: its core, and what it opened on this computer.
struct App {
    core: Core,
    steam: SteamAt,
    opener: RecordingOpener,
    _profile: TempDir,
}

impl App {
    /// Starts the app over `steam` and reads its Installed Steam Games.
    fn start(steam: &SteamFixture) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let locator = SteamAt(Arc::new(Mutex::new(Some(steam.path().to_path_buf()))));
        let opener = RecordingOpener::default();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(locator.clone()),
                downloader: Box::new(Offline),
                opener: Box::new(opener.clone()),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        );
        core.read_installed_games().unwrap();
        Self { core, steam: locator, opener, _profile: profile }
    }
}

#[test]
fn launching_an_installed_steam_game_opens_its_run_game_url() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);

    assert_eq!(app.core.launch("steam:620"), Ok(Launch::Launched));
    assert_eq!(app.opener.opened(), ["steam://rungameid/620"]);
}

#[test]
fn when_steam_is_no_longer_installed_the_launch_says_so_and_opens_nothing() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    app.steam.uninstall_steam();

    assert_eq!(app.core.launch("steam:620"), Ok(Launch::SteamNotFound));
    assert!(app.opener.opened().is_empty());
}

#[test]
fn a_steam_game_that_is_no_longer_installed_is_not_launched() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    steam.uninstall(620);
    app.core.read_installed_games().unwrap();

    assert!(app.core.launch("steam:620").is_err());
    assert!(app.opener.opened().is_empty());
}

#[test]
fn a_game_that_is_not_in_the_library_is_not_launched() {
    let steam = SteamFixture::new();
    let app = App::start(&steam);

    assert!(app.core.launch("steam:620").is_err());
    assert!(app.core.launch(r"C:\Windows\notepad.exe").is_err());
    assert!(app.opener.opened().is_empty());
}

#[test]
fn a_launch_fails_when_the_url_cannot_be_opened() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    app.opener.fail_with(OpenError::Failed("the program could not be started".into()));

    assert!(app.core.launch("steam:620").is_err());
}

#[test]
fn when_steam_no_longer_handles_its_urls_the_launch_says_steam_is_missing() {
    // The Steam folder is still there, but Steam's link registration is gone
    // (a broken or half-removed installation).
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    app.opener.fail_with(OpenError::NoProgram);

    assert_eq!(app.core.launch("steam:620"), Ok(Launch::SteamNotFound));
}

#[test]
fn a_launch_reaches_the_interface_in_the_shape_the_interface_expects() {
    assert_eq!(serde_json::to_value(Launch::Launched).unwrap(), serde_json::json!({ "outcome": "launched" }));
    assert_eq!(
        serde_json::to_value(Launch::SteamNotFound).unwrap(),
        serde_json::json!({ "outcome": "steamNotFound" })
    );
}

/// The app knowing Portal 2 as a Steam Game whose files are not on this computer.
fn app_with_portal_2_not_installed(steam: &SteamFixture) -> App {
    steam.install(620, "Portal 2");
    let app = App::start(steam);
    steam.uninstall(620);
    app.core.read_installed_games().unwrap();
    app
}

#[test]
fn installing_a_steam_game_that_is_not_installed_opens_steams_install_dialog_for_it() {
    let steam = SteamFixture::new();
    let app = app_with_portal_2_not_installed(&steam);

    assert_eq!(app.core.install("steam:620"), Ok(Install::Started));
    assert_eq!(app.opener.opened(), ["steam://install/620"]);
}

#[test]
fn an_installed_steam_game_is_not_installed_again() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);

    assert!(app.core.install("steam:620").is_err());
    assert!(app.opener.opened().is_empty());
}

#[test]
fn a_game_that_is_not_in_the_library_is_not_installed() {
    let steam = SteamFixture::new();
    let app = App::start(&steam);

    assert!(app.core.install("steam:620").is_err());
    assert!(app.core.install(r"C:\Windows\notepad.exe").is_err());
    assert!(app.opener.opened().is_empty());
}

#[test]
fn when_steam_is_no_longer_installed_the_install_says_so_and_opens_nothing() {
    let steam = SteamFixture::new();
    let app = app_with_portal_2_not_installed(&steam);
    app.steam.uninstall_steam();

    assert_eq!(app.core.install("steam:620"), Ok(Install::SteamNotFound));
    assert!(app.opener.opened().is_empty());
}

#[test]
fn when_steam_no_longer_handles_its_urls_the_install_says_steam_is_missing() {
    let steam = SteamFixture::new();
    let app = app_with_portal_2_not_installed(&steam);
    app.opener.fail_with(OpenError::NoProgram);

    assert_eq!(app.core.install("steam:620"), Ok(Install::SteamNotFound));
}

#[test]
fn an_install_fails_when_the_url_cannot_be_opened() {
    let steam = SteamFixture::new();
    let app = app_with_portal_2_not_installed(&steam);
    app.opener.fail_with(OpenError::Failed("the program could not be started".into()));

    assert!(app.core.install("steam:620").is_err());
}

#[test]
fn an_install_reaches_the_interface_in_the_shape_the_interface_expects() {
    assert_eq!(serde_json::to_value(Install::Started).unwrap(), serde_json::json!({ "outcome": "started" }));
    assert_eq!(
        serde_json::to_value(Install::SteamNotFound).unwrap(),
        serde_json::json!({ "outcome": "steamNotFound" })
    );
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
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
