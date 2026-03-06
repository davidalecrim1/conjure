/// Check accessibility permission and prompt the user if not yet granted.
/// Must be called at startup before any AX API usage.
pub fn check_and_request() {
    #[cfg(target_os = "macos")]
    unsafe {
        use accessibility_sys::{AXIsProcessTrusted, AXIsProcessTrustedWithOptions};
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        if AXIsProcessTrusted() {
            return;
        }

        // Show the system accessibility permission dialog
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let val = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), val.as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}
