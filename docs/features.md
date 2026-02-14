# Features — Prompt OS

> Part of the [Prompt OS PRD](./PRD.md). Read this when implementing or modifying user-facing features.

---

## Core User Flow

1. **User types `/` in any text field** → Rust CGEvent tap detects it
2. **Rust gets text field bounds** → AXUIElement via Accessibility API
3. **Rust emits `trigger-detected` event** → React receives bounds via Tauri event
4. **React positions & shows overlay window** → Tauri window API
5. **User types prompt and submits** → TypeScript streams from Gemini API
6. **Response displayed in overlay** → React renders streaming text
7. **User clicks Insert** → TypeScript invokes Rust `insert_text` command
8. **Overlay closes** → Tauri hides window

---

## Overlay UI (React + TypeScript)

### OverlayView Component

```tsx
// src/components/OverlayView.tsx
import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { insertText, insertTextViaPaste } from "../lib/commands";
import { streamGemini } from "../lib/gemini";
import type { TextFieldBounds } from "../lib/commands";

export function OverlayView() {
  const [prompt, setPrompt] = useState("");
  const [response, setResponse] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    // Listen for trigger from Rust keystroke monitor
    const unlisten = listen<TextFieldBounds>("trigger-detected", async (event) => {
      const bounds = event.payload;
      const appWindow = getCurrentWindow();

      // Position overlay near text field
      const overlayHeight = 140;
      const padding = 8;
      const y = bounds.y + bounds.height + padding;

      await appWindow.setPosition({ x: bounds.x, y, type: "Physical" });
      await appWindow.show();
      await appWindow.setFocus();

      // Reset state
      setPrompt("");
      setResponse("");
      setIsGenerating(false);
      inputRef.current?.focus();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleSubmit = async () => {
    if (!prompt.trim() || isGenerating) return;
    setIsGenerating(true);
    setResponse("");

    abortRef.current = new AbortController();

    try {
      await streamGemini(
        prompt,
        (chunk) => setResponse((prev) => prev + chunk),
        abortRef.current.signal
      );
    } catch (err: any) {
      if (err.name !== "AbortError") {
        setResponse(`Error: ${err.message}`);
      }
    } finally {
      setIsGenerating(false);
    }
  };

  const handleInsert = async () => {
    try {
      await insertText(response);
    } catch {
      // Fallback to paste method
      await insertTextViaPaste(response);
    }
    await getCurrentWindow().hide();
  };

  const handleCancel = async () => {
    abortRef.current?.abort();
    await getCurrentWindow().hide();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
    if (e.key === "Escape") {
      handleCancel();
    }
  };

  return (
    <div className="overlay-container">
      <textarea
        ref={inputRef}
        className="overlay-input"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Ask AI anything..."
        rows={2}
      />

      {response && (
        <div className="overlay-response">
          {response}
        </div>
      )}

      <div className="overlay-actions">
        <button onClick={handleCancel} className="btn-cancel">
          Cancel
        </button>
        <div className="btn-group">
          {response && (
            <button
              onClick={() => navigator.clipboard.writeText(response)}
              className="btn-secondary"
            >
              Copy
            </button>
          )}
          <button
            onClick={handleInsert}
            disabled={!response || isGenerating}
            className="btn-primary"
          >
            {isGenerating ? "Generating..." : "Insert ✓"}
          </button>
        </div>
      </div>
    </div>
  );
}
```

### Overlay Styles

```css
/* src/components/OverlayView.css */
.overlay-container {
  display: flex;
  flex-direction: column;
  background: rgba(30, 30, 30, 0.85);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  overflow: hidden;
  color: #e0e0e0;
  font-family: -apple-system, BlinkMacSystemFont, sans-serif;
}

.overlay-input {
  background: transparent;
  border: none;
  color: #fff;
  padding: 12px 16px;
  font-size: 14px;
  resize: none;
  outline: none;
}

.overlay-response {
  padding: 12px 16px;
  font-size: 13px;
  max-height: 200px;
  overflow-y: auto;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  white-space: pre-wrap;
}

.overlay-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.btn-primary {
  background: #4A9EFF;
  color: white;
  border: none;
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-secondary, .btn-cancel {
  background: rgba(255, 255, 255, 0.08);
  color: #ccc;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.btn-group {
  display: flex;
  gap: 8px;
}
```

---

## Gemini API Streaming (TypeScript)

```typescript
// src/lib/gemini.ts
import { retrieveApiKey } from "./commands";

const BASE_URL =
  "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:streamGenerateContent";

export async function streamGemini(
  prompt: string,
  onChunk: (text: string) => void,
  signal?: AbortSignal,
  systemPrompt?: string
): Promise<void> {
  const apiKey = await retrieveApiKey();
  if (!apiKey) {
    throw new Error("No API key configured. Add it in Settings.");
  }

  const body: any = {
    contents: [{ parts: [{ text: prompt }] }],
  };

  if (systemPrompt) {
    body.systemInstruction = { parts: [{ text: systemPrompt }] };
  }

  const res = await fetch(`${BASE_URL}?key=${apiKey}&alt=sse`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });

  if (!res.ok) {
    const errorText = await res.text();
    throw new Error(`Gemini API error ${res.status}: ${errorText}`);
  }

  const reader = res.body?.getReader();
  if (!reader) throw new Error("No response body");

  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() || "";

    for (const line of lines) {
      if (line.startsWith("data: ")) {
        try {
          const json = JSON.parse(line.slice(6));
          const text = json?.candidates?.[0]?.content?.parts?.[0]?.text;
          if (text) onChunk(text);
        } catch {
          // Skip malformed SSE lines
        }
      }
    }
  }
}
```

---

## Rust Commands — Stubs + Claude Web Prompts

The following Rust modules need to be implemented. Each has a **ready-to-use prompt** for Claude web.

### 1. Keystroke Monitor

**Rust stub** (what I provide):
```rust
// src-tauri/src/keystroke_monitor.rs
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn start_monitoring(app: AppHandle) -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}

#[tauri::command]
pub async fn stop_monitoring() -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}
```

**Claude web prompt:**
> I'm building a Tauri v2 macOS app. I need a Rust module `keystroke_monitor.rs` that:
>
> 1. Uses `core-graphics` crate to create a `CGEventTap` that monitors all keyDown events
> 2. When the "/" key (keycode 0x2C) is pressed, it should:
>    a. Suppress the keystroke (return `None` from the callback)
>    b. Call `text_field_detector::get_focused_text_field_bounds()` to get the focused text field position
>    c. Emit a Tauri event called `"trigger-detected"` with the bounds as payload: `{ x: f64, y: f64, width: f64, height: f64 }`
> 3. Expose two Tauri commands: `start_monitoring(app: AppHandle)` and `stop_monitoring()`
> 4. Use a static `Mutex<Option<CFMachPort>>` to store the event tap for cleanup
> 5. The event tap must run on a background thread with its own `CFRunLoop`
>
> Dependencies: `core-graphics = "0.24"`, `core-foundation = "0.10"`, `tauri = "2"`
>
> The `TextFieldBounds` struct should derive `Serialize` and `Clone`:
> ```rust
> #[derive(serde::Serialize, Clone)]
> pub struct TextFieldBounds { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
> ```

---

### 2. Text Field Detector

**Rust stub:**
```rust
// src-tauri/src/text_field_detector.rs
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct TextFieldBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[tauri::command]
pub fn get_focused_text_field_bounds() -> Result<TextFieldBounds, String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}
```

**Claude web prompt:**
> I need a Rust function for a Tauri v2 app that uses macOS Accessibility API to detect the currently focused text field and return its screen bounds.
>
> Function signature:
> ```rust
> #[tauri::command]
> pub fn get_focused_text_field_bounds() -> Result<TextFieldBounds, String>
> ```
>
> Where `TextFieldBounds` is `{ x: f64, y: f64, width: f64, height: f64 }` (derive Serialize, Clone).
>
> It should:
> 1. Use `accessibility-sys` crate for AXUIElement bindings
> 2. Create a system-wide AXUIElement via `AXUIElementCreateSystemWide()`
> 3. Get the focused element via `kAXFocusedUIElementAttribute`
> 4. Get the selected text range via `kAXSelectedTextRangeAttribute`
> 5. Get the bounds for that range via `kAXBoundsForRangeParameterizedAttribute`
> 6. Convert the AXValue CGRect to the TextFieldBounds struct
> 7. Handle multi-screen: the bounds should be in global screen coordinates
> 8. Return `Err("No focused text field")` if any step fails
>
> Dependencies: `accessibility-sys = "0.1"`, `core-foundation = "0.10"`
>
> Include all necessary `use` statements and `unsafe` blocks with proper error handling.

---

### 3. Text Injector

**Rust stub:**
```rust
// src-tauri/src/text_injector.rs

#[tauri::command]
pub fn insert_text(text: String) -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}

#[tauri::command]
pub fn insert_text_via_paste(text: String) -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}
```

**Claude web prompt:**
> I need two Rust functions for a Tauri v2 macOS app that insert text into the currently focused text field.
>
> **Function 1: `insert_text(text: String) -> Result<(), String>`**
> - Use `accessibility-sys` to get the focused element via `AXUIElementCreateSystemWide()` + `kAXFocusedUIElementAttribute`
> - Set `kAXSelectedTextAttribute` (NOT `kAXValueAttribute` — that overwrites the entire field)
> - This inserts text at the current cursor position / replaces current selection
>
> **Function 2: `insert_text_via_paste(text: String) -> Result<(), String>`**
> - Save current clipboard contents
> - Put `text` on the clipboard via `NSPasteboard` (use `objc` crate or `cocoa` crate)
> - Simulate Cmd+V using `CGEvent` (keycode 0x09 with Command flag)
> - After 500ms, restore the original clipboard contents
>
> Both should be annotated with `#[tauri::command]`.
>
> Dependencies: `accessibility-sys = "0.1"`, `core-graphics = "0.24"`, `core-foundation = "0.10"`, `cocoa = "0.26"`, `objc = "0.2"`

---

### 4. Keychain Access

**Rust stub:**
```rust
// src-tauri/src/keychain.rs

#[tauri::command]
pub fn store_api_key(key: String) -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}

#[tauri::command]
pub fn retrieve_api_key() -> Result<Option<String>, String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}

#[tauri::command]
pub fn delete_api_key() -> Result<(), String> {
    // IMPLEMENT: See Claude web prompt below
    todo!()
}
```

**Claude web prompt:**
> I need three Rust functions for a Tauri v2 app that store, retrieve, and delete a Gemini API key in the macOS Keychain.
>
> Use the `security-framework` crate (version 3.x). The service name should be `"com.promptos.gemini-api-key"` and the account should be `"default"`.
>
> ```rust
> #[tauri::command]
> pub fn store_api_key(key: String) -> Result<(), String>
>
> #[tauri::command]
> pub fn retrieve_api_key() -> Result<Option<String>, String>
>
> #[tauri::command]
> pub fn delete_api_key() -> Result<(), String>
> ```
>
> - `store_api_key`: Delete existing entry first (ignore NotFound), then add new generic password
> - `retrieve_api_key`: Return `Ok(None)` if not found, `Ok(Some(key))` if found, `Err` on other errors
> - `delete_api_key`: Delete the entry, ignore NotFound error
>
> Use `security_framework::passwords::{set_generic_password, get_generic_password, delete_generic_password}`.
>
> Dependencies: `security-framework = "3"`
