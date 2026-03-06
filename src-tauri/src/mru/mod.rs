use std::sync::Mutex;

use crate::windows::types::WindowInfo;

static MRU: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
const MAX_MRU: usize = 200;

/// Record a window activation. Key is (bundle_id_or_app_name, title).
pub fn record(bundle_id: Option<&str>, app_name: &str, title: &str) {
    let key = (
        bundle_id.unwrap_or(app_name).to_owned(),
        title.to_owned(),
    );
    let mut mru = MRU.lock().unwrap();
    mru.retain(|k| k != &key);
    mru.insert(0, key);
    if mru.len() > MAX_MRU {
        mru.truncate(MAX_MRU);
    }
}

/// Sort windows by MRU order. Windows not in MRU appear last, in original order.
pub fn sort(windows: Vec<WindowInfo>) -> Vec<WindowInfo> {
    let mru = MRU.lock().unwrap();

    let mut indexed: Vec<(usize, WindowInfo)> = windows
        .into_iter()
        .map(|w| {
            let key = (
                w.app_bundle_id.as_deref().unwrap_or(&w.app_name).to_owned(),
                w.title.clone(),
            );
            let rank = mru.iter().position(|k| k == &key).unwrap_or(usize::MAX);
            (rank, w)
        })
        .collect();

    indexed.sort_by_key(|(rank, _)| *rank);
    indexed.into_iter().map(|(_, w)| w).collect()
}
