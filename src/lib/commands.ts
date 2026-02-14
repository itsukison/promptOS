import { invoke } from "@tauri-apps/api/core";

export interface TextFieldBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

// Text field detection
export async function getFocusedTextFieldBounds(): Promise<TextFieldBounds> {
  return invoke<TextFieldBounds>("get_focused_text_field_bounds");
}

// Text insertion
export async function insertText(text: string): Promise<void> {
  return invoke("insert_text", { text });
}

export async function insertTextViaPaste(text: string): Promise<void> {
  return invoke("insert_text_via_paste", { text });
}

// Keychain
export async function storeApiKey(key: string): Promise<void> {
  return invoke("store_api_key", { key });
}

export async function retrieveApiKey(): Promise<string | null> {
  return invoke<string | null>("retrieve_api_key");
}

export async function deleteApiKey(): Promise<void> {
  return invoke("delete_api_key");
}

// Keystroke monitoring
export async function startMonitoring(): Promise<void> {
  return invoke("start_monitoring");
}

export async function stopMonitoring(): Promise<void> {
  return invoke("stop_monitoring");
}

// Permission check
export async function checkAccessibilityPermission(): Promise<boolean> {
  return invoke<boolean>("check_accessibility_permission");
}
