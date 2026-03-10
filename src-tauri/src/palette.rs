use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

#[cfg(target_os = "macos")]
fn set_floating_level(window: &WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // NSModalPanelWindowLevel = 8 — above all normal app windows, below screen saver.
    // This prevents other apps from covering the palette when the mouse enters it.
    let ns_win = window.ns_window().expect("ns_window unavailable");
    unsafe {
        let win = ns_win as *mut AnyObject;
        let _: () = msg_send![win, setLevel: 8i64];
    }
}

pub static INCLUDE_MINIMIZED: AtomicBool = AtomicBool::new(true);

pub fn toggle(app: &AppHandle) {
    let app = app.clone();
    // Window operations must run on the main thread. The tray handler already
    // is on the main thread, but the global shortcut callback is not.
    let _ = app.clone().run_on_main_thread(move || {
        let window: WebviewWindow = match app.get_webview_window("main") {
            Some(w) => w,
            None => return,
        };

        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.center();
            let _ = window.show();
            let _ = window.set_focus();
            #[cfg(target_os = "macos")]
            set_floating_level(&window);
            let _ = app.emit("palette-opened", ());
        }
    });
}

#[tauri::command]
pub fn hide_palette(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn set_include_minimized(include: bool) {
    INCLUDE_MINIMIZED.store(include, Ordering::Relaxed);
}
