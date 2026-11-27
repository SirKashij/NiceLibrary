//! A Local Game is added by choosing its executable. The app proposes a title
//! derived from the executable's and its folders' names, the user may correct
//! it, and the Game joins the Library: always Installed, of the Local Source.
//! The same executable is never added twice.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, ExecutableChoice, ExecutablePicker, Game, OpenError, Opener, Ports,
    Source, Status, SteamLocator, Store,
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

    fn to_cancel(&self) {
        *self.0.lock().unwrap() = None;
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

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
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
}

impl App {
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let user = UserChooses::default();
        let core = Self::core_in(profile.path(), &user);
        Self { core, user, profile, disk: tempfile::tempdir().unwrap() }
    }

    fn core_in(profile: &Path, user: &UserChooses) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(NoSecretsNoWebApi),
                web_api: Box::new(NoSecretsNoWebApi),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(user.clone()),
                autostart: Box::new(NoAutostart),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")),
        )
    }

    /// The app started again over the same profile.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.user);
        Self { core, ..self }
    }

    /// Puts an executable at `relative` (folders separated by `/`) on the disk.
    fn put(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"MZ").unwrap();
        path
    }

    /// The title the app proposes when the user chooses the executable at `relative`.
    fn proposed_title(&self, relative: &str) -> String {
        self.user.executable(&self.put(relative));
        match self.core.choose_local_game_executable().unwrap() {
            ExecutableChoice::Chosen { title, .. } => title,
            other => panic!("expected a chosen executable, got {other:?}"),
        }
    }

    /// Chooses the executable at `path` and adds it under the proposed title.
    fn add(&self, path: &Path) -> AddLocalGame {
        self.user.executable(path);
        let ExecutableChoice::Chosen { executable, title } = self.core.choose_local_game_executable().unwrap() else {
            panic!("expected a chosen executable");
        };
        self.core.add_local_game(&executable, &title).unwrap()
    }
}

fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

#[test]
fn choosing_an_executable_proposes_a_title_and_adding_it_puts_the_game_in_the_library() {
    let app = App::start();
    let celeste = app.put("Games/Celeste/Celeste.exe");
    app.user.executable(&celeste);

    let choice = app.core.choose_local_game_executable().unwrap();
    assert_eq!(choice, ExecutableChoice::Chosen { executable: path_text(&celeste), title: "Celeste".into() });

    let added = app.core.add_local_game(&path_text(&celeste), "Celeste").unwrap();
    let AddLocalGame::Added { game } = added else { panic!("expected the game to be added, got {added:?}") };
    assert_eq!(app.core.library().unwrap(), [game]);
}

#[test]
fn a_local_game_is_installed_belongs_to_the_local_source_and_has_not_been_played() {
    let app = App::start();

    let AddLocalGame::Added { game } = app.add(&app.put("Games/Celeste/Celeste.exe")) else { panic!() };

    assert_eq!(
        game,
        Game {
            id: game.id.clone(),
            title: "Celeste".into(),
            source: Source::Local,
            installed: true,
            playtime_minutes: Some(0),
            last_played: None,
            progress: None,
            status: Status::Unplayed,
        }
    );
    assert!(game.id.starts_with("local:"), "{}", game.id);
}

#[test]
fn a_local_game_is_found_for_its_detail_page() {
    let app = App::start();

    let AddLocalGame::Added { game } = app.add(&app.put("Games/Celeste/Celeste.exe")) else { panic!() };

    assert_eq!(app.core.game(&game.id).unwrap(), Some(game));
    assert_eq!(app.core.game("local:999").unwrap(), None);
}

#[test]
fn the_user_may_correct_the_proposed_title() {
    let app = App::start();
    let path = app.put("Games/HK/hollow_knight.exe");

    let added = app.core.add_local_game(&path_text(&path), "  Hollow Knight: Voidheart  ").unwrap();

    assert!(matches!(added, AddLocalGame::Added { ref game } if game.title == "Hollow Knight: Voidheart"));
    assert_eq!(app.core.library().unwrap()[0].title, "Hollow Knight: Voidheart");
}

#[test]
fn an_empty_title_is_refused_and_adds_nothing() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");

    assert_eq!(app.core.add_local_game(&path_text(&path), "   ").unwrap(), AddLocalGame::NoTitle);
    assert!(app.core.library().unwrap().is_empty());
}

#[test]
fn an_executable_that_is_not_there_is_refused_and_adds_nothing() {
    let app = App::start();
    let gone = app.disk.path().join("Gone").join("gone.exe");

    assert_eq!(app.core.add_local_game(&path_text(&gone), "Gone").unwrap(), AddLocalGame::NotFound);
    assert!(app.core.library().unwrap().is_empty());
}

#[test]
fn cancelling_the_choice_adds_nothing() {
    let app = App::start();
    app.user.to_cancel();

    assert_eq!(app.core.choose_local_game_executable().unwrap(), ExecutableChoice::Cancelled);
    assert!(app.core.library().unwrap().is_empty());
}

#[test]
fn a_failing_file_dialog_is_an_error() {
    struct BrokenDialog;
    impl ExecutablePicker for BrokenDialog {
        fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
            Err("no dialog today".into())
        }
    }
    let profile = tempfile::tempdir().unwrap();
    let core = Core::new(
        Ports {
            steam_locator: Box::new(NoSteam),
            downloader: Box::new(Offline),
            opener: Box::new(NoUrls),
            secrets: Box::new(NoSecretsNoWebApi),
            web_api: Box::new(NoSecretsNoWebApi),
            clock: Box::new(StoppedClock),
            executable_picker: Box::new(BrokenDialog),
            autostart: Box::new(NoAutostart),
        },
        Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.path().join("covers")),
    );

    assert!(core.choose_local_game_executable().is_err());
}

#[test]
fn adding_the_same_executable_twice_is_refused_with_the_title_it_has_already() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    app.core.add_local_game(&path_text(&path), "My Celeste").unwrap();

    assert_eq!(
        app.core.add_local_game(&path_text(&path), "Celeste again").unwrap(),
        AddLocalGame::AlreadyAdded { title: "My Celeste".into() }
    );
    assert_eq!(app.core.library().unwrap().len(), 1);
}

#[test]
fn choosing_an_executable_already_added_says_so_at_once() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    app.core.add_local_game(&path_text(&path), "My Celeste").unwrap();
    app.user.executable(&path);

    assert_eq!(
        app.core.choose_local_game_executable().unwrap(),
        ExecutableChoice::AlreadyAdded { title: "My Celeste".into() }
    );
}

#[test]
fn the_same_executable_by_another_spelling_of_its_path_is_still_the_same() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    app.core.add_local_game(&path_text(&path), "Celeste").unwrap();
    let detour = app.disk.path().join("Games").join("Celeste").join("..").join("Celeste").join("Celeste.exe");

    assert_eq!(
        app.core.add_local_game(&path_text(&detour), "Celeste").unwrap(),
        AddLocalGame::AlreadyAdded { title: "Celeste".into() }
    );
}

#[test]
fn two_executables_in_one_folder_are_two_games() {
    let app = App::start();
    let game = app.put("Games/Pack/first.exe");
    let other = app.put("Games/Pack/second.exe");

    assert!(matches!(app.core.add_local_game(&path_text(&game), "First").unwrap(), AddLocalGame::Added { .. }));
    assert!(matches!(app.core.add_local_game(&path_text(&other), "Second").unwrap(), AddLocalGame::Added { .. }));
    assert_eq!(app.core.library().unwrap().len(), 2);
}

#[test]
fn local_games_are_kept_after_a_restart_and_still_refused_twice() {
    let app = App::start();
    let path = app.put("Games/Celeste/Celeste.exe");
    let AddLocalGame::Added { game } = app.add(&path) else { panic!() };

    let app = app.restart();

    assert_eq!(app.core.library().unwrap(), [game]);
    assert_eq!(
        app.core.add_local_game(&path_text(&path), "Celeste").unwrap(),
        AddLocalGame::AlreadyAdded { title: "Celeste".into() }
    );
}

#[test]
fn the_library_orders_local_games_by_title() {
    let app = App::start();
    app.add(&app.put("Games/Zork/Zork.exe"));
    app.add(&app.put("Games/Celeste/celeste.exe"));
    app.add(&app.put("Games/Antichamber/Antichamber.exe"));

    let titles: Vec<_> = app.core.library().unwrap().into_iter().map(|game| game.title).collect();

    assert_eq!(titles, ["Antichamber", "Celeste", "Zork"]);
}

#[test]
fn a_local_game_has_no_cover_yet() {
    let app = App::start();
    let AddLocalGame::Added { game } = app.add(&app.put("Games/Celeste/Celeste.exe")) else { panic!() };

    assert_eq!(app.core.cover(&game.id).unwrap(), None);
}

#[test]
fn the_executable_is_stored_as_chosen_without_windows_only_rewriting() {
    let app = App::start();
    // A name only some systems would treat specially; it is kept as it is.
    let path = app.put("Spiele/Ünïcode Game/Ünïcode Game.exe");

    let ExecutableChoice::Chosen { executable, .. } = ({
        app.user.executable(&path);
        app.core.choose_local_game_executable().unwrap()
    }) else {
        panic!()
    };

    assert_eq!(executable, path_text(&path));
}

#[test]
fn a_game_choice_reaches_the_interface_in_the_shape_the_interface_expects() {
    let chosen = ExecutableChoice::Chosen { executable: "/games/celeste/Celeste.exe".into(), title: "Celeste".into() };
    let refused = AddLocalGame::AlreadyAdded { title: "Celeste".into() };

    assert_eq!(
        serde_json::to_value(&chosen).unwrap(),
        serde_json::json!({ "outcome": "chosen", "executable": "/games/celeste/Celeste.exe", "title": "Celeste" })
    );
    assert_eq!(serde_json::to_value(ExecutableChoice::Cancelled).unwrap(), serde_json::json!({ "outcome": "cancelled" }));
    assert_eq!(serde_json::to_value(&refused).unwrap(), serde_json::json!({ "outcome": "alreadyAdded", "title": "Celeste" }));
    assert_eq!(serde_json::to_value(AddLocalGame::NoTitle).unwrap(), serde_json::json!({ "outcome": "noTitle" }));
    assert_eq!(serde_json::to_value(AddLocalGame::NotFound).unwrap(), serde_json::json!({ "outcome": "notFound" }));
}

// Title derivation.

#[test]
fn the_title_is_the_executable_name_without_its_extension() {
    assert_eq!(App::start().proposed_title("Games/Celeste/Celeste.exe"), "Celeste");
}

#[test]
fn separators_become_spaces_and_lowercase_words_are_capitalised() {
    let app = App::start();
    assert_eq!(app.proposed_title("Games/HK/hollow_knight.exe"), "Hollow Knight");
    assert_eq!(app.proposed_title("Games/X/risk-of-rain.exe"), "Risk Of Rain");
    assert_eq!(app.proposed_title("Games/Y/Super.Meat.Boy.exe"), "Super Meat Boy");
    assert_eq!(app.proposed_title("Games/Z/Into  the__Breach.exe"), "Into The Breach");
}

#[test]
fn version_numbers_are_stripped_but_a_number_in_the_title_stays() {
    let app = App::start();
    assert_eq!(app.proposed_title("Games/A/Hades_v1.38.exe"), "Hades");
    assert_eq!(app.proposed_title("Games/B/Celeste-1.4.0.0.exe"), "Celeste");
    assert_eq!(app.proposed_title("Games/C/Game Name v2.exe"), "Game Name");
    assert_eq!(app.proposed_title("Games/D/Portal 2.exe"), "Portal 2");
    assert_eq!(app.proposed_title("Games/E/Witcher3.exe"), "Witcher3");
}

#[test]
fn platform_suffixes_are_stripped() {
    let app = App::start();
    assert_eq!(app.proposed_title("Games/A/Satisfactory-Win64-Shipping.exe"), "Satisfactory");
    assert_eq!(app.proposed_title("Games/B/Terraria_x64.exe"), "Terraria");
    assert_eq!(app.proposed_title("Games/C/Factorio_x86_64.exe"), "Factorio");
    assert_eq!(app.proposed_title("Games/D/Doom (64-bit).exe"), "Doom");
    assert_eq!(app.proposed_title("Games/E/Quake_dx12_Win32.exe"), "Quake");
}

#[test]
fn a_generic_executable_name_gives_way_to_the_folder_name() {
    let app = App::start();
    assert_eq!(app.proposed_title("Games/Stardew Valley/game.exe"), "Stardew Valley");
    assert_eq!(app.proposed_title("Games/Outer Wilds/Launcher.exe"), "Outer Wilds");
    assert_eq!(app.proposed_title("Games/Some_Indie_Game v1.2/Start.exe"), "Some Indie Game");
    assert_eq!(app.proposed_title("Games/Valheim/game_x64.exe"), "Valheim");
}

#[test]
fn generic_folders_are_passed_over_for_the_one_above() {
    let app = App::start();
    assert_eq!(app.proposed_title("Games/Deep Rock/Binaries/Win64/launcher.exe"), "Deep Rock");
    assert_eq!(app.proposed_title("Games/Noita/bin/x64/game.exe"), "Noita");
}

#[test]
fn a_local_game_is_found_by_the_local_source_and_installed_only_filters() {
    let app = App::start();
    let AddLocalGame::Added { game } = app.add(&app.put("Games/Celeste/Celeste.exe")) else { panic!() };
    let filter = nicelibrary_core::LibraryFilter { source: Some(Source::Local), installed_only: true, ..Default::default() };

    assert_eq!(app.core.filter_library(&filter).unwrap(), [game]);
}

/// A computer where the app does not start with Windows; these tests do not turn it on.
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
