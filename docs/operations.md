# Operations — Prompt OS

> Part of the [Prompt OS PRD](./PRD.md). Read this when handling permissions, errors, security, deployment, or config.

---

## macOS Permissions & Entitlements

### Required Permissions
1. **Accessibility Access** — Text field detection + text insertion
2. **Input Monitoring** — Global keystroke detection via CGEvent tap

### Entitlements File

```xml
<!-- src-tauri/Entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.automation.apple-events</key>
    <true/>
    <key>com.apple.security.device.input-monitoring</key>
    <true/>
</dict>
</plist>
```

> **No sandbox**: Prompt OS requires Accessibility API and Input Monitoring, which are incompatible with the macOS App Sandbox. Distribute outside the Mac App Store via DMG.

### Permission Check (TypeScript)

The app should check on launch whether permissions are granted. If not, guide the user:

```typescript
// src/lib/permissions.ts
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";

export async function checkAccessibility(): Promise<boolean> {
  return invoke<boolean>("check_accessibility_permission");
}

export async function openAccessibilitySettings() {
  await open(
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
  );
}
```

---

## Error Handling

| Scenario | Handling |
|----------|---------|
| **No focused text field** | Toast: "No text field detected. Click in a text field and try again." |
| **Accessibility denied** | Show setup guide with link to System Settings |
| **Gemini API rate limit** | Exponential backoff (1s, 2s, 4s), then show error |
| **Network timeout** | Retry once, then show "Connection failed" |
| **Invalid API key** | Prompt user to enter valid key in Settings |
| **Token limit exceeded** | Show upgrade prompt |
| **Text insertion fails** | Auto-fallback to Cmd+V paste method |
| **Stream interrupted** | Display partial response with Retry + Insert buttons |

### Error Handling in TypeScript

```typescript
// src/lib/gemini.ts (error handling addition)
export class GeminiError extends Error {
  constructor(
    message: string,
    public status?: number,
    public retryable: boolean = false
  ) {
    super(message);
    this.name = "GeminiError";
  }
}

// In streamGemini():
if (res.status === 429) {
  throw new GeminiError("Rate limited. Please wait.", 429, true);
}
if (res.status === 401 || res.status === 403) {
  throw new GeminiError("Invalid API key. Update it in Settings.", res.status);
}
```

### Known App Compatibility Issues

| App | Issue | Solution |
|-----|-------|----------|
| Password fields | Blocked by macOS | Skip (intentional) |
| Electron apps (Slack, VS Code) | Text range bugs | Paste fallback |
| Terminal | Limited Accessibility | Paste fallback |
| `contenteditable` in browsers | AX attribute inconsistency | Paste fallback |

---

## Security

### API Key Storage
- ✅ Stored in macOS Keychain via Rust `security-framework` (see [features.md](./features.md))
- ❌ **Never** use `localStorage`, `sessionStorage`, or Tauri's filesystem for API keys
- ❌ **Never** store API key in environment variables that ship with the bundle

### Supabase Auth
- Auth tokens managed by `@supabase/supabase-js` in-memory + localStorage
- The anon key is a publishable key — safe to include in frontend code
- All data access is protected by RLS policies (see [backend.md](./backend.md))

### Privacy
- ✅ Prompts sent directly to Gemini API (no middle servers)
- ✅ User data protected by Supabase RLS
- ✅ No prompt content logged server-side (only token counts)

---

## Settings UI (React)

```tsx
// src/components/SettingsView.tsx
import { useState, useEffect } from "react";
import { storeApiKey, retrieveApiKey, deleteApiKey } from "../lib/commands";
import { useSupabase } from "../hooks/useSupabase";

export function SettingsView() {
  const [activeTab, setActiveTab] = useState<"general" | "account">("general");

  return (
    <div className="settings-container">
      <div className="settings-tabs">
        <button
          className={activeTab === "general" ? "active" : ""}
          onClick={() => setActiveTab("general")}
        >
          General
        </button>
        <button
          className={activeTab === "account" ? "active" : ""}
          onClick={() => setActiveTab("account")}
        >
          Account
        </button>
      </div>

      {activeTab === "general" && <GeneralSettings />}
      {activeTab === "account" && <AccountSettings />}
    </div>
  );
}

function GeneralSettings() {
  const [apiKey, setApiKey] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    retrieveApiKey().then((key) => {
      if (key) setApiKey(key);
    });
  }, []);

  const handleSave = async () => {
    await storeApiKey(apiKey);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="settings-section">
      <h3>Gemini API Key</h3>
      <p className="settings-hint">
        Get your API key from{" "}
        <a href="https://aistudio.google.com" target="_blank">
          aistudio.google.com
        </a>
      </p>
      <div className="settings-row">
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="Enter your Gemini API key"
        />
        <button onClick={handleSave}>
          {saved ? "Saved ✓" : "Save"}
        </button>
      </div>
    </div>
  );
}

function AccountSettings() {
  const { user, signIn, signUp, signOut, getRemainingTokens } = useSupabase();
  const [tokens, setTokens] = useState(0);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (user) {
      getRemainingTokens().then(setTokens);
    }
  }, [user]);

  if (!user) {
    return (
      <div className="settings-section">
        <h3>Sign In</h3>
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="Email"
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
        />
        <div className="btn-group">
          <button onClick={() => signIn(email, password)}>Sign In</button>
          <button onClick={() => signUp(email, password)}>Sign Up</button>
        </div>
      </div>
    );
  }

  return (
    <div className="settings-section">
      <h3>Account</h3>
      <p>Signed in as: {user.email}</p>
      <div className="token-display">
        <span className="token-count">{tokens.toLocaleString()}</span>
        <span className="token-label">tokens remaining</span>
      </div>
      <button onClick={signOut}>Sign Out</button>
    </div>
  );
}
```

---

## Deployment & Distribution

### Build for macOS

```bash
# Development
npm run tauri dev

# Production build (DMG + .app)
npm run tauri build
```

### Code Signing & Notarization

```bash
# Set environment variables before building
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_ID="your-email@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="YOUR_TEAM_ID"

# Build with signing + notarization
npm run tauri build
```

Tauri automatically:
1. Code signs with `APPLE_SIGNING_IDENTITY`
2. Submits to Apple's notary service
3. Staples the notarization ticket to the DMG

### Bundle Configuration

Configured in `tauri.conf.json` (see [architecture.md](./architecture.md)):
- `bundle.macOS.entitlements` → points to `Entitlements.plist`
- `bundle.macOS.signingIdentity` → or env var `APPLE_SIGNING_IDENTITY`
- `bundle.macOS.minimumSystemVersion` → `13.0`
- `bundle.targets` → `["dmg", "app"]`

---

## Implementation Checklist

### Phase 1: Project Setup (Day 1)
- [ ] Init Tauri v2 project: `npm create tauri-app@latest`
- [ ] Configure `tauri.conf.json` (overlay + settings windows, tray)
- [ ] Add npm dependencies (`@supabase/supabase-js`, `@tauri-apps/api`)
- [ ] Add Rust dependencies in `Cargo.toml`
- [ ] Create `Entitlements.plist`
- [ ] Set up file structure per architecture.md

### Phase 2: Rust Commands via Claude Web (Day 2)
- [ ] Get `keychain.rs` from Claude web → paste + test
- [ ] Get `text_field_detector.rs` from Claude web → paste + test
- [ ] Get `keystroke_monitor.rs` from Claude web → paste + test
- [ ] Get `text_injector.rs` from Claude web → paste + test
- [ ] Register all commands in `lib.rs`

### Phase 3: Frontend — Overlay (Day 3-4)
- [ ] Build `OverlayView.tsx` component
- [ ] Implement Gemini streaming in `gemini.ts`
- [ ] Create `commands.ts` typed invoke wrappers
- [ ] Wire up Tauri event listener for trigger
- [ ] Style overlay with glassmorphism CSS
- [ ] Test overlay positioning + animation

### Phase 4: Frontend — Settings & Auth (Day 5)
- [ ] Build `SettingsView.tsx` with API key + account tabs
- [ ] Set up Supabase client in `supabase.ts`
- [ ] Create `useSupabase.ts` hook
- [ ] Build `AuthView` (login/signup)
- [ ] Test auth flow end-to-end

### Phase 5: Backend Setup (Day 6)
- [ ] Create Supabase tables (schema from backend.md)
- [ ] Add RLS policies
- [ ] Create `consume_tokens` function
- [ ] Test usage tracking

### Phase 6: Integration & Polish (Day 7-8)
- [ ] End-to-end flow: trigger → overlay → submit → stream → insert
- [ ] Error handling for all edge cases
- [ ] Test across 10+ apps (Mail, Slack, Safari, Chrome, Notes, etc.)
- [ ] Keyboard shortcuts (Esc, Cmd+Enter)

### Phase 7: Distribution (Day 9)
- [ ] Code sign with Developer ID
- [ ] Notarize with Apple
- [ ] Create DMG
- [ ] Write README + installation guide

---

## Claude Web Prompts Index

Quick reference for all Rust components that need Claude web:

| Module | Prompt Location | Summary |
|--------|----------------|---------|
| `keystroke_monitor.rs` | [features.md § Keystroke Monitor](./features.md#1-keystroke-monitor) | CGEvent tap for "/" detection |
| `text_field_detector.rs` | [features.md § Text Field Detector](./features.md#2-text-field-detector) | AXUIElement focused field bounds |
| `text_injector.rs` | [features.md § Text Injector](./features.md#3-text-injector) | Insert text at cursor + paste fallback |
| `keychain.rs` | [features.md § Keychain Access](./features.md#4-keychain-access) | Store/retrieve/delete API key in Keychain |
