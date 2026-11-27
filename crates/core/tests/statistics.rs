//! Statistics: totals over the Library, worked out by the core. The total
//! Playtime, how many Achievements are unlocked of all and their share, how
//! many Games are at 100 % Progress, and how many Games have each Status.
//! Hidden Games are left out of every figure.
//!
//! Also the Playtime of each of the last eight weeks, from the Sessions the
//! app recorded (not from Steam's figure, which says nothing about when a
//! Game was played), and the five most played Games by their Playtime.

use std::collections::VecDeque;
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, Downloader, GamesByStatus, OpenError, Opener, PlayedWeek, Ports,
    SecretStore, Statistics, Status, SteamLocator, Store, SyncOutcome, Tracking, Wakeup, WebAnswer, WebApi,
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

/// A clock that stands at NOW, except while scripted looks at the running
/// processes set it to the time of each look.
#[derive(Clone)]
struct ScriptedClock(Arc<Mutex<u64>>);

impl ScriptedClock {
    fn set(&self, at: u64) {
        *self.0.lock().unwrap() = at;
    }
}

impl Clock for ScriptedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(*self.0.lock().unwrap())
    }

    /// The script decides when the next look happens.
    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// A substitute for the process list: answers each scripted look at its
/// time with the executables that run then. Once the script is over, it
/// fails, which ends the tracking in these tests.
#[derive(Clone)]
struct ScriptedProcesses {
    looks: Arc<Mutex<VecDeque<(u64, Vec<PathBuf>)>>>,
    clock: ScriptedClock,
}

impl nicelibrary_core::ProcessList for ScriptedProcesses {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        let Some((at, running)) = self.looks.lock().unwrap().pop_front() else {
            return Err("the script is over".into());
        };
        self.clock.set(at);
        Ok(running)
    }
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
struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
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
    processes: ScriptedProcesses,
    _profile: TempDir,
    disk: TempDir,
    steam: TempDir,
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
        let processes =
            ScriptedProcesses { looks: Arc::default(), clock: ScriptedClock(Arc::new(Mutex::new(NOW))) };
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam_folder.path().to_path_buf())),
                downloader: Box::new(steam.clone()),
                opener: Box::new(NoUrls),
                secrets: Box::new(KeyStored),
                web_api: Box::new(steam.clone()),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(processes.clone()),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                data_file_picker: Box::new(NoExportFile),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        );
        Self { core, processes, _profile: profile, disk: tempfile::tempdir().unwrap(), steam: steam_folder }
    }

    /// The executable of the Local Game `title` on the disk.
    fn executable(&self, title: &str) -> PathBuf {
        self.disk.path().join(title).join(format!("{title}.exe"))
    }

    /// Records a Session of the Local Game `title` from `from` to `to`: the
    /// app sees its process run at both times and gone right after. The
    /// clock stands at NOW again afterwards.
    fn play(&self, title: &str, from: u64, to: u64) {
        self.run(&self.executable(title), from, to);
    }

    /// Records a Session of whatever Game `executable` is from `from` to
    /// `to`, as [`Self::play`] does for a Local Game.
    fn run(&self, executable: &Path, from: u64, to: u64) {
        let running = vec![executable.to_path_buf()];
        self.processes.looks.lock().unwrap().extend([(from, running.clone()), (to, running), (to + 1, Vec::new())]);
        self.core.track_sessions(|event| match event {
            Tracking::Failed { .. } => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        });
        self.processes.clock.set(NOW);
    }

    /// Fully installs the Steam Game `app_id` in the Steam folder, as Steam
    /// does, reads it in, and answers its executable.
    fn install_steam_game(&self, app_id: u32, name: &str) -> PathBuf {
        let steamapps = self.steam.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        let folder = self.steam.path().display().to_string().replace('\\', "\\\\");
        fs::write(
            steamapps.join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{folder}\"\n\t}}\n}}\n"),
        )
        .unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
        let executable = steamapps.join("common").join(name).join("game.exe");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"MZ").unwrap();
        self.core.read_installed_games().unwrap();
        executable
    }

    /// The app after a Sync with Steam.
    fn synced(steam: &FakeSteam) -> Self {
        let app = Self::start(steam);
        assert_eq!(app.core.sync().unwrap(), SyncOutcome::Synced { at: NOW });
        app
    }

    /// Adds the Local Game `title` and answers its id.
    fn add_local_game(&self, title: &str) -> String {
        let executable = self.executable(title);
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
            weeks: (0..8).rev().map(|ago| week(ago, 0)).collect(),
            most_played: Vec::new(),
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
    app.play("Celeste", NOW - 2 * HOUR, NOW - HOUR);
    app.core.set_manual_progress(&celeste, Some(100)).unwrap();
    app.core.set_status(&celeste, Status::Dropped).unwrap();
    app.core.set_status("steam:400", Status::Completed).unwrap();

    app.core.hide_game("steam:400").unwrap();
    app.core.hide_game(&celeste).unwrap();

    let statistics = app.statistics();
    assert_eq!(most_played(&statistics), ["Portal 2", "Half-Life"]);
    assert_eq!(
        Statistics { most_played: Vec::new(), ..statistics },
        Statistics {
            games: 2,
            playtime_minutes: 1_260 + 30,
            achievements_unlocked: 2,
            achievements_total: 4,
            achievement_share: Some(50),
            complete_games: 0,
            games_by_status: GamesByStatus { unplayed: 2, playing: 0, completed: 0, dropped: 0 },
            weeks: (0..8).rev().map(|ago| week(ago, 0)).collect(),
            most_played: Vec::new(),
        }
    );

    app.core.unhide_game(&celeste).unwrap();
    assert_eq!(app.statistics().weeks[7].minutes, 60);
}

#[test]
fn statistics_reach_the_interface_in_its_shape() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2()]));

    let mut statistics = serde_json::to_value(app.statistics()).unwrap();
    let weeks = statistics.as_object_mut().unwrap().remove("weeks").unwrap();
    let most_played = statistics.as_object_mut().unwrap().remove("mostPlayed").unwrap();

    assert_eq!(
        statistics,
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
    // 2026-11-18 12:00:00 to 2026-11-25 12:00:00 UTC.
    assert_eq!(weeks[7], serde_json::json!({ "startsAt": 1_795_003_200, "endsAt": NOW, "minutes": 0 }));
    assert_eq!(most_played[0]["id"], "steam:620");
    assert_eq!(most_played[0]["playtimeMinutes"], 1_260);
}

/// An hour, in seconds.
const HOUR: u64 = 3_600;
/// A day, in seconds.
const DAY: u64 = 24 * HOUR;

/// The week that ended `ago` weeks before NOW, with `minutes` played in it.
fn week(ago: u64, minutes: u32) -> PlayedWeek {
    PlayedWeek { starts_at: NOW - (ago + 1) * 7 * DAY, ends_at: NOW - ago * 7 * DAY, minutes }
}

/// The titles of the most played Games, most played first.
fn most_played(statistics: &Statistics) -> Vec<&str> {
    statistics.most_played.iter().map(|game| game.title.as_str()).collect()
}

#[test]
fn the_weeks_are_the_last_eight_weeks_up_to_now_oldest_first() {
    let app = App::start(&FakeSteam::default());

    let weeks = app.statistics().weeks;

    assert_eq!(weeks.len(), 8);
    assert_eq!(weeks[0], PlayedWeek { starts_at: NOW - 56 * DAY, ends_at: NOW - 49 * DAY, minutes: 0 });
    assert_eq!(weeks[7], PlayedWeek { starts_at: NOW - 7 * DAY, ends_at: NOW, minutes: 0 });
}

#[test]
fn a_session_counts_in_the_week_it_was_played_in() {
    let app = App::start(&FakeSteam::default());
    app.add_local_game("Celeste");
    app.add_local_game("Hades");
    app.play("Celeste", NOW - 3 * DAY, NOW - 3 * DAY + 2 * HOUR);
    app.play("Hades", NOW - 2 * DAY, NOW - 2 * DAY + 45 * 60);
    app.play("Celeste", NOW - 10 * DAY, NOW - 10 * DAY + 30 * 60);
    app.play("Hades", NOW - 50 * DAY, NOW - 50 * DAY + HOUR);

    assert_eq!(
        app.statistics().weeks,
        [week(7, 60), week(6, 0), week(5, 0), week(4, 0), week(3, 0), week(2, 0), week(1, 30), week(0, 165)]
    );
}

#[test]
fn a_session_across_the_end_of_a_week_counts_in_both_weeks_with_its_share() {
    let app = App::start(&FakeSteam::default());
    app.add_local_game("Celeste");
    app.play("Celeste", NOW - 7 * DAY - 40 * 60, NOW - 7 * DAY + 20 * 60);

    let weeks = app.statistics().weeks;

    assert_eq!((weeks[6].minutes, weeks[7].minutes), (40, 20));
}

#[test]
fn sessions_older_than_eight_weeks_count_in_no_week() {
    let app = App::start(&FakeSteam::default());
    app.add_local_game("Celeste");
    app.play("Celeste", NOW - 57 * DAY, NOW - 57 * DAY + 3 * HOUR);
    app.play("Celeste", NOW - 56 * DAY - HOUR, NOW - 56 * DAY + HOUR);

    let statistics = app.statistics();

    assert_eq!(statistics.weeks[0].minutes, 60);
    assert_eq!(statistics.weeks.iter().map(|week| week.minutes).sum::<u32>(), 60);
    assert_eq!(statistics.playtime_minutes, 5 * 60);
}

#[test]
fn steam_playtime_counts_in_no_week_since_steam_says_not_when_it_was_played() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), portal()]));

    let statistics = app.statistics();

    assert_eq!(statistics.playtime_minutes, 1_260 + 95);
    assert!(statistics.weeks.iter().all(|week| week.minutes == 0));

    // The Sessions the app records of a Steam Game count in their weeks;
    // Steam's figure stays the Game's Playtime.
    let portal_2 = app.install_steam_game(620, "Portal 2");
    app.run(&portal_2, NOW - 2 * DAY, NOW - 2 * DAY + 90 * 60);
    app.run(&portal_2, NOW - 9 * DAY, NOW - 9 * DAY + 20 * 60);

    let statistics = app.statistics();
    assert_eq!((statistics.weeks[6].minutes, statistics.weeks[7].minutes), (20, 90));
    assert_eq!(statistics.weeks.iter().map(|week| week.minutes).sum::<u32>(), 110);
}

#[test]
fn the_sessions_of_a_hidden_steam_game_count_in_no_week() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2()]));
    let portal_2 = app.install_steam_game(620, "Portal 2");
    app.run(&portal_2, NOW - 2 * DAY, NOW - 2 * DAY + HOUR);
    assert_eq!(app.statistics().weeks[7].minutes, 60);

    app.core.hide_game("steam:620").unwrap();

    assert!(app.statistics().weeks.iter().all(|week| week.minutes == 0));
}

#[test]
fn a_playtime_adjustment_counts_in_the_total_and_the_most_played_but_in_no_week() {
    let app = App::synced(&FakeSteam::owning(vec![portal_2(), half_life()]));
    let celeste = app.add_local_game("Celeste");
    let tunic = app.add_local_game("Tunic");
    app.play("Celeste", NOW - DAY, NOW - DAY + HOUR);
    app.core.set_playtime_adjustment(&celeste, Some(1_500)).unwrap();
    app.core.set_playtime_adjustment(&tunic, Some(45)).unwrap();

    let statistics = app.statistics();

    assert_eq!(statistics.playtime_minutes, 1_260 + 30 + 60 + 1_500 + 45);
    assert_eq!(most_played(&statistics), ["Celeste", "Portal 2", "Tunic", "Half-Life"]);
    assert_eq!(statistics.most_played[0].playtime_minutes, Some(60 + 1_500));
    assert_eq!(statistics.weeks[7].minutes, 60);
}

#[test]
fn the_five_most_played_games_come_from_both_sources_most_played_first() {
    let app = App::synced(&FakeSteam::owning(vec![
        portal_2(),
        portal(),
        half_life(),
        owned(10, "Counter-Strike", 600, None),
        owned(220, "Half-Life 2", 0, None),
    ]));
    app.add_local_game("Celeste");
    app.add_local_game("Hades");
    app.add_local_game("Tunic");
    app.play("Celeste", NOW - 3 * DAY, NOW - 3 * DAY + 2 * HOUR);
    app.play("Hades", NOW - 90 * DAY, NOW - 90 * DAY + 30 * HOUR);

    assert_eq!(most_played(&app.statistics()), ["Hades", "Portal 2", "Counter-Strike", "Celeste", "Portal"]);
}

#[test]
fn games_not_played_at_all_are_not_among_the_most_played() {
    let app = App::synced(&FakeSteam::owning(vec![half_life(), owned(220, "Half-Life 2", 0, None)]));
    app.add_local_game("Celeste");

    assert_eq!(most_played(&app.statistics()), ["Half-Life"]);
}

#[test]
fn games_played_as_long_as_each_other_are_most_played_in_title_order() {
    let app = App::synced(&FakeSteam::owning(vec![
        owned(10, "Counter-Strike", 60, None),
        owned(70, "Half-Life", 60, None),
    ]));
    app.add_local_game("Amnesia");
    app.play("Amnesia", NOW - DAY, NOW - DAY + HOUR);

    assert_eq!(most_played(&app.statistics()), ["Amnesia", "Counter-Strike", "Half-Life"]);
}

/// No export or import is made here: the user cancels every file dialog.
struct NoExportFile;

impl nicelibrary_core::DataFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }

    fn choose_import_file(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
