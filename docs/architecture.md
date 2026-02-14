# Architecture — Prompt OS

> Part of the [Prompt OS PRD](./PRD.md). Read this when setting up the project or understanding the Tauri ↔ React ↔ Rust boundary.

---

## Technology Stack

### Frontend (React + TypeScript)
- **Framework**: React 18+ with TypeScript
- **Build tool**: Vite
- **Styling**: CSS (vanilla) with glassmorphism effects
- **State management**: React hooks + context

### Backend (Rust via Tauri)
- **Runtime**: Tauri v2
- **Language**: Rust (2021 edition)
- **macOS minimum**: 13.0 (Ventura)

### Rust Crate Dependencies

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# macOS-specific (provided by Claude web)
accessibility-sys = "0.1"          # AXUIElement bindings
core-graphics = "0.24"             # CGEvent tap
core-foundation = "0.10"           # CFString, CFRunLoop
security-framework = "3"           # Keychain access
```

### Frontend Dependencies

```json
// package.json (partial)
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "@supabase/supabase-js": "^2.0.0",
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  }
}
```

---

## System Architecture Diagram

```
┌─────────────────────────────────────────────┐
│          macOS Application Layer            │
│  (Mail, Slack, Browsers, Notes, etc.)      │
└─────────────────┬───────────────────────────┘
                  │
                  │ User types "/" in text field
                  ↓
┌─────────────────────────────────────────────┐
│          Prompt OS (Tauri App)              │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │  RUST BACKEND (src-tauri/)           │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ keystroke_monitor.rs           │  │  │
│  │  │ - CGEvent tap detects "/"      │  │  │
│  │  │ - Emits event to frontend      │  │  │
│  │  └──────────────┬─────────────────┘  │  │
│  │                 │                     │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ text_field_detector.rs         │  │  │
│  │  │ - AXUIElement focused field    │  │  │
│  │  │ - Returns bounds (x,y,w,h)     │  │  │
│  │  └──────────────┬─────────────────┘  │  │
│  │                 │                     │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ text_injector.rs               │  │  │
│  │  │ - Insert via AXSelectedText    │  │  │
│  │  │ - Fallback: clipboard paste    │  │  │
│  │  └────────────────────────────────┘  │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ keychain.rs                    │  │  │
│  │  │ - Store/retrieve API key       │  │  │
│  │  └────────────────────────────────┘  │  │
│  └──────────────────────────────────────┘  │
│                    ↕ IPC (invoke / events)  │
│  ┌──────────────────────────────────────┐  │
│  │  REACT FRONTEND (src/)              │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ OverlayWindow (React)          │  │  │
│  │  │ - Input bar + streaming view   │  │  │
│  │  │ - Calls Gemini API (fetch)     │  │  │
│  │  └────────────────────────────────┘  │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ SettingsWindow (React)         │  │  │
│  │  │ - API key config → Keychain    │  │  │
│  │  │ - Account & usage stats        │  │  │
│  │  └────────────────────────────────┘  │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │ Supabase Client (TS)           │  │  │
│  │  │ - Auth, usage tracking         │  │  │
│  │  └────────────────────────────────┘  │  │
│  └──────────────────────────────────────┘  │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│            Backend (Supabase)               │
│  - Auth: User authentication                │
│  - Database: Usage tracking & profiles      │
└─────────────────────────────────────────────┘
```

---

## File Organization

```
prompt-os/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── index.html
│
├── src/                          # React frontend
│   ├── main.tsx                  # React entry point
│   ├── App.tsx                   # Root component + routing
│   ├── App.css                   # Global styles
│   ├── components/
│   │   ├── OverlayView.tsx       # Floating prompt input + response
│   │   ├── SettingsView.tsx      # Settings tabs
│   │   ├── AuthView.tsx          # Login/signup
│   │   └── AccountView.tsx       # Usage stats
│   ├── hooks/
│   │   ├── useGemini.ts          # Gemini streaming hook
│   │   └── useSupabase.ts        # Auth + DB hook
│   ├── lib/
│   │   ├── supabase.ts           # Supabase client init
│   │   ├── gemini.ts             # Gemini API service
│   │   └── commands.ts           # Typed Tauri invoke wrappers
│   └── types/
│       └── index.ts              # Shared TypeScript types
│
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri config (windows, tray, bundle)
│   ├── capabilities/
│   │   └── default.json          # IPC permissions
│   ├── Entitlements.plist        # macOS entitlements
│   ├── src/
│   │   ├── main.rs               # Desktop entry point
│   │   ├── lib.rs                # Tauri app setup + command registration
│   │   ├── keystroke_monitor.rs  # CGEvent tap (Claude web)
│   │   ├── text_field_detector.rs # AXUIElement (Claude web)
│   │   ├── text_injector.rs      # Text insertion (Claude web)
│   │   └── keychain.rs           # Keychain access (Claude web)
│   └── icons/                    # App icons
│       ├── icon.icns
│       └── icon.png
```

---

## Tauri Configuration

```jsonc
// src-tauri/tauri.conf.json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-config-schema/schema.json",
  "productName": "Prompt OS",
  "version": "0.1.0",
  "identifier": "com.promptos.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    },
    "windows": [
      {
        "label": "overlay",
        "title": "",
        "url": "/overlay",
        "width": 500,
        "height": 140,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "visible": false,
        "resizable": false,
        "skipTaskbar": true,
        "shadow": true
      },
      {
        "label": "settings",
        "title": "Prompt OS Settings",
        "url": "/settings",
        "width": 500,
        "height": 400,
        "visible": false,
        "center": true
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "macOS": {
      "entitlements": "./Entitlements.plist",
      "signingIdentity": null,
      "minimumSystemVersion": "13.0"
    }
  }
}
```

---

## IPC: Rust Commands ↔ TypeScript

### Command Registration (Rust)

```rust
// src-tauri/src/lib.rs
mod keystroke_monitor;
mod text_field_detector;
mod text_injector;
mod keychain;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            text_field_detector::get_focused_text_field_bounds,
            text_injector::insert_text,
            text_injector::insert_text_via_paste,
            keychain::store_api_key,
            keychain::retrieve_api_key,
            keychain::delete_api_key,
            keystroke_monitor::start_monitoring,
            keystroke_monitor::stop_monitoring,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Prompt OS");
}
```

### Typed Invoke Wrappers (TypeScript)

```typescript
// src/lib/commands.ts
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
```

### Event Listening (Trigger Detection)

```typescript
// In React component — listen for "/" trigger from Rust
import { listen } from "@tauri-apps/api/event";

// Rust emits "trigger-detected" event when "/" is pressed
const unlisten = await listen<TextFieldBounds>("trigger-detected", (event) => {
  const bounds = event.payload;
  showOverlay(bounds);
});
```

---

## Development Workflow

### Who Builds What

| Component | Language | Built by |
|-----------|----------|----------|
| Overlay UI | React/TS | Coding AI (me) |
| Settings UI | React/TS | Coding AI |
| Gemini streaming | TypeScript | Coding AI |
| Supabase client | TypeScript | Coding AI |
| `commands.ts` wrappers | TypeScript | Coding AI |
| `lib.rs` registration | Rust | Coding AI |
| `keystroke_monitor.rs` | Rust | Claude web (prompt provided) |
| `text_field_detector.rs` | Rust | Claude web (prompt provided) |
| `text_injector.rs` | Rust | Claude web (prompt provided) |
| `keychain.rs` | Rust | Claude web (prompt provided) |

See [features.md](./features.md) and [operations.md](./operations.md) for the specific Claude web prompts.
