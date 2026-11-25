use std::path::PathBuf;

use nicelibrary_core::{Core, CoverCache, Downloader, Game, Opener, Ports, Source, SteamLocator, Store};

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), String> {
        Err(format!("this test opens no URL, not even {url}"))
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
        Ports { steam_locator: Box::new(NoSteam), downloader: Box::new(Offline), opener: Box::new(NoUrls) },
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
    };

    assert_eq!(
        serde_json::to_value(&game).unwrap(),
        serde_json::json!({
            "id": "steam:620",
            "title": "Portal 2",
            "source": "steam",
            "installed": true
        })
    );
}
