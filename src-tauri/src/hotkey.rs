use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::App;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::palette;

pub fn register(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::ALT), Code::KeyP);
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
        palette::toggle(&app_handle);
    })?;

    Ok(())
}
