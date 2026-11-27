use nicelibrary_core::{Core, CoverCache, Game, Source, Status, Store};
use nicelibrary_core::stand_ins;

#[test]
fn a_fresh_core_has_an_empty_library() {
    let profile = tempfile::tempdir().unwrap();
    let store = Store::open(&profile.path().join("nicelibrary.db")).unwrap();
    let core = Core::new(
        stand_ins::ports(),
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
        progress: Some(42),
        status: Status::Unplayed,
        favorite: false,
    };

    assert_eq!(
        serde_json::to_value(&game).unwrap(),
        serde_json::json!({
            "id": "steam:620",
            "title": "Portal 2",
            "source": "steam",
            "installed": true,
            "playtimeMinutes": 1234,
            "lastPlayed": 1795205700,
            "progress": 42,
            "status": "unplayed",
            "favorite": false
        })
    );
}

#[test]
fn a_game_without_playtime_last_played_and_progress_reaches_the_interface_with_null() {
    let game = Game {
        id: "steam:620".into(),
        title: "Portal 2".into(),
        source: Source::Steam,
        installed: true,
        playtime_minutes: None,
        last_played: None,
        progress: None,
        status: Status::Unplayed,
        favorite: false,
    };

    let json = serde_json::to_value(&game).unwrap();

    assert_eq!(json["playtimeMinutes"], serde_json::Value::Null);
    assert_eq!(json["lastPlayed"], serde_json::Value::Null);
    assert_eq!(json["progress"], serde_json::Value::Null);
}
