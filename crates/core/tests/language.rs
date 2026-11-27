//! The interface language: on first start the app follows the system
//! language, falling back to English; once the user chooses German or English
//! in the settings, that choice holds, also after a restart.

use std::path::Path;

use nicelibrary_core::{Core, CoverCache, Language, Store};
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
fn on_first_start_the_language_follows_the_system() {
    let profile = tempfile::tempdir().unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::System);
}

#[test]
fn the_chosen_language_is_the_language() {
    let profile = tempfile::tempdir().unwrap();
    let core = start(profile.path());

    core.choose_language(Language::German).unwrap();
    assert_eq!(core.language().unwrap(), Language::German);

    core.choose_language(Language::English).unwrap();
    assert_eq!(core.language().unwrap(), Language::English);
}

#[test]
fn the_chosen_language_survives_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_language(Language::German).unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::German);
}

#[test]
fn choosing_the_system_again_follows_it_again_after_a_restart() {
    let profile = tempfile::tempdir().unwrap();
    start(profile.path()).choose_language(Language::English).unwrap();
    start(profile.path()).choose_language(Language::System).unwrap();

    assert_eq!(start(profile.path()).language().unwrap(), Language::System);
}

#[test]
fn the_language_reaches_the_interface_and_comes_back_in_the_shape_the_interface_uses() {
    for (language, text) in [(Language::System, "system"), (Language::German, "de"), (Language::English, "en")] {
        assert_eq!(serde_json::to_value(language).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<Language>(serde_json::json!(text)).unwrap(), language);
    }
}
