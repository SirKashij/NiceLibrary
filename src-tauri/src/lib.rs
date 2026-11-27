//! The desktop shell. Each Tauri command forwards to one operation of the
//! application core and adds nothing of its own.

pub mod ports;
mod tray;

use std::ops::ControlFlow;

use nicelibrary_core::{
    About, AboutPage, AddLocalGame, AutomaticSteamAppListFetch, AutomaticSync, Collection, CollectionNaming, Core, CoverCache,
    CoverHue, CoverImageChoice, EditLocalGame, Error, ExecutableChoice, FolderSuggestions, Game, GameAchievements, Install, Language, Launch, LaunchOptions,
    LibraryFilter, LibraryLayout, LimitedMode, LibrarySort, LocalGame, Log, Session, Status, SteamAccountChoice, SteamAppList, SteamAppListFetch,
    SteamInstallation, SteamTitle, SteamWebApiKeyCheck, Store, SyncOutcome, Theme, Tracking, started_with_windows,
};
use percent_encoding::percent_decode_str;
use tauri::http::{Response, StatusCode, header};
use tauri::{App, AppHandle, Emitter, Manager, WebviewWindowBuilder, WindowEvent};
use tray::TrayMenuTexts;

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

/// Async, like `launch`: it opens a `steam://` URL too.
#[tauri::command]
async fn install(core: tauri::State<'_, Core>, game_id: String) -> Result<Install, Error> {
    core.install(&game_id)
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

#[tauri::command]
fn game(core: tauri::State<'_, Core>, game_id: String) -> Result<Option<Game>, Error> {
    core.game(&game_id)
}

#[tauri::command]
fn library_sort(core: tauri::State<'_, Core>) -> Result<LibrarySort, Error> {
    core.library_sort()
}

#[tauri::command]
fn choose_library_sort(core: tauri::State<'_, Core>, sort: LibrarySort) -> Result<(), Error> {
    core.choose_library_sort(sort)
}

#[tauri::command]
fn set_status(core: tauri::State<'_, Core>, game_id: String, status: Status) -> Result<Game, Error> {
    core.set_status(&game_id, status)
}

#[tauri::command]
fn achievements(core: tauri::State<'_, Core>, game_id: String) -> Result<GameAchievements, Error> {
    core.achievements(&game_id)
}

/// Async, so a slow start of the browser does not block the window.
#[tauri::command]
async fn open_steam_web_api_key_page(core: tauri::State<'_, Core>) -> Result<(), Error> {
    core.open_steam_web_api_key_page()
}

#[tauri::command]
fn starts_with_windows(core: tauri::State<'_, Core>) -> Result<bool, Error> {
    core.starts_with_windows()
}

#[tauri::command]
fn set_start_with_windows(core: tauri::State<'_, Core>, on: bool) -> Result<(), Error> {
    core.set_start_with_windows(on)
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

#[tauri::command]
fn local_game(core: tauri::State<'_, Core>, game_id: String) -> Result<Option<LocalGame>, Error> {
    core.local_game(&game_id)
}

/// Async, so the window stays responsive while the file dialog is open.
#[tauri::command]
async fn choose_executable_for(core: tauri::State<'_, Core>, game_id: String) -> Result<ExecutableChoice, Error> {
    core.choose_executable_for(&game_id)
}

#[tauri::command]
fn edit_local_game(
    core: tauri::State<'_, Core>,
    game_id: String,
    title: String,
    executable: String,
) -> Result<EditLocalGame, Error> {
    core.edit_local_game(&game_id, &title, &executable)
}

#[tauri::command]
fn remove_local_game(core: tauri::State<'_, Core>, game_id: String) -> Result<(), Error> {
    core.remove_local_game(&game_id)
}

#[tauri::command]
fn library_layout(core: tauri::State<'_, Core>) -> Result<LibraryLayout, Error> {
    core.library_layout()
}

#[tauri::command]
fn choose_library_layout(core: tauri::State<'_, Core>, layout: LibraryLayout) -> Result<(), Error> {
    core.choose_library_layout(layout)
}

/// Async, so the window stays responsive while the folder dialog is open.
#[tauri::command]
async fn suggest_executables_in_folder(core: tauri::State<'_, Core>) -> Result<FolderSuggestions, Error> {
    core.suggest_executables_in_folder()
}

#[tauri::command]
fn theme(core: tauri::State<'_, Core>) -> Result<Theme, Error> {
    core.theme()
}

#[tauri::command]
fn choose_theme(core: tauri::State<'_, Core>, theme: Theme) -> Result<(), Error> {
    core.choose_theme(theme)
}

#[tauri::command]
fn collections(core: tauri::State<'_, Core>) -> Result<Vec<Collection>, Error> {
    core.collections()
}

#[tauri::command]
fn create_collection(core: tauri::State<'_, Core>, name: String) -> Result<CollectionNaming, Error> {
    core.create_collection(&name)
}

#[tauri::command]
fn rename_collection(core: tauri::State<'_, Core>, id: i64, name: String) -> Result<CollectionNaming, Error> {
    core.rename_collection(id, &name)
}

#[tauri::command]
fn delete_collection(core: tauri::State<'_, Core>, id: i64) -> Result<(), Error> {
    core.delete_collection(id)
}

#[tauri::command]
fn game_collections(core: tauri::State<'_, Core>, game_id: String) -> Result<Vec<i64>, Error> {
    core.game_collections(&game_id)
}

#[tauri::command]
fn add_game_to_collection(core: tauri::State<'_, Core>, collection: i64, game_id: String) -> Result<(), Error> {
    core.add_game_to_collection(collection, &game_id)
}

#[tauri::command]
fn remove_game_from_collection(core: tauri::State<'_, Core>, collection: i64, game_id: String) -> Result<(), Error> {
    core.remove_game_from_collection(collection, &game_id)
}

#[tauri::command]
fn close_to_tray(core: tauri::State<'_, Core>) -> Result<bool, Error> {
    core.close_to_tray()
}

#[tauri::command]
fn set_close_to_tray(core: tauri::State<'_, Core>, on: bool) -> Result<(), Error> {
    core.set_close_to_tray(on)
}

/// Closes the window as its close button does (see [`Core::close_window`]).
#[tauri::command]
fn close_window(app: AppHandle) {
    tray::close_main_window(&app);
}

/// The Tray's texts come from the interface, which owns every text.
#[tauri::command]
fn show_tray_menu(app: AppHandle, texts: TrayMenuTexts) -> Result<(), String> {
    tray::show_menu(&app, &texts).map_err(|error| error.to_string())
}

#[tauri::command]
fn steam_app_list(core: tauri::State<'_, Core>) -> Result<SteamAppList, Error> {
    core.steam_app_list()
}

/// Async, so asking Steam for every page runs off the main thread.
#[tauri::command]
async fn fetch_steam_app_list(core: tauri::State<'_, Core>) -> Result<SteamAppListFetch, Error> {
    core.fetch_steam_app_list()
}

/// Async, so the window stays responsive while the file dialog is open.
#[tauri::command]
async fn choose_cover_image(core: tauri::State<'_, Core>, game_id: String) -> Result<CoverImageChoice, Error> {
    core.choose_cover_image(&game_id)
}

#[tauri::command]
fn cover_image_chosen(core: tauri::State<'_, Core>, game_id: String) -> Result<bool, Error> {
    core.cover_image_chosen(&game_id)
}

#[tauri::command]
fn remove_cover_image(core: tauri::State<'_, Core>, game_id: String) -> Result<(), Error> {
    core.remove_cover_image(&game_id)
}

/// Async, so loading the stored Steam App List for the first lookup runs off the main thread.
#[tauri::command]
async fn find_steam_titles(core: tauri::State<'_, Core>, typed: String) -> Result<Vec<SteamTitle>, Error> {
    core.find_steam_titles(&typed)
}

/// Async, like `find_steam_titles`.
#[tauri::command]
async fn propose_cover_match(core: tauri::State<'_, Core>, title: String) -> Result<Option<SteamTitle>, Error> {
    core.propose_cover_match(&title)
}

#[tauri::command]
fn cover_match(core: tauri::State<'_, Core>, game_id: String) -> Result<Option<SteamTitle>, Error> {
    core.cover_match(&game_id)
}

#[tauri::command]
fn set_cover_match(core: tauri::State<'_, Core>, game_id: String, app_id: Option<u32>) -> Result<(), Error> {
    core.set_cover_match(&game_id, app_id)
}

/// Async, so a slow start of File Explorer does not block the window.
#[tauri::command]
async fn open_log_folder(core: tauri::State<'_, Core>) -> Result<(), Error> {
    core.open_log_folder()
}

#[tauri::command]
fn launch_options(core: tauri::State<'_, Core>, game_id: String) -> Result<Option<LaunchOptions>, Error> {
    core.launch_options(&game_id)
}

#[tauri::command]
fn set_launch_options(core: tauri::State<'_, Core>, game_id: String, options: LaunchOptions) -> Result<(), Error> {
    core.set_launch_options(&game_id, &options)
}

#[tauri::command]
fn hidden_games(core: tauri::State<'_, Core>) -> Result<Vec<Game>, Error> {
    core.hidden_games()
}

#[tauri::command]
fn hide_game(core: tauri::State<'_, Core>, game_id: String) -> Result<(), Error> {
    core.hide_game(&game_id)
}

#[tauri::command]
fn unhide_game(core: tauri::State<'_, Core>, game_id: String) -> Result<(), Error> {
    core.unhide_game(&game_id)
}

#[tauri::command]
fn mark_favorite(core: tauri::State<'_, Core>, game_id: String, favorite: bool) -> Result<(), Error> {
    core.mark_favorite(&game_id, favorite)
}

#[tauri::command]
fn favorite_count(core: tauri::State<'_, Core>) -> Result<u32, Error> {
    core.favorite_count()
}

#[tauri::command]
fn about(core: tauri::State<'_, Core>) -> About {
    core.about()
}

#[tauri::command]
fn limited_mode(core: tauri::State<'_, Core>) -> Result<LimitedMode, Error> {
    core.limited_mode()
}

/// Async, so a slow start of the browser does not block the window.
#[tauri::command]
async fn open_about_page(core: tauri::State<'_, Core>, page: AboutPage) -> Result<(), Error> {
    core.open_about_page(page)
}

#[tauri::command]
fn language(core: tauri::State<'_, Core>) -> Result<Language, Error> {
    core.language()
}

#[tauri::command]
fn choose_language(core: tauri::State<'_, Core>, language: Language) -> Result<(), Error> {
    core.choose_language(language)
}

/// Async: the Cover may have to be downloaded, and its image is decoded.
#[tauri::command]
async fn cover_hue(core: tauri::State<'_, Core>, game_id: String) -> Result<Option<CoverHue>, Error> {
    core.cover_hue(&game_id)
}

/// Async, so a slow start of the browser does not block the window.
#[tauri::command]
async fn open_steam_privacy_settings(core: tauri::State<'_, Core>) -> Result<(), Error> {
    core.open_steam_privacy_settings()
}

#[tauri::command]
fn sessions(core: tauri::State<'_, Core>, game_id: String) -> Result<Vec<Session>, Error> {
    core.sessions(&game_id)
}

/// The URL scheme the interface loads Covers from: `cover://localhost/<game id>`,
/// on Windows `http://cover.localhost/<game id>` (see `coverUrl` in src/operations.ts).
/// Achievement icons come from `<game id>/achievements/<icon>` under the same scheme
/// (see `achievementIconUrl` there).
pub const COVER_PROTOCOL: &str = "cover";

/// Answers a request for `cover://localhost/<game id>` with the Game's Cover,
/// or for `cover://localhost/<game id>/achievements/<icon>` with that
/// Achievement icon: the image, 404 when there is none, 503 when it could
/// not be fetched. The interface shows a placeholder for both.
pub fn cover_response(core: &Core, uri_path: &str) -> Response<Vec<u8>> {
    let path = percent_decode_str(uri_path.trim_start_matches('/')).decode_utf8_lossy();
    let image = match path.split_once("/achievements/") {
        Some((game_id, icon)) => core.achievement_icon(game_id, icon),
        None => core.cover(&path),
    };
    let (status, media_type, body) = match image {
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

/// The folder of images the user chose as Covers, in the app's data folder
/// next to the database: data, not cache, as they cannot be fetched again.
const CHOSEN_COVERS: &str = "covers";

/// The folder of the log files in the app's data folder, next to the
/// database (`%APPDATA%\<identifier>\logs` on Windows).
const LOGS: &str = "logs";

pub fn run() {
    tauri::Builder::default()
        // First, so a second start hands over and ends before it opens anything.
        // A start with Windows while the app already runs leaves it in the Tray.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !started_with_windows(&args) {
                tray::show_main_window(app);
            }
        }))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == tray::MAIN_WINDOW
            {
                api.prevent_close();
                tray::close_main_window(window.app_handle());
            }
        })
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
            theme,
            choose_theme,
            collections,
            create_collection,
            rename_collection,
            delete_collection,
            steam_app_list,
            fetch_steam_app_list,
            filter_library,
            game,
            library_sort,
            choose_library_sort,
            choose_local_game_executable,
            add_local_game,
            game_collections,
            add_game_to_collection,
            remove_game_from_collection,
            library_layout,
            choose_library_layout,
            close_to_tray,
            set_close_to_tray,
            close_window,
            show_tray_menu,
            choose_cover_image,
            cover_image_chosen,
            remove_cover_image,
            find_steam_titles,
            propose_cover_match,
            cover_match,
            set_cover_match,
            set_status,
            open_log_folder,
            limited_mode,
            open_steam_privacy_settings,
            starts_with_windows,
            set_start_with_windows,
            achievements,
            launch_options,
            set_launch_options,
            hidden_games,
            hide_game,
            unhide_game,
            local_game,
            choose_executable_for,
            edit_local_game,
            remove_local_game,
            mark_favorite,
            favorite_count,
            about,
            open_about_page,
            language,
            choose_language,
            cover_hue,
            install,
            suggest_executables_in_folder,
            sessions
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
            let covers = CoverCache::new(app.path().app_cache_dir()?.join(COVERS))
                .with_chosen_images_in(app.path().app_data_dir()?.join(CHOSEN_COVERS));
            let log = Log::new(app.path().app_data_dir()?.join(LOGS));
            app.manage(Core::new(ports::real_ports(), store, covers).with_log(log));
            start_automatic_syncs(app)?;
            tray::create(app.handle())?;
            start_automatic_steam_app_list_fetches(app)?;
            start_session_tracking(app)?;
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

/// The event the interface hears the automatic fetches of the Steam App List
/// through (`onAutomaticSteamAppListFetch` in src/operations.ts). A round that
/// fetches nothing tells the interface nothing.
const AUTOMATIC_STEAM_APP_LIST_FETCH_EVENT: &str = "automatic-steam-app-list-fetch";

/// Runs the core's automatic fetches of the Steam App List on a thread of
/// their own for as long as the app runs.
fn start_automatic_steam_app_list_fetches(app: &App) -> std::io::Result<()> {
    let app = app.handle().clone();
    std::thread::Builder::new().name("automatic-steam-app-list-fetch".into()).spawn(move || {
        app.state::<Core>().fetch_steam_app_list_automatically(|event| {
            if event != AutomaticSteamAppListFetch::Skipped {
                // No window to tell is no reason to stop fetching.
                let _ = app.emit(AUTOMATIC_STEAM_APP_LIST_FETCH_EVENT, &event);
            }
            ControlFlow::Continue(())
        });
    })?;
    Ok(())
}

/// The event the interface hears Sessions begin and end through
/// (`onSession` in src/operations.ts). A failed look tells the interface nothing.
const SESSION_EVENT: &str = "session";

/// Records the Sessions of Local Games and Installed Steam Games on a thread
/// of its own for as long as the app runs. Each Session that ends starts an
/// automatic Sync, which the interface hears through `AUTOMATIC_SYNC_EVENT`.
fn start_session_tracking(app: &App) -> std::io::Result<()> {
    let app = app.handle().clone();
    std::thread::Builder::new().name("session-tracking".into()).spawn(move || {
        app.state::<Core>().track_sessions(|event| {
            if !matches!(event, Tracking::Failed { .. }) {
                // No window to tell is no reason to stop tracking.
                let _ = app.emit(SESSION_EVENT, &event);
            }
            ControlFlow::Continue(())
        });
    })?;
    Ok(())
}

/// Creates the window declared in tauri.conf.json (`"create": false` there).
/// Started with Windows, the app starts hidden in the Tray: the window is
/// created, so the interface runs, but not shown.
fn create_main_window(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = app.config().app.windows.first().ok_or("tauri.conf.json declares no window")?.clone();
    if started_with_windows(std::env::args()) {
        config.visible = false;
    }
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
