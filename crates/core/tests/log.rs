//! The Log: errors and notable events are written to a log file in the
//! app's data folder, which the user can open from the settings and attach
//! to a bug report themselves. The Log never holds the Steam Web API key or
//! a full SteamID, its files are rotated and capped in size, and nothing in
//! it is ever sent anywhere.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, Log, OpenError, Opener, Ports, SecretStore, SteamLocator, Store, SyncOutcome,
    Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account that logged in on the test computer.
const PLAYER: &str = "76561197960287930";

/// 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch.
const NOW: u64 = 1_795_608_000;

/// A clock that stands still at NOW.
struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW)
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// Steam on disk with one Steam Account that has logged in.
fn steam_folder() -> TempDir {
    let steam = tempfile::tempdir().unwrap();
    fs::create_dir_all(steam.path().join("config")).unwrap();
    fs::write(
        steam.path().join("config").join("loginusers.vdf"),
        format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
    )
    .unwrap();
    steam
}

struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// A secret store that holds what it is given, or fails with a reason that
/// (against its contract) repeats the secret and the account.
#[derive(Clone, Default)]
struct Secrets {
    key: Arc<Mutex<Option<String>>>,
    broken: Arc<Mutex<bool>>,
}

impl SecretStore for Secrets {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        if *self.broken.lock().unwrap() {
            return Err(format!("the store refused {KEY} for {PLAYER}"));
        }
        Ok(self.key.lock().unwrap().clone())
    }

    fn write(&self, _name: &str, secret: &str) -> Result<(), String> {
        *self.key.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        *self.key.lock().unwrap() = None;
        Ok(())
    }
}

/// Everything the app sends over the network: Web API requests and downloads.
#[derive(Clone, Default)]
struct Network {
    sent: Arc<Mutex<Vec<String>>>,
}

impl Network {
    fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }
}

impl WebApi for Network {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        self.sent.lock().unwrap().push(url.to_owned());
        // Steam refuses every key: the Sync is refused, without the app failing.
        Ok(WebAnswer { status: 403, body: b"<html><body>Forbidden</body></html>".to_vec() })
    }
}

impl Downloader for Network {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        self.sent.lock().unwrap().push(url.to_owned());
        Err("no network connection".into())
    }
}

/// Records every URL and folder the app opens; can be made to fail.
#[derive(Clone, Default)]
struct RecordingOpener {
    opened: Arc<Mutex<Vec<String>>>,
    folders: Arc<Mutex<Vec<PathBuf>>>,
    broken: Arc<Mutex<bool>>,
}

impl Opener for RecordingOpener {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        if *self.broken.lock().unwrap() {
            return Err(OpenError::Failed("no browser is available".into()));
        }
        self.opened.lock().unwrap().push(url.to_owned());
        Ok(())
    }

    fn open_folder(&self, folder: &Path) -> Result<(), OpenError> {
        if *self.broken.lock().unwrap() {
            return Err(OpenError::Failed("Explorer is not available".into()));
        }
        self.folders.lock().unwrap().push(folder.to_owned());
        Ok(())
    }
}

/// A file dialog the user always cancels; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    fn choose_folder(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// One running app with a Log in `profile/logs`.
struct App {
    core: Core,
    secrets: Secrets,
    network: Network,
    opener: RecordingOpener,
    profile: TempDir,
    _steam: TempDir,
}

impl App {
    fn start() -> Self {
        Self::start_with(|logs| Log::new(logs))
    }

    fn start_with(log: impl FnOnce(PathBuf) -> Log) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let steam = steam_folder();
        let secrets = Secrets::default();
        let network = Network::default();
        let opener = RecordingOpener::default();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.path().to_owned())),
                downloader: Box::new(network.clone()),
                opener: Box::new(opener.clone()),
                secrets: Box::new(secrets.clone()),
                web_api: Box::new(network.clone()),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                export_file_picker: Box::new(NoExportFile),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        )
        .with_log(log(profile.path().join("logs")));
        Self { core, secrets, network, opener, profile, _steam: steam }
    }

    fn logs(&self) -> PathBuf {
        self.profile.path().join("logs")
    }

    /// The current log file's text; empty when there is none yet.
    fn log(&self) -> String {
        fs::read_to_string(self.logs().join("nicelibrary.log")).unwrap_or_default()
    }

    /// Every log file's text, current and rotated.
    fn all_logs(&self) -> String {
        log_files(&self.logs()).iter().map(|file| fs::read_to_string(file).unwrap()).collect()
    }
}

fn log_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries.map(|entry| entry.unwrap().path()).collect();
    files.sort();
    files
}

#[test]
fn an_operation_that_fails_writes_when_and_why_to_the_log() {
    let app = App::start();

    assert!(app.core.launch("steam:999").is_err());

    assert_eq!(app.log(), "2026-11-25T12:00:00Z ERROR launch: no such game in the library: steam:999\n");
}

#[test]
fn a_sync_is_written_to_the_log_as_a_notable_event() {
    let app = App::start();
    app.secrets.write("key", KEY).unwrap();

    assert_eq!(app.core.sync(), Ok(SyncOutcome::KeyRejected));

    assert!(app.log().contains("INFO sync: keyRejected"), "{}", app.log());
}

#[test]
fn creating_a_collection_and_fetching_the_steam_app_list_are_written_as_notable_events() {
    let app = App::start();
    app.secrets.write("key", KEY).unwrap();

    app.core.create_collection("Backlog").unwrap();
    app.core.create_collection("  ").unwrap();
    app.core.fetch_steam_app_list().unwrap();

    let log = app.log();
    assert!(log.contains("INFO create_collection: named"), "{log}");
    assert!(log.contains("INFO create_collection: emptyName"), "{log}");
    assert!(log.contains("INFO fetch_steam_app_list: keyRejected"), "{log}");
}

#[test]
fn suggesting_the_executables_in_a_folder_is_written_as_a_notable_event() {
    let app = App::start();

    app.core.suggest_executables_in_folder().unwrap();

    let log = app.log();
    assert!(log.contains("INFO suggest_executables_in_folder: cancelled"), "{log}");
}

#[test]
fn setting_the_launch_options_of_a_game_that_is_not_a_local_game_writes_why_to_the_log() {
    let app = App::start();

    let options = nicelibrary_core::LaunchOptions { arguments: "-windowed".into(), working_directory: None };
    assert!(app.core.set_launch_options("steam:620", &options).is_err());

    let log = app.log();
    assert!(log.contains("ERROR set_launch_options: no such Local Game in the library: steam:620"), "{log}");
    assert!(!log.contains("-windowed"), "the Log holds no Launch Arguments: {log}");
}

#[test]
fn setting_the_status_of_a_game_that_is_not_in_the_library_writes_why_to_the_log() {
    let app = App::start();

    assert!(app.core.set_status("steam:999", nicelibrary_core::Status::Playing).is_err());

    assert_eq!(app.log(), "2026-11-25T12:00:00Z ERROR set_status: no such game in the library: steam:999\n");
}

#[test]
fn the_log_never_holds_the_steam_web_api_key_or_a_full_steam_id() {
    let app = App::start();
    app.secrets.write("key", KEY).unwrap();
    app.core.sync().unwrap();
    app.core.choose_steam_account(PLAYER).unwrap();
    assert!(app.core.launch(&format!("steam:{PLAYER}")).is_err());
    *app.secrets.broken.lock().unwrap() = true;
    assert!(app.core.sync().is_err());
    assert!(app.core.steam_web_api_key_stored().is_err());

    let log = app.all_logs();
    assert!(!log.contains(KEY) && !log.to_lowercase().contains(&KEY.to_lowercase()), "{log}");
    assert!(!log.contains(PLAYER), "{log}");
    // What failed is still told, with the secret and the SteamID left out.
    assert!(log.contains("ERROR steam_web_api_key_stored: secret store: the store refused [key] for [SteamID]"), "{log}");
    assert!(log.contains("no such game in the library: steam:[SteamID]"), "{log}");
}

#[test]
fn a_full_log_file_is_rotated_and_the_oldest_is_dropped() {
    let app = App::start_with(|logs| Log::with_limits(logs, 400, 3));

    for game in 1..=100 {
        let _ = app.core.launch(&format!("steam:{game}"));
    }

    let files = log_files(&app.logs());
    let names: Vec<_> = files.iter().map(|file| file.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["nicelibrary.1.log", "nicelibrary.2.log", "nicelibrary.log"]);
    for file in &files {
        assert!(fs::metadata(file).unwrap().len() <= 400, "{} is too big", file.display());
    }
    assert!(app.log().ends_with("no such game in the library: steam:100\n"), "{}", app.log());
    assert!(!app.all_logs().contains("steam:1\n"), "the oldest entries are still there");
}

#[test]
fn an_entry_longer_than_a_whole_file_is_cut_to_fit() {
    let app = App::start_with(|logs| Log::with_limits(logs, 120, 2));

    let _ = app.core.launch(&format!("steam:{}", "x".repeat(500)));

    let log = app.log();
    assert!(log.len() <= 120, "{log}");
    assert!(log.starts_with("2026-11-25T12:00:00Z ERROR launch: no such game") && log.ends_with('\n'), "{log}");
}

#[test]
fn a_log_folder_that_cannot_be_written_fails_no_operation() {
    let profile = tempfile::tempdir().unwrap();
    let blocked = profile.path().join("logs");
    // A file where the folder should be: nothing can be written there.
    fs::write(&blocked, "not a folder").unwrap();
    let app = App::start_with(|_| Log::new(blocked.clone()));

    assert_eq!(app.core.library(), Ok(Vec::new()));
    assert!(app.core.launch("steam:999").unwrap_err().to_string().contains("no such game"));
}

#[test]
fn nothing_in_the_log_is_ever_sent() {
    let app = App::start();
    app.secrets.write("key", KEY).unwrap();
    assert!(app.core.launch("steam:999").is_err());
    app.core.sync().unwrap();
    app.core.open_log_folder().unwrap();

    // The only request is the Sync's own, to Steam.
    let sent = app.network.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?"), "{sent:?}");
    assert!(app.opener.opened.lock().unwrap().is_empty());
}

#[test]
fn the_log_folder_is_opened_for_the_user_even_before_anything_was_written() {
    let app = App::start();

    assert_eq!(app.core.open_log_folder(), Ok(()));

    assert_eq!(*app.opener.folders.lock().unwrap(), [app.logs()]);
    assert!(app.logs().is_dir());
}

#[test]
fn a_log_folder_that_cannot_be_opened_is_an_error() {
    let app = App::start();
    *app.opener.broken.lock().unwrap() = true;

    let error = app.core.open_log_folder().unwrap_err();

    assert!(error.to_string().contains("Explorer is not available"), "{error}");
}

#[test]
fn an_about_page_that_cannot_be_opened_writes_why_to_the_log() {
    let app = App::start();
    *app.opener.broken.lock().unwrap() = true;

    assert!(app.core.open_about_page(nicelibrary_core::AboutPage::License).is_err());

    let log = app.log();
    assert!(log.contains("ERROR open_about_page: "), "{log}");
    assert!(log.contains("no browser is available"), "{log}");
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
