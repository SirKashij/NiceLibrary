//! Launch: starting a Game from the app. A Steam Game is launched through
//! Steam itself, by opening its `steam://` URL, never through its executable
//! (ADR-0002). A Local Game is launched by starting its executable with its
//! Launch Arguments in its Working Directory.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::local_games::StoredLocalGame;

/// Port: opens a URL with the program the operating system has registered
/// for its scheme, as a click on a link would.
///
/// Contract: `Ok` once the registered program has been handed the URL; it
/// does not wait for that program to finish. `Err(NoProgram)` when no
/// program is registered for the scheme, `Err(Failed)` when it cannot be
/// handed the URL. Anything that is not a URL, and every `file://` URL, is
/// refused with `Failed`, so no program can be started through the opener
/// by its path. Never shows a dialog of its own.
///
/// `open_folder` shows a folder in the operating system's file manager.
/// Contract: `Ok` once the file manager has been handed the folder;
/// `Err(Failed)` when it cannot be, and for anything that is not an
/// existing folder, so no file or program is ever opened through it. An
/// opener that cannot show folders keeps the refusing default.
pub trait Opener: Send + Sync {
    fn open_url(&self, url: &str) -> Result<(), OpenError>;

    fn open_folder(&self, folder: &Path) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("showing the folder {} is not supported here", folder.display())))
    }
}

/// Why the [`Opener`] could not open a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// No program is registered for the URL's scheme.
    NoProgram,
    /// Anything else, described for a log or a developer.
    Failed(String),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProgram => f.write_str("no program is registered for this kind of URL"),
            Self::Failed(reason) => f.write_str(reason),
        }
    }
}

/// How a Launch went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Launch {
    /// The Game has been handed to whatever launches it.
    Launched,
    /// The Game is a Steam Game and Steam is not available on this computer:
    /// not installed, or no longer registered for its `steam://` URLs.
    SteamNotFound,
    /// The Game is a Local Game and no file is at its executable's path any
    /// more; nothing was started.
    ExecutableNotFound,
    /// The Game is a Local Game and its Working Directory is not a folder
    /// that exists; nothing was started.
    WorkingDirectoryNotFound,
}

/// The URL that tells Steam to launch the app `app_id`.
pub(crate) fn steam_launch_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// Launches the Local Game `game` by starting its executable with its Launch
/// Arguments, in its Working Directory or else in the executable's folder. A
/// relative Working Directory is read from the executable's folder. Nothing
/// is started when the executable or the Working Directory is missing.
pub(crate) fn launch_local_game(starter: &dyn ProcessStarter, game: &StoredLocalGame) -> Result<Launch, Error> {
    let executable = Path::new(&game.executable);
    if !executable.is_file() {
        return Ok(Launch::ExecutableNotFound);
    }
    let folder = executable.parent().unwrap_or(Path::new(""));
    let working_directory = match &game.launch_options.working_directory {
        Some(chosen) => folder.join(chosen),
        None => folder.to_path_buf(),
    };
    if !working_directory.is_dir() {
        return Ok(Launch::WorkingDirectoryNotFound);
    }
    starter
        .start(executable, &split_arguments(&game.launch_options.arguments), &working_directory)
        .map_err(|reason| Error::new(format!("{} could not be launched: {reason}", game.title)))?;
    Ok(Launch::Launched)
}

/// Port: starts a program's executable as a process of its own, directly
/// and never through a shell, so no argument is read as a command.
///
/// Contract: `Ok` once the process has started; it does not wait for it to
/// end, and the process keeps running when the app ends. Each of
/// `arguments` reaches the program as one argument, exactly as given,
/// spaces and quotes included. The process runs in `working_directory`.
/// `Err` when the process cannot be started, for instance when no file is
/// at `executable`, described for a log or a developer. A program the
/// operating system would run through a shell, such as a batch file, is
/// refused with `Err` and not started. Never shows a dialog of its own.
pub trait ProcessStarter: Send + Sync {
    fn start(&self, executable: &Path, arguments: &[String], working_directory: &Path) -> Result<(), String>;
}

/// How a Local Game is launched, as the user set it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    /// The Launch Arguments as the user typed them, split when the Game is
    /// launched: at spaces, except within double quotes;
    /// empty for none.
    pub arguments: String,
    /// The Working Directory; `None` for the executable's folder.
    pub working_directory: Option<String>,
}

/// The Launch Arguments `typed` as separate arguments: separated by spaces,
/// and a part in double quotes is one argument with its spaces, without the
/// quotes; `""` is an empty argument. Backslashes are kept as they are, so a
/// path needs no escaping. A quote left open runs to the end.
fn split_arguments(typed: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    // The argument being read; `None` between arguments.
    let mut current: Option<String> = None;
    let mut quoted = false;
    for c in typed.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.get_or_insert_with(String::new);
            }
            c if c.is_whitespace() && !quoted => arguments.extend(current.take()),
            c => current.get_or_insert_with(String::new).push(c),
        }
    }
    arguments.extend(current);
    arguments
}

/// How starting the installation of a Steam Game went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Install {
    /// Steam has been asked to show its install dialog for the Game; the
    /// person goes on there.
    Started,
    /// Steam is not available on this computer: not installed, or no longer
    /// registered for its `steam://` URLs.
    SteamNotFound,
}

/// The URL that opens Steam's install dialog for the app `app_id`.
pub(crate) fn steam_install_url(app_id: u32) -> String {
    format!("steam://install/{app_id}")
}
