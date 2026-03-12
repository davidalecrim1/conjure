use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub app_name: String,
    pub app_pid: i32,
    pub title: String,
    pub app_bundle_id: Option<String>,
    pub is_minimized: bool,
    /// "AppName - Window Title" used for fuzzy matching
    pub display_text: String,
    pub icon_data_url: Option<String>,
}

impl WindowInfo {
    pub fn new(
        id: u32,
        app_name: String,
        app_pid: i32,
        title: String,
        app_bundle_id: Option<String>,
        is_minimized: bool,
        icon_data_url: Option<String>,
    ) -> Self {
        let display_text = if title.is_empty() {
            app_name.clone()
        } else {
            format!("{} - {}", app_name, title)
        };
        Self {
            id,
            app_name,
            app_pid,
            title,
            app_bundle_id,
            is_minimized,
            display_text,
            icon_data_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WindowInfo;

    #[test]
    fn display_text_with_title() {
        let w = WindowInfo::new(1, "Zed".to_owned(), 100, "conjure".to_owned(), None, false, None);
        assert_eq!(w.display_text, "Zed - conjure");
    }

    #[test]
    fn display_text_empty_title() {
        let w = WindowInfo::new(1, "Finder".to_owned(), 100, "".to_owned(), None, false, None);
        assert_eq!(w.display_text, "Finder");
    }
}
