//! Choosing the Steam Account the app works with, among the ones that have
//! logged in on this computer or by entering a SteamID by hand.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{Core, CoverCache, Downloader, Game, OpenError, Opener, Ports, SteamAccountChoice, SteamLocator, Store};
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

const PLAYER_ONE: &str = "76561197960287930";
const PLAYER_TWO: &str = "76561197960287931";

/// A Steam folder on disk.
struct Steam(TempDir);

impl Steam {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    /// Writes `loginusers.vdf` with these accounts; the first one logged in
    /// most recently.
    fn logged_in(&self, steam_ids: &[&str]) {
        let mut text = String::from("\"users\"\n{\n");
        for (index, steam_id) in steam_ids.iter().enumerate() {
            let most_recent = u8::from(index == 0);
            text += &format!("\t\"{steam_id}\"\n\t{{\n\t\t\"PersonaName\"\t\t\"Player {index}\"\n\t\t\"MostRecent\"\t\t\"{most_recent}\"\n\t}}\n");
        }
        text += "}\n";
        fs::create_dir_all(self.path().join("config")).unwrap();
        fs::write(self.path().join("config").join("loginusers.vdf"), text).unwrap();
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        let steamapps = self.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(r#""AppState" {{ "appid" "{app_id}" "name" "{name}" "StateFlags" "4" "installdir" "{name}" }}"#),
        )
        .unwrap();
    }
}

/// The user profile of one test: where the database lives between starts.
struct Profile(TempDir);

impl Profile {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    /// Starts the app: a core over this profile's database.
    fn start(&self, steam: Option<&Steam>) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.map(|s| s.path().to_path_buf()))),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
            },
            Store::open(&self.0.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.0.path().join("covers")),
        )
    }
}

#[test]
fn the_only_account_on_this_computer_is_preselected() {
    let steam = Steam::new();
    steam.logged_in(&[PLAYER_ONE]);

    let core = Profile::new().start(Some(&steam));

    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn with_several_accounts_the_most_recent_is_preselected_until_the_user_chooses_another() {
    let steam = Steam::new();
    steam.logged_in(&[PLAYER_TWO, PLAYER_ONE]);
    let core = Profile::new().start(Some(&steam));
    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_TWO.to_owned()));

    assert_eq!(core.choose_steam_account(PLAYER_ONE).unwrap(), SteamAccountChoice::Chosen);

    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn the_chosen_account_survives_a_restart() {
    let steam = Steam::new();
    steam.logged_in(&[PLAYER_TWO, PLAYER_ONE]);
    let profile = Profile::new();
    profile.start(Some(&steam)).choose_steam_account(PLAYER_ONE).unwrap();

    let core = profile.start(Some(&steam));

    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn installed_games_are_read_for_the_chosen_account_and_do_not_replace_the_choice() {
    let steam = Steam::new();
    steam.logged_in(&[PLAYER_TWO, PLAYER_ONE]);
    steam.install(620, "Portal 2");
    let profile = Profile::new();
    let core = profile.start(Some(&steam));
    core.choose_steam_account(PLAYER_ONE).unwrap();

    assert_eq!(titles(&core.read_installed_games().unwrap()), ["Portal 2"]);

    // The next start reads Steam again; the most recently used account is
    // still Player Two, but the user's choice stands.
    let core = profile.start(Some(&steam));
    core.read_installed_games().unwrap();
    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));

    // The Games were stored for Player One: Player Two has not been read yet.
    core.choose_steam_account(PLAYER_TWO).unwrap();
    assert!(core.library().unwrap().is_empty());
}

#[test]
fn after_choosing_another_account_the_library_shows_its_games_once_they_are_read() {
    let steam = Steam::new();
    steam.logged_in(&[PLAYER_TWO, PLAYER_ONE]);
    steam.install(620, "Portal 2");
    let core = Profile::new().start(Some(&steam));
    core.read_installed_games().unwrap();

    core.choose_steam_account(PLAYER_ONE).unwrap();

    assert!(core.library().unwrap().is_empty(), "Player One's Games have not been read yet");
    assert_eq!(titles(&core.read_installed_games().unwrap()), ["Portal 2"]);
}

fn titles(games: &[Game]) -> Vec<&str> {
    games.iter().map(|g| g.title.as_str()).collect()
}

#[test]
fn without_steam_a_steamid_entered_by_hand_becomes_the_account_and_survives_a_restart() {
    let profile = Profile::new();
    let core = profile.start(None);
    assert_eq!(core.steam_account().unwrap(), None);

    assert_eq!(core.choose_steam_account(PLAYER_ONE).unwrap(), SteamAccountChoice::Chosen);

    assert_eq!(profile.start(None).steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn spaces_around_a_steamid_entered_by_hand_are_ignored() {
    let core = Profile::new().start(None);

    assert_eq!(core.choose_steam_account(" 76561197960287930\n").unwrap(), SteamAccountChoice::Chosen);

    assert_eq!(core.steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn anything_but_a_steamid64_of_a_personal_account_is_refused_and_changes_nothing() {
    let profile = Profile::new();
    let core = profile.start(None);
    core.choose_steam_account(PLAYER_ONE).unwrap();

    for not_a_steamid in [
        "",
        "Player One",
        "22202",                                            // the account number alone
        "STEAM_0:0:11101",                               // the older text form
        "[U:1:22202]",                                   // SteamID3
        "7656119796028793",                                 // one digit short
        "765611979602879300",                               // one digit too many
        "076561197960287930",                               // a leading zero
        "7656119796028793a",
        "76561197960265728",                                // account number 0
        "103582791429521412",                               // a Steam group, not a person
        "https://steamcommunity.com/profiles/76561197960287930",
    ] {
        assert_eq!(
            core.choose_steam_account(not_a_steamid).unwrap(),
            SteamAccountChoice::InvalidSteamId,
            "{not_a_steamid:?}"
        );
    }

    assert_eq!(profile.start(None).steam_account().unwrap(), Some(PLAYER_ONE.to_owned()));
}

#[test]
fn the_highest_personal_steamid_is_accepted() {
    let core = Profile::new().start(None);

    assert_eq!(core.choose_steam_account("76561202255233023").unwrap(), SteamAccountChoice::Chosen);
}

#[test]
fn with_steam_but_no_login_the_installed_games_belong_to_the_steamid_entered_by_hand() {
    let steam = Steam::new();
    steam.install(620, "Portal 2");
    let core = Profile::new().start(Some(&steam));
    assert!(core.read_installed_games().unwrap().is_empty(), "no account, nothing to store the Games for");

    core.choose_steam_account(PLAYER_ONE).unwrap();

    assert_eq!(titles(&core.read_installed_games().unwrap()), ["Portal 2"]);
}

#[test]
fn a_choice_reaches_the_interface_in_the_shape_the_interface_expects() {
    assert_eq!(serde_json::to_value(SteamAccountChoice::Chosen).unwrap(), serde_json::json!({ "outcome": "chosen" }));
    assert_eq!(
        serde_json::to_value(SteamAccountChoice::InvalidSteamId).unwrap(),
        serde_json::json!({ "outcome": "invalidSteamId" })
    );
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }
}
