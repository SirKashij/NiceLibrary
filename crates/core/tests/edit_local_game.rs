//! The user changes a Local Game's title and executable, or removes the Game.
//! Removing takes the Game out of the Library with everything the app keeps
//! about it (its Collection memberships, Sessions, Status, hiding, Cover and
//! the image chosen as its Cover), but never touches its files on disk.
//! Steam Games are neither edited nor removed this way.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, EditLocalGame, ExecutableChoice, ExecutablePicker, Game, LocalGame,
    OpenError, Opener, Ports, Status, SteamLocator, Store,
};
use tempfile::TempDir;

/// A substitute for the operating system's file dialog: answers what the
/// test says the user chose, or that they cancelled.
#[derive(Clone, Default)]
struct UserChooses(Arc<Mutex<Option<PathBuf>>>);

impl UserChooses {
    fn executable(&self, path: &Path) {
        *self.0.lock().unwrap() = Some(path.to_path_buf());
    }
}

impl ExecutablePicker for UserChooses {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(self.0.lock().unwrap().clone())
    }
}

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

/// One running app with a disk of its own that games can be put on.
struct App {
    core: Core,
    user: UserChooses,
    profile: TempDir,
    disk: TempDir,
    _steam: Option<TempDir>,
}

impl App {
    fn start() -> Self {
        Self::start_with_steam(None)
    }

    fn start_with_steam(steam: Option<TempDir>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let user = UserChooses::default();
        let core = Self::core_in(profile.path(), &user, steam.as_ref().map(|steam| steam.path()));
        Self { core, user, profile, disk: tempfile::tempdir().unwrap(), _steam: steam }
    }

    fn core_in(profile: &Path, user: &UserChooses, steam: Option<&Path>) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.map(Path::to_path_buf))),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(user.clone()),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                export_file_picker: Box::new(NoExportFile),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")),
        )
    }

    /// The app started again over the same profile.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.user, None);
        Self { core, ..self }
    }

    /// Puts an executable at `relative` (folders separated by `/`) on the disk.
    fn put(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// Adds the executable at `path` as the Local Game `title`.
    fn add(&self, path: &Path, title: &str) -> Game {
        match self.core.add_local_game(&path_text(path), title).unwrap() {
            AddLocalGame::Added { game } => game,
            other => panic!("expected the game to be added, got {other:?}"),
        }
    }

    fn database(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.profile.path().join("nicelibrary.db")).unwrap()
    }
}

fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
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

// What the edit form shows.

#[test]
fn a_local_game_is_answered_with_its_title_and_executable_for_editing() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");

    assert_eq!(
        app.core.local_game(&game.id).unwrap(),
        Some(LocalGame { id: game.id.clone(), title: "Celeste".into(), executable: path_text(&path) })
    );
}

#[test]
fn a_steam_game_or_an_unknown_game_is_no_local_game_to_edit() {
    let app = App::start_with_steam(Some(steam_with_portal_2()));
    app.core.read_installed_games().unwrap();

    assert_eq!(app.core.local_game("steam:620").unwrap(), None);
    assert_eq!(app.core.local_game("local:999").unwrap(), None);
}

// Editing.

#[test]
fn a_new_title_is_shown_in_the_library_and_kept_after_a_restart() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");

    let edited = app.core.edit_local_game(&game.id, "  Celeste Classic  ", &path_text(&path)).unwrap();

    let renamed = Game { title: "Celeste Classic".into(), ..game.clone() };
    assert_eq!(edited, EditLocalGame::Edited { game: renamed.clone() });
    let app = app.restart();
    assert_eq!(app.core.library().unwrap(), [renamed]);
}

#[test]
fn a_new_executable_replaces_the_old_one_and_the_game_keeps_its_id() {
    let app = App::start();
    let old = app.put("Games/Celeste/Celeste.exe");
    let new = app.put("Games/Celeste 2/Celeste.exe");
    let game = app.add(&old, "Celeste");

    let edited = app.core.edit_local_game(&game.id, "Celeste", &path_text(&new)).unwrap();

    assert_eq!(edited, EditLocalGame::Edited { game: game.clone() });
    assert_eq!(app.core.local_game(&game.id).unwrap().unwrap().executable, path_text(&new));
    // The old executable is free again for a Game of its own.
    assert!(matches!(app.core.add_local_game(&path_text(&old), "Old").unwrap(), AddLocalGame::Added { .. }));
}

#[test]
fn an_empty_title_is_refused_and_changes_nothing() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");

    assert_eq!(app.core.edit_local_game(&game.id, "  ", &path_text(&path)).unwrap(), EditLocalGame::NoTitle);
    assert_eq!(app.core.library().unwrap(), [game]);
}

#[test]
fn a_new_executable_that_is_not_there_is_refused_and_changes_nothing() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");
    let gone = app.disk.path().join("Gone").join("gone.exe");

    assert_eq!(app.core.edit_local_game(&game.id, "Gone", &path_text(&gone)).unwrap(), EditLocalGame::NotFound);
    assert_eq!(app.core.local_game(&game.id).unwrap().unwrap().executable, path_text(&path));
    assert_eq!(app.core.library().unwrap(), [game]);
}

#[test]
fn the_title_can_be_changed_while_the_executable_is_missing() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");
    fs::remove_file(&path).unwrap();

    let edited = app.core.edit_local_game(&game.id, "Celeste (moved)", &path_text(&path)).unwrap();

    assert_eq!(edited, EditLocalGame::Edited { game: Game { title: "Celeste (moved)".into(), ..game } });
}

#[test]
fn an_executable_of_another_local_game_is_refused_with_that_games_title() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    let zork = app.put("Games/Zork/Zork.exe");
    let game = app.add(&celeste, "Celeste");
    app.add(&zork, "Zork");

    assert_eq!(
        app.core.edit_local_game(&game.id, "Celeste", &path_text(&zork)).unwrap(),
        EditLocalGame::AlreadyAdded { title: "Zork".into() }
    );
    assert_eq!(app.core.local_game(&game.id).unwrap().unwrap().executable, path_text(&celeste));
}

#[test]
fn the_games_own_executable_by_another_spelling_is_not_another_game() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");
    let detour = app.disk.path().join("Games").join("Celeste").join("..").join("Celeste").join("Celeste.exe");

    assert!(matches!(
        app.core.edit_local_game(&game.id, "Celeste", &path_text(&detour)).unwrap(),
        EditLocalGame::Edited { .. }
    ));
}

#[test]
fn choosing_a_new_executable_proposes_its_title_and_does_not_count_the_game_itself() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "My Celeste");
    let new = app.put("Games/Hollow Knight/hollow_knight.exe");

    app.user.executable(&path);
    assert_eq!(
        app.core.choose_executable_for(&game.id).unwrap(),
        ExecutableChoice::Chosen { executable: path_text(&path), title: "Celeste".into() }
    );
    app.user.executable(&new);
    assert_eq!(
        app.core.choose_executable_for(&game.id).unwrap(),
        ExecutableChoice::Chosen { executable: path_text(&new), title: "Hollow Knight".into() }
    );
}

#[test]
fn choosing_the_executable_of_another_local_game_says_so_at_once() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    let zork = app.put("Games/Zork/Zork.exe");
    let game = app.add(&celeste, "Celeste");
    app.add(&zork, "Zork");
    app.user.executable(&zork);

    assert_eq!(app.core.choose_executable_for(&game.id).unwrap(), ExecutableChoice::AlreadyAdded { title: "Zork".into() });
}

#[test]
fn a_steam_game_cannot_be_edited() {
    let app = App::start_with_steam(Some(steam_with_portal_2()));
    let before = app.core.read_installed_games().unwrap();
    let path = app.put("Games/Portal 2/portal2.exe");

    assert!(app.core.edit_local_game("steam:620", "Portal Two", &path_text(&path)).is_err());
    assert!(app.core.choose_executable_for("steam:620").is_err());
    assert_eq!(app.core.library().unwrap(), before);
}

#[test]
fn a_local_game_that_is_gone_cannot_be_edited() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");

    assert!(app.core.edit_local_game("local:999", "Celeste", &path_text(&path)).is_err());
    assert!(app.core.library().unwrap().is_empty());
}

#[test]
fn edit_outcomes_reach_the_interface_in_the_shape_the_interface_expects() {
    let game = LocalGame { id: "local:1".into(), title: "Celeste".into(), executable: "/games/Celeste.exe".into() };

    assert_eq!(
        serde_json::to_value(&game).unwrap(),
        serde_json::json!({ "id": "local:1", "title": "Celeste", "executable": "/games/Celeste.exe" })
    );
    assert_eq!(serde_json::to_value(EditLocalGame::NoTitle).unwrap(), serde_json::json!({ "outcome": "noTitle" }));
    assert_eq!(serde_json::to_value(EditLocalGame::NotFound).unwrap(), serde_json::json!({ "outcome": "notFound" }));
    assert_eq!(
        serde_json::to_value(EditLocalGame::AlreadyAdded { title: "Zork".into() }).unwrap(),
        serde_json::json!({ "outcome": "alreadyAdded", "title": "Zork" })
    );
    let app = App::start();
    let added = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let edited = serde_json::to_value(EditLocalGame::Edited { game: added.clone() }).unwrap();
    assert_eq!(edited["outcome"], serde_json::json!("edited"));
    assert_eq!(edited["game"], serde_json::to_value(&added).unwrap());
}

// Removing.

#[test]
fn a_removed_local_game_leaves_the_library_also_after_a_restart() {
    let app = App::start();
    let celeste = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let zork = app.add(&app.put("Games/Zork/Zork.exe"), "Zork");

    app.core.remove_local_game(&celeste.id).unwrap();

    assert_eq!(app.core.library().unwrap(), [zork.clone()]);
    assert_eq!(app.core.game(&celeste.id).unwrap(), None);
    let app = app.restart();
    assert_eq!(app.core.library().unwrap(), [zork]);
}

#[test]
fn removing_never_touches_the_games_files_on_disk() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    fs::write(path.parent().unwrap().join("save.dat"), b"progress").unwrap();
    let game = app.add(&path, "Celeste");

    app.core.remove_local_game(&game.id).unwrap();

    assert_eq!(fs::read(&path).unwrap(), b"MZ");
    assert_eq!(fs::read(path.parent().unwrap().join("save.dat")).unwrap(), b"progress");
}

#[test]
fn a_removed_executable_can_be_added_again_as_a_new_game_without_its_old_memberships() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let game = app.add(&path, "Celeste");
    let collection = match app.core.create_collection("Platformers").unwrap() {
        nicelibrary_core::CollectionNaming::Named { id } => id,
        other => panic!("expected a Collection, got {other:?}"),
    };
    // No operation puts a Game into a Collection yet; the store is written as one would.
    app.database()
        .execute("INSERT INTO collection_games (collection_id, game_id) VALUES (?1, ?2)", rusqlite::params![collection, game.id])
        .unwrap();
    assert_eq!(app.core.collections().unwrap()[0].game_count, 1);

    app.core.remove_local_game(&game.id).unwrap();
    let again = app.add(&path, "Celeste");

    assert_ne!(again.id, game.id);
    assert_eq!(app.core.collections().unwrap()[0].game_count, 0);
}

#[test]
fn removing_a_local_game_deletes_its_collection_memberships_and_keeps_the_collection() {
    let app = App::start();
    let celeste = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let zork = app.add(&app.put("Games/Zork/Zork.exe"), "Zork");
    let collection = match app.core.create_collection("Favourites").unwrap() {
        nicelibrary_core::CollectionNaming::Named { id } => id,
        other => panic!("expected a Collection, got {other:?}"),
    };
    let database = app.database();
    for game in [&celeste, &zork] {
        database
            .execute("INSERT INTO collection_games (collection_id, game_id) VALUES (?1, ?2)", rusqlite::params![collection, game.id])
            .unwrap();
    }

    app.core.remove_local_game(&celeste.id).unwrap();

    // Nothing of the Game stays behind, not even where no operation looks.
    let members: Vec<String> = database
        .prepare("SELECT game_id FROM collection_games")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(members, [zork.id.clone()]);
    assert_eq!(app.core.collections().unwrap()[0].game_count, 1);
}

#[test]
fn removing_a_local_game_deletes_its_cached_cover() {
    let app = App::start();
    let game = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let other = app.add(&app.put("Games/Zork/Zork.exe"), "Zork");
    // The cache keeps a Game's Cover in a file named after its id.
    let covers = app.profile.path().join("covers");
    fs::create_dir_all(&covers).unwrap();
    let cover_of = |game: &Game| covers.join(game.id.replace(':', "-"));
    fs::write(cover_of(&game), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(cover_of(&other), b"\x89PNG\r\n\x1a\n").unwrap();

    app.core.remove_local_game(&game.id).unwrap();

    assert!(!cover_of(&game).exists());
    assert!(cover_of(&other).exists());
}

#[test]
fn removing_a_local_game_deletes_the_image_the_user_chose_as_its_cover() {
    let app = App::start();
    let game = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let other = app.add(&app.put("Games/Zork/Zork.exe"), "Zork");
    // Chosen images are kept as `local-<number>` beside the cached Covers,
    // in `chosen` unless the app names another folder.
    let chosen = app.profile.path().join("covers").join("chosen");
    fs::create_dir_all(&chosen).unwrap();
    let image_of = |game: &Game| chosen.join(game.id.replace(':', "-"));
    fs::write(image_of(&game), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(image_of(&other), b"\x89PNG\r\n\x1a\n").unwrap();
    assert!(app.core.cover_image_chosen(&game.id).unwrap());

    app.core.remove_local_game(&game.id).unwrap();

    assert!(!image_of(&game).exists());
    assert!(image_of(&other).exists());
    assert!(app.core.cover_image_chosen(&other.id).unwrap());
}

#[test]
fn removing_a_local_game_deletes_its_sessions_status_hiding_and_cover_match() {
    let app = App::start();
    let game = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");
    let other = app.add(&app.put("Games/Zork/Zork.exe"), "Zork");
    let database = app.database();
    for (number, game) in [(1, &game), (2, &other)] {
        assert_eq!(game.id, format!("local:{number}"));
        database
            .execute(
                "INSERT INTO local_game_sessions (local_game, started_at, ended_at) VALUES (?1, 1000, 2000)",
                [number],
            )
            .unwrap();
        database
            .execute("INSERT INTO cover_matches (local_game, app_id, name) VALUES (?1, 220, 'Half-Life 2')", [number])
            .unwrap();
        app.core.set_status(&game.id, Status::Playing).unwrap();
        app.core.hide_game(&game.id).unwrap();
    }

    app.core.remove_local_game(&game.id).unwrap();

    // Nothing of the Game stays behind, not even where no operation looks.
    let left = |query: &str| -> Vec<String> {
        database.prepare(query).unwrap().query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(left("SELECT 'local:' || local_game FROM local_game_sessions"), [other.id.clone()]);
    assert_eq!(left("SELECT game_id FROM game_statuses"), [other.id.clone()]);
    assert_eq!(left("SELECT game_id FROM hidden_games"), [other.id.clone()]);
    assert_eq!(left("SELECT 'local:' || local_game FROM cover_matches"), [other.id.clone()]);
    assert_eq!(app.core.hidden_games().unwrap().iter().map(|game| game.id.clone()).collect::<Vec<_>>(), [other.id]);
}

#[test]
fn removing_a_local_game_that_is_gone_already_changes_nothing() {
    let app = App::start();
    let game = app.add(&app.put("Games/Celeste/Celeste.exe"), "Celeste");

    app.core.remove_local_game("local:999").unwrap();

    assert_eq!(app.core.library().unwrap(), [game]);
}

#[test]
fn a_steam_game_cannot_be_removed() {
    let app = App::start_with_steam(Some(steam_with_portal_2()));
    let before = app.core.read_installed_games().unwrap();

    assert!(app.core.remove_local_game("steam:620").is_err());
    assert_eq!(app.core.library().unwrap(), before);
    assert_eq!(before.len(), 1);
}

/// A computer on which the app never starts with Windows.
struct NoAutostart;

impl nicelibrary_core::Autostart for NoAutostart {
    fn is_on(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn turn_on(&self, _arguments: &[&str]) -> Result<(), String> {
        Err("this test does not start the app with Windows".into())
    }

    fn turn_off(&self) -> Result<(), String> {
        Ok(())
    }
}

/// A computer whose executables show no icon and whose user chooses no
/// image; these tests need neither.
struct NoIconsNoImages;

impl nicelibrary_core::IconExtractor for NoIconsNoImages {
    fn largest_icon(&self, _executable: &std::path::Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}

impl nicelibrary_core::ImagePicker for NoIconsNoImages {
    fn choose_image(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(
        &self,
        executable: &std::path::Path,
        _arguments: &[String],
        _working_directory: &std::path::Path,
    ) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// No export or import is made here: the user cancels every file dialog.
struct NoExportFile;

impl nicelibrary_core::ExportFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }

    fn choose_import_file(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
