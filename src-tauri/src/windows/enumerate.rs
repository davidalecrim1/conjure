use super::types::WindowInfo;
use accessibility_sys::{
    kAXTitleAttribute, kAXWindowsAttribute, AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication,
};
use cocoa::base::{id, nil};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    dictionary::CFDictionary,
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use core_graphics::window::{
    kCGWindowLayer, kCGWindowListOptionOnScreenOnly, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID, CGWindowListCopyWindowInfo,
};
use objc::{msg_send, sel, sel_impl};
use std::collections::HashMap;

const EXCLUDED_APPS: &[&str] = &[
    "Window Server",
    "Dock",
    "SystemUIServer",
    "Control Center",
    "Notification Center",
    "Spotlight",
];

pub fn list() -> Vec<WindowInfo> {
    let own_pid = std::process::id() as i32;
    let cg_windows = get_cg_windows(own_pid);

    let pids: Vec<i32> = cg_windows
        .iter()
        .map(|w| w.pid)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let ax_titles = get_ax_titles(&pids);

    cg_windows
        .into_iter()
        .map(|cg| {
            let title = ax_titles
                .get(&cg.pid)
                .and_then(|titles| titles.first())
                .cloned()
                .unwrap_or_default();
            WindowInfo::new(cg.id, cg.owner_name, cg.pid, title, cg.bundle_id, false)
        })
        .collect()
}

struct CgWindowRaw {
    id: u32,
    owner_name: String,
    pid: i32,
    bundle_id: Option<String>,
}

fn get_cg_windows(own_pid: i32) -> Vec<CgWindowRaw> {
    let mut results = Vec::new();

    unsafe {
        let window_list = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0);
        if window_list.is_null() {
            return results;
        }

        let array: CFArray<CFDictionary<CFString, CFType>> =
            CFArray::wrap_under_get_rule(window_list as CFArrayRef);

        for window in array.iter() {
            let layer = cf_dict_number(&window, kCGWindowLayer).unwrap_or(1);
            if layer != 0 {
                continue;
            }

            let pid = match cf_dict_number(&window, kCGWindowOwnerPID) {
                Some(p) => p as i32,
                None => continue,
            };

            if pid == own_pid {
                continue;
            }

            let owner_name = match cf_dict_string(&window, kCGWindowOwnerName) {
                Some(n) if !n.is_empty() => n,
                _ => continue,
            };

            if EXCLUDED_APPS.contains(&owner_name.as_str()) {
                continue;
            }

            let id = match cf_dict_number(&window, kCGWindowNumber) {
                Some(n) => n as u32,
                None => continue,
            };

            let bundle_id = bundle_id_for_pid(pid);

            results.push(CgWindowRaw {
                id,
                owner_name,
                pid,
                bundle_id,
            });
        }
    }

    // One entry per PID -- AX enrichment handles per-window titles
    let mut seen = std::collections::HashSet::new();
    results.retain(|w| seen.insert(w.pid));
    results
}

fn get_ax_titles(pids: &[i32]) -> HashMap<i32, Vec<String>> {
    let mut map = HashMap::new();
    for &pid in pids {
        let titles = ax_titles_for_pid(pid);
        if !titles.is_empty() {
            map.insert(pid, titles);
        }
    }
    map
}

fn ax_titles_for_pid(pid: i32) -> Vec<String> {
    let mut titles = Vec::new();

    unsafe {
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            return titles;
        }

        let mut windows_ref: CFTypeRef = std::ptr::null();
        let attr = CFString::new(kAXWindowsAttribute);
        let result =
            AXUIElementCopyAttributeValue(app_element, attr.as_concrete_TypeRef(), &mut windows_ref);

        if result != 0 || windows_ref.is_null() {
            return titles;
        }

        let windows: CFArray<CFType> = CFArray::wrap_under_get_rule(windows_ref as CFArrayRef);

        for window in windows.iter() {
            let mut title_ref: CFTypeRef = std::ptr::null();
            let title_attr = CFString::new(kAXTitleAttribute);
            let r = AXUIElementCopyAttributeValue(
                window.as_concrete_TypeRef() as _,
                title_attr.as_concrete_TypeRef(),
                &mut title_ref,
            );

            if r == 0 && !title_ref.is_null() {
                let title: CFString = CFString::wrap_under_get_rule(title_ref as CFStringRef);
                let s = title.to_string();
                if !s.is_empty() {
                    titles.push(s);
                }
            }
        }
    }

    titles
}

fn bundle_id_for_pid(pid: i32) -> Option<String> {
    unsafe {
        let cls = objc::runtime::Class::get("NSRunningApplication")?;
        let app: id = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
        if app == nil {
            return None;
        }
        let bundle_id: id = msg_send![app, bundleIdentifier];
        if bundle_id == nil {
            return None;
        }
        let s: *const std::os::raw::c_char = msg_send![bundle_id, UTF8String];
        if s.is_null() {
            return None;
        }
        Some(std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned())
    }
}

/// Look up a number value in a CG window info dictionary.
/// The CG constants are `*const __CFString` (= CFStringRef) -- wrap without retaining.
fn cf_dict_number(dict: &CFDictionary<CFString, CFType>, key: CFStringRef) -> Option<i64> {
    unsafe {
        let cf_key: CFString = CFString::wrap_under_get_rule(key);
        dict.find(&cf_key)
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
    }
}

fn cf_dict_string(dict: &CFDictionary<CFString, CFType>, key: CFStringRef) -> Option<String> {
    unsafe {
        let cf_key: CFString = CFString::wrap_under_get_rule(key);
        dict.find(&cf_key)
            .and_then(|v| v.downcast::<CFString>())
            .map(|s| s.to_string())
    }
}
