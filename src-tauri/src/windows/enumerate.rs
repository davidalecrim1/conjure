use super::types::WindowInfo;
use accessibility_sys::{
    kAXMinimizedAttribute, kAXTitleAttribute, kAXWindowsAttribute, AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication,
};
use base64::Engine;
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    dictionary::CFDictionary,
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use core_graphics::window::{
    kCGWindowLayer, kCGWindowListOptionOnScreenOnly, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID, CGWindowListCopyWindowInfo,
};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{
    NSApplicationActivationPolicy, NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication,
};
use objc2_foundation::{NSDictionary, NSSize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static ICON_CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const EXCLUDED_APPS: &[&str] = &[
    "Window Server",
    "Dock",
    "SystemUIServer",
    "Control Center",
    "Notification Center",
    "Spotlight",
];

pub fn list(include_minimized: bool) -> Vec<WindowInfo> {
    let own_pid = std::process::id() as i32;
    let cg_windows = get_cg_windows(own_pid);

    // PIDs already represented by on-screen CG windows
    let on_screen_pids: std::collections::HashSet<i32> =
        cg_windows.iter().map(|w| w.pid).collect();

    let ax_titles = get_ax_titles(on_screen_pids.iter().copied().collect::<Vec<_>>().as_slice());

    let mut pid_counters: HashMap<i32, usize> = HashMap::new();

    let mut results: Vec<WindowInfo> = cg_windows
        .into_iter()
        .map(|cg| {
            let idx = pid_counters.entry(cg.pid).or_insert(0);
            let title = ax_titles
                .get(&cg.pid)
                .and_then(|titles| titles.get(*idx))
                .cloned()
                .unwrap_or_default();
            *idx += 1;
            WindowInfo::new(cg.id, cg.owner_name, cg.pid, title, cg.bundle_id, false, cg.icon_data_url)
        })
        .collect();

    if include_minimized {
        let minimized = get_minimized_windows(own_pid, &on_screen_pids);
        results.extend(minimized);
    }

    results
}

struct CgWindowRaw {
    id: u32,
    owner_name: String,
    pid: i32,
    bundle_id: Option<String>,
    icon_data_url: Option<String>,
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
            let cache_key = bundle_id.clone().unwrap_or_else(|| owner_name.clone());
            let icon_data_url = cached_icon_for_pid(pid, &cache_key);

            results.push(CgWindowRaw {
                id,
                owner_name,
                pid,
                bundle_id,
                icon_data_url,
            });
        }
    }

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

/// Returns WindowInfo entries for minimized windows belonging to regular user apps
/// that are not already represented in `on_screen_pids`.
///
/// Uses CG with kCGWindowListOptionAll only to discover candidate PIDs, then
/// filters to .Regular activation policy apps via NSRunningApplication (thread-safe),
/// then queries AX for minimized windows. This avoids NSWorkspace (main-thread only).
fn get_minimized_windows(
    own_pid: i32,
    on_screen_pids: &std::collections::HashSet<i32>,
) -> Vec<WindowInfo> {
    // Collect candidate PIDs from the full CG window list (off-screen included).
    // We only use this for PID discovery — no WindowInfo is built from these entries.
    let candidate_pids: std::collections::HashSet<i32> = unsafe {
        let window_list = CGWindowListCopyWindowInfo(
            core_graphics::window::kCGWindowListOptionAll,
            0,
        );
        if window_list.is_null() {
            return Vec::new();
        }
        let array: CFArray<CFDictionary<CFString, CFType>> =
            CFArray::wrap_under_get_rule(window_list as CFArrayRef);

        array
            .iter()
            .filter_map(|w| {
                let layer = cf_dict_number(&w, kCGWindowLayer).unwrap_or(1);
                if layer != 0 {
                    return None;
                }
                let pid = cf_dict_number(&w, kCGWindowOwnerPID)? as i32;
                if pid == own_pid || on_screen_pids.contains(&pid) {
                    return None;
                }
                Some(pid)
            })
            .collect()
    };

    let mut results = Vec::new();

    for pid in candidate_pids {
        let ns_app = match NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
            Some(a) => a,
            None => continue,
        };

        // Skip agents, daemons, UIViewServices — only real user-facing apps
        if ns_app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let app_name = match ns_app.localizedName() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let bundle_id = ns_app.bundleIdentifier().map(|b| b.to_string());
        let cache_key = bundle_id.clone().unwrap_or_else(|| app_name.clone());
        let icon_data_url = cached_icon_for_pid(pid, &cache_key);

        unsafe {
            let app_element = AXUIElementCreateApplication(pid);
            if app_element.is_null() {
                continue;
            }

            let mut windows_ref: CFTypeRef = std::ptr::null();
            let attr = CFString::new(kAXWindowsAttribute);
            let r = AXUIElementCopyAttributeValue(
                app_element,
                attr.as_concrete_TypeRef(),
                &mut windows_ref,
            );
            if r != 0 || windows_ref.is_null() {
                continue;
            }

            let windows: CFArray<CFType> =
                CFArray::wrap_under_get_rule(windows_ref as CFArrayRef);

            for window in windows.iter() {
                let elem = window.as_concrete_TypeRef() as accessibility_sys::AXUIElementRef;

                let mut min_ref: CFTypeRef = std::ptr::null();
                let min_attr = CFString::new(kAXMinimizedAttribute);
                let r2 = AXUIElementCopyAttributeValue(
                    elem,
                    min_attr.as_concrete_TypeRef(),
                    &mut min_ref,
                );
                let is_minimized = r2 == 0
                    && !min_ref.is_null()
                    && CFBoolean::wrap_under_get_rule(min_ref as _) == CFBoolean::true_value();

                if !is_minimized {
                    continue;
                }

                let mut title_ref: CFTypeRef = std::ptr::null();
                let title_attr = CFString::new(kAXTitleAttribute);
                let r3 = AXUIElementCopyAttributeValue(
                    elem,
                    title_attr.as_concrete_TypeRef(),
                    &mut title_ref,
                );
                let title = if r3 == 0 && !title_ref.is_null() {
                    CFString::wrap_under_get_rule(title_ref as CFStringRef).to_string()
                } else {
                    String::new()
                };

                // Minimized windows have no CG window ID; use 0 as sentinel.
                // activate.rs ignores window_id and matches by title via AX.
                results.push(WindowInfo::new(
                    0,
                    app_name.clone(),
                    pid,
                    title,
                    bundle_id.clone(),
                    true,
                    icon_data_url.clone(),
                ));
            }
        }
    }

    results
}

fn icon_for_pid(pid: i32) -> Option<String> {
    unsafe {
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
        let ns_image: Retained<objc2_app_kit::NSImage> = app.icon()?;

        // Downsample to 32×32 before rasterizing to keep IPC payload small
        ns_image.setSize(NSSize::new(32.0, 32.0));

        let tiff_data = ns_image.TIFFRepresentation()?;
        let bitmap: Retained<NSBitmapImageRep> =
            NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &tiff_data)?;

        let props = NSDictionary::new();
        let png_data =
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(png_data.to_vec());
        Some(format!("data:image/png;base64,{}", encoded))
    }
}

fn cached_icon_for_pid(pid: i32, cache_key: &str) -> Option<String> {
    {
        let cache = ICON_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(cache_key) {
            return entry.clone();
        }
    }
    let result = icon_for_pid(pid);
    ICON_CACHE
        .lock()
        .unwrap()
        .insert(cache_key.to_string(), result.clone());
    result
}

fn bundle_id_for_pid(pid: i32) -> Option<String> {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
    let bundle_id = app.bundleIdentifier()?;
    Some(bundle_id.to_string())
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
