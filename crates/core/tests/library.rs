use std::path::PathBuf;

use nicelibrary_core::{Core, Game, Ports, Source, SteamLocator, Store};

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

#[test]
fn a_fresh_core_has_an_empty_library() {
    let profile = tempfile::tempdir().unwrap();
    let store = Store::open(&profile.path().join("nicelibrary.db")).unwrap();
    let core = Core::new(Ports { steam_locator: Box::new(NoSteam) }, store);

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
