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
    kCGWindowLayer, kCGWindowListOptionAll, kCGWindowListOptionOnScreenOnly, kCGWindowNumber,
    kCGWindowOwnerName, kCGWindowOwnerPID, CGWindowListCopyWindowInfo,
};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication};
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

    // Single CG pass: fetch all windows (on-screen + off-screen) once.
    // When include_minimized is false we use the cheaper on-screen-only option.
    let cg_windows = get_cg_windows(own_pid, include_minimized);

    // Collect unique PIDs that have at least one on-screen window.
    let on_screen_pids: std::collections::HashSet<i32> = cg_windows
        .iter()
        .filter(|w| w.on_screen)
        .map(|w| w.pid)
        .collect();

    // Unique PIDs we need AX data for.
    let all_pids: Vec<i32> = cg_windows
        .iter()
        .map(|w| w.pid)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Fetch AX window entries (title + minimized) for all PIDs in parallel.
    let ax_data = get_ax_data_parallel(&all_pids);

    let mut pid_counters: HashMap<i32, usize> = HashMap::new();

    cg_windows
        .into_iter()
        .filter_map(|cg| {
            // Off-screen entries are only included when include_minimized is true.
            // For off-screen PIDs that are also present on-screen (e.g. an app with
            // both visible and minimized windows), skip the off-screen CG row —
            // the minimized window is surfaced via the AX index below.
            if !cg.on_screen && on_screen_pids.contains(&cg.pid) {
                return None;
            }

            let idx = pid_counters.entry(cg.pid).or_insert(0);
            let (title, is_minimized) = ax_data
                .get(&cg.pid)
                .and_then(|entries| entries.get(*idx))
                .cloned()
                .unwrap_or_default();
            *idx += 1;

            // For off-screen rows, only emit if the AX window is actually minimized.
            // This filters out background/invisible windows that appear in the CG list.
            if !cg.on_screen && !is_minimized {
                return None;
            }

            Some(WindowInfo::new(
                cg.id,
                cg.owner_name,
                cg.pid,
                title,
                cg.bundle_id,
                is_minimized,
                cg.icon_data_url,
            ))
        })
        .collect()
}

struct CgWindowRaw {
    id: u32,
    owner_name: String,
    pid: i32,
    bundle_id: Option<String>,
    icon_data_url: Option<String>,
    on_screen: bool,
}

fn get_cg_windows(own_pid: i32, include_minimized: bool) -> Vec<CgWindowRaw> {
    let mut results = Vec::new();
    // Per-call PID → (bundle_id, icon) cache to avoid redundant NSRunningApplication
    // lookups when the same app has multiple windows in the CG list.
    let mut pid_meta: HashMap<i32, (Option<String>, Option<String>)> = HashMap::new();

    unsafe {
        // When include_minimized, fetch the full window list. We also fetch the
        // on-screen-only list to build a reliable set of on-screen window IDs,
        // since the kCGWindowIsOnscreen dict key is not accessible via CFString lookup.
        let full_list = CGWindowListCopyWindowInfo(kCGWindowListOptionAll, 0);
        if full_list.is_null() {
            return results;
        }

        let on_screen_ids: std::collections::HashSet<u32> = if include_minimized {
            let on_screen_list = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0);
            if on_screen_list.is_null() {
                std::collections::HashSet::new()
            } else {
                let arr: CFArray<CFDictionary<CFString, CFType>> =
                    CFArray::wrap_under_get_rule(on_screen_list as CFArrayRef);
                arr.iter()
                    .filter_map(|w| {
                        let layer = cf_dict_number(&w, kCGWindowLayer).unwrap_or(1);
                        if layer != 0 { return None; }
                        cf_dict_number(&w, kCGWindowNumber).map(|n| n as u32)
                    })
                    .collect()
            }
        } else {
            std::collections::HashSet::new()
        };

        let array: CFArray<CFDictionary<CFString, CFType>> =
            CFArray::wrap_under_get_rule(full_list as CFArrayRef);

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

            let on_screen = if include_minimized {
                on_screen_ids.contains(&id)
            } else {
                true // kCGWindowListOptionOnScreenOnly — all results are on-screen
            };

            // Lookup bundle_id + icon once per unique PID
            let (bundle_id, icon_data_url) = pid_meta.entry(pid).or_insert_with(|| {
                let bundle_id = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                    .and_then(|app| app.bundleIdentifier().map(|b| b.to_string()));
                let cache_key = bundle_id.clone().unwrap_or_else(|| owner_name.clone());
                let icon = cached_icon_for_pid(pid, &cache_key);
                (bundle_id, icon)
            });

            results.push(CgWindowRaw {
                id,
                owner_name,
                pid,
                bundle_id: bundle_id.clone(),
                icon_data_url: icon_data_url.clone(),
                on_screen,
            });
        }
    }

    results
}

/// Fetch AX window entries (title, is_minimized) for all given PIDs in parallel.
fn get_ax_data_parallel(pids: &[i32]) -> HashMap<i32, Vec<(String, bool)>> {
    let mut map = HashMap::with_capacity(pids.len());
    std::thread::scope(|s| {
        let handles: Vec<_> = pids
            .iter()
            .map(|&pid| s.spawn(move || (pid, ax_entries_for_pid(pid))))
            .collect();
        for handle in handles {
            if let Ok((pid, entries)) = handle.join() {
                if !entries.is_empty() {
                    map.insert(pid, entries);
                }
            }
        }
    });
    map
}

fn ax_entries_for_pid(pid: i32) -> Vec<(String, bool)> {
    let mut entries = Vec::new();

    unsafe {
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            return entries;
        }

        let mut windows_ref: CFTypeRef = std::ptr::null();
        let attr = CFString::new(kAXWindowsAttribute);
        let result = AXUIElementCopyAttributeValue(
            app_element,
            attr.as_concrete_TypeRef(),
            &mut windows_ref,
        );

        if result != 0 || windows_ref.is_null() {
            return entries;
        }

        let windows: CFArray<CFType> = CFArray::wrap_under_get_rule(windows_ref as CFArrayRef);

        for window in windows.iter() {
            let elem = window.as_concrete_TypeRef() as accessibility_sys::AXUIElementRef;

            let mut title_ref: CFTypeRef = std::ptr::null();
            let title_attr = CFString::new(kAXTitleAttribute);
            let r = AXUIElementCopyAttributeValue(
                elem,
                title_attr.as_concrete_TypeRef(),
                &mut title_ref,
            );
            let title = if r == 0 && !title_ref.is_null() {
                let s: CFString = CFString::wrap_under_get_rule(title_ref as CFStringRef);
                s.to_string()
            } else {
                String::new()
            };

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

            if !title.is_empty() {
                entries.push((title, is_minimized));
            }
        }
    }

    entries
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

