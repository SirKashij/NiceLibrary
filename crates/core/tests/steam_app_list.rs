//! The Steam App List: Steam's list of titles, by app id and name, fetched
//! page by page with the user's Steam Web API key and kept on this computer
//! as the basis for Cover Matches. An interrupted fetch resumes where it
//! stopped; the list is fetched again at most once a week, and whenever the
//! user asks. Titles are looked up in the stored list as the user types.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AutomaticSteamAppListFetch, Clock, Core, CoverCache, Downloader, ExecutablePicker, OpenError, Opener, Ports,
    SecretStore, SteamAppList, SteamAppListFetch, SteamLocator, SteamTitle, Store, Wakeup, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";

/// 2026-11-25 12:00:00 UTC, in seconds since the Unix epoch.
const NOW: u64 = 1_795_608_000;
const HOUR: u64 = 3_600;
const DAY: u64 = 24 * HOUR;

/// How the fake Steam Web API behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Network {
    /// Steam answers as it does.
    Online,
    /// Steam answers this many pages of the app list, then the connection is gone.
    LostAfterPages(usize),
    /// Steam refuses the key.
    KeyRejected,
    /// Something in between, such as a network's sign-in page, answers.
    SignInPage,
}

/// A fake of `IStoreService/GetAppList` in the shape Steam answers with: it
/// pages through `apps` in app id order, at most `page_size` per answer,
/// however many the request asks for. It remembers every URL it was asked for.
#[derive(Clone)]
struct SteamWebApi {
    apps: Arc<Mutex<Vec<(u32, String)>>>,
    page_size: usize,
    network: Arc<Mutex<Network>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl SteamWebApi {
    fn listing(apps: &[(u32, &str)], page_size: usize) -> Self {
        Self::with(apps.iter().map(|(id, name)| (*id, (*name).to_owned())).collect(), page_size)
    }

    fn with(mut apps: Vec<(u32, String)>, page_size: usize) -> Self {
        apps.sort_by_key(|(id, _)| *id);
        Self {
            apps: Arc::new(Mutex::new(apps)),
            page_size,
            network: Arc::new(Mutex::new(Network::Online)),
            asked: Arc::default(),
        }
    }

    fn becomes(&self, network: Network) {
        *self.network.lock().unwrap() = network;
    }

    /// Steam now lists `app` as well.
    fn adds(&self, app_id: u32, name: &str) {
        let mut apps = self.apps.lock().unwrap();
        apps.push((app_id, name.to_owned()));
        apps.sort_by_key(|(id, _)| *id);
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    /// The app ids after which each request of the app list asked to continue.
    fn asked_after(&self) -> Vec<u32> {
        self.asked()
            .iter()
            .map(|url| query_value(url, "last_appid").map_or(0, |id| id.parse().unwrap()))
            .collect()
    }
}

/// The value of `name` in the query of `url`.
fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        let pages_before = self.asked.lock().unwrap().len();
        self.asked.lock().unwrap().push(url.to_owned());
        let answer = |status, text: String| Ok(WebAnswer { status, body: text.into_bytes() });
        match *self.network.lock().unwrap() {
            Network::LostAfterPages(pages) if pages_before >= pages => return Err("no connection to the server".into()),
            Network::KeyRejected => return answer(403, "<html><body>Forbidden</body></html>".into()),
            Network::SignInPage => return answer(200, "<!doctype html><title>Sign in to the Wi-Fi</title>".into()),
            _ => {}
        }
        if query_value(url, "key") != Some(KEY) {
            return answer(403, "<html><body>Forbidden</body></html>".into());
        }
        if !url.starts_with("https://api.steampowered.com/IStoreService/GetAppList/v1/?") {
            return answer(404, "<html><body>Not Found</body></html>".into());
        }
        let after: u32 = query_value(url, "last_appid").map_or(0, |id| id.parse().unwrap());
        let asked_for: usize = query_value(url, "max_results").map_or(10_000, |n| n.parse().unwrap());
        let apps = self.apps.lock().unwrap();
        let rest: Vec<_> = apps.iter().filter(|(id, _)| *id > after).collect();
        let page: Vec<_> = rest.iter().take(asked_for.min(self.page_size)).collect();
        let entries: Vec<_> = page
            .iter()
            .map(|(id, name)| serde_json::json!({ "appid": id, "name": name, "last_modified": 1_700_000_000, "price_change_number": 25_000_000 }))
            .collect();
        let mut response = serde_json::json!({ "apps": entries });
        if rest.len() > page.len() {
            response["have_more_results"] = true.into();
            response["last_appid"] = page.last().map(|(id, _)| *id).into();
        }
        answer(200, serde_json::json!({ "response": response }).to_string())
    }
}

/// A substitute for the operating system's secret store, kept in memory.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<HashMap<String, String>>>);

impl Secrets {
    fn holding_the_key() -> Self {
        let secrets = Self::default();
        secrets.0.lock().unwrap().insert("steam-web-api-key".into(), KEY.into());
        secrets
    }
}

impl SecretStore for Secrets {
    fn read(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(name);
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

    fn set(&self, seconds: u64) {
        *self.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(seconds);
    }

    fn seconds(&self) -> u64 {
        self.0.lock().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }
}

impl Clock for TestClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    /// The time passes at once, unless the wakeup has rung.
    fn sleep(&self, duration: Duration, wakeup: &Wakeup) {
        if !wakeup.take() {
            *self.0.lock().unwrap() += duration;
        }
    }
}

/// The Steam App List needs no Steam installation.
struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

/// No Covers are needed here.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// These tests open nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// The user profile of one test, and the computer the app runs on.
struct Computer {
    profile: TempDir,
    secrets: Secrets,
    clock: TestClock,
}

impl Computer {
    /// A computer with a stored key and the clock at NOW.
    fn new() -> Self {
        Self { profile: tempfile::tempdir().unwrap(), secrets: Secrets::holding_the_key(), clock: TestClock::at(NOW) }
    }

    fn without_key(mut self) -> Self {
        self.secrets = Secrets::default();
        self
    }

    /// Starts the app with Steam's Web API behaving as `api` does.
    fn start(&self, api: &SteamWebApi) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(self.secrets.clone()),
                web_api: Box::new(api.clone()),
                clock: Box::new(self.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                export_file_picker: Box::new(NoExportFile),
                autostart: Box::new(NoAutostart),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
            },
            Store::open(&self.profile.path().join("nicelibrary.db")).expect("database opens"),
            CoverCache::new(self.profile.path().join("covers")),
        )
    }
}

fn title(app_id: u32, name: &str) -> SteamTitle {
    SteamTitle { app_id, name: name.into() }
}

/// The names of the titles `core` finds for `query`, best match first.
fn found(core: &Core, query: &str) -> Vec<String> {
    core.find_steam_titles(query).unwrap().into_iter().map(|t| t.name).collect()
}

const PORTALS: &[(u32, &str)] = &[
    (400, "Portal"),
    (620, "Portal 2"),
    (70, "Half-Life"),
    (220, "Half-Life 2"),
    (367520, "Hollow Knight"),
    (1145360, "Hades"),
    (1030300, "Hollow Knight: Silksong"),
];

#[test]
fn before_the_first_fetch_the_steam_app_list_is_empty() {
    let computer = Computer::new();
    let core = computer.start(&SteamWebApi::listing(PORTALS, 3));

    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 0, fetched_at: None, unfinished: false });
    assert_eq!(found(&core, "portal"), Vec::<String>::new());
}

#[test]
fn a_fetch_stores_every_page_of_the_steam_app_list() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);

    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::Fetched { at: NOW });

    // Seven titles in pages of three: three requests, each continuing after the last title of the one before.
    assert_eq!(steam.asked_after(), vec![0, 400, 1030300]);
    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 7, fetched_at: Some(NOW), unfinished: false });
    assert_eq!(core.find_steam_titles("Hades").unwrap(), vec![title(1145360, "Hades")]);
    assert_eq!(found(&core, "half"), vec!["Half-Life", "Half-Life 2"]);
}

#[test]
fn the_stored_steam_app_list_is_there_after_a_restart() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    computer.start(&steam).fetch_steam_app_list().unwrap();

    let core = computer.start(&steam);

    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 7, fetched_at: Some(NOW), unfinished: false });
    assert_eq!(found(&core, "portal"), vec!["Portal", "Portal 2"]);
}

#[test]
fn an_interrupted_fetch_keeps_its_pages_and_the_next_one_resumes_where_it_stopped() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    steam.becomes(Network::LostAfterPages(1));
    let core = computer.start(&steam);

    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::SteamUnreachable);
    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 3, fetched_at: None, unfinished: true });
    assert_eq!(found(&core, "portal"), vec!["Portal"]);

    steam.becomes(Network::Online);
    let restarted = computer.start(&steam);
    assert_eq!(restarted.fetch_steam_app_list().unwrap(), SteamAppListFetch::Fetched { at: NOW });

    // The second request failed; after the restart the fetch asks for it again, not for the first page.
    assert_eq!(steam.asked_after(), vec![0, 400, 400, 1030300]);
    assert_eq!(restarted.steam_app_list().unwrap(), SteamAppList { titles: 7, fetched_at: Some(NOW), unfinished: false });
    assert_eq!(found(&restarted, "hollow"), vec!["Hollow Knight", "Hollow Knight: Silksong"]);
}

#[test]
fn fetching_again_brings_titles_steam_added_and_keeps_the_list_while_it_runs() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);
    core.fetch_steam_app_list().unwrap();
    steam.adds(2_000_000, "Portal: Revolution");
    steam.becomes(Network::LostAfterPages(4));
    computer.clock.set(NOW + DAY);

    // Asked for again, the whole list is fetched anew. An interrupted fetch keeps the list as it was.
    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::SteamUnreachable);
    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 7, fetched_at: Some(NOW), unfinished: true });
    assert_eq!(found(&core, "portal"), vec!["Portal", "Portal 2"]);

    steam.becomes(Network::Online);
    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::Fetched { at: NOW + DAY });
    assert_eq!(found(&core, "portal"), vec!["Portal", "Portal 2", "Portal: Revolution"]);
    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 8, fetched_at: Some(NOW + DAY), unfinished: false });
}

#[test]
fn without_a_key_nothing_is_fetched() {
    let computer = Computer::new().without_key();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);

    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::NoKey);
    assert_eq!(steam.asked(), Vec::<String>::new());
}

#[test]
fn a_rejected_key_or_an_answer_that_is_not_steams_stores_nothing() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);

    steam.becomes(Network::KeyRejected);
    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::KeyRejected);
    steam.becomes(Network::SignInPage);
    assert_eq!(core.fetch_steam_app_list().unwrap(), SteamAppListFetch::SteamUnreachable);

    assert_eq!(core.steam_app_list().unwrap(), SteamAppList { titles: 0, fetched_at: None, unfinished: false });
}

#[test]
fn every_request_carries_the_key_and_asks_for_games() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    computer.start(&steam).fetch_steam_app_list().unwrap();

    for url in steam.asked() {
        assert_eq!(query_value(&url, "key"), Some(KEY), "{url}");
        assert_eq!(query_value(&url, "include_games"), Some("true"), "{url}");
    }
}

#[test]
fn a_title_is_found_whatever_its_case_and_punctuation_best_match_first() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(
        &[
            (1, "The Witcher 3: Wild Hunt"),
            (2, "Witcher Fan Art Pack"),
            (3, "THE WITCHER® 3: WILD HUNT - GAME OF THE YEAR"),
            (4, "Witcher"),
            (5, "Twitcher Simulator"),
            (6, "Hollow Knight™"),
        ],
        10,
    );
    let core = computer.start(&steam);
    core.fetch_steam_app_list().unwrap();

    // The same name first, then names that start with it, then names with a word that does, then the rest.
    assert_eq!(
        found(&core, "witcher"),
        vec!["Witcher", "Witcher Fan Art Pack", "The Witcher 3: Wild Hunt", "THE WITCHER® 3: WILD HUNT - GAME OF THE YEAR", "Twitcher Simulator"]
    );
    assert_eq!(found(&core, "the witcher 3 wild hunt"), vec!["The Witcher 3: Wild Hunt", "THE WITCHER® 3: WILD HUNT - GAME OF THE YEAR"]);
    assert_eq!(found(&core, "  Hollow  KNIGHT "), vec!["Hollow Knight™"]);
    assert_eq!(found(&core, "100%"), Vec::<String>::new());
    assert_eq!(found(&core, " : "), Vec::<String>::new());
}

#[test]
fn a_lookup_answers_at_most_twenty_titles() {
    let computer = Computer::new();
    let apps: Vec<_> = (1..=50).map(|id| (id, format!("Puzzle {id}"))).collect();
    let core = computer.start(&SteamWebApi::with(apps, 100));
    core.fetch_steam_app_list().unwrap();

    assert_eq!(core.find_steam_titles("puzzle").unwrap().len(), 20);
}

#[test]
fn a_lookup_in_a_list_as_large_as_steams_is_fast_enough_to_run_while_the_user_types() {
    let computer = Computer::new();
    // Steam lists about 200 000 games; this list has a quarter more.
    let words = ["Dark", "Star", "Legend", "Quest", "Knight", "Racing", "Simulator", "Tales", "Battle", "World"];
    let apps: Vec<_> = (1..=250_000u32)
        .map(|id| {
            let w = |n: u32| words[(n % 10) as usize];
            (id, format!("{} {} {} {id}", w(id), w(id / 10), w(id / 100)))
        })
        .collect();
    let core = computer.start(&SteamWebApi::with(apps, 50_000));
    core.fetch_steam_app_list().unwrap();
    assert_eq!(core.steam_app_list().unwrap().titles, 250_000);

    // Each keystroke of a typed title is one lookup; the slowest must stay below what a user notices.
    // The budget holds for this unoptimised test build, which runs beside other tests; the app is
    // several times faster. Each lookup counts by its best of three, so a busy machine does not decide.
    let mut slowest = Duration::ZERO;
    for typed in ["s", "st", "sta", "star", "star ", "star q", "star qu", "star que", "star ques", "star quest", "zzz"] {
        let fastest = (0..3)
            .map(|_| {
                let started = Instant::now();
                core.find_steam_titles(typed).unwrap();
                started.elapsed()
            })
            .min()
            .unwrap();
        slowest = slowest.max(fastest);
    }
    assert!(slowest < Duration::from_millis(250), "the slowest lookup took {slowest:?}");
}

/// Runs the automatic fetches until `rounds` rounds have ended, and answers what they reported.
fn run_automatic_fetches(core: &Core, rounds: usize) -> Vec<AutomaticSteamAppListFetch> {
    let mut events = Vec::new();
    let mut ended = 0;
    core.fetch_steam_app_list_automatically(|event| {
        if !matches!(event, AutomaticSteamAppListFetch::Started) {
            ended += 1;
        }
        events.push(event);
        if ended == rounds { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });
    events
}

#[test]
fn the_list_is_fetched_on_its_own_soon_after_the_start_then_at_most_once_a_week() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);

    let events = run_automatic_fetches(&core, 3);

    let fetched_at = NOW + 60;
    assert_eq!(
        events,
        vec![
            AutomaticSteamAppListFetch::Started,
            AutomaticSteamAppListFetch::Finished(SteamAppListFetch::Fetched { at: fetched_at }),
            AutomaticSteamAppListFetch::Skipped,
            AutomaticSteamAppListFetch::Skipped,
        ]
    );
    assert_eq!(steam.asked().len(), 3);

    // A week after the fetch, the next round fetches the list again.
    computer.clock.set(fetched_at + 7 * DAY - HOUR);
    let events = run_automatic_fetches(&core, 2);
    assert_eq!(events.last(), Some(&AutomaticSteamAppListFetch::Finished(SteamAppListFetch::Fetched { at: computer.clock.seconds() })));
    assert!(computer.clock.seconds() >= fetched_at + 7 * DAY);
    assert_eq!(steam.asked().len(), 6);
}

#[test]
fn an_interrupted_automatic_fetch_resumes_in_a_later_round() {
    let computer = Computer::new();
    let steam = SteamWebApi::listing(PORTALS, 3);
    steam.becomes(Network::LostAfterPages(1));
    let core = computer.start(&steam);

    let events = run_automatic_fetches(&core, 1);
    assert_eq!(events.last(), Some(&AutomaticSteamAppListFetch::Finished(SteamAppListFetch::SteamUnreachable)));

    steam.becomes(Network::Online);
    let events = run_automatic_fetches(&core, 1);
    assert!(matches!(events.last(), Some(AutomaticSteamAppListFetch::Finished(SteamAppListFetch::Fetched { .. }))));
    assert_eq!(steam.asked_after(), vec![0, 400, 400, 1030300]);
}

#[test]
fn without_a_key_the_automatic_rounds_fetch_nothing_and_a_stored_key_starts_one_at_once() {
    let computer = Computer::new().without_key();
    let steam = SteamWebApi::listing(PORTALS, 3);
    let core = computer.start(&steam);

    assert_eq!(run_automatic_fetches(&core, 2), vec![AutomaticSteamAppListFetch::Skipped; 2]);
    assert_eq!(steam.asked(), Vec::<String>::new());

    assert_eq!(core.store_steam_web_api_key(KEY).unwrap(), nicelibrary_core::SteamWebApiKeyCheck::Stored);
    let before = computer.clock.seconds();
    let events = run_automatic_fetches(&core, 1);

    // The waiting before the first round ends at once: the key was stored.
    assert_eq!(events, vec![AutomaticSteamAppListFetch::Started, AutomaticSteamAppListFetch::Finished(SteamAppListFetch::Fetched { at: before })]);
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
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
