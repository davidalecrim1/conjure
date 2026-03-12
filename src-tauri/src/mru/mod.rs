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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows::types::WindowInfo;

    fn make_window(id: u32, app_name: &str, bundle_id: Option<&str>, title: &str) -> WindowInfo {
        WindowInfo::new(
            id,
            app_name.to_owned(),
            id as i32,
            title.to_owned(),
            bundle_id.map(str::to_owned),
            false,
            None,
        )
    }

    fn clear() {
        MRU.lock().unwrap().clear();
    }

    #[test]
    fn mru_record_adds_to_front() {
        clear();
        record(Some("com.app.a"), "AppA", "doc");
        let sorted = sort(vec![make_window(1, "AppA", Some("com.app.a"), "doc")]);
        assert_eq!(sorted[0].app_name, "AppA");
    }

    #[test]
    fn mru_record_deduplicates() {
        clear();
        record(Some("com.app.a"), "AppA", "doc");
        record(Some("com.app.b"), "AppB", "doc");
        record(Some("com.app.a"), "AppA", "doc");
        let mru = MRU.lock().unwrap();
        // "com.app.a" should appear exactly once
        let count = mru.iter().filter(|(k, _)| k == "com.app.a").count();
        assert_eq!(count, 1);
        // Most recent is at the front
        assert_eq!(mru[0].0, "com.app.a");
    }

    #[test]
    fn mru_prefers_bundle_id_over_app_name() {
        clear();
        record(Some("com.app.zed"), "Zed", "conjure");
        let windows = vec![make_window(1, "Zed", Some("com.app.zed"), "conjure")];
        let sorted = sort(windows);
        assert_eq!(sorted[0].app_name, "Zed");
    }

    #[test]
    fn mru_falls_back_to_app_name() {
        clear();
        record(None, "Finder", "");
        let windows = vec![make_window(1, "Finder", None, "")];
        let sorted = sort(windows);
        assert_eq!(sorted[0].app_name, "Finder");
    }

    #[test]
    fn mru_unknown_windows_appear_last() {
        clear();
        record(Some("com.app.a"), "AppA", "doc");
        let windows = vec![
            make_window(1, "Unknown", None, ""),
            make_window(2, "AppA", Some("com.app.a"), "doc"),
        ];
        let sorted = sort(windows);
        assert_eq!(sorted[0].app_name, "AppA");
        assert_eq!(sorted[1].app_name, "Unknown");
    }

    #[test]
    fn mru_truncates_at_200() {
        // Insert 201 unique entries and confirm the list never exceeds MAX_MRU.
        // We check after each insert to be resilient to concurrent test state.
        for i in 0..201u32 {
            record(None, &format!("TruncApp{}", i), "trunctest");
            let len = MRU.lock().unwrap().len();
            assert!(len <= MAX_MRU, "MRU grew beyond MAX_MRU after {} inserts: len={}", i + 1, len);
        }
    }
}
