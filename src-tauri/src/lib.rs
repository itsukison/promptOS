mod keychain;
mod keystroke_monitor;
mod text_field_detector;
mod text_injector;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Create tray menu
            let quit = MenuItem::with_id(app, "quit", "Quit Prompt OS", true, None::<&str>)?;
            let settings =
                MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &quit])?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("settings") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // Start keystroke monitoring on launch
            let app_handle = app.handle().clone();
            eprintln!("[DEBUG] App setup complete, starting keystroke monitor...");
            std::thread::spawn(move || {
                eprintln!("[DEBUG] Keystroke monitor thread spawned");
                match keystroke_monitor::start_monitoring(app_handle) {
                    Ok(()) => eprintln!("[DEBUG] start_monitoring returned Ok"),
                    Err(e) => eprintln!("[ERROR] start_monitoring failed: {}", e),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            text_field_detector::get_focused_text_field_bounds,
            text_field_detector::check_accessibility_permission,
            text_field_detector::get_cursor_position,
            text_injector::insert_text,
            text_injector::insert_text_via_paste,
            keychain::store_api_key,
            keychain::retrieve_api_key,
            keychain::delete_api_key,
            keystroke_monitor::start_monitoring_command,
            keystroke_monitor::stop_monitoring,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Prompt OS");
}
