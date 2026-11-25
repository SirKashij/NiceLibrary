//! The desktop shell. Each Tauri command forwards to one operation of the
//! application core and adds nothing of its own.

pub mod ports;

use nicelibrary_core::{Core, Error, Game, SteamInstallation, Store};
use tauri::{App, Manager, WebviewWindowBuilder};

#[tauri::command]
fn library(core: tauri::State<'_, Core>) -> Result<Vec<Game>, Error> {
    core.library()
}

/// Async, so reading Steam's folders runs off the main thread.
#[tauri::command]
async fn read_installed_games(core: tauri::State<'_, Core>) -> Result<Vec<Game>, Error> {
    core.read_installed_games()
}

#[tauri::command]
fn steam_installation(core: tauri::State<'_, Core>) -> SteamInstallation {
    core.steam_installation()
}

/// The database's file name in the app's data folder in the user profile
/// (`%APPDATA%\<identifier>` on Windows).
const DATABASE: &str = "nicelibrary.db";

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![library, read_installed_games, steam_installation])
        .setup(|app| {
            let store = Store::open(&app.path().app_data_dir()?.join(DATABASE))?;
            app.manage(Core::new(ports::real_ports(), store));
            create_main_window(app)
        })
        .run(tauri::generate_context!())
        .expect("error while running NiceLibrary");
}

/// Creates the window declared in tauri.conf.json (`"create": false` there).
fn create_main_window(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let config = app.config().app.windows.first().ok_or("tauri.conf.json declares no window")?.clone();
    let builder = WebviewWindowBuilder::from_config(app.handle(), &config)?;
    #[cfg(windows)]
    let builder = with_webview2_env_args(builder);
    builder.build()?;
    Ok(())
}

/// The arguments wry passes to WebView2 when the app sets none.
#[cfg(windows)]
const WRY_DEFAULT_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

/// Honours `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on every machine.
///
/// WebView2 documents the variable, but where the app passes its own browser
/// arguments (wry always does), some machines ignore it, the GitHub Windows
/// runner among them. msedgedriver relies on it for the DevTools port, so the
/// end-to-end tests (e2e/) need it. Without the variable nothing changes.
#[cfg(windows)]
fn with_webview2_env_args<R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: WebviewWindowBuilder<'_, R, M>,
) -> WebviewWindowBuilder<'_, R, M> {
    match std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
        Ok(extra) if !extra.trim().is_empty() => {
            builder.additional_browser_args(&format!("{WRY_DEFAULT_ARGS} {extra}"))
        }
        _ => builder,
    }
}
