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
