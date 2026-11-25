//! The desktop shell. Each Tauri command forwards to one operation of the
//! application core and adds nothing of its own.

use nicelibrary_core::{Core, Game};

#[tauri::command]
fn library(core: tauri::State<'_, Core>) -> Vec<Game> {
    core.library()
}

pub fn run() {
    tauri::Builder::default()
        .manage(Core::new())
        .invoke_handler(tauri::generate_handler![library])
        .run(tauri::generate_context!())
        .expect("error while running NiceLibrary");
}
