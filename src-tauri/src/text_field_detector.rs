// Text field detection using macOS Accessibility API
// Simplified: just checks if a text field is focused and gets cursor position

use accessibility_sys::*;
use cocoa::base::{id, nil};
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::string::CFString;
use objc::msg_send;
use objc::sel;
use objc::sel_impl;
use std::ptr;

#[derive(serde::Serialize, Clone)]
pub struct TextFieldBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[tauri::command]
pub fn check_accessibility_permission() -> Result<bool, String> {
    unsafe { Ok(AXIsProcessTrusted()) }
}

/// Check if a text field is currently focused (simple boolean check)
pub fn is_text_field_focused() -> bool {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return false;
        }

        let focused_attr = CFString::new("AXFocusedUIElement");
        let mut focused_element_ref: CFTypeRef = ptr::null();

        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_attr.as_concrete_TypeRef(),
            &mut focused_element_ref,
        );

        let is_focused = result == 0 && !focused_element_ref.is_null();

        // Check if the focused element is a text input (has AXValue attribute)
        let is_text_input = if is_focused {
            let value_attr = CFString::new("AXValue");
            let mut value_ref: CFTypeRef = ptr::null();
            let value_result = AXUIElementCopyAttributeValue(
                focused_element_ref as AXUIElementRef,
                value_attr.as_concrete_TypeRef(),
                &mut value_ref,
            );
            let has_value = value_result == 0;
            cf_release(value_ref);
            has_value
        } else {
            false
        };

        cf_release(focused_element_ref);
        cf_release(system_wide as CFTypeRef);

        is_text_input
    }
}

/// Get current mouse cursor position - for overlay placement
#[tauri::command]
pub fn get_cursor_position() -> Result<TextFieldBounds, String> {
    unsafe {
        // Use objc to get NSEvent.mouseLocation
        let cls = objc::runtime::Class::get("NSEvent").ok_or("Failed to get NSEvent class")?;
        let point: CGPoint = msg_send![cls, mouseLocation];

        // Get main screen height for coordinate conversion (macOS uses bottom-left origin)
        let screen_cls = objc::runtime::Class::get("NSScreen").ok_or("Failed to get NSScreen class")?;
        let main_screen: id = msg_send![screen_cls, mainScreen];
        let frame: CGRect = msg_send![main_screen, frame];
        let screen_height = frame.size.height;

        // Convert from bottom-left to top-left coordinates
        let y = screen_height - point.y;

        eprintln!("[DEBUG] Cursor position: x={}, y={} (screen height={})", point.x, y, screen_height);

        Ok(TextFieldBounds {
            x: point.x,
            y,
            width: 0.0,  // Not applicable for cursor position
            height: 0.0,
        })
    }
}

/// Simplified: returns cursor position if a text field is focused
#[tauri::command]
pub fn get_focused_text_field_bounds() -> Result<TextFieldBounds, String> {
    if is_text_field_focused() {
        eprintln!("[DEBUG] Text field IS focused, getting cursor position...");
        get_cursor_position()
    } else {
        eprintln!("[DEBUG] No text field focused");
        Err("No focused text field".to_string())
    }
}

/// Safe CFRelease wrapper
unsafe fn cf_release(cf: CFTypeRef) {
    if !cf.is_null() {
        core_foundation::base::CFRelease(cf);
    }
}
