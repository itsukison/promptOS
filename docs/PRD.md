# Product Requirements Document: Prompt OS

**macOS AI Writing Assistant — Tauri + React + Rust**

> **For AI Agents**: This is the main index. Read this first, then only read the document relevant to your current task.

| Document | What it covers | Read when... |
|----------|---------------|--------------|
| **[architecture.md](./architecture.md)** | Tauri project structure, Rust crates, IPC commands, tauri.conf.json | Setting up the project or understanding the Tauri ↔ React boundary |
| **[features.md](./features.md)** | Core user flow, React overlay UI, Gemini streaming, Rust command stubs + Claude web prompts | Implementing or modifying any user-facing feature |
| **[backend.md](./backend.md)** | Supabase schema, RLS policies, TypeScript client, Rust Keychain integration | Working on database, auth, or token management |
| **[operations.md](./operations.md)** | macOS entitlements, error handling, security, Tauri bundling, Claude web prompts index | Handling permissions, errors, deployment, or config |

---

## Executive Summary

Prompt OS is a native macOS menu bar application that enables users to invoke Gemini AI from any text input field system-wide using a customizable trigger (default: `/`). Built with **Tauri v2** (React/TypeScript frontend + Rust backend), the app provides a floating input overlay positioned near the active text field, streams AI responses, and inserts generated text directly into the original application.

### Hybrid Development Approach

This project uses a **split implementation** strategy:
- **React/TypeScript** (implemented by coding AI): All UI, state management, Gemini API streaming, Supabase integration — ~80% of the codebase
- **Rust** (implemented via Claude web prompts): macOS-specific low-level APIs — Accessibility, CGEvent, Keychain — ~20% of the codebase

Each Rust component has a ready-to-use prompt for Claude web in the relevant document.

---

## Product Overview

### Core Value Proposition
- **Universal AI Access**: Invoke Gemini from any macOS application (Mail, Slack, Notion, browsers, etc.)
- **Seamless Integration**: Non-intrusive floating UI that appears only when needed
- **Native Performance**: Rust backend for low-latency macOS integration, React for rapid UI iteration

### User Personas
- **Primary**: Knowledge workers who write frequently across multiple applications
- **Secondary**: Students, content creators, developers needing AI assistance while coding

---

## MVP Feature Set

### Must-Have (V1.0)
✅ Global "/" trigger detection (Rust CGEvent)
✅ Floating overlay positioned near text field (Tauri window + React)
✅ Gemini API integration with streaming (TypeScript)
✅ Text insertion via Accessibility API (Rust)
✅ Basic error handling & retry logic
✅ System tray app with Settings (Tauri + React)
✅ Supabase authentication (TypeScript)
✅ Usage tracking in Supabase (TypeScript)
✅ Token limit enforcement — server-side (Supabase SQL)

### Nice-to-Have (V1.1)
⭕ Customizable trigger key
⭕ Prompt history (stored locally via Tauri fs plugin)
⭕ Multi-turn conversations
⭕ Copy/Edit generated text before insertion
⭕ Keyboard shortcut alternative to "/"

### Future (V2.0)
⭕ Context awareness (read surrounding text via Accessibility)
⭕ Saved prompt templates
⭕ System prompt / persona configuration
⭕ Team accounts
⭕ Analytics dashboard

---

## Success Metrics & KPIs

### Technical Metrics
- **Activation latency**: <200ms from "/" to overlay appearance
- **API response time**: <1s to first token (streaming)
- **Insertion success rate**: >95% across common apps
- **App compatibility**: Works in >90% of text-input apps

### User Engagement
- **Daily active users**: Track via Supabase auth sessions
- **Prompts per user per day**: Log in `usage_logs`
- **Retention (Day 7)**: >60% of signups still active

---

## Open Questions

1. **Rate limiting**: Should we rate-limit requests client-side (e.g., max 30/min)?
2. **Offline mode**: Cache last N prompts for offline access?
3. **Multi-language**: Support non-English trigger keys?
4. **Context window**: Read ±500 chars around cursor for context?
5. **Update mechanism**: Use Tauri's built-in updater plugin?
6. **Model selection**: Allow switching between Gemini Flash vs. Pro?
