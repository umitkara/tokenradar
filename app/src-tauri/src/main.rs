//! TokenRadar desktop app: tray icon, on-demand windows and the core runtime.

// Release builds use the GUI subsystem so no console window flashes up on start.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chrome;
mod core_bridge;
#[cfg(debug_assertions)]
mod gallery;
mod ipc;
mod panel;
mod prefs;
mod settings;
#[cfg(any(debug_assertions, feature = "spike"))]
mod spike;
mod tray;

use tauri::RunEvent;

fn main() {
    let bridge = ipc::builder();
    // Debug starts keep the TypeScript bindings in sync with the Rust types (Step 09).
    #[cfg(debug_assertions)]
    if let Err(error) = ipc::export(&bridge, ipc::BINDINGS_PATH) {
        eprintln!("TokenRadar: could not write the TypeScript bindings: {error}");
    }
    let commands = bridge.invoke_handler();
    // Step 02 measurement scenario; never part of a regular release build.
    #[cfg(any(debug_assertions, feature = "spike"))]
    let builder = spike::register(tauri::Builder::default(), commands);
    #[cfg(not(any(debug_assertions, feature = "spike")))]
    let builder = tauri::Builder::default().invoke_handler(commands);

    let app = builder
        // Must be the first plugin so a second launch exits before doing any work.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            panel::report(panel::show(app));
        }))
        .setup(move |app| {
            // Tray first, so it shows within its 300 ms budget; the core starts behind it.
            panel::init(app.handle());
            tray::create(app.handle())?;
            bridge.mount_events(app);
            prefs::init(app.handle());
            // Fake limits replace the core entirely, so tray measurements never touch real data.
            #[cfg(any(debug_assertions, feature = "spike"))]
            let fake = tray::fake::mode();
            #[cfg(not(any(debug_assertions, feature = "spike")))]
            let fake: Option<String> = None;
            match fake {
                #[cfg(any(debug_assertions, feature = "spike"))]
                Some(mode) => tray::fake::start(app.handle(), &mode),
                _ => core_bridge::start(app.handle()),
            }
            #[cfg(any(debug_assertions, feature = "spike"))]
            spike::start(app.handle());
            #[cfg(any(debug_assertions, feature = "spike"))]
            panel::scenario::start(app.handle());
            #[cfg(debug_assertions)]
            gallery::open_on_start(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the TokenRadar app");

    app.run(|app, event| match event {
        // The app lives in the tray: closing the last window must not quit it.
        // `code` is `None` only for that implicit request; `AppHandle::exit` (tray menu) passes
        // `Some(code)` and is allowed through.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Exit => core_bridge::shutdown(app),
        _ => {}
    });
}
