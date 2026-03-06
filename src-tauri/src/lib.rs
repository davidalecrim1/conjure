mod mru;
mod permissions;
mod search;
mod windows;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

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

            // Build tray menu
            let quit = MenuItem::with_id(app, "quit", "Quit Conjure", true, Some("Cmd+Q"))?;
            let about = MenuItem::with_id(app, "about", "About Conjure", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&about, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-icon.png"
            ))?;

            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Register Cmd+Ctrl+Space global hotkey
            let shortcut = Shortcut::new(
                Some(Modifiers::SUPER | Modifiers::CONTROL),
                Code::Space,
            );
            let app_handle = app.handle().clone();
            app.global_shortcut().on_shortcut(shortcut, move |_, _, _| {
                toggle_palette(&app_handle);
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            windows::list_windows,
            windows::activate_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running conjure");
}

fn toggle_palette(app: &AppHandle) {
    let window: WebviewWindow = match app.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };

    let is_visible = window.is_visible().unwrap_or(false);

    if is_visible {
        let _ = window.hide();
    } else {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("palette-opened", ());
    }
}
