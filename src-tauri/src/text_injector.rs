// Text injection using macOS Accessibility API and clipboard fallback
// Inserts AI-generated text into the focused text field

use accessibility_sys::*;
use cocoa::appkit::{NSPasteboard, NSPasteboardTypeString};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::msg_send;
use objc::sel;
use objc::sel_impl;
use std::ptr;
use std::thread;
use std::time::Duration;

#[tauri::command]
pub fn insert_text(text: String) -> Result<(), String> {
    unsafe {
        // 1. Create system-wide AXUIElement
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return Err("Failed to create system-wide AXUIElement".to_string());
        }

        // 2. Get focused element
        let focused_attr = CFString::new("AXFocusedUIElement");
        let mut focused_element_ref: CFTypeRef = ptr::null();

        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_attr.as_concrete_TypeRef(),
            &mut focused_element_ref,
        );

        if result != 0 || focused_element_ref.is_null() {
            cf_release(system_wide as CFTypeRef);
            return Err("No focused element found".to_string());
        }

        let focused_element = focused_element_ref as AXUIElementRef;

        // 3. Set the selected text attribute (inserts at cursor/replaces selection)
        let selected_text_attr = CFString::new("AXSelectedText");
        let text_value = CFString::new(&text);

        let set_result = AXUIElementSetAttributeValue(
            focused_element,
            selected_text_attr.as_concrete_TypeRef(),
            text_value.as_concrete_TypeRef() as CFTypeRef,
        );

        cf_release(focused_element as CFTypeRef);
        cf_release(system_wide as CFTypeRef);

        if set_result != 0 {
            return Err("Failed to insert text via Accessibility API".to_string());
        }

        Ok(())
    }
}

#[tauri::command]
pub fn insert_text_via_paste(text: String) -> Result<(), String> {
    unsafe {
        // 1. Get the general pasteboard
        let pasteboard: id = NSPasteboard::generalPasteboard(nil);

        // 2. Save current clipboard contents
        let saved_contents = get_clipboard_string(pasteboard);

        // 3. Clear and set new clipboard content
        let _: () = msg_send![pasteboard, clearContents];

        let ns_string = NSString::alloc(nil);
        let ns_string = NSString::init_str(ns_string, &text);

        let array = NSArray::arrayWithObject(nil, NSPasteboardTypeString);
        let _: bool = msg_send![pasteboard, declareTypes:array owner:nil];
        let success: bool = msg_send![pasteboard, setString:ns_string forType:NSPasteboardTypeString];

        if !success {
            return Err("Failed to set clipboard content".to_string());
        }

        // 4. Simulate Cmd+V
        let v_keycode: CGKeyCode = 0x09; // V key

        // Create event source
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source".to_string())?;

        // Key down event
        let key_down = CGEvent::new_keyboard_event(source.clone(), v_keycode, true)
            .map_err(|_| "Failed to create key down event".to_string())?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);

        // Key up event
        let key_up = CGEvent::new_keyboard_event(source, v_keycode, false)
            .map_err(|_| "Failed to create key up event".to_string())?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);

        // Post events
        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);


        // 5. Restore clipboard after delay
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));

            unsafe {
                let pasteboard: id = NSPasteboard::generalPasteboard(nil);
                let _: () = msg_send![pasteboard, clearContents];

                if let Some(original_content) = saved_contents {
                    let ns_string = NSString::alloc(nil);
                    let ns_string = NSString::init_str(ns_string, &original_content);

                    let array = NSArray::arrayWithObject(nil, NSPasteboardTypeString);
                    let _: bool = msg_send![pasteboard, declareTypes:array owner:nil];
                    let _: bool =
                        msg_send![pasteboard, setString:ns_string forType:NSPasteboardTypeString];
                }
            }
        });

        Ok(())
    }
}

/// Get string content from clipboard if available
unsafe fn get_clipboard_string(pasteboard: id) -> Option<String> {
    let types: id = msg_send![pasteboard, types];
    if types == nil {
        return None;
    }

    let string_type = NSPasteboardTypeString;
    let type_array = NSArray::arrayWithObject(nil, string_type);
    let available: id = msg_send![pasteboard, availableTypeFromArray: type_array];

    if available == nil {
        return None;
    }

    let ns_string: id = msg_send![pasteboard, stringForType: string_type];
    if ns_string == nil {
        return None;
    }

    let c_str: *const i8 = msg_send![ns_string, UTF8String];
    if c_str.is_null() {
        return None;
    }

    Some(
        std::ffi::CStr::from_ptr(c_str)
            .to_string_lossy()
            .into_owned(),
    )
}

/// Safe CFRelease wrapper
unsafe fn cf_release(cf: CFTypeRef) {
    if !cf.is_null() {
        core_foundation::base::CFRelease(cf);
    }
}
