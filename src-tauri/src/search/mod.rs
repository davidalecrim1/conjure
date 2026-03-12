use crate::windows::types::WindowInfo;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};

pub fn fuzzy_search(query: &str, windows: Vec<WindowInfo>) -> Vec<WindowInfo> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(u32, WindowInfo)> = windows
        .into_iter()
        .filter_map(|w| {
            let score = pattern.score(
                nucleo_matcher::Utf32Str::new(&w.display_text, &mut Vec::new()),
                &mut matcher,
            )?;
            Some((score, w))
        })
        .collect();

    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, w)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows::types::WindowInfo;

    fn make_window(id: u32, app_name: &str, title: &str) -> WindowInfo {
        WindowInfo::new(id, app_name.to_owned(), id as i32, title.to_owned(), None, false, None)
    }

    #[test]
    fn fuzzy_search_filters_non_matching() {
        let windows = vec![
            make_window(1, "Safari", ""),
            make_window(2, "Terminal", ""),
            make_window(3, "Zed", "conjure"),
        ];
        let results = fuzzy_search("zed", windows);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].app_name, "Zed");
    }

    #[test]
    fn fuzzy_search_is_case_insensitive() {
        let windows = vec![make_window(1, "Zed", "conjure")];
        let results = fuzzy_search("ZED", windows);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn fuzzy_search_ranks_better_match_first() {
        let windows = vec![
            make_window(1, "iTerm2", ""),
            make_window(2, "Terminal", ""),
        ];
        let results = fuzzy_search("terminal", windows);
        assert!(!results.is_empty());
        assert_eq!(results[0].app_name, "Terminal");
    }

    #[test]
    fn fuzzy_search_matches_on_display_text() {
        // "conjure" appears only in the title portion of display_text
        let windows = vec![make_window(1, "Zed", "conjure")];
        let results = fuzzy_search("conjure", windows);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "conjure");
    }
}
