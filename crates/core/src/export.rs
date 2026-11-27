//! The Export File: everything that is the person's own, in one JSON file
//! they choose, as a backup or to move to another computer. Its format is
//! documented in docs/export-format.md and versioned by [`VERSION`]; an
//! import (#52) refuses a version it does not know.
//!
//! Never in it: the Steam Web API key (it lives only in the secret store),
//! and nothing the app can fetch again: Steam's figures, Achievements, the
//! Steam App List, and cached Covers. A Cover image the person chose is in
//! it, as it cannot be fetched again.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::autostart::{self, Autostart};
use crate::covers::{self, CoverCache};
use crate::language::Language;
use crate::library_filter::LibrarySort;
use crate::library_layout::LibraryLayout;
use crate::local_covers;
use crate::sessions::Tracked;
use crate::store::Store;
use crate::theme::Theme;
use crate::{Error, Status, about, updates, window};

/// What every Export File names as its `format`.
pub(crate) const FORMAT: &str = "nicelibrary-export";

/// The version of the format this app writes. It goes up whenever an
/// earlier import could not read a file correctly (docs/export-format.md).
pub(crate) const VERSION: u32 = 1;

/// Port: asks the user where to save an Export File, as a "Save as" dialog
/// does, or which one to import, as an "Open" dialog does.
///
/// Contract of both: `Ok(None)` when the user cancels; blocks until they
/// have decided; `Err` when no dialog can be shown, described for a log or a
/// developer. Neither reads nor writes the file.
pub trait DataFilePicker: Send + Sync {
    /// Offers `suggested_name` as the file name and answers `Ok(Some(path))`
    /// with the absolute path of the file the user chose, which may exist
    /// already (the user agreed to replace it).
    fn choose_export_file(&self, suggested_name: &str) -> Result<Option<PathBuf>, String>;

    /// Answers `Ok(Some(path))` with the absolute path of the file the user
    /// chose to import, which existed when they chose it.
    fn choose_import_file(&self) -> Result<Option<PathBuf>, String>;
}

/// How exporting the data went, when nothing failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum DataExport {
    /// The Export File is written, at `file`.
    Exported { file: String },
    /// The user chose no file; nothing was written.
    Cancelled,
}

/// An Export File, as written and as an import reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportFile {
    pub format: String,
    pub version: u32,
    /// The version of the app that wrote it.
    pub app_version: String,
    /// When it was written, in seconds since the Unix epoch.
    pub exported_at: u64,
    pub settings: ExportedSettings,
    pub local_games: Vec<ExportedLocalGame>,
    pub sessions: Vec<ExportedSession>,
    pub statuses: Vec<ExportedStatus>,
    pub favorites: Vec<String>,
    pub hidden_games: Vec<String>,
    pub collections: Vec<ExportedCollection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedSettings {
    pub chosen_steam_account: Option<String>,
    pub theme: Theme,
    pub language: Language,
    pub library_layout: LibraryLayout,
    pub library_sort: LibrarySort,
    pub close_to_tray: bool,
    pub start_with_windows: bool,
    /// Whether the app checks for Updates on its own; on in files written
    /// before the setting existed, as it is on until turned off.
    #[serde(default = "on")]
    pub check_for_updates: bool,
}

fn on() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedLocalGame {
    /// Its Game id in this file (`local:<n>`), which the Sessions, Statuses,
    /// Favorites, Hidden Games and Collections refer to.
    pub id: String,
    pub title: String,
    pub executable: String,
    pub launch_arguments: String,
    pub working_directory: Option<String>,
    pub manual_progress: Option<u8>,
    pub playtime_adjustment_minutes: Option<i32>,
    pub cover_image: Option<ExportedImage>,
    /// The Steam title whose Cover it borrows; optional, as files written
    /// before Cover Matches existed have none.
    #[serde(default)]
    pub cover_match: Option<ExportedCoverMatch>,
}

/// A Cover Match: the Steam title, with its name as it was when matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedCoverMatch {
    pub app_id: u32,
    pub name: String,
}

/// A Cover image the person chose, as the bytes of the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedImage {
    /// `image/png` or `image/jpeg`.
    pub media_type: String,
    /// The file's bytes in standard Base64 with padding.
    pub data: String,
}

/// One Session, of a Local Game or, with a `steam:<appid>` Game id and the
/// Steam Account it was played with, of a Steam Game (docs/export-format.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedSession {
    pub game_id: String,
    pub started_at: u64,
    pub ended_at: u64,
    /// The SteamID64 of the Steam Account a Steam Session was played with;
    /// `None` when none was known, and for a Local Game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedStatus {
    pub game_id: String,
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportedCollection {
    pub name: String,
    pub game_ids: Vec<String>,
}

/// Everything the Export File holds, read from `store`, the chosen Cover
/// images in `covers` and the autostart entry, as of `now`.
pub(crate) fn contents(
    store: &Store,
    covers: &CoverCache,
    autostart: &dyn Autostart,
    now: u64,
) -> Result<ExportFile, Error> {
    let adjustments = store.playtime_adjustments()?;
    let local_games = store
        .local_games()?
        .into_iter()
        .map(|game| {
            let cover_image = local_covers::chosen_image(covers, game.id)?.map(|cover| ExportedImage {
                media_type: cover.media_type.to_owned(),
                data: base64::engine::general_purpose::STANDARD.encode(&cover.bytes),
            });
            let cover_match = store
                .cover_match(game.id)?
                .map(|title| ExportedCoverMatch { app_id: title.app_id, name: title.name });
            Ok(ExportedLocalGame {
                id: Tracked::Local(game.id).game_id(),
                title: game.title,
                executable: game.executable,
                launch_arguments: game.launch_options.arguments,
                working_directory: game.launch_options.working_directory,
                manual_progress: game.manual_progress,
                playtime_adjustment_minutes: adjustments.get(&game.id).copied(),
                cover_image,
                cover_match,
            })
        })
        .collect::<Result<_, Error>>()?;
    let mut sessions: Vec<_> = store
        .all_local_game_sessions()?
        .into_iter()
        .map(|(game, started_at, ended_at)| ExportedSession {
            game_id: Tracked::Local(game).game_id(),
            started_at,
            ended_at,
            steam_account: None,
        })
        .chain(store.all_steam_game_sessions()?.into_iter().map(|(app_id, steam_account, started_at, ended_at)| {
            ExportedSession { game_id: Tracked::Steam(app_id).game_id(), started_at, ended_at, steam_account }
        }))
        .collect();
    // Oldest first, whatever the Source; a stable sort keeps each Source's own order on a tie.
    sessions.sort_by_key(|session| session.started_at);
    let mut statuses: Vec<_> =
        store.statuses()?.into_iter().map(|(game_id, status)| ExportedStatus { game_id, status }).collect();
    statuses.sort_by(|a, b| a.game_id.cmp(&b.game_id));
    let mut favorites: Vec<_> = store.favorites()?.into_iter().collect();
    favorites.sort();
    let mut hidden_games: Vec<_> = store.hidden_game_ids()?.into_iter().collect();
    hidden_games.sort();
    let mut collections = store.collections()?;
    collections.sort_by_key(|(id, _)| *id);
    let memberships = store.collection_games()?;
    let collections = collections
        .into_iter()
        .map(|(id, name)| {
            let mut game_ids: Vec<_> =
                memberships.iter().filter(|(collection, _)| *collection == id).map(|(_, game)| game.clone()).collect();
            game_ids.sort();
            ExportedCollection { name, game_ids }
        })
        .collect();
    Ok(ExportFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        app_version: about::about().version,
        exported_at: now,
        settings: ExportedSettings {
            chosen_steam_account: store.chosen_steam_account()?,
            theme: store.theme()?,
            language: store.language()?,
            library_layout: store.library_layout()?,
            library_sort: store.library_sort()?.and_then(|name| LibrarySort::from_stored_name(&name)).unwrap_or_default(),
            close_to_tray: window::close_to_tray(store)?,
            start_with_windows: autostart::starts_with_windows(autostart)?,
            check_for_updates: updates::checks_for_updates(store)?,
        },
        local_games,
        sessions,
        statuses,
        favorites,
        hidden_games,
        collections,
    })
}

/// Writes `export` to `file` in one step, in place of anything there, so a
/// failure never leaves half a file behind.
pub(crate) fn write(file: &Path, export: &ExportFile) -> Result<(), Error> {
    let json = serde_json::to_vec_pretty(export).map_err(|error| Error::new(format!("export: {error}")))?;
    let folder = file.parent().unwrap_or(Path::new("."));
    if !folder.is_dir() {
        return Err(Error::new(format!("export: there is no folder {}", folder.display())));
    }
    covers::keep(folder, file, &json).map_err(|error| Error::new(format!("cannot write {}: {error}", file.display())))
}

/// The file name offered for an Export File written at `now`, in seconds
/// since the Unix epoch: `NiceLibrary-<year>-<month>-<day>.json`, by UTC.
pub(crate) fn suggested_name(now: u64) -> String {
    let (year, month, day) = civil_date(now / 86_400);
    format!("NiceLibrary-{year:04}-{month:02}-{day:02}.json")
}

/// The calendar date `days` days after 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}
