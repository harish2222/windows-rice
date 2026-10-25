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

## 🚀 Turnkey TUI Installer & Configurator

The rice includes an interactive terminal installer written in Rust (`ratatui` + `crossterm`) located in `installer/` and launched via root script `install.ps1`:

```powershell
# Launch the interactive dual-pane TUI installer
.\install.ps1

# Run non-interactive health check & verification report
.\install.ps1 -Verify

# Link all configuration files (Symbolic links with copy fallback)
.\install.ps1 -LinkAll

# Install missing packages and link corresponding configs
.\install.ps1 -InstallMissing

# Bootstrap missing package managers (Scoop, Chocolatey)
.\install.ps1 -Bootstrap
```

### Installer Capabilities
- **Dual-Pane TUI Layout:** Left pane categorizes Window Managers, Terminals, Shells, Navigation, Media, and Editors with multi-select checkboxes. Right pane displays package metadata, binary locations, target configuration paths, and live execution logs.
- **Automated Package Detection:** Detects installed binaries and packages across Scoop, Winget, Chocolatey, Cargo, NPM, and Python.
- **Smart Linking:** Creates symbolic links (`mklink` / `New-Item -ItemType SymbolicLink`) with automatic fallback to file copy if Windows Developer Mode or elevation is not available.
- **Verification Engine:** Validates that all 21 binaries and 19 configuration targets are properly deployed and reachable in `PATH`.

---

## 🐚 Unified Multi-Shell Configurations

All three primary shells are unified under `shells/` with complete parity for autocompletions, fuzzy finding, and theme integration:

- **PowerShell 7 (`shells/powershell/`):** Full profile with `carapace`, `zoxide`, `fzf` prompt suggestions, Terminal-Icons, and AudioDeviceCmdlets.
- **Nushell (`shells/nushell/`):** Custom structured table pipeline with `carapace.nu` integration, `omp.nu` Catppuccin prompt, and completion modules.
- **Xonsh (`shells/xonsh/`):** Pythonic shell with `rc.xsh`, `init.xsh`, `.xonshrc`, aliases, and unified completions.

---

## ⚡ Carapace Autocompletion Catalog

Windows Rice includes **63 custom, syntax-checked Carapace completion specifications** located in `carapace/specs/` and synced to `%APPDATA%\carapace\specs\`:
- Supports subcommands, flags, flag values, and help probes for developer tools: `cargo`, `rustc`, `rustup`, `node`, `npm`, `pnpm`, `bun`, `deno`, `zig`, `python`, `pymanager`, `docker`, `podman`, `terraform`, `claude`, `claude-code`, `codex`, `opencode`, `kilocode`, `cline`, `hermes`, `freebuff`, `scoop`, `winget`, `choco`, `yazi`, `zellij`, `fastboot`, `cava`, `kew`, `cliamp`, `mpd`, `komorebi`, `glazewm`, `yasb`, and more.

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
