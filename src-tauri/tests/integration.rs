/// Live system integration tests for window enumeration.
///
/// These tests call CGWindowListCopyWindowInfo + the AX API against the running system.
/// They require Accessibility permission granted to the test binary (Terminal or your IDE).
///
/// Without Accessibility permission the AX enrichment silently returns no titles, but
/// CGWindowList still works — so the basic smoke tests remain valid. The permission
/// guard is therefore only applied to the title-format test where AX data matters.
///
/// Run with: cargo test --test integration

#[cfg(target_os = "macos")]
mod enumerate {
    use conjure_lib::windows::enumerate;

    fn ax_is_trusted() -> bool {
        // AXIsProcessTrusted is the canonical runtime check for Accessibility permission.
        unsafe { accessibility_sys::AXIsProcessTrusted() }
    }

    #[test]
    fn enumerate_returns_non_empty_list() {
        let windows = enumerate::list(false);
        assert!(
            !windows.is_empty(),
            "expected at least one window from the running system"
        );
    }

    #[test]
    fn enumerate_excludes_self() {
        let own_pid = std::process::id() as i32;
        let windows = enumerate::list(false);
        assert!(
            windows.iter().all(|w| w.app_pid != own_pid),
            "own PID should be excluded from window list"
        );
    }

    #[test]
    fn enumerate_no_duplicate_window_ids() {
        let windows = enumerate::list(false);
        let mut ids: Vec<u32> = windows.iter().map(|w| w.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            windows.len(),
            "duplicate window IDs found in enumeration result"
        );
    }

    #[test]
    fn enumerate_display_text_format() {
        if !ax_is_trusted() {
            eprintln!("SKIP: enumerate_display_text_format — Accessibility permission not granted");
            return;
        }
        let windows = enumerate::list(false);
        for w in &windows {
            if w.title.is_empty() {
                assert_eq!(
                    w.display_text, w.app_name,
                    "window without title should have display_text == app_name"
                );
            } else {
                let expected = format!("{} - {}", w.app_name, w.title);
                assert_eq!(
                    w.display_text, expected,
                    "window with title should have display_text == 'AppName - Title'"
                );
            }
        }
    }

    #[test]
    fn enumerate_include_minimized_flag() {
        // Verify that both modes return results. The exact relationship between the
        // counts depends on whether minimized windows exist and on AX permission.
        let without = enumerate::list(false).len();
        let with_min = enumerate::list(true).len();
        assert!(without > 0, "list(false) should return at least one window");
        assert!(with_min > 0, "list(true) should return at least one window");
    }
}
