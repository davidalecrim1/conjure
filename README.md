# Conjure

Keyboard-driven macOS window switcher. Press `Cmd+Ctrl+Space` to summon a fuzzy search palette over all open windows — then type, navigate with arrow keys, and hit Enter to jump there instantly.

Inspired by [Contexts](https://contexts.co) and the command palettes in Zed/VS Code. Fully mouseless by design.

## Features

- Fuzzy search across all open windows
- MRU ordering — most recently used windows appear first
- Minimized window support
- Cross-Space switching (macOS Spaces handled automatically)
- Lives in the menu bar, no Dock icon
- Dark, blurred palette UI with `backdrop-filter`

## Prerequisites

- macOS 12+
- [Rust](https://rustup.rs) (stable)
- Node.js 18+

## Setup

```sh
make install
```

## Development

```sh
make dev
```

On first launch, macOS will prompt for Accessibility permissions. Grant them in System Settings → Privacy & Security → Accessibility.

## Build

```sh
make build
```

The app bundle is output to `src-tauri/target/release/bundle/macos/`.

## Hotkey

`Cmd+Ctrl+Space` — shows/hides the palette from any app.

## Commands

| Command | Description |
|---------|-------------|
| `make dev` | Run in development mode |
| `make build` | Build release binary |
| `make install` | Install dependencies |
| `make lint` | Run clippy + tsc |
| `make fmt` | Format Rust + TypeScript |
| `make clean` | Remove build artifacts |
