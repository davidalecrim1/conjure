# Conjure — Claude Code Instructions

## Project Overview

macOS window switcher built with Rust + Tauri 2. Keyboard-driven fuzzy search palette. Background app (menu bar only, no Dock icon).

## Architecture

```
src/                        Frontend (vanilla TypeScript + Vite)
  main.ts                   All UI logic: rendering, keyboard handling, Tauri IPC
  style.css                 Palette styles — dark, blurred, mouseless
src-tauri/src/
  lib.rs                    App entry: tray setup, hotkey registration, Tauri builder
  permissions.rs            AX permission check + system prompt on startup
  windows/
    mod.rs                  Tauri commands: list_windows, activate_window
    enumerate.rs            CGWindowList + AXUIElement title enrichment
    activate.rs             NSRunningApplication.activate + AXRaise
    types.rs                WindowInfo struct
  search/mod.rs             nucleo-matcher fuzzy search
  mru/mod.rs                In-memory MRU ring buffer
```

## Build Commands

```sh
make dev        # cargo tauri dev
make build      # cargo tauri build
make install    # npm install
make lint       # clippy + tsc --noEmit
make fmt        # cargo fmt + prettier
make clean      # remove target/, node_modules/, dist/
```

## Key Technical Details

**Window enumeration**: `CGWindowListCopyWindowInfo` (fast, ~1-3ms) → per-PID AX title enrichment via `AXUIElementCreateApplication` + `kAXWindowsAttribute`. One `WindowInfo` per PID in MVP.

**Window activation**: `NSRunningApplication.activateWithOptions:` (brings app to front, handles Spaces) → `AXUIElementPerformAction(kAXRaiseAction)` on the first AX window.

**Global hotkey**: `Cmd+Ctrl+Space` via `tauri-plugin-global-shortcut`. Registered in `lib.rs` setup block. Emits `palette-opened` event to frontend on show.

**Tray icon**: `TrayIconBuilder` from Tauri 2 core (`tray-icon` feature). PNG embedded at compile time via `include_bytes!`. `icon_as_template(true)` for macOS light/dark mode.

**Dock hiding**: `NSApp.setActivationPolicy_(.Accessory)` called in setup, before window is shown.

**MRU**: `static Mutex<Vec<(bundle_id_or_name, title)>>`. Updated on every `activate_window` call. Applied as sort key when query is empty.

## Conventions

- No `unwrap()` in production paths — use `?` or explicit error handling
- All macOS API calls are `#[cfg(target_os = "macos")]` or in `[target.*.dependencies]`
- The `objc` 0.2 crate produces `unexpected_cfg` warnings from its macros — this is known, not a bug
- Frontend communicates with backend exclusively via `invoke()` and `listen()`
- No mouse interaction — no hover states, `cursor: none` everywhere
