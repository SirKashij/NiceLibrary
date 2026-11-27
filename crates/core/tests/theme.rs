//! The theme: on first start the app follows the operating system's dark or
//! light setting; once the user chooses the dark or the light theme in the
//! settings, that choice holds, also after a restart.

use std::path::Path;

use nicelibrary_core::{Core, CoverCache, Store, Theme};
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
fn on_first_start_the_theme_follows_the_system() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(start(profile.path()).theme().unwrap(), Theme::System);
}

#[test]
fn the_chosen_theme_is_the_theme() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());

    core.choose_theme(Theme::Light).unwrap();
    assert_eq!(core.theme().unwrap(), Theme::Light);

    core.choose_theme(Theme::Dark).unwrap();
    assert_eq!(core.theme().unwrap(), Theme::Dark);
}

#[test]
fn the_chosen_theme_survives_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_theme(Theme::Light).unwrap();

    assert_eq!(start(profile.path()).theme().unwrap(), Theme::Light);
}

#[test]
fn choosing_the_system_again_follows_it_again_after_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_theme(Theme::Dark).unwrap();
    start(profile.path()).choose_theme(Theme::System).unwrap();

    assert_eq!(start(profile.path()).theme().unwrap(), Theme::System);
}

#[test]
fn the_theme_reaches_the_interface_and_comes_back_in_the_shape_the_interface_uses() {
    for (theme, text) in [(Theme::System, "system"), (Theme::Dark, "dark"), (Theme::Light, "light")] {
        assert_eq!(serde_json::to_value(theme).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<Theme>(serde_json::json!(text)).unwrap(), theme);
    }
}
