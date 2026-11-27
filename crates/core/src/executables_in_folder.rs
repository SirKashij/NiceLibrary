//! Local Source: the executables in a folder the user points at, suggested
//! as Local Games to add several at once.
//!
//! Only a few folders deep is looked into, and files that are plainly not a
//! Game (installers, uninstallers, crash handlers, redistributables) are
//! passed over. Nothing is added here; the user adds the ones they tick.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::local_games::{self, StoredLocalGame};

/// What came of asking the user for a folder of Games, when nothing failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum FolderSuggestions {
    /// The user chose no folder.
    Cancelled,
    /// The user chose `folder`; these are the executables found in it, in
    /// the order of their paths.
    Suggested { folder: String, executables: Vec<SuggestedExecutable> },
}

/// An executable found in a folder of Games, which the user may add as a
/// Local Game (see `Core::add_local_game`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedExecutable {
    pub executable: String,
    /// The title the app proposes; for an executable already in the Library,
    /// the title it has there.
    pub title: String,
    /// Whether the executable is in the Library already, by any spelling of
    /// its path.
    pub already_added: bool,
}

/// The executables in `folder` and in its folders down to [`MAX_DEPTH`]
/// levels below it, without the ones that are plainly no Game, each with the
/// title the app proposes, or the one it has in the Library (`stored`).
pub(crate) fn suggestions(folder: &Path, stored: &[StoredLocalGame]) -> Vec<SuggestedExecutable> {
    let mut found = Vec::new();
    collect(folder, 0, &mut found);
    found.sort();
    found
        .into_iter()
        .filter_map(|path| {
            let executable = path.to_str()?.to_owned();
            Some(match local_games::already_added(stored, &executable) {
                Some(game) => SuggestedExecutable { executable, title: game.title.clone(), already_added: true },
                None => SuggestedExecutable { title: local_games::proposed_title(&path), executable, already_added: false },
            })
        })
        .collect()
}

/// How many levels of folders below the chosen one are looked into: enough
/// for an Unreal Engine Game in a folder of Games,
/// `Games/<Game>/<Game>/Binaries/Win64/<Game>.exe`.
const MAX_DEPTH: usize = 4;

/// Puts the executables in `folder`, `depth` levels below the chosen one,
/// and in the folders below it into `found`. Folders that cannot be read
/// are passed over.
fn collect(folder: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if kind.is_dir() {
            if depth < MAX_DEPTH && !is_no_game_folder(&name) {
                collect(&path, depth + 1, found);
            }
        } else if is_executable(&path) && !is_no_game(&name) {
            found.push(path);
        }
    }
}

/// Whether `path` names an executable, by its extension in any case.
fn is_executable(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

/// Beginnings of the lower-case names of executables that are plainly no
/// Game: installers, uninstallers, crash handlers, redistributables.
const NO_GAME_PREFIXES: &[&str] = &[
    "unins", "uninst", "setup", "install", "vcredist", "vc_redist", "dxsetup", "dxwebsetup", "dotnet", "ndp",
    "oalinst", "physx", "crashpad", "crashhandler", "crashreport", "crash_report", "unitycrashhandler",
    "ue4prereqsetup", "ueprereqsetup",
];

/// Parts of the lower-case names of executables that are plainly no Game,
/// wherever they appear in the name.
const NO_GAME_PARTS: &[&str] = &["installer", "uninstall", "redist", "crashhandler", "crashreporter", "crashsender"];

/// Lower-case folder names that hold no Game, only what is installed along
/// with one.
const NO_GAME_FOLDERS: &[&str] = &[
    "_commonredist", "commonredist", "redist", "redistributables", "__installer", "_installer", "installer",
    "installers", "directx", "vcredist", "dotnet", "support",
];

fn is_no_game(name: &str) -> bool {
    NO_GAME_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || NO_GAME_PARTS.iter().any(|part| name.contains(part))
}

fn is_no_game_folder(name: &str) -> bool {
    NO_GAME_FOLDERS.contains(&name)
}
