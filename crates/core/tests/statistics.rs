//! Statistics: totals over the Library, worked out by the core. The total
//! Playtime, how many Achievements are unlocked of all and their share, how
//! many Games are at 100 % Progress, and how many Games have each Status.
//! Hidden Games are left out of every figure.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, Downloader, GamesByStatus, OpenError, Opener, Ports, SecretStore,
    Statistics, Status, SteamLocator, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account on the test computer.
const PLAYER: &str = "76561197960287930";
/// 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch.
const NOW: u64 = 1_795_608_000;
/// 2026-11-20 20:15:00 UTC.
const LAST_WEEK: u64 = 1_795_205_700;

/// One Steam Game the account owns in the fake Steam, with which of its
/// Achievements the account unlocked; `None` for a game without stats.
#[derive(Clone)]
struct Owned {
    app_id: u32,
    name: &'static str,
    minutes: u32,
    achievements: Option<Vec<bool>>,
}

/// A fake of Steam's Web API in the shape Steam answers with.
#[derive(Clone, Default)]
struct FakeSteam(Arc<Mutex<Vec<Owned>>>);

impl FakeSteam {
    fn owning(games: Vec<Owned>) -> Self {
        Self(Arc::new(Mutex::new(games)))
    }

    fn game(&self, url: &str) -> Option<Owned> {
        let app_id: u32 = query_value(url, "appid")?.parse().ok()?;
        self.0.lock().unwrap().iter().find(|game| game.app_id == app_id).cloned()
    }
}

/// The value of `name` in the query of `url`.
fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

fn answer(status: u16, body: serde_json::Value) -> Result<WebAnswer, String> {
    Ok(WebAnswer { status, body: body.to_string().into_bytes() })
}

impl WebApi for FakeSteam {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        if query_value(url, "key") != Some(KEY) {
            return Ok(WebAnswer { status: 403, body: b"<html><body>Forbidden</body></html>".to_vec() });
        }
        if url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") {
            let games: Vec<_> = self
                .0
                .lock()
                .unwrap()
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "appid": g.app_id, "name": g.name, "playtime_forever": g.minutes,
                        "rtime_last_played": LAST_WEEK, "has_community_visible_stats": g.achievements.is_some()
                    })
                })
                .collect();
            return answer(200, serde_json::json!({ "response": { "game_count": games.len(), "games": games } }));
        }
        if url.starts_with("https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?") {
            let Some(achievements) = self.game(url).and_then(|game| game.achievements) else {
                let error = "Requested app has no stats";
                return answer(400, serde_json::json!({ "playerstats": { "error": error, "success": false } }));
            };
            let achievements: Vec<_> = achievements
                .iter()
                .enumerate()
                .map(|(n, &unlocked)| {
                    serde_json::json!({
                        "apiname": format!("ACH_{n}"),
                        "achieved": u8::from(unlocked),
                        "unlocktime": if unlocked { LAST_WEEK } else { 0 }
                    })
                })
                .collect();
            return answer(
                200,
                serde_json::json!({ "playerstats": { "steamID": PLAYER, "success": true, "achievements": achievements } }),
            );
        }
        if url.starts_with("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?") {
            let Some(achievements) = self.game(url).and_then(|game| game.achievements) else {
                return answer(200, serde_json::json!({ "game": {} }));
            };
            let defined: Vec<_> = (0..achievements.len())
                .map(|n| serde_json::json!({ "name": format!("ACH_{n}"), "displayName": format!("Achievement {n}"), "hidden": 0 }))
                .collect();
            return answer(200, serde_json::json!({ "game": { "availableGameStats": { "achievements": defined } } }));
        }
        Ok(WebAnswer { status: 404, body: b"<html><body>Not Found</body></html>".to_vec() })
    }
}

impl Downloader for FakeSteam {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
}

struct KeyStored;

impl SecretStore for KeyStored {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(Some(KEY.into()))
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}

/// A clock that stands at NOW.
struct StoppedClock;

impl Clock for StoppedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(NOW)
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// These tests add Local Games by path, without the file dialog.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// Steam on disk with PLAYER logged in.
struct SteamAt(TempDir);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.path().to_path_buf())
    }
}

/// A computer where the app does not start with Windows; these tests do not change that.
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

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// A computer that starts no process; these tests launch nothing.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// A computer whose executables show no icon and whose user chooses no image.
struct NoIconsNoImages;

impl nicelibrary_core::IconExtractor for NoIconsNoImages {
    fn largest_icon(&self, _executable: &Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}

impl nicelibrary_core::ImagePicker for NoIconsNoImages {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// One running app over a fresh profile, with a disk for Local Games.
struct App {
    core: Core,
    _profile: TempDir,
    disk: TempDir,
}

impl App {
    fn start(steam: &FakeSteam) -> Self {
        let steam_folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam_folder.path().join("config")).unwrap();
        fs::write(
            steam_folder.path().join("config").join("loginusers.vdf"),
            format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
        )
        .unwrap();
        let profile = tempfile::tempdir().unwrap();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam_folder)),
                downloader: Box::new(steam.clone()),
                opener: Box::new(NoUrls),
                secrets: Box::new(KeyStored),
                web_api: Box::new(steam.clone()),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        );
        Self { core, _profile: profile, disk: tempfile::tempdir().unwrap() }
    }

    /// The app after a Sync with Steam.
    fn synced(steam: &FakeSteam) -> Self {
        let app = Self::start(steam);
        assert_eq!(app.core.sync().unwrap(), SyncOutcome::Synced { at: NOW });
        app
    }

    /// Adds the Local Game `title` and answers its id.
    fn add_local_game(&self, title: &str) -> String {
        let executable = self.disk.path().join(title).join(format!("{title}.exe"));
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"MZ").unwrap();
        match self.core.add_local_game(executable.to_str().unwrap(), title).unwrap() {
            AddLocalGame::Added { game } => game.id,
            other => panic!("expected {title} to be added, got {other:?}"),
        }
    }

    fn statistics(&self) -> Statistics {
        self.core.statistics().unwrap()
    }
}

fn owned(app_id: u32, name: &'static str, minutes: u32, achievements: Option<Vec<bool>>) -> Owned {
    Owned { app_id, name, minutes, achievements }
}

/// Portal 2, 21 hours played, 2 of its 4 Achievements unlocked.
fn portal_2() -> Owned {
    owned(620, "Portal 2", 1_260, Some(vec![true, false, true, false]))
}

/// Portal, 95 minutes played, its one Achievement unlocked: at 100 % Progress.
fn portal() -> Owned {
    owned(400, "Portal", 95, Some(vec![true]))
}

/// Half-Life, 30 minutes played, without Achievements.
fn half_life() -> Owned {
    owned(70, "Half-Life", 30, None)
}

#[test]
fn a_fresh_library_has_statistics_of_nothing() {
    let app = App::start(&FakeSteam::default());

    assert_eq!(
        app.statistics(),
        Statistics {
            games: 0,
            playtime_minutes: 0,
            achievements_unlocked: 0,
            achievements_total: 0,
            achievement_share: None,
            complete_games: 0,
            games_by_status: GamesByStatus { unplayed: 0, playing: 0, completed: 0, dropped: 0 },
        }
    );
}

#[test]
fn the_total_playtime_is_the_playtime_of_every_game_together() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal(), half_life()]));
    app.add_local_game("Celeste");

    let statistics = app.statistics();

    assert_eq!(statistics.games, 4);
    assert_eq!(statistics.playtime_minutes, 1_260 + 95 + 30);
}

#[test]
fn achievements_are_counted_across_steam_games_with_the_share_unlocked() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal(), half_life()]));

    let statistics = app.statistics();

    assert_eq!((statistics.achievements_unlocked, statistics.achievements_total), (3, 5));
    assert_eq!(statistics.achievement_share, Some(60));
}

#[test]
fn the_share_of_achievements_is_rounded_down_so_only_all_of_them_make_100() {
    let mut almost = vec![true; 199];
    almost.push(false);
    let app = App::synced(&FakeSteam::owning(vec![owned(10, "Counter-Strike", 30, Some(almost))]));

    assert_eq!(app.statistics().achievement_share, Some(99));
}

#[test]
fn a_library_without_achievements_has_no_share_rather_than_none_of_it() {
    let app = App::synced(&FakeSteam::owning(vec![half_life()]));
    app.add_local_game("Celeste");

    let statistics = app.statistics();

    assert_eq!((statistics.achievements_unlocked, statistics.achievements_total), (0, 0));
    assert_eq!(statistics.achievement_share, None);
}

#[test]
fn games_at_100_percent_progress_are_counted_from_both_sources() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal(), half_life()]));
    let celeste = app.add_local_game("Celeste");
    let hades = app.add_local_game("Hades");
    app.add_local_game("Tunic");
    app.core.set_manual_progress(&celeste, Some(100)).unwrap();
    app.core.set_manual_progress(&hades, Some(99)).unwrap();

    assert_eq!(app.statistics().complete_games, 2);
}

#[test]
fn games_are_counted_by_status_and_a_game_without_one_set_is_unplayed() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal(), half_life()]));
    let celeste = app.add_local_game("Celeste");
    app.core.set_status("steam:620", Status::Playing).unwrap();
    app.core.set_status("steam:400", Status::Completed).unwrap();
    app.core.set_status(&celeste, Status::Completed).unwrap();

    assert_eq!(
        app.statistics().games_by_status,
        GamesByStatus { unplayed: 1, playing: 1, completed: 2, dropped: 0 }
    );
}

#[test]
fn hidden_games_are_left_out_of_every_figure() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal(), half_life()]));
    let celeste = app.add_local_game("Celeste");
    app.core.set_manual_progress(&celeste, Some(100)).unwrap();
    app.core.set_status(&celeste, Status::Dropped).unwrap();
    app.core.set_status("steam:400", Status::Completed).unwrap();

    app.core.hide_game("steam:400").unwrap();
    app.core.hide_game(&celeste).unwrap();

    assert_eq!(
        app.statistics(),
        Statistics {
            games: 2,
            playtime_minutes: 1_260 + 30,
            achievements_unlocked: 2,
            achievements_total: 4,
            achievement_share: Some(50),
            complete_games: 0,
            games_by_status: GamesByStatus { unplayed: 2, playing: 0, completed: 0, dropped: 0 },
        }
    );

    app.core.unhide_game("steam:400").unwrap();
    assert_eq!(app.statistics().complete_games, 1);
}

#[test]
fn statistics_reach_the_interface_in_its_shape() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2()]));

    assert_eq!(
        serde_json::to_value(app.statistics()).unwrap(),
        serde_json::json!({
            "games": 1,
            "playtimeMinutes": 1_260,
            "achievementsUnlocked": 2,
            "achievementsTotal": 4,
            "achievementShare": 50,
            "completeGames": 0,
            "gamesByStatus": { "unplayed": 1, "playing": 0, "completed": 0, "dropped": 0 }
        })
    );
}
