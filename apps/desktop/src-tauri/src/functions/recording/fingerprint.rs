use super::state::RecordedFingerprint;

#[cfg(target_os = "macos")]
pub fn extract_fingerprint_at(x: i32, y: i32) -> Option<RecordedFingerprint> {
    use std::ffi::c_void;
    use std::ptr;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCopyElementAtPosition(
            application: *const c_void,
            x: f32,
            y: f32,
            element: *mut *const c_void,
        ) -> i32;
        fn AXUIElementCreateSystemWide() -> *const c_void;
        fn AXUIElementCopyAttributeValue(
            element: *const c_void,
            attribute: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
    }

    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            c_str: *const i8,
            encoding: u32,
        ) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut i8,
            buffer_size: isize,
            encoding: u32,
        ) -> bool;
        fn CFGetTypeID(cf: *const c_void) -> u64;
        fn CFStringGetTypeID() -> u64;
        fn CFRelease(cf: *const c_void);
    }

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

    unsafe fn create_cf_string(s: &str) -> *const c_void {
        let c_str = match std::ffi::CString::new(s) {
            Ok(c) => c,
            Err(_) => return ptr::null(),
        };
        unsafe { CFStringCreateWithCString(ptr::null(), c_str.as_ptr(), K_CF_STRING_ENCODING_UTF8) }
    }

    unsafe fn cf_string_to_string(cf: *const c_void) -> Option<String> {
        unsafe {
            if cf.is_null() || CFGetTypeID(cf) != CFStringGetTypeID() {
                return None;
            }
            let mut buffer = [0i8; 256];
            if CFStringGetCString(cf, buffer.as_mut_ptr(), 256, K_CF_STRING_ENCODING_UTF8) {
                std::ffi::CStr::from_ptr(buffer.as_ptr())
                    .to_str()
                    .ok()
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
    }

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        let mut element: *const c_void = ptr::null();
        let result =
            AXUIElementCopyElementAtPosition(system_wide, x as f32, y as f32, &mut element);

        if result != 0 || element.is_null() {
            return None;
        }

        let mut fp = RecordedFingerprint {
            id: flow_like_types::create_id(),
            role: None,
            name: None,
            text: None,
            bounding_box: None,
        };

        let role_attr = create_cf_string("AXRole");
        if !role_attr.is_null() {
            let mut role_value: *const c_void = ptr::null();
            if AXUIElementCopyAttributeValue(element, role_attr, &mut role_value) == 0
                && !role_value.is_null()
            {
                fp.role = cf_string_to_string(role_value);
                CFRelease(role_value);
            }
            CFRelease(role_attr);
        }

        let title_attr = create_cf_string("AXTitle");
        if !title_attr.is_null() {
            let mut title_value: *const c_void = ptr::null();
            if AXUIElementCopyAttributeValue(element, title_attr, &mut title_value) == 0
                && !title_value.is_null()
            {
                let title = cf_string_to_string(title_value);
                if title.as_ref().is_some_and(|t| !t.is_empty()) {
                    fp.name = title;
                }
                CFRelease(title_value);
            }
            CFRelease(title_attr);
        }

        if fp.name.is_none() {
            let desc_attr = create_cf_string("AXDescription");
            if !desc_attr.is_null() {
                let mut desc_value: *const c_void = ptr::null();
                if AXUIElementCopyAttributeValue(element, desc_attr, &mut desc_value) == 0
                    && !desc_value.is_null()
                {
                    let desc = cf_string_to_string(desc_value);
                    if desc.as_ref().is_some_and(|d| !d.is_empty()) {
                        fp.name = desc;
                    }
                    CFRelease(desc_value);
                }
                CFRelease(desc_attr);
            }
        }

        let value_attr = create_cf_string("AXValue");
        if !value_attr.is_null() {
            let mut value_value: *const c_void = ptr::null();
            if AXUIElementCopyAttributeValue(element, value_attr, &mut value_value) == 0
                && !value_value.is_null()
            {
                let value = cf_string_to_string(value_value);
                if value.as_ref().is_some_and(|v| !v.is_empty()) {
                    fp.text = value;
                }
                CFRelease(value_value);
            }
            CFRelease(value_attr);
        }

        Some(fp)
    }
}

#[cfg(target_os = "windows")]
pub fn extract_fingerprint_at(_x: i32, _y: i32) -> Option<RecordedFingerprint> {
    None
}

#[cfg(target_os = "linux")]
pub fn extract_fingerprint_at(_x: i32, _y: i32) -> Option<RecordedFingerprint> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn extract_fingerprint_at(_x: i32, _y: i32) -> Option<RecordedFingerprint> {
    None
}
