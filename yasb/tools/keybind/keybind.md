<!--
Author -: HKDEVS
Project -: Window-Rice
Title -: YASB Hotkey Overlay and Cheatsheet Documentation
-->
# keybind — Keybindings Cheatsheet Menu

Native Win32 companion widget providing an Apple Human Interface Guidelines (HIG)-inspired cheatsheet popup for all window manager, bar, terminal, and application hotkeys.

## Overview

A frameless, topmost, layered Win32 acrylic window (zero webview, zero Qt, zero Python dependencies) displaying a searchable, categorized cheat-sheet of desktop shortcuts.

- **Hotkey**: `Win+K` (via GlazeWM keybinding layer).
- **Bar Widget**: Left-click the keyboard icon (`\uf11c`) in the Omega control group.
- **Theme integration**: Dynamically inherits background glass, surface tint, border hairline, accent, and text contrast from the active Rangalipi theme via `yasb-theme` and `yasb-chrome`.

```yaml
# config.yaml widget definition
keybind:
  type: "yasb.custom.CustomWidget"
  options:
    label: "<span>\uf11c</span>"
    label_alt: "Keybindings"
    class_name: "keybind-widget"
    tooltip: true
    tooltip_label: "Keybindings Cheatsheet (Win+K)"
    exec_options:
      run_cmd: "echo 1"
      run_interval: 0
      run_once: true
      return_format: "string"
    callbacks:
      on_left: "exec silent-run C:\\Users\\haris\\.config\\yasb\\tools\\keybind\\keybind.exe"
      on_right: "toggle_label"
```

## Features

1. **Instant Fuzzy Search**:
   - Type query characters to filter hotkeys by description, key combination, or category.
   - Subsequence fuzzy matching with search input indicator.
   - `Backspace` to delete characters, `Esc` to dismiss.
2. **Category Chip Navigation**:
   - Filter pills on top: `All`, `WM Focus`, `WM Move`, `Layout`, `Workspaces`, `Apps`, `System`.
   - Click a chip or tab through categories.
3. **Smooth Keyboard & Mouse Scrolling**:
   - `Up` / `Down` arrows move selection.
   - `PageUp` / `PageDown` scroll full pages.
   - Mouse wheel and direct scrollbar drag support.
4. **Apple HIG Acrylic Visuals**:
   - Real DWM acrylic glass blur behind (`UpdateLayeredWindow` + 32-bit ARGB DIB).
   - High-contrast ClearType anti-aliased typography (JetBrainsMono / Segoe UI Variable).
   - Subtle outer glow and hairline border matching active theme accent.

## Architecture

| Source | Role |
|---|---|
| `src/model.rs` | Shortcut catalog, category model, fuzzy subsequence filter |
| `src/layout.rs` | Window dimensions, category chip layout, scrollable entry list, scrollbar math |
| `src/main.rs` | Win32 message loop, layered window painting, GDI double-buffering, single instance guard |
| `tests/` | Unit tests for category filtering, fuzzy matching, and scroll geometry |

## Dev & Verification

```powershell
# Run unit tests (8 tests)
cargo test --manifest-path tools\keybind\Cargo.toml

# Compile release binary
cargo build --release --manifest-path tools\keybind\Cargo.toml
```
