# theme — the bar's theme-switch engine

The machinery behind the palette chip in the omega dropdown: click it and
the whole bar recolours itself between the Rangalipi themes. Two halves:

## yasb-theme (Rust crate, this folder)

`yasb-theme/src/main.rs` — every theme lives as a `/* <Name> */` block of
CSS variables inside `:root` in `styles.css`; exactly one block is
uncommented at a time (the active one). The tool toggles those comment
markers **byte-safely**: it preserves the file's newline style, trailing
newline, and UTF-8 no-BOM encoding, so YASB's style watcher never sees a
broken file.

```sh
yasb-theme.exe list      # all themes, * marks active
yasb-theme.exe current   # active theme name (the bar polls this)
yasb-theme.exe set Wine  # activate by name (case-insensitive)
yasb-theme.exe next      # cycle forward  (wraps)
yasb-theme.exe prev      # cycle backward
```

## Wrappers and shell helper

| File | Role |
|---|---|
| `yasb-theme/src/` | crate source (467 lines, zero deps; rescued into the repo from a scratch clone) |
| `yasb-theme-build.ps1` | rebuild + deploy: `cargo build --release`, copies the exe here and `silent-run` into scoop shims, smoke-tests `current` |
| `yasb-theme-current.bat` / `-next.bat` / `-prev.bat` | one-line wrappers used by bar callbacks and keyboard launchers |
| `yasb-theme-shell.ps1` | dot-source from your PowerShell profile to get `yt list / set / next / prev` |

## Bar wiring

The `palette` CustomWidget in `config.yaml` polls
`yasb-theme.exe current` every 10 s; left-click opens the graphical
palette picker (see `picker.md`), right-click runs `yasb-theme.exe next`.
Rebuild after touching the crate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\tools\theme\yasb-theme-build.ps1
```
