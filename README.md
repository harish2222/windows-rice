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

## 🚀 WINDOW-RICE Installer & Real-Time Rice Manager

### Instant Web One-Liner (iwr / irm)
Install and run on any Windows system with a single PowerShell command:

```powershell
# Web one-liner using Invoke-RestMethod:
irm https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex

# Or using Invoke-WebRequest:
iwr -useb https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex
```
*The script automatically sets up the repository at `~/.config`, verifies prerequisites, and launches the Rust Ratatui TUI installer (**WINDOW-RICE installer**).*

### Universal Shell Command (`window-rice` / `rice`)
Once installed, the `window-rice` function (and `rice` alias) is globally available in **PowerShell 7**, **Nushell**, and **Xonsh**:

```bash
# Launch interactive TUI at any time:
window-rice
rice

# Reload entire environment live:
window-rice -Reload          # PowerShell
rice -r                      # Nushell or Xonsh

# Health check & real-time component verification:
window-rice -Verify          # PowerShell
rice -v                      # Nushell or Xonsh

# Check git updates against upstream:
window-rice -Update
rice -u

# Install curated 8K wallpapers:
window-rice -Wallpapers
rice -w
```

### Local CLI Flags (`.\install.ps1` / `rice-installer.exe`)
```powershell
# Launch the interactive dual-pane TUI
.\install.ps1

# Run non-interactive health check & verification report
.\install.ps1 -Verify

# Link all configuration files (Symbolic links with copy fallback)
.\install.ps1 -LinkAll

# Install missing packages and link corresponding configs
.\install.ps1 -InstallMissing

# Bootstrap package managers (Scoop, Chocolatey, UV, Vcpkg)
.\install.ps1 -Bootstrap

# Reload Komorebi, GlazeWM, and YASB
.\install.ps1 -Reload

# Install 8K wallpapers (default: Pictures\Wallpapers or custom)
.\install.ps1 -Wallpapers -WallpaperDir "D:\Wallpapers"
```

### Installer Capabilities
- **Dual-Pane TUI Layout:** Left pane categorizes Window Managers, Terminals, Shells, Navigation, Media, and Wallpapers with multi-select checkboxes. Right pane displays package metadata, binary locations, target configuration paths, and live execution logs.
- **Automated Package Detection:** Detects installed binaries and package managers across Scoop, Winget, Chocolatey, Cargo, UV, Vcpkg, NPM, and Python.
- **Smart Linking:** Creates symbolic links (`New-Item -ItemType SymbolicLink`) with automatic fallback to file copy if Windows Developer Mode or elevation is not available.
- **Real-Time WM & Status Bar Sync:** Automatically triggers Komorebi configuration reload, GlazeWM keybindings reload, and YASB status bar replication when configs are updated.
- **Git Upstream Update Detection:** Real-time checking against `origin/main` with live status badges.
- **Verification Engine:** Validates that all 23 binaries and 20 configuration targets are properly deployed and reachable in `PATH`.

---

## 🎵 Terminal Audio & Media Ecosystem

The rice contains fine-tuned configurations for the premier terminal media players:

- **Kew Music Player (`kew/`):**
  - **Layouts (`kew/layouts/`):** Custom Apple Music-inspired 3-column layout (`current.layout`) balancing 36% Sixel artwork, 32% now-playing details, and 32% synced lyrics alongside full-width audio visualizer and pill progress bar. Also includes alternate compact layout (`alt.layout`).
  - **Themes (`kew/themes/`):** 28 curated themes including Catppuccin Mocha (`catpuccin.theme`), Apple Music, Tokyo Night, Nord, Gruvbox, and Synthwave.
  - **Features:** Gapless playback, crossfade, Discord Rich Presence, and custom visualizer presets.
- **CLIamp Player (`cliamp/`):**
  - Classic Winamp retro interface, 10-band custom EQ, 24-bit/96kHz WASAPI streaming, and radio/podcast support.
  - Includes Discord Rich Presence (`plugins/discord-rpc.lua`) and Last.fm scrobbler (`plugins/cliamp-lastfm.lua`).
- **MPD & ncmpc (`mpd/`, `ncmpc/`):**
  - Native MPD daemon with non-exclusive WASAPI output and ReplayGain album normalization (`mpd/mpd.conf`).
  - Curses-based ncmpc client styled with Apple Music accents over Catppuccin Mocha (`ncmpc/config`).

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

## 🖼️ 8K Curated Wallpaper Collection (HKDEVS Edition)

Windows Rice includes an ultra-high resolution wallpaper collection containing **1,774 unique, curated 8K & 5K wallpapers** (over 4.2 GB uncompressed), completely organized, deduplicated, and metadata-sanitized (all old legacy references stripped; author, artist, owner, and copyright attributed cleanly to `HKDEVS` with ICC color profiles preserved):

| Category | Count | Description |
|---|---|---|
| **Distros** | 1,283 files | Arch, ArcoLinux, BlackArch, CachyOS, Debian, EndeavourOS, Fedora, Garuda, Kali, Manjaro, NixOS, openSUSE, Parrot, PopOS, RedHat, Other |
| **Minimal Rice** | 306 files | Numbered minimalist wallpapers (`0001.jpg` – `0306.jpg`), stripped of legacy naming |
| **Curated Art** | 150 files | Abstract, digital art, stylized landscapes, and dark anime themes |
| **Apple macOS 5K/6K** | 17 files | Ultra-res macOS desert/mountain dynamic captures (Catalina, Mojave, Sierra, etc.) |
| **Nature & Landscapes** | 12 files | 8K nature panoramas, misty mountains, and dramatic twilight |
| **Space & Cosmos** | 4 files | High-depth nebula, planetary alignments, and galactic cores |
| **City & Architecture** | 2 files | Cyberpunk and nocturnal metropolis compositions |

### 📦 Modular 3-Pack Distribution (< 2 GB each for GitHub Releases)
To comply with GitHub's 2 GB asset limit while preserving pixel-perfect ultra lossless quality:
- **Pack 1:** `Wallpapers_Pack1_Rice_macOS_Art.7z` (792 MB) — Minimal Rice, macOS 5K/6K, Curated Art, Nature, Space, and City.
- **Pack 2:** `Wallpapers_Pack2_Distros_Fedora_Arch.7z` (1.86 GB) — Fedora, Arch Linux, ArcoLinux, BlackArch, and CachyOS.
- **Pack 3:** `Wallpapers_Pack3_Distros_Endeavour_Kali_Debian.7z` (1.52 GB) — EndeavourOS, Garuda, Kali Linux, Debian, Manjaro, NixOS, openSUSE, Parrot, PopOS, RedHat.

Install wallpapers instantly to Windows `Pictures\Wallpapers` (or any custom location) directly from the TUI (`[w]`), via PowerShell (`.\install.ps1 -Wallpapers`), or shell (`window-rice --wallpapers`).

---

## 🔄 Live Environment Synchronization & Reload

No manual individual commands needed! The **WINDOW-RICE installer** and shell function manage the entire live stack:

```powershell
# Universal one-shot reload across all shells (PowerShell, Nushell, Xonsh):
window-rice -Reload
# Or using the shorthand alias:
rice -r
```

This automatically:
1. Reloads Komorebi configuration (`komorebic replace-configuration ~/.config/komorebi/komorebi.json`)
2. Reloads GlazeWM hotkey layer (`glazewm command wm-reload-config`)
3. Re-runs YASB status bar verification and companion tools replication (`yasb-setup.ps1`)
