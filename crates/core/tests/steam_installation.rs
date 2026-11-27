//! Finding the Steam Installation and its Steam Logins
//! on this computer, as the settings show them.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{Core, CoverCache, Ports, SteamInstallation, SteamLocator, SteamLogin, Store};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
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
            Ports { steam_locator: Box::new(SteamAt(folder.map(Path::to_path_buf))), ..stand_ins::ports() },
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

fn login(steam_id: &str, display_name: &str, most_recent: bool) -> SteamLogin {
    SteamLogin { steam_id: steam_id.into(), display_name: display_name.into(), most_recent }
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
fn lists_each_steam_login_by_display_name_with_the_most_recent_first() {
    let steam = steam_folder(Some(TWO_ACCOUNTS));

    let found = core_with_steam_at(Some(steam.path())).steam_installation();

    assert_eq!(
        found,
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            logins: vec![
                login("76561197960287931", "Player Two", true),
                login("76561197960287930", "Player One", false),
            ],
        }
    );
}

#[test]
fn a_steam_login_without_a_display_name_shows_its_account_name() {
    let steam = steam_folder(Some(
        r#""users" { "76561197960287930" { "AccountName" "player_one" "MostRecent" "1" } }"#,
    ));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            logins: vec![login("76561197960287930", "player_one", true)],
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
            logins: vec![
                login("76561197960287930", "Player One", true),
                login("76561197960287931", "Player Two", false),
            ],
        }
    );
}

#[test]
fn without_any_login_time_no_steam_login_is_marked_most_recent() {
    let steam = steam_folder(Some(
        r#""users" { "76561197960287930" { "PersonaName" "Player One" } "76561197960287931" { "PersonaName" "Player Two" } }"#,
    ));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found {
            path: steam.path().display().to_string(),
            logins: vec![
                login("76561197960287930", "Player One", false),
                login("76561197960287931", "Player Two", false),
            ],
        }
    );
}

#[test]
fn without_steam_says_it_is_not_found() {
    assert_eq!(core_with_steam_at(None).steam_installation(), SteamInstallation::NotFound);
}

#[test]
fn steam_without_any_login_has_no_steam_logins() {
    let steam = steam_folder(None);

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found { path: steam.path().display().to_string(), logins: vec![] }
    );
}

#[test]
fn a_damaged_login_file_leaves_steam_found_without_steam_logins() {
    let steam = steam_folder(Some(r#""users" { "76561197960287930" { "PersonaName" "#));

    assert_eq!(
        core_with_steam_at(Some(steam.path())).steam_installation(),
        SteamInstallation::Found { path: steam.path().display().to_string(), logins: vec![] }
    );
}

#[test]
fn reaches_the_interface_in_the_shape_the_interface_expects() {
    let found = SteamInstallation::Found {
        path: r"C:\Steam".into(),
        logins: vec![login("76561197960287930", "Player One", true)],
    };

    assert_eq!(
        serde_json::to_value(&found).unwrap(),
        serde_json::json!({
            "state": "found",
            "path": r"C:\Steam",
            "logins": [{ "steamId": "76561197960287930", "displayName": "Player One", "mostRecent": true }]
        })
    );
    assert_eq!(
        serde_json::to_value(SteamInstallation::NotFound).unwrap(),
        serde_json::json!({ "state": "notFound" })
    );
}
