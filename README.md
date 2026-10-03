# windows-rice

My Windows desktop rice. Three cooperating layers, all configs under `~/.config`:

- **komorebi** - the tiling window manager (i3/hyprland-style rules: float
  popups, workspace assignments, cloak hiding, macOS-style EaseOutBack window
  animations).
- **GlazeWM** - hotkey layer only. It manages zero windows (ignore-all rule)
  and shells every binding to `komorebic`, so Alt-based muscle memory works
  without touching Windows defaults.
- **YASB** - the status bar, with a switchable Rangalipi theme and a uniform
  UI system: one font (FiraCode Nerd Font Mono), one weight (600), a three
  tier radius scale (14 / 10 / 4), and colour-token-only styling.

## Layout

| Directory | What lives there |
|---|---|
| `yasb/` | Bar config, full stylesheet, and all companion tools under `yasb/tools/` |
| `glazewm/` | Keybindings and WM behaviour (hardlinked to `.glzr` for the default path) |
| `komorebi/` | Tiling rules, float rules, workspace rules, autostart script |
| `hotkeys-overlay/` | Rust `Alt+/` hotkey cheatsheet overlay (zero dependencies) |
| `zellij/`, `nvim/`, `nushell/`, `oh-my-posh/`, ... | Terminal and shell configs |

## Tool & script shortcuts

Every script and tool in this rice has a one-page doc next to its source:

| Doc | Tool |
|---|---|
| [`yasb/tools/saka/saka.md`](yasb/tools/saka/saka.md) | **Saka calendar converter** (Rust) — Indian national date on the bar; includes the why-1948-in-2026 era math |
| [`yasb/tools/pomodoro/pomodoro.md`](yasb/tools/pomodoro/pomodoro.md) | **Pomodoro focus timer** — the bar now uses YASB's native `PomodoroWidget` (55/5); the old Rust timer is kept as a fallback |
| [`yasb/tools/theme/theme.md`](yasb/tools/theme/theme.md) | **Theme switch engine** (Rust) — recolours the whole bar between Rangalipi themes; also a shared library |
| [`yasb/tools/saka-popup/saka-popup.md`](yasb/tools/saka-popup/saka-popup.md) | **Panchangam panel** (Rust/Win32) — the full Telugu panchangam in a layered top-level window |
| [`yasb/tools/picker/picker.md`](yasb/tools/picker/picker.md) | **Palette & font pickers** — the palette picker is Rust/Win32 (layered, no Qt); a PyQt6 version is kept as a fallback |
| [`yasb/tools/setup/setup.md`](yasb/tools/setup/setup.md) | **Bootstrap & submission pipeline** — replicate the rice; package a theme for yasb-themes |
| [`hotkeys-overlay/hotkeys-overlay.md`](hotkeys-overlay/hotkeys-overlay.md) | **Hotkey cheatsheet overlay** (Rust/Win32) — the `Alt+/` popup |
| [`yasb/configs/README.md`](yasb/configs/README.md) | Snapshots of the live GlazeWM/Komorebi configs |

## Keybindings (quick reference)

GlazeWM is the hotkey layer; komorebic does the tiling. Full list:
`Alt+/` in the overlay.

| Keys | Action |
|---|---|
| `Alt + h j k l` | focus left/down/up/right |
| `Alt + Shift + h j k l` | move window |
| `Alt + r` | resize mode (hjkl/arrows, Esc to exit) |
| `Alt + Return` | terminal (again: fullscreen) |
| `Alt + Shift + e / b / o` | file explorer / browser / editor |

## Applying changes live

```sh
komorebic replace-configuration ~/.config/komorebi/komorebi.json
glazewm command wm-reload-config
yasbc reload
```

Rust tools rebuild with `cargo build --release` inside their crate
(`yasb/tools/saka`, `yasb/tools/saka-popup`,
`yasb/tools/picker/palette-picker`, `yasb/tools/theme/yasb-theme`,
`yasb/tools/pomodoro`, `hotkeys-overlay`); the theme exe redeploys via
`yasb/tools/theme/yasb-theme-build.ps1`. Saka, saka-popup, the picker
and yasb-theme carry `cargo test` suites (saka's pins the official
Gazette month table); the whole chain rebuilds + verifies via
`yasb/tools/setup/yasb-setup.ps1`.

The bar also ships a **55/5 pomodoro chip** (`pomodoro` widget, right
section), driven by YASB's native `PomodoroWidget`: left-click opens the
circular-timer popup, middle-click starts/pauses, right-click swaps the
label. The popup and chip follow the active Rangalipi theme, and
`yasb-theme` rewrites the two circle colours on every switch.

## Notes

This repo is public and machine-specific: paths reference one Windows user.
Credential files, chat logs, binaries, and nested repositories are excluded
via `.gitignore`.
