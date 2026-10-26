//! TokenRadar desktop app: tray icon, on-demand windows and (later) the core runtime.

// Release builds use the GUI subsystem so no console window flashes up on start.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod panel;
mod tray;

use tauri::RunEvent;

fn main() {
    let app = tauri::Builder::default()
        // Must be the first plugin so a second launch exits before doing any work.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            panel::report(panel::show(app));
        }))
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the TokenRadar app");

    app.run(|_app, event| {
        // The app lives in the tray: closing the last window must not quit it.
        // `code` is `None` only for that implicit request; `AppHandle::exit` (tray menu) passes
        // `Some(code)` and is allowed through.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
