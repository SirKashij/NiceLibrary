//! Local Source: Local Games, added by hand by choosing their executable, and
//! the title the app proposes for one.
//!
//! An executable is kept as the path the operating system answered, as text;
//! nothing here assumes Windows' drive letters, separators or letter case.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::Game;

/// Port: asks the user to choose a Game's executable, as a file dialog does.
///
/// Contract: `Ok(Some(path))` with the absolute path of the file the user
/// chose, which exists; `Ok(None)` when they cancel. Blocks until they have
/// decided. `Err` when no dialog can be shown, described for a log or a
/// developer.
pub trait ExecutablePicker: Send + Sync {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String>;
}

/// What came of asking the user for a Local Game's executable, when nothing
/// failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum ExecutableChoice {
    /// The user chose no executable.
    Cancelled,
    /// The user chose `executable`; `title` is the title the app proposes.
    Chosen { executable: String, title: String },
    /// The executable is already in the Library, as the Local Game `title`.
    AlreadyAdded { title: String },
}

/// How adding a Local Game went, when nothing failed. Every outcome but
/// `Added` changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum AddLocalGame {
    /// The Game is in the Library now.
    Added { game: Game },
    /// The executable is already in the Library, as the Local Game `title`.
    AlreadyAdded { title: String },
    /// The title is empty.
    NoTitle,
    /// No file is at the executable's path.
    NotFound,
}

/// A Local Game as the store keeps it.
pub(crate) struct StoredLocalGame {
    pub id: i64,
    pub title: String,
    pub executable: String,
    pub launch_options: crate::LaunchOptions,
}

impl Game {
    /// The Local Game stored under `id`. A Local Game is always Installed;
    /// without Sessions its Playtime is nothing yet.
    pub(crate) fn local(id: i64, title: String) -> Self {
        Self {
            id: format!("local:{id}"),
            title,
            source: crate::Source::Local,
            installed: true,
            playtime_minutes: Some(0),
            last_played: None,
            status: crate::Status::Unplayed,
        }
    }
}

/// The stored Local Game whose executable is the file at `executable`, by
/// whatever spelling of its path.
pub(crate) fn already_added<'a>(stored: &'a [StoredLocalGame], executable: &str) -> Option<&'a StoredLocalGame> {
    let wanted = fs::canonicalize(executable).ok();
    stored.iter().find(|game| {
        game.executable == executable
            || wanted.as_ref().is_some_and(|wanted| fs::canonicalize(&game.executable).ok().as_ref() == Some(wanted))
    })
}

/// The title the app proposes for the executable at `path`: its name without
/// extension, version numbers, platform suffixes and separators; the name of
/// its folder instead when the executable's name says nothing, such as
/// "game" or "launcher". Folders that say nothing either, such as "bin" or
/// "Win64", are passed over for the one above.
pub(crate) fn proposed_title(path: &Path) -> String {
    let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    let from_executable = cleaned(&stem);
    if !from_executable.is_empty() && !GENERIC_EXECUTABLES.contains(&from_executable.to_lowercase().as_str()) {
        return from_executable;
    }
    let from_folder = path
        .ancestors()
        .skip(1)
        .take(MAX_FOLDERS_UP)
        .filter_map(Path::file_name)
        .map(|name| cleaned(&name.to_string_lossy()))
        .find(|name| !name.is_empty() && !GENERIC_FOLDERS.contains(&name.to_lowercase().as_str()));
    match from_folder {
        Some(title) => title,
        None if !from_executable.is_empty() => from_executable,
        None => stem,
    }
}

/// How many folders above the executable the title is looked for in.
const MAX_FOLDERS_UP: usize = 4;

/// Executable names that say nothing about the Game, once cleaned and in lower case.
const GENERIC_EXECUTABLES: &[&str] =
    &["game", "launcher", "launch", "start", "starter", "play", "run", "main", "app", "client", "game launcher"];

/// Folder names that say nothing about the Game, once cleaned and in lower case.
const GENERIC_FOLDERS: &[&str] = &[
    "game", "games", "bin", "bin32", "bin64", "binaries", "release", "build", "retail", "program", "launcher",
];

/// Words that name a platform or build rather than the Game, in lower case.
const PLATFORM_WORDS: &[&str] = &[
    "win32", "win64", "x64", "x86", "amd64", "arm64", "64bit", "32bit", "dx9", "dx10", "dx11", "dx12", "shipping",
];

/// Spellings of a platform that separators would tear apart, in lower case;
/// each is read as one platform word.
const SPLIT_PLATFORMS: &[&str] = &["x86_64", "x86-64", "64-bit", "32-bit", "64 bit", "32 bit"];

/// `name` without version numbers, platform words and separators, its words
/// separated by single spaces; a word written in lower case only begins with
/// a capital.
fn cleaned(name: &str) -> String {
    let mut name = name.to_owned();
    for split in SPLIT_PLATFORMS {
        // ASCII lower case keeps every byte where it is, so positions carry over.
        while let Some(at) = name.to_ascii_lowercase().find(split) {
            name.replace_range(at..at + split.len(), " x64 ");
        }
    }
    let mut words = Vec::new();
    for token in name.split(|c: char| c.is_whitespace() || "_-+()[]{}".contains(c)) {
        words.extend(words_without_versions(token));
    }
    words
        .into_iter()
        .filter(|word| !PLATFORM_WORDS.contains(&word.to_lowercase().as_str()))
        .map(capitalised)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The words of `token`, split at dots, without version numbers: a run of
/// numbers separated by dots (`1.4.0`), or a number after a `v` (`v2`,
/// `v1.38`). A lone number such as the 2 of "Portal 2" is kept.
fn words_without_versions(token: &str) -> Vec<&str> {
    let parts: Vec<&str> = token.split('.').filter(|part| !part.is_empty()).collect();
    let mut words = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        let part = parts[index];
        let versioned = part.strip_prefix(['v', 'V']).is_some_and(is_number);
        if versioned || is_number(part) {
            let run = parts[index..].iter().skip(1).take_while(|part| is_number(part)).count();
            if versioned || run > 0 {
                index += 1 + run;
                continue;
            }
        }
        words.push(part);
        index += 1;
    }
    words
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

/// `word` with a capital first letter if it is written in lower case only.
fn capitalised(word: &str) -> String {
    if word.chars().any(char::is_uppercase) {
        return word.to_owned();
    }
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}
