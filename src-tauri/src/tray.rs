//! The Tray: the app's icon in the notification area, where the app keeps
//! running while its window is closed. What closing the window does is the
//! core's to say ([`Core::close_window`]); this module carries it out.

use nicelibrary_core::{Core, WindowClose};
use serde::Deserialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// The label of the app's one window (tauri.conf.json).
pub const MAIN_WINDOW: &str = "main";

/// The id of the Tray icon.
const TRAY: &str = "main";

/// The event that asks the interface to show the Tray notice (`onTrayNotice`
/// in src/operations.ts).
const TRAY_NOTICE_EVENT: &str = "tray-notice";

/// The ids of the Tray menu's entries.
const OPEN: &str = "open";
const QUIT: &str = "quit";

/// The texts of the Tray icon, in the interface language. The interface owns
/// every text (src/i18n), so it hands them over. Mirrors `TrayMenuTexts` in
/// src/operations.ts.
#[derive(Debug, Deserialize)]
pub struct TrayMenuTexts {
    pub tooltip: String,
    pub open: String,
    pub quit: String,
}

/// Puts the app's icon into the Tray. A click on it opens the window; its
/// menu arrives with the interface's texts ([`show_menu`]).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN => show_main_window(app),
            QUIT => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    if let Some(name) = &app.config().product_name {
        builder = builder.tooltip(name);
    }
    builder.build(app)?;
    Ok(())
}

/// Gives the Tray icon its tooltip and its menu: open the window, quit the app.
pub fn show_menu(app: &AppHandle, texts: &TrayMenuTexts) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY) else {
        return Ok(());
    };
    let open = MenuItem::with_id(app, OPEN, &texts.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, &texts.quit, true, None::<&str>)?;
    tray.set_menu(Some(Menu::with_items(app, &[&open, &quit])?))?;
    tray.set_tooltip(Some(&texts.tooltip))
}

/// Closes the main window the way the core says: hides it into the Tray,
/// keeps it open for the Tray notice, or quits the app.
pub fn close_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    // Without a store (Unreadable Database) there is no Tray to keep running in.
    let Some(core) = app.try_state::<Core>() else {
        app.exit(0);
        return;
    };
    match core.close_window() {
        Ok(WindowClose::ShowTrayNotice) => {
            let _ = window.emit(TRAY_NOTICE_EVENT, ());
        }
        // The Tray icon still opens the window again, so a failure to read
        // the setting keeps the app running rather than quitting it.
        Ok(WindowClose::KeepRunningInTray) | Err(_) => {
            let _ = window.hide();
        }
        Ok(WindowClose::Quit) => app.exit(0),
    }
}

/// Brings the main window forward: shown, restored and focused.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
