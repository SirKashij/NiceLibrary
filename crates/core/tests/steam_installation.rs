//! Finding the Steam installation and the Steam Accounts that have logged in
//! on this computer, as the settings show them.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{Core, CoverCache, Downloader, OpenError, Opener, Ports, SteamAccount, SteamInstallation, SteamLocator, Store};
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

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A core with a database of its own, removed after the test.
struct TestCore {
    core: Core,
    _profile: TempDir,
}

impl std::ops::Deref for TestCore {
    type Target = Core;

    fn deref(&self) -> &Core {
        &self.core
    }
}

fn core_with_steam_at(folder: Option<&Path>) -> TestCore {
    let profile = tempfile::tempdir().unwrap();
    let store = Store::open(&profile.path().join("nicelibrary.db")).unwrap();
    TestCore {
        core: Core::new(
            Ports { steam_locator: Box::new(SteamAt(folder.map(Path::to_path_buf))), downloader: Box::new(Offline), opener: Box::new(NoUrls), secrets: Box::new(NoSecretsNoWebApi), web_api: Box::new(NoSecretsNoWebApi), clock: Box::new(StoppedClock),executable_picker: Box::new(NoExecutableChosen), icon_extractor: Box::new(NoIconsNoImages), image_picker: Box::new(NoIconsNoImages), },
            store,
            CoverCache::new(profile.path().join("covers")),
        ),
        _profile: profile,
    }
}

/// A Steam folder on disk with the given `config/loginusers.vdf`.
fn steam_folder(loginusers: Option<&str>) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    if let Some(text) = loginusers {
        fs::create_dir(dir.path().join("config")).unwrap();
        fs::write(dir.path().join("config").join("loginusers.vdf"), text).unwrap();
    }
    dir
}

fn account(steam_id: &str, display_name: &str, most_recent: bool) -> SteamAccount {
    SteamAccount { steam_id: steam_id.into(), display_name: display_name.into(), most_recent }
}

const TWO_ACCOUNTS: &str = r#"
"users"
{
	"76561197960287930"
	{
		"AccountName"		"player_one"
		"PersonaName"		"Player One"
		"RememberPassword"		"1"
		"MostRecent"		"0"
		"Timestamp"		"1700000000"
	}
	"76561197960287931"
	{
		"AccountName"		"player_two"
		"PersonaName"		"Player Two"
		"RememberPassword"		"1"
		"MostRecent"		"1"
		"Timestamp"		"1700001000"
	}
}
"#;

#[test]
fn lists_each_steam_account_by_display_name_with_the_most_recent_first() {
    let steam = steam_folder(Some(TWO_ACCOUNTS));

    let found = core_with_steam_at(Some(steam.path())).steam_installation();

    assert_eq!(
        found,
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            accounts: vec![
                account("76561197960287931", "Player Two", true),
                account("76561197960287930", "Player One", false),
            ],
        }
    );
}

#[test]
fn an_account_without_a_display_name_shows_its_account_name() {
    let steam = steam_folder(Some(
        r#""users" { "76561197960287930" { "AccountName" "player_one" "MostRecent" "1" } }"#,
    ));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            accounts: vec![account("76561197960287930", "player_one", true)],
        }
    );
}

#[test]
fn without_a_most_recent_flag_the_latest_login_counts_as_most_recent() {
    let steam = steam_folder(Some(
        r#""users" {
            "76561197960287930" { "PersonaName" "Player One" "Timestamp" "1700002000" }
            "76561197960287931" { "PersonaName" "Player Two" "Timestamp" "1700001000" }
        }"#,
    ));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            accounts: vec![
                account("76561197960287930", "Player One", true),
                account("76561197960287931", "Player Two", false),
            ],
        }
    );
}

#[test]
fn without_any_login_time_no_account_is_marked_most_recent() {
    let steam = steam_folder(Some(
        r#""users" { "76561197960287930" { "PersonaName" "Player One" } "76561197960287931" { "PersonaName" "Player Two" } }"#,
    ));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            accounts: vec![
                account("76561197960287930", "Player One", false),
                account("76561197960287931", "Player Two", false),
            ],
        }
    );
}

#[test]
fn without_steam_says_it_is_not_found() {
    assert_eq!(core_with_steam_at(None).steam_installation(), SteamInstallation::NotFound);
}

#[test]
fn steam_without_any_login_has_no_accounts() {
    let steam = steam_folder(None);

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found { path: steam.path().display().to_string(), accounts: vec![] }
    );
}

#[test]
fn a_damaged_login_file_leaves_steam_found_without_accounts() {
    let steam = steam_folder(Some(r#""users" { "76561197960287930" { "PersonaName" "#));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found { path: steam.path().display().to_string(), accounts: vec![] }
    );
}

#[test]
fn reaches_the_interface_in_the_shape_the_interface_expects() {
    let found = SteamInstallation::Found {
        path: r"C:\Steam".into(),
        accounts: vec![account("76561197960287930", "Player One", true)],
    };

    assert_eq!(
        serde_json::to_value(&found).unwrap(),
        serde_json::json!({
            "state": "found",
            "path": r"C:\Steam",
            "accounts": [{ "steamId": "76561197960287930", "displayName": "Player One", "mostRecent": true }]
        })
    );
    assert_eq!(
        serde_json::to_value(SteamInstallation::NotFound).unwrap(),
        serde_json::json!({ "state": "notFound" })
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
