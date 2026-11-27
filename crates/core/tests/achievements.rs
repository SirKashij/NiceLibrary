//! Achievements: a Sync also fetches the Achievements of the Steam Account's
//! Steam Games and their definitions from Steam. A Game's detail page lists
//! them, unlocked ones first, with how many of all are unlocked. Their icons
//! are cached on disk like Covers. Fetching is spread over Syncs so that a
//! large Library stays well within Steam's daily limit of Web API calls.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    Achievement, Clock, Core, Cover, CoverCache, Downloader, GameAchievements, OpenError, Opener, Ports, SecretStore,
    SteamLocator, Store, SyncOutcome, Wakeup, WebAnswer, WebApi,
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
/// 2026-10-01 18:00:00 UTC.
const IN_OCTOBER: u64 = 1_790_877_600;

/// The icon of an Achievement as Steam names it: a hash of the image.
fn icon(name: &str, unlocked: bool) -> String {
    let seed: u32 = name.bytes().map(u32::from).sum::<u32>() + u32::from(unlocked);
    format!("{seed:040x}")
}

/// One Achievement of a Steam Game in the fake Steam, with the account's state.
#[derive(Clone)]
struct Defined {
    api_name: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    /// When the account unlocked it; `Some(0)` for unlocked at an unknown time.
    unlocked_at: Option<u64>,
}

fn unlocked(api_name: &'static str, name: &'static str, description: &'static str, at: u64) -> Defined {
    Defined { api_name, name, description: Some(description), unlocked_at: Some(at) }
}

fn locked(api_name: &'static str, name: &'static str, description: &'static str) -> Defined {
    Defined { api_name, name, description: Some(description), unlocked_at: None }
}

/// One Steam Game the account owns in the fake Steam.
#[derive(Clone)]
struct Owned {
    app_id: u32,
    name: &'static str,
    minutes: u32,
    last_played: u64,
    /// `None` for a game without stats, which Steam answers with an error.
    achievements: Option<Vec<Defined>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Network {
    Online,
    Offline,
    /// Steam's achievement services fail, its list of owned games works.
    AchievementsDown,
}

#[derive(Default)]
struct SteamState {
    games: Vec<Owned>,
    network: Option<Network>,
    asked: Vec<String>,
    downloaded: Vec<String>,
}

/// A fake of Steam: its Web API in the shape Steam answers with, and its
/// image servers for Achievement icons. It remembers what it was asked.
#[derive(Clone, Default)]
struct FakeSteam(Arc<Mutex<SteamState>>);

impl FakeSteam {
    fn owning(games: Vec<Owned>) -> Self {
        let steam = Self::default();
        steam.0.lock().unwrap().games = games;
        steam
    }

    fn becomes(&self, network: Network) {
        self.0.lock().unwrap().network = Some(network);
    }

    fn owns(&self, games: Vec<Owned>) {
        self.0.lock().unwrap().games = games;
    }

    fn asked(&self) -> Vec<String> {
        self.0.lock().unwrap().asked.clone()
    }

    fn forget_what_was_asked(&self) {
        let mut state = self.0.lock().unwrap();
        state.asked.clear();
        state.downloaded.clear();
    }

    fn downloaded(&self) -> Vec<String> {
        self.0.lock().unwrap().downloaded.clone()
    }

    fn network(&self) -> Network {
        self.0.lock().unwrap().network.unwrap_or(Network::Online)
    }

    fn game(&self, url: &str) -> Option<Owned> {
        let app_id: u32 = query_value(url, "appid")?.parse().ok()?;
        self.0.lock().unwrap().games.iter().find(|game| game.app_id == app_id).cloned()
    }
}

/// The value of `name` in the query of `url`.
fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

fn answer(status: u16, body: serde_json::Value) -> Result<WebAnswer, String> {
    Ok(WebAnswer { status, body: body.to_string().into_bytes() })
}

const NO_STATS: &str = "Requested app has no stats";

impl WebApi for FakeSteam {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        self.0.lock().unwrap().asked.push(url.to_owned());
        let network = self.network();
        if network == Network::Offline {
            return Err("no connection to the server".into());
        }
        if query_value(url, "key") != Some(KEY) {
            return Ok(WebAnswer { status: 403, body: b"<html><body>Forbidden</body></html>".to_vec() });
        }
        if url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") {
            let games: Vec<_> = self
                .0
                .lock()
                .unwrap()
                .games
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "appid": g.app_id, "name": g.name, "playtime_forever": g.minutes,
                        "rtime_last_played": g.last_played, "has_community_visible_stats": g.achievements.is_some()
                    })
                })
                .collect();
            return answer(200, serde_json::json!({ "response": { "game_count": games.len(), "games": games } }));
        }
        let achievements_asked = url.starts_with("https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?")
            || url.starts_with("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?");
        if achievements_asked && network == Network::AchievementsDown {
            return Ok(WebAnswer { status: 503, body: b"<html><body>Service Unavailable</body></html>".to_vec() });
        }
        if url.starts_with("https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?") {
            assert_eq!(query_value(url, "steamid"), Some(PLAYER));
            let Some(game) = self.game(url) else {
                return answer(400, serde_json::json!({ "playerstats": { "error": "Invalid appid", "success": false } }));
            };
            let Some(achievements) = game.achievements else {
                return answer(400, serde_json::json!({ "playerstats": { "error": NO_STATS, "success": false } }));
            };
            let mut stats = serde_json::json!({ "steamID": PLAYER, "gameName": game.name, "success": true });
            if !achievements.is_empty() {
                stats["achievements"] = achievements
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "apiname": a.api_name,
                            "achieved": u8::from(a.unlocked_at.is_some()),
                            "unlocktime": a.unlocked_at.unwrap_or(0)
                        })
                    })
                    .collect();
            }
            return answer(200, serde_json::json!({ "playerstats": stats }));
        }
        if url.starts_with("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?") {
            let Some(achievements) = self.game(url).and_then(|game| game.achievements).filter(|a| !a.is_empty()) else {
                return answer(200, serde_json::json!({ "game": {} }));
            };
            let app_id = query_value(url, "appid").unwrap();
            let image = |hash: String| {
                format!("https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/{app_id}/{hash}.jpg")
            };
            let defined: Vec<_> = achievements
                .iter()
                .map(|a| {
                    let mut entry = serde_json::json!({
                        "name": a.api_name, "defaultvalue": 0, "displayName": a.name, "hidden": 0,
                        "icon": image(icon(a.api_name, true)), "icongray": image(icon(a.api_name, false))
                    });
                    if let Some(description) = a.description {
                        entry["description"] = description.into();
                    }
                    entry
                })
                .collect();
            return answer(
                200,
                serde_json::json!({ "game": { "gameName": "x", "gameVersion": "1", "availableGameStats": { "achievements": defined } } }),
            );
        }
        Ok(WebAnswer { status: 404, body: b"<html><body>Not Found</body></html>".to_vec() })
    }
}

/// An Achievement icon as Steam's image servers send it.
const ICON: &[u8] = b"\xFF\xD8\xFF generated for the test";

impl Downloader for FakeSteam {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        if self.network() == Network::Offline {
            return Err("no network connection".into());
        }
        self.0.lock().unwrap().downloaded.push(url.to_owned());
        let icon_url = url.starts_with("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/");
        Ok(icon_url.then(|| ICON.to_vec()))
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

/// A clock the test sets.
#[derive(Clone)]
struct TestClock(Arc<Mutex<SystemTime>>);

impl TestClock {
    fn at(seconds: u64) -> Self {
        Self(Arc::new(Mutex::new(UNIX_EPOCH + Duration::from_secs(seconds))))
    }

    fn advance(&self, seconds: u64) {
        *self.0.lock().unwrap() += Duration::from_secs(seconds);
    }
}

impl Clock for TestClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    fn sleep(&self, duration: Duration, wakeup: &Wakeup) {
        if !wakeup.take() {
            *self.0.lock().unwrap() += duration;
        }
    }
}

struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// These tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// Steam on disk with PLAYER logged in.
struct SteamAt(TempDir);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<std::path::PathBuf> {
        Some(self.0.path().to_path_buf())
    }
}

fn steam_on_disk() -> SteamAt {
    let steam = tempfile::tempdir().unwrap();
    fs::create_dir_all(steam.path().join("config")).unwrap();
    fs::write(
        steam.path().join("config").join("loginusers.vdf"),
        format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
    )
    .unwrap();
    SteamAt(steam)
}

/// The user profile of one test.
struct Computer {
    profile: TempDir,
    clock: TestClock,
}

impl Computer {
    fn new() -> Self {
        Self { profile: tempfile::tempdir().unwrap(), clock: TestClock::at(NOW) }
    }

    fn start(&self, steam: &FakeSteam) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(steam_on_disk()),
                downloader: Box::new(steam.clone()),
                opener: Box::new(NoUrls),
                secrets: Box::new(KeyStored),
                web_api: Box::new(steam.clone()),
                clock: Box::new(self.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.covers()),
        )
    }

    fn covers(&self) -> std::path::PathBuf {
        self.profile.path().join("covers")
    }
}

fn portal_2() -> Owned {
    Owned {
        app_id: 620,
        name: "Portal 2",
        minutes: 1_260,
        last_played: LAST_WEEK,
        achievements: Some(vec![
            locked("ACH_SPEED_RUN", "Speed run", "Finish the game in under two hours"),
            unlocked("ACH_WAKE_UP", "Wake up call", "Survive the manual override", IN_OCTOBER),
            locked("ACH_PARTNER", "Partner", "Finish the cooperative campaign"),
            unlocked("ACH_FINISH", "Lunacy", "Finish the single-player campaign", LAST_WEEK),
        ]),
    }
}

fn achievement(api_name: &str, name: &str, description: &str, unlocked_at: Option<u64>) -> Achievement {
    Achievement {
        id: api_name.into(),
        name: name.into(),
        description: Some(description.into()),
        unlocked: unlocked_at.is_some(),
        unlocked_at,
        icon: Some(icon(api_name, unlocked_at.is_some())),
    }
}

fn played(app_id: u32, name: &'static str, minutes: u32, achievements: Option<Vec<Defined>>) -> Owned {
    Owned { app_id, name, minutes, last_played: LAST_WEEK - u64::from(app_id), achievements }
}

fn one_achievement() -> Option<Vec<Defined>> {
    Some(vec![unlocked("ACH_ONE", "One", "The first", IN_OCTOBER)])
}

fn listed(core: &Core, game_id: &str) -> Option<(u32, u32)> {
    match core.achievements(game_id).unwrap() {
        GameAchievements::Listed { unlocked, total, .. } => Some((unlocked, total)),
        _ => None,
    }
}

#[test]
fn after_a_sync_a_steam_game_lists_its_achievements_unlocked_ones_first_with_how_many_are_unlocked() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW });

    assert_eq!(
        core.achievements("steam:620").unwrap(),
        GameAchievements::Listed {
            unlocked: 2,
            total: 4,
            achievements: vec![
                achievement("ACH_FINISH", "Lunacy", "Finish the single-player campaign", Some(LAST_WEEK)),
                achievement("ACH_WAKE_UP", "Wake up call", "Survive the manual override", Some(IN_OCTOBER)),
                achievement("ACH_SPEED_RUN", "Speed run", "Finish the game in under two hours", None),
                achievement("ACH_PARTNER", "Partner", "Finish the cooperative campaign", None),
            ],
        }
    );
}

#[test]
fn the_achievements_are_kept_after_a_restart() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    computer.start(&steam).sync().unwrap();
    steam.becomes(Network::Offline);

    assert_eq!(listed(&computer.start(&steam), "steam:620"), Some((2, 4)));
}

#[test]
fn an_achievement_unlocked_at_an_unknown_time_counts_as_unlocked_without_a_date() {
    let computer = Computer::new();
    let mut game = portal_2();
    game.achievements = Some(vec![
        locked("ACH_PARTNER", "Partner", "Finish the cooperative campaign"),
        Defined { unlocked_at: Some(0), ..unlocked("ACH_OLD", "Old", "Unlocked long ago", 0) },
    ]);
    let steam = FakeSteam::owning(vec![game]);
    let core = computer.start(&steam);
    core.sync().unwrap();

    let GameAchievements::Listed { unlocked, achievements, .. } = core.achievements("steam:620").unwrap() else {
        panic!("listed");
    };
    assert_eq!(unlocked, 1);
    assert_eq!((achievements[0].id.as_str(), achievements[0].unlocked, achievements[0].unlocked_at), ("ACH_OLD", true, None));
}

#[test]
fn an_achievement_without_a_description_has_none() {
    let computer = Computer::new();
    let mut game = portal_2();
    game.achievements = Some(vec![Defined { description: None, ..locked("ACH_SECRET", "Secret", "") }]);
    let steam = FakeSteam::owning(vec![game]);
    let core = computer.start(&steam);
    core.sync().unwrap();

    let GameAchievements::Listed { achievements, .. } = core.achievements("steam:620").unwrap() else {
        panic!("listed");
    };
    assert_eq!(achievements[0].description, None);
}

#[test]
fn a_steam_game_without_achievements_says_so() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![
        played(400, "Portal", 95, None),
        played(70, "Half-Life", 30, Some(vec![])),
    ]);
    let core = computer.start(&steam);
    core.sync().unwrap();

    assert_eq!(core.achievements("steam:400").unwrap(), GameAchievements::NoAchievements);
    assert_eq!(core.achievements("steam:70").unwrap(), GameAchievements::NoAchievements);
}

#[test]
fn before_a_sync_has_fetched_them_a_steam_game_says_that_its_achievements_are_not_fetched_yet() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);

    assert_eq!(core.achievements("steam:620").unwrap(), GameAchievements::NotFetchedYet);
}

#[test]
fn a_local_game_has_no_achievements() {
    let computer = Computer::new();
    let core = computer.start(&FakeSteam::default());

    assert_eq!(core.achievements("local:1").unwrap(), GameAchievements::LocalGame);
}

#[test]
fn a_later_sync_brings_an_achievement_unlocked_since() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    let mut game = portal_2();
    game.minutes += 30;
    game.last_played = NOW + 600;
    game.achievements.as_mut().unwrap()[2].unlocked_at = Some(NOW + 600);
    steam.owns(vec![game]);
    computer.clock.advance(1_800);

    core.sync().unwrap();

    let GameAchievements::Listed { unlocked, achievements, .. } = core.achievements("steam:620").unwrap() else {
        panic!("listed");
    };
    assert_eq!(unlocked, 3);
    assert_eq!(achievements[0], achievement("ACH_PARTNER", "Partner", "Finish the cooperative campaign", Some(NOW + 600)));
}

#[test]
fn a_game_not_played_since_its_achievements_were_fetched_is_not_asked_again() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2(), played(400, "Portal", 95, None)]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    steam.forget_what_was_asked();
    computer.clock.advance(1_800);

    core.sync().unwrap();

    let asked = steam.asked();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].contains("GetOwnedGames"));
}

#[test]
fn the_definitions_of_a_games_achievements_are_fetched_once() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    let mut game = portal_2();
    game.minutes += 30;
    steam.owns(vec![game]);
    steam.forget_what_was_asked();
    computer.clock.advance(1_800);

    core.sync().unwrap();

    let asked = steam.asked();
    assert!(asked.iter().any(|url| url.contains("GetPlayerAchievements")), "{asked:?}");
    assert!(!asked.iter().any(|url| url.contains("GetSchemaForGame")), "{asked:?}");
}

#[test]
fn when_steam_cannot_give_the_achievements_the_sync_still_succeeds_and_keeps_those_stored() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    let mut game = portal_2();
    game.minutes += 30;
    game.achievements.as_mut().unwrap()[2].unlocked_at = Some(NOW + 600);
    steam.owns(vec![game]);
    steam.becomes(Network::AchievementsDown);
    computer.clock.advance(1_800);

    assert_eq!(core.sync().unwrap(), SyncOutcome::Synced { at: NOW + 1_800 });
    assert_eq!(listed(&core, "steam:620"), Some((2, 4)));

    // The game is asked again with the next Sync.
    steam.becomes(Network::Online);
    computer.clock.advance(1_800);
    core.sync().unwrap();
    assert_eq!(listed(&core, "steam:620"), Some((3, 4)));
}

#[test]
fn a_large_library_is_fetched_over_several_syncs_most_recently_played_first() {
    let computer = Computer::new();
    let games: Vec<Owned> = (1..=120).map(|n| played(n * 10, "Game", 60, one_achievement())).collect();
    let steam = FakeSteam::owning(games);
    let core = computer.start(&steam);

    core.sync().unwrap();

    let fetched: Vec<u32> = (1..=120).map(|n| n * 10).filter(|app| listed(&core, &format!("steam:{app}")).is_some()).collect();
    assert!(!fetched.is_empty() && fetched.len() < 120, "fetched {} games in one Sync", fetched.len());
    // Last Played is earlier the larger the app id.
    assert_eq!(fetched, (1..=fetched.len() as u32).map(|n| n * 10).collect::<Vec<_>>());

    for _ in 0..10 {
        computer.clock.advance(1_800);
        core.sync().unwrap();
    }
    assert!((1..=120).all(|n| listed(&core, &format!("steam:{}", n * 10)).is_some()));
}

#[test]
fn a_game_played_since_its_last_fetch_comes_before_games_never_fetched() {
    let computer = Computer::new();
    let mut games: Vec<Owned> = (1..=120).map(|n| played(n * 10, "Game", 60, one_achievement())).collect();
    // Never played, so fetched last.
    games.push(Owned { last_played: 0, minutes: 0, ..played(5, "Unplayed", 0, one_achievement()) });
    let steam = FakeSteam::owning(games.clone());
    let core = computer.start(&steam);
    core.sync().unwrap();
    assert_eq!(listed(&core, "steam:5"), None);
    // The first game fetched is played again.
    games[0].minutes += 30;
    games[0].achievements = Some(vec![locked("ACH_ONE", "One", "The first"), unlocked("ACH_TWO", "Two", "The second", NOW)]);
    steam.owns(games);
    steam.forget_what_was_asked();
    computer.clock.advance(1_800);

    core.sync().unwrap();

    assert_eq!(listed(&core, "steam:10"), Some((1, 2)));
    let first_achievements = steam.asked().into_iter().find(|url| url.contains("GetPlayerAchievements")).unwrap();
    assert_eq!(query_value(&first_achievements, "appid"), Some("10"));
}

#[test]
fn the_syncs_of_one_day_ask_steam_far_less_often_than_its_daily_limit() {
    // Steam allows 100,000 Web API calls a day. Every game is played between
    // any two Syncs, and the user syncs every minute.
    let computer = Computer::new();
    let mut games: Vec<Owned> = (1..=300).map(|n| played(n, "Game", 60, one_achievement())).collect();
    let steam = FakeSteam::owning(games.clone());
    let core = computer.start(&steam);

    let start_of_day = NOW - NOW % 86_400;
    let mut syncs = 0;
    while computer.clock.now().duration_since(UNIX_EPOCH).unwrap().as_secs() < start_of_day + 86_400 - 60 && syncs < 600 {
        core.sync().unwrap();
        syncs += 1;
        for game in &mut games {
            game.minutes += 1;
        }
        steam.owns(games.clone());
        computer.clock.advance(60);
    }
    let asked_today = steam.asked().len();
    assert!(asked_today <= 20_000, "asked Steam {asked_today} times in one day");

    // The next day the achievements are fetched again.
    computer.clock.advance(86_400);
    steam.forget_what_was_asked();
    core.sync().unwrap();
    assert!(steam.asked().iter().any(|url| url.contains("GetPlayerAchievements")));
}

#[test]
fn the_achievements_are_asked_over_https_for_the_steam_account() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    computer.start(&steam).sync().unwrap();

    for url in steam.asked() {
        assert!(url.starts_with("https://api.steampowered.com/"), "{url}");
    }
}

#[test]
fn an_achievement_icon_is_downloaded_from_steam_once_and_then_served_from_the_cache() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    let GameAchievements::Listed { achievements, .. } = core.achievements("steam:620").unwrap() else {
        panic!("listed");
    };
    let icon = achievements[0].icon.clone().unwrap();

    let expected = Cover { bytes: ICON.to_vec(), media_type: "image/jpeg" };
    assert_eq!(core.achievement_icon("steam:620", &icon).unwrap(), Some(expected.clone()));
    assert_eq!(
        steam.downloaded(),
        [format!("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/620/{icon}.jpg")]
    );

    steam.becomes(Network::Offline);
    assert_eq!(computer.start(&steam).achievement_icon("steam:620", &icon).unwrap(), Some(expected));
    assert!(fs::read_dir(computer.covers()).unwrap().count() >= 1);
}

#[test]
fn locked_and_unlocked_achievements_have_different_icons() {
    let computer = Computer::new();
    let steam = FakeSteam::owning(vec![portal_2()]);
    let core = computer.start(&steam);
    core.sync().unwrap();
    let GameAchievements::Listed { achievements, .. } = core.achievements("steam:620").unwrap() else {
        panic!("listed");
    };

    assert_eq!(achievements[0].icon, Some(icon("ACH_FINISH", true)));
    assert_eq!(achievements[3].icon, Some(icon("ACH_PARTNER", false)));
}

#[test]
fn an_icon_that_is_not_steams_is_never_asked_for() {
    let computer = Computer::new();
    let steam = FakeSteam::default();
    let core = computer.start(&steam);

    for icon in ["../../evil", "abc.jpg", "", "ABCDEF/../x", "0123?x=1"] {
        assert_eq!(core.achievement_icon("steam:620", icon).unwrap(), None, "{icon:?}");
    }
    assert_eq!(core.achievement_icon("local:1", "abcdef").unwrap(), None);
    assert_eq!(steam.downloaded(), Vec::<String>::new());
    assert!(!Path::new(&computer.covers()).exists());
}

#[test]
fn the_achievements_reach_the_interface_in_the_shape_the_interface_expects() {
    let listed = GameAchievements::Listed {
        unlocked: 1,
        total: 2,
        achievements: vec![
            achievement("ACH_FINISH", "Lunacy", "Finish", Some(LAST_WEEK)),
            Achievement { description: None, icon: None, ..achievement("ACH_X", "X", "", None) },
        ],
    };
    assert_eq!(
        serde_json::to_value(listed).unwrap(),
        serde_json::json!({ "state": "listed", "unlocked": 1, "total": 2, "achievements": [
            { "id": "ACH_FINISH", "name": "Lunacy", "description": "Finish", "unlocked": true, "unlockedAt": LAST_WEEK, "icon": icon("ACH_FINISH", true) },
            { "id": "ACH_X", "name": "X", "description": null, "unlocked": false, "unlockedAt": null, "icon": null }
        ] })
    );
    for (state, json) in [
        (GameAchievements::LocalGame, "localGame"),
        (GameAchievements::NotFetchedYet, "notFetchedYet"),
        (GameAchievements::NoAchievements, "noAchievements"),
    ] {
        assert_eq!(serde_json::to_value(state).unwrap(), serde_json::json!({ "state": json }));
    }
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

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
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
