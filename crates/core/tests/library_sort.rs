//! Sorting the Library: by recently played (the default), by name or by
//! Playtime. Every order is stable, puts the Games without a value last, and
//! the order the user chose survives a restart. The core answers it, not the
//! interface.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    Clock, Core, CoverCache, Downloader, LibraryFilter, LibrarySort, OpenError, Opener, Ports, SecretStore,
    SteamLocator, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account logged in on the test computer.
const PLAYER: &str = "76561197960287930";

/// A Steam Web API where PLAYER owns Portal 2, Half-Life 2, Hades and Braid,
/// and a secret store that holds the key. Braid was played last together with
/// Portal 2 and as long as Hades; Half-Life 2 was never played.
struct SteamOwning;

impl WebApi for SteamOwning {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        assert!(url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?"), "asked for {url}");
        let body = serde_json::json!({ "response": { "game_count": 4, "games": [
            { "appid": 620, "name": "Portal 2", "playtime_forever": 600, "rtime_last_played": 1_700_000_000 },
            { "appid": 220, "name": "Half-Life 2", "playtime_forever": 0, "rtime_last_played": 0 },
            { "appid": 1145360, "name": "Hades", "playtime_forever": 90, "rtime_last_played": 1_750_000_000 },
            { "appid": 26800, "name": "Braid", "playtime_forever": 90, "rtime_last_played": 1_700_000_000 },
        ] } });
        Ok(WebAnswer { status: 200, body: body.to_string().into_bytes() })
    }
}

impl SecretStore for SteamOwning {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(Some(KEY.into()))
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Err("this test removes no secret".into())
    }
}

struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// No Covers are needed here.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// These tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &Wakeup) {}
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// The test computer: Steam with PLAYER logged in, Portal 2 and "celeste"
/// Installed. "celeste" is not among the Games Steam says PLAYER owns, so it
/// has neither Playtime nor Last Played.
struct Computer {
    profile: TempDir,
    steam: TempDir,
}

impl Computer {
    fn new() -> Self {
        let steam = tempfile::tempdir().unwrap();
        let path = steam.path();
        fs::create_dir_all(path.join("config")).unwrap();
        fs::write(
            path.join("config").join("loginusers.vdf"),
            format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "One" "MostRecent" "1" }} }}"#),
        )
        .unwrap();
        fs::create_dir_all(path.join("steamapps")).unwrap();
        let library = path.display().to_string().replace('\\', "\\\\");
        fs::write(
            path.join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{library}\"\n\t}}\n}}\n"),
        )
        .unwrap();
        for (app_id, name) in [(620, "Portal 2"), (504230, "celeste")] {
            install(path, app_id, name);
        }
        Self { profile: tempfile::tempdir().unwrap(), steam }
    }

    /// Starts the app on this computer, with whatever an earlier start stored.
    fn start(&self) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(self.steam.path().to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(SteamOwning),
                web_api: Box::new(SteamOwning),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        )
    }

    /// Starts the app, reads what is Installed and syncs, so the Library holds all five Games.
    fn start_and_sync(&self) -> Core {
        let core = self.start();
        core.read_installed_games().unwrap();
        assert!(matches!(core.sync().unwrap(), SyncOutcome::Synced { .. }));
        core
    }
}

/// Writes an app manifest as Steam does for a fully installed app.
fn install(steam: &Path, app_id: u32, name: &str) {
    fs::write(
        steam.join("steamapps").join(format!("appmanifest_{app_id}.acf")),
        format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
        ),
    )
    .unwrap();
}

/// The titles of the whole Library in `sort`, in the order `core` answers them.
fn sorted(core: &Core, sort: LibrarySort) -> Vec<String> {
    let filter = LibraryFilter { sort, ..Default::default() };
    core.filter_library(&filter).unwrap().into_iter().map(|game| game.title).collect()
}

#[test]
fn recently_played_puts_the_game_played_last_first_and_the_games_never_played_last() {
    let core = Computer::new().start_and_sync();

    // Braid and Portal 2 were played last at the same moment, so they keep the order of their titles,
    // and so do celeste and Half-Life 2, which were never played.
    assert_eq!(sorted(&core, LibrarySort::RecentlyPlayed), ["Hades", "Braid", "Portal 2", "celeste", "Half-Life 2"]);
}

#[test]
fn name_orders_by_title_ignoring_case() {
    let core = Computer::new().start_and_sync();

    assert_eq!(sorted(&core, LibrarySort::Name), ["Braid", "celeste", "Hades", "Half-Life 2", "Portal 2"]);
}

#[test]
fn playtime_puts_the_game_played_longest_first_and_the_games_without_playtime_last() {
    let core = Computer::new().start_and_sync();

    // Braid and Hades were played as long, so they keep the order of their titles. Half-Life 2 has a
    // Playtime of nothing, which is still a Playtime; celeste has none at all.
    assert_eq!(sorted(&core, LibrarySort::Playtime), ["Portal 2", "Braid", "Hades", "Half-Life 2", "celeste"]);
}

#[test]
fn the_order_sorts_what_the_filters_leave() {
    let core = Computer::new().start_and_sync();

    let filter = LibraryFilter { installed_only: true, sort: LibrarySort::Playtime, ..Default::default() };
    let titles: Vec<_> = core.filter_library(&filter).unwrap().into_iter().map(|game| game.title).collect();
    assert_eq!(titles, ["Portal 2", "celeste"]);
}

#[test]
fn the_library_is_sorted_by_recently_played_until_the_user_chooses() {
    let core = Computer::new().start();

    assert_eq!(core.library_sort().unwrap(), LibrarySort::RecentlyPlayed);
    assert_eq!(LibraryFilter::default().sort, LibrarySort::RecentlyPlayed);
}

#[test]
fn the_chosen_order_survives_a_restart() {
    let computer = Computer::new();
    computer.start().choose_library_sort(LibrarySort::Playtime).unwrap();

    assert_eq!(computer.start().library_sort().unwrap(), LibrarySort::Playtime);

    computer.start().choose_library_sort(LibrarySort::Name).unwrap();
    assert_eq!(computer.start().library_sort().unwrap(), LibrarySort::Name);
}

#[test]
fn an_order_arrives_from_the_interface_in_the_shape_the_interface_sends() {
    let sent = serde_json::json!({ "search": "", "source": null, "installedOnly": false, "sort": "playtime" });
    let filter: LibraryFilter = serde_json::from_value(sent).unwrap();
    assert_eq!(filter, LibraryFilter { sort: LibrarySort::Playtime, ..Default::default() });

    for (sent, sort) in [
        ("recentlyPlayed", LibrarySort::RecentlyPlayed),
        ("name", LibrarySort::Name),
        ("playtime", LibrarySort::Playtime),
    ] {
        assert_eq!(serde_json::from_value::<LibrarySort>(serde_json::json!(sent)).unwrap(), sort);
        assert_eq!(serde_json::to_value(sort).unwrap(), serde_json::json!(sent));
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
