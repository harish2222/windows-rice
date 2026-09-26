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
| `yasb/` | Bar config, full stylesheet, theme switcher tools |
| `glazewm/` | Keybindings and WM behaviour (hardlinked to `.glzr` for the default path) |
| `komorebi/` | Tiling rules, float rules, workspace rules, autostart script |
| `hotkeys-overlay/` | Rust `Alt+/` hotkey cheatsheet overlay (zero dependencies) |
| `zellij/`, `nvim/`, `nushell/`, `oh-my-posh/`, ... | Terminal and shell configs |

## Applying changes live

```sh
komorebic replace-configuration ~/.config/komorebi/komorebi.json
glazewm command wm-reload-config
yasbc reload
```

The hotkey overlay rebuilds with `cargo build --release` inside
`hotkeys-overlay/`.

## Notes

This repo is public and machine-specific: paths reference one Windows user.
Credential files, chat logs, binaries, and nested repositories are excluded
via `.gitignore`.
