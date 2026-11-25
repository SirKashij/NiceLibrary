//! The core must stay usable without Tauri (ADR-0001, spec "Shape").

#[test]
fn the_core_does_not_depend_on_tauri() {
    let manifest = include_str!("../Cargo.toml");

    let tauri_keys: Vec<&str> = manifest
        .lines()
        .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim()))
        .filter(|key| key.to_lowercase().contains("tauri"))
        .collect();

    assert!(
        tauri_keys.is_empty(),
        "crates/core/Cargo.toml must not depend on tauri, found: {tauri_keys:?}"
    );
}
