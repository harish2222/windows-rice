# picker — Qt palette picker & font tool

GUI companions to `theme.md`, living on the same tools shelf.

## palette-picker.py → palette-picker.exe

A suckless **dmenu-style palette picker** (PyQt6, 418 lines). One slim
frameless bar: a filter line on top, a flexbox flow of theme cards below.
Type to narrow, arrows to move, Enter to apply. It reads the theme catalog
`palette-themes.json` and paints the cards with the **active** theme's own
CSS variables, so the picker always matches whatever the bar currently
looks like.

- Enter applies a theme by calling `yasb-theme.exe set <name>` — see
  `theme.md` for what that does to `styles.css`.
- Launched by left-clicking the `palette` chip in the bar's omega dropdown
  (`exec silent-run ...\palette-picker.exe`).
- Built by `palette-picker-build.ps1` (PyInstaller onefile).

## yasb-font.exe

Font-picker sibling of the palette picker (same dmenu pattern): lists
installed Nerd Fonts and writes the chosen family into the bar's font
tokens. Built with PyInstaller from `yasb-font.spec`.

> ⚠ Source note: `yasb-font.py` no longer exists anywhere on disk — only
> the PyInstaller spec and the built exe remain (the exe is still
> functional). If this tool needs changes, decompile-free recovery isn't
> realistic; it should be re-written from the spec's entry point or
> retired in favour of a Rust twin of `yasb-font`'s behaviour.

## Files

| File | Role |
|---|---|
| `palette-picker.py` | picker source (PyQt6) |
| `palette-themes.json` | theme catalog shown as cards |
| `palette-picker-build.ps1` / `.spec` / `.bat` | build wrapper, PyInstaller spec, bar-friendly launcher |
| `yasb-font.spec` / `yasb-font.exe` | font picker spec + binary (source missing, see above) |

`picker/` is fully gitignored (binaries, build dirs, and — at the moment —
its sources) precisely because of the churn above; the authoritative
copies of both tools' *behaviour* are documented here so they can be
rebuilt from scratch if the folder is ever lost.
