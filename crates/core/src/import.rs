//! Importing an Export File (see `crate::export`, docs/export-format.md):
//! everything that is the person's own is replaced with what the file holds,
//! all at once or not at all.
//!
//! Replaced: the Local Games with how they are launched, their Manual
//! Progress, Playtime Adjustments, Cover Matches and the Cover images the
//! person chose, every Session (of Local and Steam Games), Status, Favorite, Hidden Game and
//! Collection, and the settings. Kept as they are: the Steam Games and
//! Steam's figures (they come from Steam), the Steam App List, the last Sync,
//! the Log, and the Steam Web API key, which an import never touches.
//!
//! Each Local Game gets a new number here, and every entry that names it by
//! the number in the file follows. A file that is no Export File, is
//! damaged, or names a version this app does not know is refused before
//! anything changes.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::autostart::{self, Autostart};
use crate::covers::{Cover, CoverCache};
use crate::export::{ExportFile, FORMAT, VERSION};
use crate::local_covers;
use crate::store::Store;
use crate::sessions::Tracked;
use crate::{Error, collections, local_games};

/// How importing the data went, when nothing failed. Every outcome but
/// `Imported` changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum DataImport {
    /// The data of the Export File at `file` replaced the data here; it held
    /// `local_games` Local Games.
    #[serde(rename_all = "camelCase")]
    Imported { file: String, local_games: u32 },
    /// The user chose no file.
    Cancelled,
    /// The file was written by a newer version of the app, in format
    /// `version`, which this version cannot read.
    NewerVersion { version: u64 },
    /// The file is no Export File, or it is damaged.
    Damaged,
}

/// An Export File that passed every check, with its Cover images decoded.
pub(crate) struct CheckedFile {
    pub file: ExportFile,
    /// The bytes of each chosen Cover image, by the Local Game's id in the file.
    pub images: HashMap<String, Vec<u8>>,
}

/// Reads `bytes` as an Export File and checks everything an import relies
/// on; the refusal, as the outcome to answer, when it cannot be imported.
pub(crate) fn check(bytes: &[u8]) -> Result<CheckedFile, DataImport> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| DataImport::Damaged)?;
    if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(DataImport::Damaged);
    }
    match value.get("version").and_then(Value::as_u64) {
        Some(version) if version > u64::from(VERSION) => return Err(DataImport::NewerVersion { version }),
        Some(version) if version >= 1 => {}
        _ => return Err(DataImport::Damaged),
    }
    let file: ExportFile = serde_json::from_value(value).map_err(|_| DataImport::Damaged)?;
    let images = consistent(&file).ok_or(DataImport::Damaged)?;
    Ok(CheckedFile { file, images })
}

/// The decoded Cover images of `file` when everything in it fits together:
/// each Local Game has a number and an executable of its own, every Game id
/// names a Steam Game or one of the file's Local Games, and every value is
/// one the app can keep. `None` for a damaged file.
fn consistent(file: &ExportFile) -> Option<HashMap<String, Vec<u8>>> {
    let mut local_ids = HashSet::new();
    let mut executables = HashSet::new();
    let mut images = HashMap::new();
    for game in &file.local_games {
        let fresh = local_games::local_number(&game.id).is_some()
            && local_ids.insert(game.id.as_str())
            && executables.insert(game.executable.as_str());
        if !fresh
            || game.title.trim().is_empty()
            || game.executable.is_empty()
            || game.manual_progress.is_some_and(|percent| percent > 100)
        {
            return None;
        }
        if let Some(image) = &game.cover_image {
            let bytes = base64::engine::general_purpose::STANDARD.decode(&image.data).ok()?;
            if Cover::from_bytes(bytes.clone())?.media_type != image.media_type {
                return None;
            }
            images.insert(game.id.clone(), bytes);
        }
    }
    let known = |id: &str| local_ids.contains(id) || id.strip_prefix("steam:").is_some_and(|app| app.parse::<u32>().is_ok());
    let sessions_fit = file.sessions.iter().all(|session| {
        known(&session.game_id)
            && session.started_at <= session.ended_at
            && i64::try_from(session.ended_at).is_ok()
            && session.steam_account.as_deref().is_none_or(is_steam_id)
    });
    let mut names: Vec<String> = Vec::new();
    for collection in &file.collections {
        let name = collections::name_in(&collection.name)?;
        if names.iter().any(|other| collections::same_name(other, name))
            || !collection.game_ids.iter().all(|id| known(id))
        {
            return None;
        }
        names.push(name.to_owned());
    }
    let all_known = file.statuses.iter().all(|status| known(&status.game_id))
        && file.favorites.iter().all(|id| known(id))
        && file.hidden_games.iter().all(|id| known(id));
    let account_fits = file.settings.chosen_steam_account.as_deref().is_none_or(is_steam_id);
    (sessions_fit && all_known && account_fits).then_some(images)
}

/// Whether `text` can be a SteamID64: digits only.
fn is_steam_id(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Replaces the data here with `checked`, as the module documentation says:
/// the autostart entry first, then the store and the chosen Cover images in
/// one step. When anything fails, what was changed is put back and the
/// error answered.
pub(crate) fn replace(
    store: &Store,
    covers: &CoverCache,
    autostart_entry: &dyn Autostart,
    checked: &CheckedFile,
) -> Result<(), Error> {
    let wanted = checked.file.settings.start_with_windows;
    let changed_autostart = autostart::starts_with_windows(autostart_entry)? != wanted;
    if changed_autostart {
        autostart::set_start_with_windows(autostart_entry, wanted)?;
    }
    // The images kept so far, to remove again when the store refuses.
    let kept = RefCell::new(Vec::new());
    let replaced = store.replace_own_data(&checked.file, |numbers| {
        for (id, bytes) in &checked.images {
            let number = numbers[id.as_str()];
            local_covers::keep_chosen_bytes(covers, number, bytes)?;
            kept.borrow_mut().push(number);
        }
        Ok(())
    });
    match replaced {
        Ok(removed) => {
            for number in removed {
                covers.forget(&Tracked::Local(number).game_id());
            }
            Ok(())
        }
        Err(error) => {
            for number in kept.into_inner() {
                let _ = local_covers::remove_chosen_image(covers, number);
            }
            if changed_autostart {
                let _ = autostart::set_start_with_windows(autostart_entry, !wanted);
            }
            Err(error)
        }
    }
}
