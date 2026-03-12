mod hotkey;
mod mru;
mod palette;
mod permissions;
mod search;
mod tray;
pub mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Hide from Dock -- run as background accessory app
            #[cfg(target_os = "macos")]
            {
                use objc2::MainThreadMarker;
                use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                // SAFETY: Tauri's setup closure always runs on the main thread
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let ns_app = NSApplication::sharedApplication(mtm);
                ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            }

            permissions::check_and_request();
            tray::setup(app)?;
            hotkey::register(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            windows::refresh_windows,
            windows::list_windows,
            windows::activate_window,
            palette::hide_palette,
            palette::set_include_minimized,
            palette::resize_palette,
        ])
        .run(tauri::generate_context!())
        .expect("error while running conjure");
}
