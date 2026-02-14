// Keystroke monitoring using macOS CGEvent tap
// Detects "/" key press and emits trigger-detected event

use core_foundation::base::TCFType;
use core_foundation::mach_port::CFMachPort;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use std::ffi::c_void;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
#[cfg(target_os = "macos")]
use cocoa::base::id;
#[cfg(target_os = "macos")]
use objc::msg_send;
#[cfg(target_os = "macos")]
use objc::sel;
#[cfg(target_os = "macos")]
use objc::sel_impl;

// FFI declarations for CGEvent APIs
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        eventsOfInterest: u64,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
}

type CFMachPortRef = *mut c_void;
type CGEventRef = *mut c_void;
type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

// Event type constants
const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_TAP_LOCATION_HID: u32 = 0;
const K_CG_EVENT_TAP_HEAD_INSERT: u32 = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

// "/" key virtual keycode on macOS
const VK_SLASH: i64 = 0x2C;

// CGPoint for mouse position
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Copy, Clone)]
struct NSPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Copy, Clone)]
struct CGRect {
    origin: NSPoint,
    size: NSPoint, // Using NSPoint for size (width, height)
}

// Static storage - using raw pointer for thread safety
static EVENT_TAP_REF: Mutex<Option<usize>> = Mutex::new(None);
static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

unsafe extern "C" fn event_tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    if event_type == K_CG_EVENT_KEY_DOWN {
        let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
        eprintln!("[DEBUG] Key pressed: keycode={}", keycode);

        if keycode == VK_SLASH {
            eprintln!("[DEBUG] SLASH detected! Checking for text field...");

            // Get focused text field bounds
            match crate::text_field_detector::get_focused_text_field_bounds() {
                Ok(bounds) => {
                    eprintln!("[DEBUG] Text field bounds: x={}, y={}, w={}, h={}",
                        bounds.x, bounds.y, bounds.width, bounds.height);

                    // Emit Tauri event with bounds and show overlay
                    if let Ok(guard) = APP_HANDLE.lock() {
                        if let Some(app) = guard.as_ref() {
                            eprintln!("[DEBUG] Emitting trigger-detected event");
                            let _ = app.emit("trigger-detected", bounds);

                            // Show and focus the overlay window
                            if let Some(window) = app.get_webview_window("overlay") {
                                eprintln!("[DEBUG] Showing overlay window");
                                
                                // Position at cursor first
                                #[cfg(target_os = "macos")]
                                {
                                    let cls = objc::runtime::Class::get("NSEvent").unwrap();
                                    let mouse_loc: NSPoint = msg_send![cls, mouseLocation];
                                    
                                    let screen_cls = objc::runtime::Class::get("NSScreen").unwrap();
                                    let main_screen: id = msg_send![screen_cls, mainScreen];
                                    let frame: CGRect = msg_send![main_screen, frame];
                                    let screen_height = frame.size.y;
                                    
                                    let y = screen_height - mouse_loc.y;
                                    eprintln!("[DEBUG] Positioning at: x={}, y={}", mouse_loc.x, y);
                                    
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: mouse_loc.x as i32,
                                            y: y as i32,
                                        }
                                    ));
                                }
                                
                                let _ = window.show();
                                let _ = window.set_focus();
                                
                                eprintln!("[DEBUG] Window shown, is_visible: {:?}", window.is_visible());
                            }

                            // Return null to suppress the "/" keystroke
                            return std::ptr::null_mut();
                        } else {
                            eprintln!("[DEBUG] App handle is None");
                        }
                    } else {
                        eprintln!("[DEBUG] Failed to lock APP_HANDLE");
                    }
                }
                Err(e) => {
                    eprintln!("[DEBUG] No text field found: {}", e);
                }
            }
        }
    }

    // Pass through all other events
    event
}

pub fn start_monitoring(app: AppHandle) -> Result<(), String> {
    eprintln!("[DEBUG] start_monitoring called");

    // Store app handle for event emission
    *APP_HANDLE.lock().unwrap() = Some(app);

    // Spawn background thread for event monitoring
    std::thread::spawn(|| {
        unsafe {
            eprintln!("[DEBUG] Event monitor thread started");

            // Event mask for keyDown events
            let event_mask: u64 = 1 << K_CG_EVENT_KEY_DOWN;

            // Create the event tap
            let tap = CGEventTapCreate(
                K_CG_EVENT_TAP_LOCATION_HID,
                K_CG_EVENT_TAP_HEAD_INSERT,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                event_tap_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                eprintln!("[ERROR] Failed to create event tap - NO ACCESSIBILITY PERMISSION!");
                eprintln!("[ERROR] Grant Accessibility permission to: target/debug/prompt-os");
                return;
            }

            eprintln!("[DEBUG] Event tap created successfully!");

            // Store for cleanup (as usize for thread safety)
            *EVENT_TAP_REF.lock().unwrap() = Some(tap as usize);

            // Wrap in CFMachPort to create run loop source
            let mach_port = CFMachPort::wrap_under_create_rule(tap as *mut _);

            // Create run loop source
            let run_loop_source = mach_port
                .create_runloop_source(0)
                .expect("Failed to create run loop source");

            // Add to current run loop
            let run_loop = CFRunLoop::get_current();
            run_loop.add_source(&run_loop_source, kCFRunLoopCommonModes);

            // Enable the tap
            CGEventTapEnable(tap, true);
            eprintln!("[DEBUG] Event tap enabled, entering run loop...");

            // Run the event loop (blocks this thread)
            CFRunLoop::run_current();
        }
    });

    Ok(())
}

#[tauri::command]
pub fn start_monitoring_command(app: AppHandle) -> Result<(), String> {
    start_monitoring(app)
}

#[tauri::command]
pub fn stop_monitoring() -> Result<(), String> {
    eprintln!("[DEBUG] stop_monitoring called");

    // Stop the event tap
    if let Ok(mut guard) = EVENT_TAP_REF.lock() {
        if let Some(tap_addr) = guard.take() {
            unsafe {
                let tap = tap_addr as CFMachPortRef;
                CGEventTapEnable(tap, false);
            }
        }
    }

    // Clear app handle
    *APP_HANDLE.lock().unwrap() = None;

    Ok(())
}
