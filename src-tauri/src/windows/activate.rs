use accessibility_sys::{
    kAXMinimizedAttribute, kAXRaiseAction, AXUIElementCreateApplication,
    AXUIElementPerformAction, AXUIElementSetAttributeValue, AXUIElementCopyAttributeValue,
    kAXWindowsAttribute,
};
use cocoa::base::{id, nil};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    string::CFString,
};
use objc::{msg_send, sel, sel_impl};

pub fn activate(window_id: u32, app_pid: i32) -> Result<(), String> {
    unsafe {
        // Step 1: Activate the app (brings it to foreground, handles Spaces)
        activate_app(app_pid)?;

        // Step 2: Find and raise the specific AX window
        raise_window(app_pid, window_id)?;
    }
    Ok(())
}

unsafe fn activate_app(pid: i32) -> Result<(), String> {
    let cls =
        objc::runtime::Class::get("NSRunningApplication").ok_or("NSRunningApplication not found")?;
    let app: id = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
    if app == nil {
        return Err(format!("No running app for pid {}", pid));
    }
    // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
    let _: bool = msg_send![app, activateWithOptions: 2u64];
    Ok(())
}

unsafe fn raise_window(pid: i32, _window_id: u32) -> Result<(), String> {
    let app_element = AXUIElementCreateApplication(pid);
    if app_element.is_null() {
        return Err(format!("Could not create AX element for pid {}", pid));
    }

    let mut windows_ref: CFTypeRef = std::ptr::null();
    let attr = CFString::new(kAXWindowsAttribute);
    let result =
        AXUIElementCopyAttributeValue(app_element, attr.as_concrete_TypeRef(), &mut windows_ref);

    if result != 0 || windows_ref.is_null() {
        return Ok(()); // App has no AX windows -- app activation is enough
    }

    let windows: CFArray<CFType> = CFArray::wrap_under_get_rule(windows_ref as CFArrayRef);

    // Raise the first (frontmost) window -- for MVP we raise the first window.
    // Phase 4 enumeration returns one entry per PID, so this is correct.
    if let Some(window) = windows.iter().next() {
        let window_elem = window.as_concrete_TypeRef();

        // Un-minimize if needed
        let minimized_key = CFString::new(kAXMinimizedAttribute);
        let false_val = CFBoolean::false_value();
        AXUIElementSetAttributeValue(
            window_elem as _,
            minimized_key.as_concrete_TypeRef(),
            false_val.as_CFTypeRef(),
        );

        // Raise
        let raise_action = CFString::new(kAXRaiseAction);
        AXUIElementPerformAction(window_elem as _, raise_action.as_concrete_TypeRef());
    }

    Ok(())
}
