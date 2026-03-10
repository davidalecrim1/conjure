mod activate;
mod enumerate;
pub mod types;

use types::WindowInfo;

#[tauri::command]
pub fn list_windows(query: String) -> Vec<WindowInfo> {
    let include_minimized = crate::INCLUDE_MINIMIZED.load(std::sync::atomic::Ordering::Relaxed);
    let windows = enumerate::list(include_minimized);
    if query.is_empty() {
        crate::mru::sort(windows)
    } else {
        crate::search::fuzzy_search(&query, windows)
    }
}

#[tauri::command]
pub fn activate_window(window_id: u32, app_pid: i32) -> Result<(), String> {
    // Record in MRU before activating so ranking updates immediately
    // We need the WindowInfo to record -- look it up from current window list
    let include_minimized = crate::INCLUDE_MINIMIZED.load(std::sync::atomic::Ordering::Relaxed);
    let windows = enumerate::list(include_minimized);
    let title = windows
        .iter()
        .find(|w| w.id == window_id)
        .map(|w| {
            crate::mru::record(w.app_bundle_id.as_deref(), &w.app_name, &w.title);
            w.title.clone()
        })
        .unwrap_or_default();
    activate::activate(window_id, app_pid, &title)
}
