# windows-rice

My Windows desktop rice. Three cooperating layers, all configs under `~/.config`:

- **komorebi** — the tiling window manager (clean ignore rules, workspace assignments, cloak window hiding, smooth animations).
- **GlazeWM** — global hotkey layer only. Shells every binding to `komorebic` through `silent-run.exe` (`CREATE_NO_WINDOW`) so Alt-based hotkeys execute with zero black console flash.
- **YASB** — the status bar with 21 switchable Rangalipi themes, live Cava spectrum synchronization, icon-only active window/stack displays, and native Apple HIG custom Win32 companion widgets.

---

## 📸 Desktop Showcase & Gallery

Real desktop captures across themes with active Cava spectrum audio visualizer and layered popups:

| Japanese Demon Scroll (Dark Theme) | Hummingbird Rice (Matcha Light Theme) |
|---|---|
| ![Demon Scroll](yasb/screenshot/Screenshot%20(62).png) | ![Hummingbird](yasb/screenshot/Screenshot%20(63).png) |

| Firewatch Blue Mountain (Night Theme) | Earth Orbit (Cava Spectrum Active) |
|---|---|
| ![Firewatch](yasb/screenshot/Screenshot%20(64).png) | ![Earth Orbit](yasb/screenshot/Screenshot%20(65).png) |

| Media Player Popup (Lyrics & Audio Spectrum) | Demon Scroll with Active Media |
|---|---|
| ![Media Player](yasb/screenshot/Screenshot%20(72).png) | ![Media Playing](yasb/screenshot/Screenshot%20(66).png) |

### Bar Precision Views

| Rangalipi Wine Dark |
|---|
| ![Wine Dark Bar](yasb/screenshot/yasb_bar_primary-bar_20261025_131423.png) |

| Rangalipi Wine Light (High Contrast) |
|---|
| ![Wine Light Bar](yasb/screenshot/yasb_bar_primary-bar_20261025_131613.png) |

---

## 📁 Directory Structure & Organization

```
.config/
├── glazewm/                   # GlazeWM keybindings (global hotkey layer)
├── komorebi/                  # Komorebi tiling engine rules & startup script
├── cava/                      # Cava audio spectrum visualizer configuration & shaders
├── yasb/                      # YASB bar configuration, themes, assets, and companion tools
│   ├── assets/                # Graphic assets
│   │   ├── mandala/           # Processed mandala artwork PNGs
│   │   ├── motifs/            # SVG motifs & runners dealt out by yasb-theme
│   │   └── raw-art/           # Vector artwork sources (.ai, .eps, .jpg)
│   ├── configs/               # Shared WM config snapshots (glazewm, komorebi)
│   ├── screenshot/            # Desktop captures & bar crops
│   ├── themes/                # Themes directory
│   ├── tools/                 # Native Win32/Rust companion widgets
│   │   ├── chrome/            # yasb-chrome acrylic canvas & DWM blur library
│   │   ├── keybind/           # Apple HIG cheatsheet menu (Win+K)
│   │   ├── komorebi-sync/     # Workspace state synchronizer
│   │   ├── picker/            # palette-picker (21-palette theme browser)
│   │   ├── pomodoro/          # Focus timer state machine
│   │   ├── saka/              # Vedic Panchang calculation engine
│   │   ├── saka-popup/        # Dual hero moons & telescopic night sky panel
│   │   ├── screen-shot/       # Layered-window screen capture tool (CAPTUREBLT)
│   │   ├── setup/             # Turnkey replication script (yasb-setup.ps1)
│   │   └── theme/             # yasb-theme engine & silent-run shim
│   ├── config.yaml            # YASB bar configuration
│   ├── styles.css             # Active theme stylesheet
│   ├── DESIGN.md              # Apple HIG visual design contract
│   ├── rangalipi-options.css  # Widget styling options
│   └── yasb_colors.css        # Color palette variables
└── fastfetch/, nvim/, ...     # Terminal and shell configurations
```

---

## 🌟 Native Win32 / Rust Companion Widgets

All custom companion widgets run in pure Rust Win32 with GDI hardware-buffered layered acrylic surfaces (zero Python, zero Qt, zero webview dependencies):

| Tool | Trigger | Description |
|---|---|---|
| [`saka-popup`](yasb/tools/saka-popup/saka-popup.md) | Click Saka bar widget | Vedic Panchangam panel with dual $116\text{px}$ hero moons: 3D rotating moon + telescopic night sky moon with authentic lunar maria (*Imbrium*, *Procellarum*, *Serenitatis*) and ray craters (*Tycho*, *Copernicus*). 3 layout modes (`M` to cycle) and English/Telugu bilingual toggle (`Tab`). |
| [`palette-picker`](yasb/tools/picker/picker.md) | Click Palette icon / `yt` | Win32 acrylic theme switcher for all 21 Rangalipi themes (11 dark + 10 light). Type to fuzzy-filter, instant live re-theme with zero restart. |
| [`keybind`](yasb/tools/keybind/keybind.md) | **`Win + K`** / Click keyboard | Apple HIG cheatsheet popup with category chips (WM Focus, Move, Layout, Workspaces, Apps, System) and live fuzzy search. |
| [`silent-run`](yasb/tools/theme/theme.md) | GlazeWM / scripts | Spawns console children with `CREATE_NO_WINDOW` (`0x08000000`) for zero-flash hotkey execution. |
| [`pomodoro`](yasb/tools/pomodoro/pomodoro.md) | Click Pomodoro widget | Persistent focus timer with work/break cycle tracking. |

---

## ⌨️ Essential Hotkeys (Quick Reference)

GlazeWM is the hotkey layer; Komorebi does the tiling.

| Keys | Action |
|---|---|
| **Win + K** | Open Keybindings Cheatsheet Menu |
| **Alt + h / j / k / l** | Focus Left / Down / Up / Right window |
| **Alt + Shift + h / j / k / l** | Move focused window Left / Down / Up / Right |
| **Alt + Return** | Launch terminal |
| **Alt + Shift + b** / **Alt + z** | Launch browser |
| **Alt + o** | Launch editor |
| **Alt + q** | Close window |
| **Alt + f** | Toggle maximize |
| **Alt + Shift + f** | Toggle monocle layout |
| **Alt + r** | Resize mode (arrow keys / hjkl to adjust, Escape to exit) |
| **Alt + Shift + r** | Reload GlazeWM & Komorebi configurations |

---

## 🔄 Applying Changes Live

```powershell
# Reload Komorebi configuration
komorebic replace-configuration ~/.config/komorebi/komorebi.json

# Reload GlazeWM hotkey layer
glazewm command wm-reload-config

# Re-run replication / verification
powershell -ExecutionPolicy Bypass -File ~/.config/yasb/tools/setup/yasb-setup.ps1
```
