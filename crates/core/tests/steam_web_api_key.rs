//! The Steam Web API key: entered by the user, checked against Steam, kept
//! only in the operating system's secret store, replaced or removed in the
//! settings.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    Core, CoverCache, Downloader, OpenError, Opener, Ports, SecretStore, SteamLocator, SteamWebApiKeyCheck, Store,
    WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests. Real keys are 32 hexadecimal digits.
const VALID_KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// Another key Steam accepts, for replacing the first.
const OTHER_VALID_KEY: &str = "FEDCBA9876543210FEDCBA9876543210";
/// A key in the right format that Steam does not know.
const UNKNOWN_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// A substitute for the operating system's secret store, kept in memory.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<HashMap<String, String>>>);

impl Secrets {
    fn all(&self) -> HashMap<String, String> {
        self.0.lock().unwrap().clone()
    }
}

impl SecretStore for Secrets {
    fn read(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

/// A secret store that refuses every change, as a locked-down one may.
struct LockedSecrets;

impl SecretStore for LockedSecrets {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("access denied".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Err("access denied".into())
    }
}

/// How the fake Steam Web API behaves.
#[derive(Clone, Copy)]
enum Network {
    /// Steam answers as it does: JSON for a known key, 403 for any other.
    Online,
    /// No answer arrives at all.
    Offline,
    /// Something in between, such as a network's sign-in page, answers.
    SignInPage,
    /// Steam answers, but with a server error.
    SteamDown,
}

/// A fake of the Steam Web API that answers in the shape Steam does. It
/// remembers every URL it was asked for.
#[derive(Clone)]
struct SteamWebApi {
    network: Network,
    asked: Arc<Mutex<Vec<String>>>,
}

impl SteamWebApi {
    fn new(network: Network) -> Self {
        Self { network, asked: Arc::default() }
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        self.asked.lock().unwrap().push(url.to_owned());
        let html = |status, text: &str| Ok(WebAnswer { status, body: text.as_bytes().to_vec() });
        match self.network {
            Network::Offline => Err(format!("could not connect to {url}")),
            Network::SignInPage => html(200, "<!doctype html><title>Sign in to the Wi-Fi</title>"),
            Network::SteamDown => html(503, "<html><body>Service Unavailable</body></html>"),
            Network::Online => {
                let known = [VALID_KEY, OTHER_VALID_KEY].iter().any(|key| url.contains(&format!("key={key}")));
                if known {
                    html(200, r#"{"response":{"apps":[{"appid":10,"name":"Counter-Strike","last_modified":1745368572,"price_change_number":32366426}],"have_more_results":true,"last_appid":10}}"#)
                } else {
                    // Steam's answer to a key it does not know.
                    html(403, "<html><head><title>Forbidden</title></head><body><h1>Forbidden</h1>Access is denied. Retrying will not help. Please verify your <pre>key=</pre> parameter.</body></html>")
                }
            }
        }
    }
}

/// A computer without Steam.
struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

/// No Covers are needed here.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// An opener that remembers the URLs it was handed.
#[derive(Clone, Default)]
struct Browser(Arc<Mutex<Vec<String>>>);

impl Opener for Browser {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        self.0.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// The user profile of one test, with the secret store of its computer.
struct Profile {
    folder: TempDir,
    secrets: Secrets,
    browser: Browser,
}

impl Profile {
    fn new() -> Self {
        Self { folder: tempfile::tempdir().unwrap(), secrets: Secrets::default(), browser: Browser::default() }
    }

    /// Starts the app on this computer with Steam's Web API behaving as `api` does.
    fn start(&self, api: &SteamWebApi) -> Core {
        self.start_with(api, Box::new(self.secrets.clone()))
    }

    fn start_with(&self, api: &SteamWebApi, secrets: Box<dyn SecretStore>) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(self.browser.clone()),
                secrets,
                web_api: Box::new(api.clone()),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
            },
            Store::open(&self.folder.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.folder.path().join("covers")),
        )
    }

    /// Whether `text` appears in any file in the profile: the database, the
    /// Cover cache, anything else the app wrote.
    fn any_file_contains(&self, text: &str) -> bool {
        fn search(folder: &std::path::Path, text: &[u8]) -> bool {
            fs::read_dir(folder).unwrap().flatten().any(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    search(&path, text)
                } else {
                    fs::read(&path).unwrap().windows(text.len()).any(|window| window == text)
                }
            })
        }
        search(self.folder.path(), text.as_bytes())
    }
}

#[test]
fn a_valid_key_is_confirmed_and_kept_after_a_restart() {
    let profile = Profile::new();
    let steam = SteamWebApi::new(Network::Online);
    let core = profile.start(&steam);
    assert!(!core.steam_web_api_key_stored().unwrap());

    assert_eq!(core.store_steam_web_api_key(VALID_KEY).unwrap(), SteamWebApiKeyCheck::Stored);

    assert!(core.steam_web_api_key_stored().unwrap());
    assert!(profile.start(&steam).steam_web_api_key_stored().unwrap());
}

#[test]
fn the_key_is_checked_with_steam_over_https() {
    let profile = Profile::new();
    let steam = SteamWebApi::new(Network::Online);

    profile.start(&steam).store_steam_web_api_key(VALID_KEY).unwrap();

    let asked = steam.asked();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].starts_with("https://api.steampowered.com/"), "{}", asked[0]);
}

#[test]
fn a_key_steam_rejects_is_invalid_and_not_stored() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));

    assert_eq!(core.store_steam_web_api_key(UNKNOWN_KEY).unwrap(), SteamWebApiKeyCheck::Invalid);

    assert!(!core.steam_web_api_key_stored().unwrap());
    assert!(profile.secrets.all().is_empty());
}

#[test]
fn what_cannot_be_a_key_is_invalid_without_asking_steam() {
    let steam = SteamWebApi::new(Network::Online);
    let core = Profile::new().start(&steam);

    for not_a_key in [
        "",
        "   ",
        "0123456789ABCDEF0123456789ABCDE",   // one digit short
        "0123456789ABCDEF0123456789ABCDEF0", // one digit too many
        "0123456789ABCDEF0123456789ABCDEG",
        "0123456789ABCDEF 123456789ABCDEF",
        "0123456789ABCDEF0123456789ABCDEF&steamid=1",
        "key=0123456789ABCDEF0123456789ABCDEF",
    ] {
        assert_eq!(core.store_steam_web_api_key(not_a_key).unwrap(), SteamWebApiKeyCheck::Invalid, "{not_a_key:?}");
    }
    assert_eq!(steam.asked(), Vec::<String>::new());
}

#[test]
fn spaces_around_a_pasted_key_are_ignored() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));

    assert_eq!(core.store_steam_web_api_key(&format!("  {VALID_KEY}\n")).unwrap(), SteamWebApiKeyCheck::Stored);

    assert!(profile.secrets.all().values().any(|secret| secret == VALID_KEY));
}

#[test]
fn a_key_typed_in_lower_case_is_used_as_steam_issued_it() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));

    assert_eq!(
        core.store_steam_web_api_key(&VALID_KEY.to_lowercase()).unwrap(),
        SteamWebApiKeyCheck::Stored
    );

    assert_eq!(profile.secrets.all().values().collect::<Vec<_>>(), [VALID_KEY]);
}

#[test]
fn when_steam_cannot_be_reached_the_key_is_not_stored_and_not_called_invalid() {
    for network in [Network::Offline, Network::SignInPage, Network::SteamDown] {
        let profile = Profile::new();
        let core = profile.start(&SteamWebApi::new(network));

        assert_eq!(core.store_steam_web_api_key(VALID_KEY).unwrap(), SteamWebApiKeyCheck::SteamUnreachable);

        assert!(!core.steam_web_api_key_stored().unwrap());
    }
}

#[test]
fn the_key_is_stored_only_in_the_secret_store() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));

    core.store_steam_web_api_key(VALID_KEY).unwrap();

    assert_eq!(profile.secrets.all().values().collect::<Vec<_>>(), [VALID_KEY]);
    drop(core);
    assert!(!profile.any_file_contains(VALID_KEY), "the key was written to a file in the profile");
}

#[test]
fn no_error_names_the_key() {
    let profile = Profile::new();
    let core = profile.start_with(&SteamWebApi::new(Network::Online), Box::new(LockedSecrets));

    let error = core.store_steam_web_api_key(VALID_KEY).expect_err("the secret store refuses");

    assert!(!error.to_string().contains(VALID_KEY), "{error}");
}

#[test]
fn a_new_key_replaces_the_stored_one() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));
    core.store_steam_web_api_key(VALID_KEY).unwrap();

    assert_eq!(core.store_steam_web_api_key(OTHER_VALID_KEY).unwrap(), SteamWebApiKeyCheck::Stored);

    assert_eq!(profile.secrets.all().values().collect::<Vec<_>>(), [OTHER_VALID_KEY]);
}

#[test]
fn a_rejected_replacement_keeps_the_stored_key() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));
    core.store_steam_web_api_key(VALID_KEY).unwrap();

    assert_eq!(core.store_steam_web_api_key(UNKNOWN_KEY).unwrap(), SteamWebApiKeyCheck::Invalid);

    assert!(core.steam_web_api_key_stored().unwrap());
    assert_eq!(profile.secrets.all().values().collect::<Vec<_>>(), [VALID_KEY]);
}

#[test]
fn a_removed_key_is_gone_also_after_a_restart() {
    let profile = Profile::new();
    let steam = SteamWebApi::new(Network::Online);
    let core = profile.start(&steam);
    core.store_steam_web_api_key(VALID_KEY).unwrap();

    core.remove_steam_web_api_key().unwrap();

    assert!(!core.steam_web_api_key_stored().unwrap());
    assert!(!profile.start(&steam).steam_web_api_key_stored().unwrap());
    assert!(profile.secrets.all().is_empty());
}

#[test]
fn removing_when_no_key_is_stored_changes_nothing() {
    let core = Profile::new().start(&SteamWebApi::new(Network::Online));

    core.remove_steam_web_api_key().unwrap();

    assert!(!core.steam_web_api_key_stored().unwrap());
}

#[test]
fn a_secret_store_that_refuses_the_key_is_an_error() {
    let core = Profile::new().start_with(&SteamWebApi::new(Network::Online), Box::new(LockedSecrets));

    assert!(core.store_steam_web_api_key(VALID_KEY).is_err());
    assert!(core.remove_steam_web_api_key().is_err());
}

#[test]
fn the_steam_page_for_creating_a_key_opens_in_the_browser() {
    let profile = Profile::new();
    let core = profile.start(&SteamWebApi::new(Network::Online));

    core.open_steam_web_api_key_page().unwrap();

    assert_eq!(*profile.browser.0.lock().unwrap(), ["https://steamcommunity.com/dev/apikey"]);
}

#[test]
fn a_check_reaches_the_interface_in_the_shape_the_interface_expects() {
    for (check, outcome) in [
        (SteamWebApiKeyCheck::Stored, "stored"),
        (SteamWebApiKeyCheck::Invalid, "invalid"),
        (SteamWebApiKeyCheck::SteamUnreachable, "steamUnreachable"),
    ] {
        assert_eq!(serde_json::to_value(check).unwrap(), serde_json::json!({ "outcome": outcome }));
    }
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
    fn start(&self, executable: &std::path::Path, _arguments: &[String], _working_directory: &std::path::Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}
