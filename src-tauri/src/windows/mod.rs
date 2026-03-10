mod activate;
mod enumerate;
pub mod types;

use std::sync::{LazyLock, Mutex};
use types::WindowInfo;

static WINDOW_CACHE: LazyLock<Mutex<Vec<WindowInfo>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[tauri::command]
pub fn refresh_windows() {
    let include_minimized = crate::INCLUDE_MINIMIZED.load(std::sync::atomic::Ordering::Relaxed);
    let windows = enumerate::list(include_minimized);
    *WINDOW_CACHE.lock().unwrap() = windows;
}

#[tauri::command]
pub fn list_windows(query: String) -> Vec<WindowInfo> {
    let cache = WINDOW_CACHE.lock().unwrap().clone();
    if query.is_empty() {
        crate::mru::sort(cache)
    } else {
        crate::search::fuzzy_search(&query, cache)
    }
}

#[tauri::command]
pub fn activate_window(window_id: u32, app_pid: i32) -> Result<(), String> {
    // Record in MRU before activating so ranking updates immediately
    // Look up from cache to avoid re-enumerating
    let cache = WINDOW_CACHE.lock().unwrap().clone();
    let title = cache
        .iter()
        .find(|w| w.id == window_id)
        .map(|w| {
            crate::mru::record(w.app_bundle_id.as_deref(), &w.app_name, &w.title);
            w.title.clone()
        })
        .unwrap_or_default();
    activate::activate(window_id, app_pid, &title)
}
