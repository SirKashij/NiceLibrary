//! The Steam App List: Steam's list of titles, by app id and name, fetched
//! page by page from `IStoreService/GetAppList` with the user's Steam Web API
//! key and kept in the store as the basis for Cover Matches (ADR-0005).
//!
//! Each page is stored as it arrives, together with where the next one
//! begins, so a fetch that is interrupted, even by quitting the app, resumes
//! there instead of starting over. Until a fetch has run to its end, the list
//! stored before stays as it was, with the pages of the new fetch added.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::steam_web_api::WebApi;
use crate::store::Store;

/// What is stored of the Steam App List.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamAppList {
    /// How many titles are stored.
    pub titles: u32,
    /// When the last fetch that ran to its end ended, in seconds since the
    /// Unix epoch; `None` before the first.
    pub fetched_at: Option<u64>,
    /// Whether a fetch stopped before its end; the next one resumes it.
    pub unfinished: bool,
}

/// One title in the Steam App List.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamTitle {
    /// Steam's app id.
    pub app_id: u32,
    pub name: String,
}

/// How fetching the Steam App List went, when nothing failed in the app
/// itself. The pages fetched before Steam stopped answering stay stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum SteamAppListFetch {
    /// Every page is stored. `at` is when, in seconds since the Unix epoch.
    Fetched { at: u64 },
    /// No Steam Web API key is stored.
    NoKey,
    /// Steam refuses the stored key.
    KeyRejected,
    /// Steam could not be asked or gave no usable answer; the next fetch
    /// resumes after the last page stored.
    SteamUnreachable,
}

/// What happens in a round of the automatic fetches, as it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum AutomaticSteamAppListFetch {
    /// No fetch begins this round: no key is stored, or the list was fetched
    /// to its end less than a week ago.
    Skipped,
    /// A fetch begins.
    Started,
    /// The fetch ended with this outcome.
    Finished(SteamAppListFetch),
    /// The fetch failed in the app itself.
    Failed { reason: Error },
}

/// How long after the app starts the first automatic round begins: after
/// the first Sync, which the user sees.
pub(crate) const FIRST_AUTOMATIC_FETCH_AFTER: Duration = Duration::from_secs(60);

/// How long after one automatic round the next begins.
pub(crate) const AUTOMATIC_FETCH_CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// How long a list fetched to its end is kept before it is fetched again on its own.
const FETCH_AGAIN_AFTER: u64 = 7 * 24 * 60 * 60;

/// The most titles a lookup answers.
pub(crate) const LOOKUP_LIMIT: usize = 20;

/// How many titles one request asks for; Steam's own default.
const PAGE_SIZE: u32 = 10_000;

/// Whether the automatic fetches fetch the list now, at `now` seconds since the epoch.
pub(crate) fn due(list: &SteamAppList, now: u64) -> bool {
    list.unfinished || list.fetched_at.is_none_or(|at| now.saturating_sub(at) >= FETCH_AGAIN_AFTER)
}

/// Fetches the Steam App List with `key`, page by page, storing each, and
/// resumes after the last page stored if a fetch stopped before its end.
/// `now` answers the time a fetch ends, in seconds since the epoch.
pub(crate) fn fetch(api: &dyn WebApi, store: &Store, key: &str, now: impl Fn() -> u64) -> Result<SteamAppListFetch, Error> {
    let mut after = store.steam_app_list_resume_after()?;
    loop {
        let page = match page(api, key, after) {
            Ok(page) => page,
            Err(outcome) => return Ok(outcome),
        };
        match page.next_after {
            // A page that does not move on would be asked for forever.
            Some(next) if after.is_some_and(|after| next <= after) => return Ok(SteamAppListFetch::SteamUnreachable),
            Some(next) => {
                store.record_steam_app_list_page(&page.titles, Some(next), None)?;
                after = Some(next);
            }
            None => {
                let at = now();
                store.record_steam_app_list_page(&page.titles, None, Some(at))?;
                return Ok(SteamAppListFetch::Fetched { at });
            }
        }
    }
}

/// One page of the Steam App List.
struct Page {
    titles: Vec<SteamTitle>,
    /// The app id the next page begins after; `None` on the last page.
    next_after: Option<u32>,
}

#[derive(Deserialize)]
struct AppListAnswer {
    response: AppListResponse,
}

#[derive(Deserialize)]
struct AppListResponse {
    #[serde(default)]
    apps: Vec<AppListEntry>,
    #[serde(default)]
    have_more_results: bool,
    last_appid: Option<u32>,
}

#[derive(Deserialize)]
struct AppListEntry {
    appid: u32,
    #[serde(default)]
    name: String,
}

/// The page of games after the app id `after`, or from the start.
fn page(api: &dyn WebApi, key: &str, after: Option<u32>) -> Result<Page, SteamAppListFetch> {
    let mut url = format!(
        "https://api.steampowered.com/IStoreService/GetAppList/v1/?key={key}&include_games=true&max_results={PAGE_SIZE}"
    );
    if let Some(after) = after {
        url.push_str(&format!("&last_appid={after}"));
    }
    // The reason is dropped: it may repeat the URL, and with it the key.
    let answer = api.get(&url).map_err(|_| SteamAppListFetch::SteamUnreachable)?;
    match answer.status {
        401 | 403 => return Err(SteamAppListFetch::KeyRejected),
        200 => {}
        _ => return Err(SteamAppListFetch::SteamUnreachable),
    }
    // Anything but Steam's JSON, such as a network's sign-in page, is no answer from Steam.
    let answer: AppListAnswer =
        serde_json::from_slice(&answer.body).map_err(|_| SteamAppListFetch::SteamUnreachable)?;
    let response = answer.response;
    let next_after = match (response.have_more_results, response.last_appid) {
        (false, _) => None,
        (true, Some(last)) => Some(last),
        (true, None) => return Err(SteamAppListFetch::SteamUnreachable),
    };
    let titles = response
        .apps
        .into_iter()
        .filter(|app| !app.name.trim().is_empty())
        .map(|app| SteamTitle { app_id: app.appid, name: app.name.trim().to_owned() })
        .collect();
    Ok(Page { titles, next_after })
}

/// The names of the stored Steam App List as lookups search them, held in
/// memory: a scan through the database takes too long to answer each
/// keystroke. All names, written as [`searchable`] writes them, sit in one
/// text, one per line, so one search runs through all of them at once.
pub(crate) struct TitleIndex {
    /// Every searchable name followed by a line break.
    text: String,
    /// Each name's app id and where it begins in `text`, in that order.
    rows: Vec<(u32, u32)>,
}

/// A name that matches a lookup: its rank (0 the same name, 1 begins with
/// the search, 2 a word does, 3 anything else), its length and its row.
type Match = (u8, usize, usize);

impl TitleIndex {
    /// An index of `titles`: app ids with their names written as [`searchable`] writes them.
    pub(crate) fn new(titles: impl IntoIterator<Item = (u32, String)>) -> Self {
        let mut index = Self { text: String::new(), rows: Vec::new() };
        for (app_id, search) in titles {
            // Searchable names hold no line break; one would join two names.
            let start = u32::try_from(index.text.len()).expect("the Steam App List is far below 4 GB");
            index.rows.push((app_id, start));
            index.text.push_str(&search.replace('\n', " "));
            index.text.push('\n');
        }
        index
    }

    /// The app ids of up to `limit` names that contain `search`, written as
    /// [`searchable`] writes it: the same name first, then names that begin
    /// with it, then names with a word that does, then the rest; within each,
    /// shorter names first, then in the order of the alphabet.
    pub(crate) fn find(&self, search: &str, limit: usize) -> Vec<u32> {
        if search.is_empty() || limit == 0 {
            return Vec::new();
        }
        // The best names so far, best first, as (rank, length, row).
        let mut best: Vec<Match> = Vec::with_capacity(limit + 1);
        // The name the last match was in, ranked by its best match so far.
        let mut current: Option<Match> = None;
        let mut row = 0;
        // `search` holds no line break, so no match runs from one name into the next.
        for (at, _) in self.text.match_indices(search) {
            // Matches come in the order of the text, so the name of each is found by walking on.
            while self.bounds(row).1 < at {
                row += 1;
            }
            let (start, end) = self.bounds(row);
            let rank = if at == start && end - start == search.len() {
                0
            } else if at == start {
                1
            } else if self.text.as_bytes()[at - 1] == b' ' {
                2
            } else {
                3
            };
            match &mut current {
                Some(name) if name.2 == row => name.0 = name.0.min(rank),
                _ => {
                    if let Some(name) = current.replace((rank, end - start, row)) {
                        self.keep_if_among_best(&mut best, name, limit);
                    }
                }
            }
        }
        if let Some(name) = current {
            self.keep_if_among_best(&mut best, name, limit);
        }
        best.into_iter().map(|(_, _, row)| self.rows[row].0).collect()
    }

    /// Puts `name` into `best` in its place, if it belongs among the best `limit`.
    fn keep_if_among_best(&self, best: &mut Vec<Match>, name: Match, limit: usize) {
        let order = |a: &Match, b: &Match| {
            (a.0, a.1).cmp(&(b.0, b.1)).then_with(|| self.name(a.2).cmp(self.name(b.2))).then(a.2.cmp(&b.2))
        };
        // Most names are worse than the last of a full list; this answers them cheaply.
        if best.len() == limit && (name.0, name.1) > (best[limit - 1].0, best[limit - 1].1) {
            return;
        }
        let place = best.partition_point(|kept| order(kept, &name).is_lt());
        if place < limit {
            best.insert(place, name);
            best.truncate(limit);
        }
    }

    /// Where the name in `row` begins and ends in `text`, without its line break.
    fn bounds(&self, row: usize) -> (usize, usize) {
        let start = self.rows[row].1 as usize;
        let end = self.rows.get(row + 1).map_or(self.text.len(), |&(_, next)| next as usize) - 1;
        (start, end)
    }

    fn name(&self, row: usize) -> &str {
        let (start, end) = self.bounds(row);
        &self.text[start..end]
    }
}

/// `text` as titles are compared: in lower case, with every run of
/// characters that are neither letters nor digits as one space, so
/// "THE WITCHER® 3: Wild Hunt" and "the witcher 3 wild hunt" are the same.
pub(crate) fn searchable(text: &str) -> String {
    let mut words = String::with_capacity(text.len());
    for word in text.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()) {
        if !words.is_empty() {
            words.push(' ');
        }
        words.extend(word.chars().flat_map(char::to_lowercase));
    }
    words
}
