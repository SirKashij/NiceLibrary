use std::path::PathBuf;

use nicelibrary_core::{Core, CoverCache, Downloader, Game, OpenError, Opener, Ports, Source, SteamLocator, Store};

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

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

#[test]
fn a_fresh_core_has_an_empty_library() {
    let profile = tempfile::tempdir().unwrap();
    let store = Store::open(&profile.path().join("nicelibrary.db")).unwrap();
    let core = Core::new(
        Ports { steam_locator: Box::new(NoSteam), downloader: Box::new(Offline), opener: Box::new(NoUrls), secrets: Box::new(NoSecretsNoWebApi), web_api: Box::new(NoSecretsNoWebApi), clock: Box::new(StoppedClock) },
        store,
        CoverCache::new(profile.path().join("covers")),
    );

    assert!(core.library().unwrap().is_empty());
}

#[test]
fn the_database_folder_is_created_when_it_does_not_exist_yet() {
    let profile = tempfile::tempdir().unwrap();

    Store::open(&profile.path().join("AppData").join("NiceLibrary").join("nicelibrary.db")).unwrap();

    assert!(profile.path().join("AppData").join("NiceLibrary").join("nicelibrary.db").is_file());
}

#[test]
fn a_game_reaches_the_interface_in_the_shape_the_interface_expects() {
    let game = Game {
        id: "steam:620".into(),
        title: "Portal 2".into(),
        source: Source::Steam,
        installed: true,
        playtime_minutes: Some(1_234),
        last_played: Some(1_795_205_700),
    };

    assert_eq!(
        serde_json::to_value(&game).unwrap(),
        serde_json::json!({
            "id": "steam:620",
            "title": "Portal 2",
            "source": "steam",
            "installed": true,
            "playtimeMinutes": 1234,
            "lastPlayed": 1795205700
        })
    );
}

#[test]
fn a_game_without_playtime_and_last_played_reaches_the_interface_with_null() {
    let game = Game {
        id: "steam:620".into(),
        title: "Portal 2".into(),
        source: Source::Steam,
        installed: true,
        playtime_minutes: None,
        last_played: None,
    };

    let json = serde_json::to_value(&game).unwrap();

    assert_eq!(json["playtimeMinutes"], serde_json::Value::Null);
    assert_eq!(json["lastPlayed"], serde_json::Value::Null);
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}
