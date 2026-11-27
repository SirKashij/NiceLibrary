//! The Library Layout: on first start the Library shows its Games as a cover
//! grid; once the user switches to the list, or back, that choice holds, also
//! after a restart.

use std::path::Path;

use nicelibrary_core::{Core, CoverCache, Store, LibraryLayout, Theme};
use nicelibrary_core::stand_ins;

/// The app started on the profile in `profile`, as on every start.
fn start(profile: &Path) -> Core {
    Core::new(
        stand_ins::ports(),
        Store::open(&profile.join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.join("covers")),
    )
}

#[test]
fn on_first_start_the_library_shows_the_cover_grid() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_chosen_library_layout_is_the_library_layout() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());

    core.choose_library_layout(LibraryLayout::List).unwrap();
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::List);

    core.choose_library_layout(LibraryLayout::CoverGrid).unwrap();
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_chosen_library_layout_survives_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::List).unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::List);
}

#[test]
fn switching_back_to_the_cover_grid_holds_after_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::List).unwrap();
    start(profile.path()).choose_library_layout(LibraryLayout::CoverGrid).unwrap();

    assert_eq!(start(profile.path()).library_layout().unwrap(), LibraryLayout::CoverGrid);
}

#[test]
fn the_library_layout_and_the_theme_are_kept_apart() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());
    core.choose_theme(Theme::Light).unwrap();

    core.choose_library_layout(LibraryLayout::List).unwrap();

    assert_eq!(core.theme().unwrap(), Theme::Light);
    assert_eq!(core.library_layout().unwrap(), LibraryLayout::List);
}

#[test]
fn the_library_layout_reaches_the_interface_and_comes_back_in_the_shape_the_interface_uses() {
    for (layout, text) in [(LibraryLayout::CoverGrid, "coverGrid"), (LibraryLayout::List, "list")] {
        assert_eq!(serde_json::to_value(layout).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<LibraryLayout>(serde_json::json!(text)).unwrap(), layout);
    }
}
