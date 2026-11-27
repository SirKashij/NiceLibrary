//! Which Games are in which Collection: the user puts a Game into several
//! Collections at once and takes it out again, the sidebar counts follow,
//! and choosing a Collection narrows the Library to its Games.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{AddLocalGame, CollectionNaming, Core, CoverCache, LibraryFilter, Ports, SteamLocator, Store};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// The user profile of one test: where the database lives between starts.
struct Profile {
    folder: TempDir,
}

impl Profile {
    fn new() -> Self {
        Self { folder: tempfile::tempdir().unwrap() }
    }

    /// Starts the app on a computer without Steam.
    fn start(&self) -> Core {
        self.start_with_steam(None)
    }

    /// Starts the app on a computer with Steam at `steam`, or without.
    fn start_with_steam(&self, steam: Option<&Path>) -> Core {
        let store = Store::open(&self.folder.path().join("nicelibrary.db")).unwrap();
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.map(Path::to_path_buf))),
                ..stand_ins::ports()
            },
            store,
            CoverCache::new(self.folder.path().join("covers")),
        )
    }
}

/// A Steam installation with one account that has logged in and one
/// Installed Steam Game, Portal 2.
fn steam_with_portal_2() -> TempDir {
    let steam = tempfile::tempdir().unwrap();
    fs::create_dir_all(steam.path().join("config")).unwrap();
    fs::write(
        steam.path().join("config").join("loginusers.vdf"),
        r#""users" { "76561197960287930" { "PersonaName" "Player" "MostRecent" "1" } }"#,
    )
    .unwrap();
    let path = steam.path().display().to_string().replace('\\', "\\\\");
    fs::create_dir_all(steam.path().join("steamapps")).unwrap();
    fs::write(
        steam.path().join("steamapps").join("libraryfolders.vdf"),
        format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t}}\n}}\n"),
    )
    .unwrap();
    fs::write(
        steam.path().join("steamapps").join("appmanifest_620.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"name\"\t\t\"Portal 2\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"Portal 2\"\n}\n",
    )
    .unwrap();
    steam
}

const PORTAL_2: &str = "steam:620";

/// The id of the Collection a naming created.
fn created(naming: CollectionNaming) -> i64 {
    match naming {
        CollectionNaming::Named { id } => id,
        other => panic!("expected the Collection to be named, got {other:?}"),
    }
}

/// Each Collection's name with how many Games the sidebar shows for it.
fn counts(core: &Core) -> Vec<(String, u32)> {
    core.collections().unwrap().into_iter().map(|collection| (collection.name, collection.game_count)).collect()
}

fn titles(core: &Core, filter: LibraryFilter) -> Vec<String> {
    core.filter_library(&filter).unwrap().into_iter().map(|game| game.title).collect()
}

/// A Library with the Steam Game Portal 2, Installed, and the Local Game
/// Hades; answers the Core, the Steam folder and Hades' id, which must stay
/// alive together.
fn library_of_two(profile: &Profile) -> (Core, TempDir, String) {
    let steam = steam_with_portal_2();
    let core = profile.start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    let executable = profile.folder.path().join("Hades.exe");
    fs::write(&executable, b"MZ").unwrap();
    let hades = match core.add_local_game(executable.to_str().unwrap(), "Hades").unwrap() {
        AddLocalGame::Added { game } => game.id,
        other => panic!("expected Hades to be added, got {other:?}"),
    };
    (core, steam, hades)
}

#[test]
fn a_game_can_be_in_several_collections_at_once() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    let coop = created(core.create_collection("Co-op").unwrap());
    let backlog = created(core.create_collection("Backlog").unwrap());
    created(core.create_collection("Roguelikes").unwrap());

    core.add_game_to_collection(coop, PORTAL_2).unwrap();
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();

    assert_eq!(core.game_collections(PORTAL_2).unwrap(), [backlog, coop]);
    assert_eq!(counts(&core), [("Backlog".into(), 1), ("Co-op".into(), 1), ("Roguelikes".into(), 0)]);
}

#[test]
fn a_game_taken_out_of_a_collection_stays_in_its_other_collections_and_in_the_library() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    let library = core.read_installed_games().unwrap();
    let backlog = created(core.create_collection("Backlog").unwrap());
    let coop = created(core.create_collection("Co-op").unwrap());
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();
    core.add_game_to_collection(coop, PORTAL_2).unwrap();

    core.remove_game_from_collection(backlog, PORTAL_2).unwrap();

    assert_eq!(core.game_collections(PORTAL_2).unwrap(), [coop]);
    assert_eq!(counts(&core), [("Backlog".into(), 0), ("Co-op".into(), 1)]);
    assert_eq!(core.library().unwrap(), library);
}

#[test]
fn a_game_put_into_a_collection_twice_counts_once() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    let backlog = created(core.create_collection("Backlog").unwrap());

    core.add_game_to_collection(backlog, PORTAL_2).unwrap();
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();

    assert_eq!(counts(&core), [("Backlog".into(), 1)]);
}

#[test]
fn taking_a_game_out_of_a_collection_it_is_not_in_changes_nothing() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    let backlog = created(core.create_collection("Backlog").unwrap());

    core.remove_game_from_collection(backlog, PORTAL_2).unwrap();
    core.remove_game_from_collection(42, PORTAL_2).unwrap();

    assert!(core.game_collections(PORTAL_2).unwrap().is_empty());
    assert_eq!(counts(&core), [("Backlog".into(), 0)]);
}

#[test]
fn a_game_is_in_no_collection_until_it_is_put_into_one() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    created(core.create_collection("Backlog").unwrap());

    assert!(core.game_collections(PORTAL_2).unwrap().is_empty());
}

#[test]
fn putting_a_game_into_a_collection_that_does_not_exist_fails_and_changes_nothing() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    core.read_installed_games().unwrap();
    let backlog = created(core.create_collection("Backlog").unwrap());
    core.delete_collection(backlog).unwrap();

    assert!(core.add_game_to_collection(backlog, PORTAL_2).is_err());
    assert!(core.game_collections(PORTAL_2).unwrap().is_empty());
}

#[test]
fn putting_a_game_that_is_not_in_the_library_into_a_collection_fails_and_changes_nothing() {
    let core = Profile::new().start();
    let backlog = created(core.create_collection("Backlog").unwrap());

    assert!(core.add_game_to_collection(backlog, PORTAL_2).is_err());
    assert_eq!(counts(&core), [("Backlog".into(), 0)]);
    assert!(core.game_collections(PORTAL_2).unwrap().is_empty());
}

#[test]
fn choosing_a_collection_narrows_the_library_to_its_games() {
    let profile = Profile::new();
    let (core, _steam, hades) = library_of_two(&profile);
    let backlog = created(core.create_collection("Backlog").unwrap());
    let coop = created(core.create_collection("Co-op").unwrap());
    let empty = created(core.create_collection("Empty").unwrap());
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();
    core.add_game_to_collection(backlog, &hades).unwrap();
    core.add_game_to_collection(coop, PORTAL_2).unwrap();

    let only = |collection| LibraryFilter { collection: Some(collection), ..Default::default() };
    assert_eq!(titles(&core, only(backlog)), ["Hades", "Portal 2"]);
    assert_eq!(titles(&core, only(coop)), ["Portal 2"]);
    assert!(titles(&core, only(empty)).is_empty());
    core.remove_game_from_collection(coop, PORTAL_2).unwrap();
    assert!(titles(&core, only(coop)).is_empty());
}

#[test]
fn a_hidden_game_is_neither_shown_nor_counted_in_its_collections_until_it_is_unhidden() {
    let profile = Profile::new();
    let (core, _steam, hades) = library_of_two(&profile);
    let backlog = created(core.create_collection("Backlog").unwrap());
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();
    core.add_game_to_collection(backlog, &hades).unwrap();
    let only = LibraryFilter { collection: Some(backlog), ..Default::default() };

    core.hide_game(PORTAL_2).unwrap();

    assert_eq!(titles(&core, only.clone()), ["Hades"]);
    assert_eq!(counts(&core), [("Backlog".into(), 1)]);
    // Hidden, it is in no view the user puts Games into Collections from.
    let coop = created(core.create_collection("Co-op").unwrap());
    assert!(core.add_game_to_collection(coop, PORTAL_2).is_err());

    core.unhide_game(PORTAL_2).unwrap();
    assert_eq!(titles(&core, only).len(), 2);
    assert_eq!(counts(&core), [("Backlog".into(), 2), ("Co-op".into(), 0)]);
    assert_eq!(core.game_collections(PORTAL_2).unwrap(), [backlog]);
}

#[test]
fn a_collection_combines_with_the_other_filters_and_the_search() {
    let profile = Profile::new();
    let (core, _steam, hades) = library_of_two(&profile);
    let backlog = created(core.create_collection("Backlog").unwrap());
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();
    core.add_game_to_collection(backlog, &hades).unwrap();

    let installed = LibraryFilter { collection: Some(backlog), installed_only: true, ..Default::default() };
    assert_eq!(titles(&core, installed), ["Hades", "Portal 2"]);
    let searched = LibraryFilter { collection: Some(backlog), search: "port".into(), ..Default::default() };
    assert_eq!(titles(&core, searched), ["Portal 2"]);
}

#[test]
fn a_deleted_collection_narrows_the_library_to_no_game() {
    let profile = Profile::new();
    let (core, _steam, _hades) = library_of_two(&profile);
    let backlog = created(core.create_collection("Backlog").unwrap());
    core.add_game_to_collection(backlog, PORTAL_2).unwrap();

    core.delete_collection(backlog).unwrap();

    assert!(titles(&core, LibraryFilter { collection: Some(backlog), ..Default::default() }).is_empty());
    assert!(core.game_collections(PORTAL_2).unwrap().is_empty());
}

#[test]
fn which_games_are_in_a_collection_survives_a_restart() {
    let profile = Profile::new();
    let steam = steam_with_portal_2();
    let backlog = {
        let core = profile.start_with_steam(Some(steam.path()));
        core.read_installed_games().unwrap();
        let backlog = created(core.create_collection("Backlog").unwrap());
        core.add_game_to_collection(backlog, PORTAL_2).unwrap();
        backlog
    };

    let core = profile.start_with_steam(Some(steam.path()));

    assert_eq!(core.game_collections(PORTAL_2).unwrap(), [backlog]);
    assert_eq!(counts(&core), [("Backlog".into(), 1)]);
}

#[test]
fn a_chosen_collection_reaches_the_core_in_the_shape_the_interface_sends() {
    let sent = serde_json::json!({ "search": "", "source": null, "installedOnly": false, "collection": 7 });

    let filter: LibraryFilter = serde_json::from_value(sent).unwrap();

    assert_eq!(filter, LibraryFilter { collection: Some(7), ..Default::default() });
}
