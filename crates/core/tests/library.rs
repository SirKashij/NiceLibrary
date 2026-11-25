use nicelibrary_core::{Core, Game, Source};

#[test]
fn a_fresh_core_has_an_empty_library() {
    let core = Core::new();

    assert!(core.library().is_empty());
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
