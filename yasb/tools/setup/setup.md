# setup — one-shot bootstrap & yasb-themes submission pipeline

Scripts that either (re)build this whole toolchain from nothing or package
the bar for the amnweb/yasb-themes gallery.

## yasb-setup.ps1 — replicate this rice

Idempotent one-shot setup: installs missing toolchains (Rust via winget,
PyQt6 + PyInstaller via pip), rebuilds every companion binary (theme exe,
palette picker, font tool), snapshots the live GlazeWM/Komorebi configs
into `configs\`, verifies the whole chain end-to-end (theme tool round-trip,
`silent-run` on PATH, picker exe boots bundled Qt), and installs the `yt`
shell helper into the PowerShell profile. Never touches `config.yaml` /
`styles.css`.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\tools\setup\yasb-setup.ps1
```

## export-theme.py — build a submission pack

Reads the live bar files and emits a gallery-compliant pack (single-theme
`styles.css` with the chosen theme uncommented and all dead blocks gone,
blank-line-free validated `config.yaml`, gallery README, live bar-only
preview screenshot, ready-made issue body). Enforces every
CONTRIBUTING.md rejection rule that can be checked locally.

## build-submission.py — squeeze it under GitHub limits

Takes the pack and assembles the final issue body: compacts the CSS
one-rule-per-line with an assertion that the *declaration set is
byte-identical* before/after, strips config comments, revalidates the
exact submitted bytes against YASB's own bot validators
(`migrate_config` + `YasbConfig.model_validate`), and asserts the body
fits GitHub's 65 536-char cap. Live files are never touched.

> Submission flow (never a manual PR): screenshot only via the bar's
> built-in Take Screenshot, then file the issue with the body these
> scripts produce.

## Order of operations for a theme submission

```powershell
python tools\setup\export-theme.py      # 1. pack from live files
python tools\setup\build-submission.py  # 2. validate + final issue body
# 3. upload preview, fill image URL, open the issue
```
