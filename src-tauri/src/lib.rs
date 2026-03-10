mod mru;
mod permissions;
mod search;
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem, CheckMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

pub static INCLUDE_MINIMIZED: AtomicBool = AtomicBool::new(true);

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
            let switch = MenuItem::with_id(app, "switch", "Switch Windows", true, None::<&str>)?;
            let show_minimized = CheckMenuItem::with_id(
                app,
                "show_minimized",
                "Show Minimized Windows",
                true,
                INCLUDE_MINIMIZED.load(Ordering::Relaxed),
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit Conjure", true, Some("Cmd+Q"))?;
            let menu = Menu::with_items(app, &[&switch, &show_minimized, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-icon.png"
            ))?;

            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
                    "switch" => toggle_palette(app),
                    "show_minimized" => {
                        let current = INCLUDE_MINIMIZED.load(Ordering::Relaxed);
                        INCLUDE_MINIMIZED.store(!current, Ordering::Relaxed);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Register Cmd+Option+P global hotkey
            let shortcut = Shortcut::new(
                Some(Modifiers::SUPER | Modifiers::ALT),
                Code::KeyP,
            );
            let app_handle = app.handle().clone();
            // Debounce: the shortcut fires on both key-down and key-up,
            // so ignore calls within 300ms of the last one.
            let last_toggle: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
            app.global_shortcut().on_shortcut(shortcut, move |_, _, _| {
                let mut last = last_toggle.lock().unwrap();
                let now = Instant::now();
                if last.is_some_and(|t| now.duration_since(t) < Duration::from_millis(300)) {
                    return;
                }
                *last = Some(now);
                drop(last);
                toggle_palette(&app_handle);
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            windows::refresh_windows,
            windows::list_windows,
            windows::activate_window,
            hide_palette,
            set_include_minimized,
        ])
        .run(tauri::generate_context!())
        .expect("error while running conjure");
}

#[tauri::command]
fn set_include_minimized(include: bool) {
    INCLUDE_MINIMIZED.store(include, Ordering::Relaxed);
}

#[tauri::command]
fn hide_palette(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn toggle_palette(app: &AppHandle) {
    let app = app.clone();
    // Window operations must run on the main thread. The tray handler already
    // is on the main thread, but the global shortcut callback is not.
    let _ = app.clone().run_on_main_thread(move || {
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
    });
}
