// Keychain access for storing Gemini API key
// Uses macOS Keychain via security-framework crate

use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

const SERVICE: &str = "com.promptos.gemini-api-key";
const ACCOUNT: &str = "default";

#[tauri::command]
pub fn store_api_key(key: String) -> Result<(), String> {
    // Delete existing entry first (ignore if not found)
    let _ = delete_generic_password(SERVICE, ACCOUNT);

    // Store the new key
    set_generic_password(SERVICE, ACCOUNT, key.as_bytes())
        .map_err(|e| format!("Failed to store API key: {}", e))
}

#[tauri::command]
pub fn retrieve_api_key() -> Result<Option<String>, String> {
    match get_generic_password(SERVICE, ACCOUNT) {
        Ok(password_bytes) => {
            let key = String::from_utf8(password_bytes)
                .map_err(|e| format!("Invalid UTF-8 in stored key: {}", e))?;
            Ok(Some(key))
        }
        Err(e) => {
            // Check if it's a "not found" error
            let error_string = e.to_string();
            if error_string.contains("not found") || error_string.contains("SecItemNotFound") || error_string.contains("-25300") {
                Ok(None)
            } else {
                Err(format!("Failed to retrieve API key: {}", e))
            }
        }
    }
}

#[tauri::command]
pub fn delete_api_key() -> Result<(), String> {
    match delete_generic_password(SERVICE, ACCOUNT) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Ignore "not found" errors
            let error_string = e.to_string();
            if error_string.contains("not found") || error_string.contains("SecItemNotFound") || error_string.contains("-25300") {
                Ok(())
            } else {
                Err(format!("Failed to delete API key: {}", e))
            }
        }
    }
}
