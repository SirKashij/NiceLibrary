//! Collections: named groups of Games the user creates, renames and deletes.
//! A name is never empty and never shared by two Collections, whatever its
//! case; deleting a Collection removes no Game; Collections survive restarts.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    Collection, CollectionNaming, Core, CoverCache, Downloader, OpenError, Opener, Ports, SteamLocator, Store,
};
use tempfile::TempDir;

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

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
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
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(NothingRuns),
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

/// The id of the Collection a naming created.
fn created(naming: CollectionNaming) -> i64 {
    match naming {
        CollectionNaming::Named { id } => id,
        other => panic!("expected the Collection to be named, got {other:?}"),
    }
}

fn names(core: &Core) -> Vec<String> {
    core.collections().unwrap().into_iter().map(|collection| collection.name).collect()
}

#[test]
fn a_fresh_core_has_no_collections() {
    let profile = Profile::new();

    assert!(profile.start().collections().unwrap().is_empty());
}

#[test]
fn a_created_collection_is_listed_with_its_name_and_no_games() {
    let core = Profile::new().start();

    let id = created(core.create_collection("Couch co-op").unwrap());

    assert_eq!(core.collections().unwrap(), vec![Collection { id, name: "Couch co-op".into(), game_count: 0 }]);
}

#[test]
fn collections_are_listed_by_name_whatever_its_case() {
    let core = Profile::new().start();

    for name in ["Roguelikes", "backlog", "Couch co-op"] {
        created(core.create_collection(name).unwrap());
    }

    assert_eq!(names(&core), ["backlog", "Couch co-op", "Roguelikes"]);
}

#[test]
fn a_name_is_saved_without_the_spaces_around_it() {
    let core = Profile::new().start();

    created(core.create_collection("  Backlog \t").unwrap());

    assert_eq!(names(&core), ["Backlog"]);
}

#[test]
fn an_empty_name_is_refused_and_creates_nothing() {
    let core = Profile::new().start();

    assert_eq!(core.create_collection("").unwrap(), CollectionNaming::EmptyName);
    assert_eq!(core.create_collection("   ").unwrap(), CollectionNaming::EmptyName);

    assert!(core.collections().unwrap().is_empty());
}

#[test]
fn a_name_another_collection_has_is_refused_whatever_its_case() {
    let core = Profile::new().start();
    created(core.create_collection("Backlog").unwrap());
    created(core.create_collection("Ärger").unwrap());

    assert_eq!(core.create_collection("Backlog").unwrap(), CollectionNaming::NameTaken);
    assert_eq!(core.create_collection(" BACKLOG ").unwrap(), CollectionNaming::NameTaken);
    assert_eq!(core.create_collection("ÄRGER").unwrap(), CollectionNaming::NameTaken);

    assert_eq!(core.collections().unwrap().len(), 2);
}

#[test]
fn a_renamed_collection_keeps_its_id_and_shows_the_new_name() {
    let core = Profile::new().start();
    let id = created(core.create_collection("Backlog").unwrap());

    assert_eq!(core.rename_collection(id, " Pile of shame ").unwrap(), CollectionNaming::Named { id });

    assert_eq!(core.collections().unwrap(), vec![Collection { id, name: "Pile of shame".into(), game_count: 0 }]);
}

#[test]
fn a_collection_can_be_renamed_to_its_own_name_in_another_case() {
    let core = Profile::new().start();
    let id = created(core.create_collection("backlog").unwrap());

    assert_eq!(core.rename_collection(id, "Backlog").unwrap(), CollectionNaming::Named { id });

    assert_eq!(names(&core), ["Backlog"]);
}

#[test]
fn renaming_to_an_empty_name_or_one_another_collection_has_is_refused_and_changes_nothing() {
    let core = Profile::new().start();
    let backlog = created(core.create_collection("Backlog").unwrap());
    created(core.create_collection("Roguelikes").unwrap());

    assert_eq!(core.rename_collection(backlog, "  ").unwrap(), CollectionNaming::EmptyName);
    assert_eq!(core.rename_collection(backlog, "roguelikes").unwrap(), CollectionNaming::NameTaken);

    assert_eq!(names(&core), ["Backlog", "Roguelikes"]);
}

#[test]
fn renaming_a_collection_that_does_not_exist_fails() {
    let core = Profile::new().start();

    assert!(core.rename_collection(42, "Backlog").is_err());
    assert!(core.collections().unwrap().is_empty());
}

#[test]
fn a_deleted_collection_is_gone_and_its_name_free_again() {
    let core = Profile::new().start();
    let backlog = created(core.create_collection("Backlog").unwrap());
    created(core.create_collection("Roguelikes").unwrap());

    core.delete_collection(backlog).unwrap();

    assert_eq!(names(&core), ["Roguelikes"]);
    created(core.create_collection("Backlog").unwrap());
}

#[test]
fn deleting_a_collection_removes_no_game() {
    let steam = steam_with_portal_2();
    let core = Profile::new().start_with_steam(Some(steam.path()));
    let before = core.read_installed_games().unwrap();
    assert_eq!(before.len(), 1);
    let id = created(core.create_collection("Backlog").unwrap());

    core.delete_collection(id).unwrap();

    assert_eq!(core.library().unwrap(), before);
}

#[test]
fn deleting_a_collection_that_does_not_exist_changes_nothing() {
    let core = Profile::new().start();
    created(core.create_collection("Backlog").unwrap());

    core.delete_collection(42).unwrap();

    assert_eq!(names(&core), ["Backlog"]);
}

#[test]
fn collections_survive_a_restart() {
    let profile = Profile::new();
    let (backlog, kept) = {
        let core = profile.start();
        let backlog = created(core.create_collection("Backlog").unwrap());
        let kept = created(core.create_collection("Roguelikes").unwrap());
        core.rename_collection(backlog, "Pile of shame").unwrap();
        let gone = created(core.create_collection("Gone").unwrap());
        core.delete_collection(gone).unwrap();
        (backlog, kept)
    };

    let core = profile.start();

    assert_eq!(
        core.collections().unwrap(),
        vec![
            Collection { id: backlog, name: "Pile of shame".into(), game_count: 0 },
            Collection { id: kept, name: "Roguelikes".into(), game_count: 0 },
        ]
    );
}

#[test]
fn a_collection_reaches_the_interface_in_the_shape_the_interface_expects() {
    let collection = Collection { id: 7, name: "Backlog".into(), game_count: 3 };

    assert_eq!(
        serde_json::to_value(&collection).unwrap(),
        serde_json::json!({ "id": 7, "name": "Backlog", "gameCount": 3 })
    );
    assert_eq!(
        serde_json::to_value(CollectionNaming::Named { id: 7 }).unwrap(),
        serde_json::json!({ "outcome": "named", "id": 7 })
    );
    assert_eq!(
        serde_json::to_value(CollectionNaming::EmptyName).unwrap(),
        serde_json::json!({ "outcome": "emptyName" })
    );
    assert_eq!(
        serde_json::to_value(CollectionNaming::NameTaken).unwrap(),
        serde_json::json!({ "outcome": "nameTaken" })
    );
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}
