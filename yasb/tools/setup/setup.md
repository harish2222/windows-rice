<!--
Author -: HKDEVS
Project -: Window-Rice
Title -: yasb\tools\setup / setup
-->
# setup — One-Shot Bootstrap & Replication Pipeline

Scripts that compile and replicate the entire Rangalipi rice, window manager layers, and companion Win32 tools across any fresh Windows machine.

## yasb-setup.ps1 — Replicate this Rice

An idempotent one-shot replicator that:
1. **Checks Prerequisites**: Validates presence of YASB, Komorebi, GlazeWM, Cava, and Nerd Fonts (`JetBrainsMono-NF`, `FiraCode-NF-Mono`).
2. **Backs up Live Files**: Safe timestamped backups of existing user configurations in `~/.config/yasb`, `~/.glzr/glazewm`, and `~/.config/komorebi`.
3. **Installs Toolchains**: Installs Rust toolchain via winget if missing.
4. **Builds Companion Tools (Pure Rust Workspace)**:
   Runs `cargo build --workspace --release` across all companion crates with zero Python or Qt runtime dependencies:
   - `yasb-theme` & `silent-run` (theme manager and zero-flash process launcher)
   - `palette-picker` (Win32 Apple HIG acrylic palette browser)
   - `saka` & `saka-popup` (Vedic Panchangam engine and dual-hero moon panel)
   - `keybind` (Win32 Apple HIG cheatsheet menu)
   - `pomodoro` (persistent focus timer)
   - `komorebi-sync` (tiling state synchronizer)
   - `screen-shot` (layered-window screen capture tool)
5. **Deploys Binaries & Shims**:
   Installs `silent-run.exe` to `~/scoop/shims` and `~/.local/bin`, and populates `~/.config/yasb/tools/`.
6. **Adapts Paths Dynamically**:
   Inspects the local `$HOME` / `$USERPROFILE` and automatically adapts file paths in `config.yaml`, `styles.css` (for mandala motif URLs), `configs/glazewm/config.yaml`, and `configs/komorebi/komorebi.json` so every callback works out-of-the-box on any user account.
7. **Deploys Window Manager Layers**:
   Installs GlazeWM hotkey layer and Komorebi tiling configuration (with clean ignore rules and zero unneeded window rules).
8. **Verifies the Entire Chain**:
   - `yasb-theme.exe current` and `yasb-theme.exe list` (asserts all 22 themes)
   - `saka.exe (Get-Date) --long`
   - `saka-popup.exe --smoke` (headless double-buffered paint self-test)
   - `palette-picker.exe --smoke` (headless theme catalog and keyboard self-test)
   - `pomodoro.exe tick`
   - `silent-run` command availability.
9. **Registers Shell Integration**:
   Appends the `yt` shell helper shortcut to PowerShell `$PROFILE`.

## Usage

From a clone of this repository:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\yasb-setup.ps1
```

Once completed:
1. Restart YASB: `yasb`
2. Restart GlazeWM & Komorebi: `alt+shift+r`
3. Launch a new terminal and run: `yt list`
