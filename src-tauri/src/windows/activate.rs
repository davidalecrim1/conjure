use accessibility_sys::{
    kAXMinimizedAttribute, kAXRaiseAction, kAXWindowsAttribute, AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication, AXUIElementPerformAction, AXUIElementSetAttributeValue,
};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    string::CFString,
};
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

pub fn activate(window_id: u32, app_pid: i32) -> Result<(), String> {
    unsafe {
        activate_app(app_pid)?;
        raise_window(app_pid, window_id)?;
    }
    Ok(())
}

unsafe fn activate_app(pid: i32) -> Result<(), String> {
    let target = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .ok_or_else(|| format!("No running app for pid {}", pid))?;

    // Use the modern API: pass conjure itself as the sender app
    let own_pid = std::process::id() as i32;
    let sender = NSRunningApplication::runningApplicationWithProcessIdentifier(own_pid)
        .ok_or("Could not get own NSRunningApplication")?;

    target.activateFromApplication_options(&sender, NSApplicationActivationOptions(0));
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
        return Ok(());
    }

    let windows: CFArray<CFType> = CFArray::wrap_under_get_rule(windows_ref as CFArrayRef);

    if let Some(window) = windows.iter().next() {
        let window_elem = window.as_concrete_TypeRef();

        let minimized_key = CFString::new(kAXMinimizedAttribute);
        let false_val = CFBoolean::false_value();
        AXUIElementSetAttributeValue(
            window_elem as _,
            minimized_key.as_concrete_TypeRef(),
            false_val.as_CFTypeRef(),
        );

        let raise_action = CFString::new(kAXRaiseAction);
        AXUIElementPerformAction(window_elem as _, raise_action.as_concrete_TypeRef());
    }

    Ok(())
}
