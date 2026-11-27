//! The desktop shell. Each Tauri command forwards to one operation of the
//! application core and adds nothing of its own.

pub mod ports;

use std::ops::ControlFlow;

use nicelibrary_core::{
    AddLocalGame, AutomaticSync, Core, CoverCache, Error, ExecutableChoice, Game, Launch, LibraryFilter,
    SteamAccountChoice, SteamInstallation, SteamWebApiKeyCheck, Store, SyncOutcome,
};
use percent_encoding::percent_decode_str;
use tauri::http::{Response, StatusCode, header};
use tauri::{App, Emitter, Manager, WebviewWindowBuilder};

#[tauri::command]
fn library(core: tauri::State<'_, Core>) -> Result<Vec<Game>, Error> {
    core.library()
}

/// Async, so reading Steam's folders runs off the main thread.
#[tauri::command]
async fn read_installed_games(core: tauri::State<'_, Core>) -> Result<Vec<Game>, Error> {
    core.read_installed_games()
}

/// Async, so a slow start of the registered program does not block the window.
#[tauri::command]
async fn launch(core: tauri::State<'_, Core>, game_id: String) -> Result<Launch, Error> {
    core.launch(&game_id)
}

#[tauri::command]
fn steam_installation(core: tauri::State<'_, Core>) -> SteamInstallation {
    core.steam_installation()
}

#[tauri::command]
fn steam_account(core: tauri::State<'_, Core>) -> Result<Option<String>, Error> {
    core.steam_account()
}

#[tauri::command]
fn choose_steam_account(core: tauri::State<'_, Core>, steam_id: String) -> Result<SteamAccountChoice, Error> {
    core.choose_steam_account(&steam_id)
}

#[tauri::command]
fn steam_web_api_key_stored(core: tauri::State<'_, Core>) -> Result<bool, Error> {
    core.steam_web_api_key_stored()
}

/// Async, so asking Steam runs off the main thread.
#[tauri::command]
async fn store_steam_web_api_key(core: tauri::State<'_, Core>, key: String) -> Result<SteamWebApiKeyCheck, Error> {
    core.store_steam_web_api_key(&key)
}

#[tauri::command]
fn remove_steam_web_api_key(core: tauri::State<'_, Core>) -> Result<(), Error> {
    core.remove_steam_web_api_key()
}

/// Async, so asking Steam runs off the main thread.
#[tauri::command]
async fn sync(core: tauri::State<'_, Core>) -> Result<SyncOutcome, Error> {
    core.sync()
}

#[tauri::command]
fn last_sync(core: tauri::State<'_, Core>) -> Result<Option<u64>, Error> {
    core.last_sync()
}

#[tauri::command]
fn filter_library(core: tauri::State<'_, Core>, filter: LibraryFilter) -> Result<Vec<Game>, Error> {
    core.filter_library(&filter)
}

/// Async, so a slow start of the browser does not block the window.
#[tauri::command]
async fn open_steam_web_api_key_page(core: tauri::State<'_, Core>) -> Result<(), Error> {
    core.open_steam_web_api_key_page()
}

/// Async, so the window stays responsive while the file dialog is open.
#[tauri::command]
async fn choose_local_game_executable(core: tauri::State<'_, Core>) -> Result<ExecutableChoice, Error> {
    core.choose_local_game_executable()
}

#[tauri::command]
fn add_local_game(core: tauri::State<'_, Core>, executable: String, title: String) -> Result<AddLocalGame, Error> {
    core.add_local_game(&executable, &title)
}

/// The URL scheme the interface loads Covers from: `cover://localhost/<game id>`,
/// on Windows `http://cover.localhost/<game id>` (see `coverUrl` in src/operations.ts).
pub const COVER_PROTOCOL: &str = "cover";

/// Answers a request for `cover://localhost/<game id>` with the Game's Cover:
/// the image, 404 when the Game has none, 503 when it could not be fetched.
/// The interface shows a placeholder for both.
pub fn cover_response(core: &Core, uri_path: &str) -> Response<Vec<u8>> {
    let game_id = percent_decode_str(uri_path.trim_start_matches('/')).decode_utf8_lossy();
    let (status, media_type, body) = match core.cover(&game_id) {
        Ok(Some(cover)) => (StatusCode::OK, cover.media_type, cover.bytes),
        Ok(None) => (StatusCode::NOT_FOUND, "text/plain", Vec::new()),
        Err(error) => (StatusCode::SERVICE_UNAVAILABLE, "text/plain", error.to_string().into_bytes()),
    };
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, media_type)
        .body(body)
        .expect("a valid response")
}

/// The database's file name in the app's data folder in the user profile
/// (`%APPDATA%\<identifier>` on Windows).
const DATABASE: &str = "nicelibrary.db";

/// The folder of downloaded Covers in the app's cache folder in the user
/// profile (`%LOCALAPPDATA%\<identifier>` on Windows).
const COVERS: &str = "covers";

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            library,
            read_installed_games,
            launch,
            steam_installation,
            steam_account,
            choose_steam_account,
            steam_web_api_key_stored,
            store_steam_web_api_key,
            remove_steam_web_api_key,
            open_steam_web_api_key_page,
            sync,
            last_sync,
            choose_local_game_executable,
            add_local_game,
            filter_library
        ])
        .register_asynchronous_uri_scheme_protocol(COVER_PROTOCOL, |context, request, responder| {
            let app = context.app_handle().clone();
            let path = request.uri().path().to_owned();
            // Off the main thread: a Cover may have to be downloaded first.
            tauri::async_runtime::spawn_blocking(move || {
                responder.respond(cover_response(&app.state::<Core>(), &path));
            });
        })
        .setup(|app| {
            let store = Store::open(&app.path().app_data_dir()?.join(DATABASE))?;
            let covers = CoverCache::new(app.path().app_cache_dir()?.join(COVERS));
            app.manage(Core::new(ports::real_ports(), store, covers));
            start_automatic_syncs(app)?;
            create_main_window(app)
        })
        .run(tauri::generate_context!())
        .expect("error while running NiceLibrary");
}

/// The event the interface hears the automatic Syncs through (`onAutomaticSync`
/// in src/operations.ts). A round without a key tells the interface nothing.
const AUTOMATIC_SYNC_EVENT: &str = "automatic-sync";

/// Runs the core's automatic Syncs on a thread of their own for as long as
/// the app runs.
fn start_automatic_syncs(app: &App) -> std::io::Result<()> {
    let app = app.handle().clone();
    std::thread::Builder::new().name("automatic-sync".into()).spawn(move || {
        app.state::<Core>().sync_automatically(|event| {
            if event != AutomaticSync::Skipped {
                // No window to tell is no reason to stop syncing.
                let _ = app.emit(AUTOMATIC_SYNC_EVENT, &event);
            }
            ControlFlow::Continue(())
        });
    })?;
    Ok(())
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
