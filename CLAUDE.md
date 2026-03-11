# Conjure — Claude Code Instructions

## Project Overview

macOS window switcher built with Rust + Tauri 2. Keyboard-driven fuzzy search palette. Background app (menu bar only, no Dock icon).

## Build Commands

```sh
make dev        # cargo tauri dev
make build      # cargo tauri build
make install    # cp production .app to /Applications
make npm-install # npm install
make lint       # clippy + tsc --noEmit
make fmt        # cargo fmt + prettier
make clean      # remove target/, node_modules/, dist/
```

## Key Technical Details

**Window enumeration**: `CGWindowListCopyWindowInfo` (fast, ~1-3ms) → per-PID AX title enrichment via `AXUIElementCreateApplication` + `kAXWindowsAttribute`. One `WindowInfo` per PID in MVP.

**Window activation**: `NSRunningApplication.activateWithOptions:` (brings app to front, handles Spaces) → `AXUIElementPerformAction(kAXRaiseAction)` on the first AX window.

**Global hotkey**: `Cmd+Alt+P` via `tauri-plugin-global-shortcut`. Registered in `hotkey.rs`. Emits `palette-opened` event to frontend on show.

**Tray icon**: `TrayIconBuilder` from Tauri 2 core (`tray-icon` feature). PNG embedded at compile time via `include_bytes!`. `icon_as_template(true)` for macOS light/dark mode.

**Dock hiding**: `NSApp.setActivationPolicy_(.Accessory)` called in setup, before window is shown.

**MRU**: `static Mutex<Vec<(bundle_id_or_name, title)>>`. Updated on every `activate_window` call. Applied as sort key when query is empty.

## Permissions & TCC

macOS TCC (privacy) grants are path- and bundle-ID-specific. Dev and production builds must have distinct identifiers to avoid conflicts:

- **Dev** (`make dev`): `Conjure Dev` / `com.davidalecrim.conjure-dev`
- **Production** (`make build && make install`): `Conjure` / `com.davidalecrim.conjure`

`make install` copies to `/Applications/Conjure.app`. Always install there before granting Accessibility — TCC ties the grant to the app path. To reset a stale grant: `tccutil reset Accessibility com.davidalecrim.conjure`.

Window titles come from `kAXTitleAttribute` via the AX API — **only Accessibility permission is required**, not Screen Recording.

## Conventions

- No `unwrap()` in production paths — use `?` or explicit error handling
- All macOS API calls are `#[cfg(target_os = "macos")]` or in `[target.*.dependencies]`
- The `objc` 0.2 crate produces `unexpected_cfg` warnings from its macros — this is known, not a bug
- Frontend communicates with backend exclusively via `invoke()` and `listen()`
- No mouse interaction — no hover states, `cursor: none` everywhere
