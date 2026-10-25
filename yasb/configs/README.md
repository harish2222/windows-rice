# Shared Window Manager Configurations

Configurations that govern window management and keyboard interaction alongside the YASB bar, designed for turnkey replication.

## Architecture

- **GlazeWM (`glazewm/config.yaml`)**:
  Acts exclusively as the **global hotkey layer** (`Alt`-based navigation, `Win+K` cheatsheet, launcher shortcuts). Commands invoke `komorebic` through `silent-run.exe` with `CREATE_NO_WINDOW` to eliminate console window flashes.
- **Komorebi (`komorebi/komorebi.json`)**:
  Acts exclusively as the **tiling engine**. Configured with smooth animations, custom theme-coordinated borders, and clean ignore rules for:
  - `yasb.exe`
  - `glazewm.exe`
  - `saka-popup.exe`
  - `palette-picker.exe`
  - `keybind.exe`
  - `komorebi-sync.exe`
  All non-essential window rules have been cleanly stripped to avoid tiling conflicts.

## Authoritative Live Locations

| File | Deployed Path | Role |
|---|---|---|
| `glazewm/config.yaml` | `~\.glzr\glazewm\config.yaml` | GlazeWM hotkey layer |
| `komorebi/komorebi.json` | `~\.config\komorebi\komorebi.json` & `~\komorebi.json` | Komorebi tiling engine |
| `komorebi/komorebi.bar.json` | `~\.config\komorebi\komorebi.bar.json` | Fallback komorebi bar config |
| `komorebi/applications.json` | `~\.config\komorebi\applications.json` | Application-specific rules |
| `komorebi/komorebi-startup.ps1` | `~\.config\komorebi\komorebi-startup.ps1` | Window manager startup chain |

Replication and path adaptation are handled automatically by `tools\setup\yasb-setup.ps1`.
