# picker — theme palette picker & font tool

GUI companions to `theme.md`, living on the same tools shelf.

## palette-picker (Rust) — the palette picker

A suckless **dmenu-style palette picker** in Rust: a single frameless,
topmost, layered Win32 window (no Qt, no webview, no Python). 22 theme
cards in a 3-column grid, each with three colour swatches, a filter line
on top and a footer showing `n of 22 themes`.

It reads three sources at startup:

| Source | Used for |
|---|---|
| `../palette-themes.json` | the 22-item catalog (`name`, `colors[3]`, `section`) |
| `../theme/yasb-theme` (lib) | the active theme's CSS variables → window chrome |
| `../../styles.css` | the active theme block, via `active_theme_vars` |

Because the chrome comes from the same Rust library `yasb-theme` uses to
write the stylesheet, the picker is always pixel-consistent with the bar.
It floors the theme's background alpha to 235 — Rangalipi's glass is ~1%
alpha, which would be invisible as a solid window.

Keys: type to filter, `Backspace` to delete, arrows / `Tab` to move,
`PgUp`/`PgDn` to page, `Enter` to apply, `Esc` to dismiss. Clicking a card
applies it. Applying a theme shells out to `yasb-theme.exe set <name>`,
which rewrites `styles.css` and re-syncs the cava/pomodoro colours in
`config.yaml` — see `theme.md`.

Launched by left-clicking the `palette` chip in the bar:

```
on_left: "exec silent-run C:\Users\haris\.config\yasb\tools\picker\palette-picker\target\release\palette-picker.exe"
```

### Source layout

| File | Role |
|---|---|
| `palette-picker/src/catalog.rs` | hand-rolled JSON parser → `Item { name, colors, section }` |
| `palette-picker/src/theme.rs` | `Theme` (bg/surface/text/subtext/border/accent) from `active_theme_vars` |
| `palette-picker/src/layout.rs` | grid geometry, filtering, hit-testing, selection movement |
| `palette-picker/src/main.rs` | layered window, GDI paint, keyboard/mouse input |

No `serde` — the catalog is a fixed 22-line shape and a hand-rolled parser
keeps the dependency list at `windows` + the local `yasb-theme` crate.

### Dev flags

| Flag | Why |
|---|---|
| `--smoke` | headless self-test: catalog parse, theme read, filter + keyboard path |
| `--no-layered` | blit to the screen instead of `UpdateLayeredWindow`, so screen-capture tools can see it (verification only) |
| `--debug-paint` | dump paint metrics |

> ⚠ **Layered-window capture.** Windows drawn with `UpdateLayeredWindow`
> are invisible to `PIL.ImageGrab` and to `PrintWindow`+`PW_RENDERFULLCONTENT`
> (they come back blank). Use `BitBlt` with `CAPTUREBLT` (`0x40000000`) from
> the screen DC — `palette-picker/capture-screen.py` does this and works.
> To screenshot the real picker at all, run it with `--no-layered`.

> ⚠ **The alpha trap.** GDI (`FillRect`, `DrawText`, `FrameRect`) never
> writes the alpha byte of a pixel, so a freshly created 32-bit DIB is
> `alpha = 0` and `UpdateLayeredWindow` + `AC_SRC_ALPHA` renders *nothing*.
> Call `yasb_theme::force_opaque_alpha(bits, w, h)` after all GDI work and
> before `UpdateLayeredWindow`. Same bug bit `saka-popup`; same fix.

### Tests

`cargo test --release` → 17 tests (5 catalog, 3 theme, 9 layout).

## palette-picker.py — documented fallback

The original PyQt6 picker (418 lines) is **kept on disk as a fallback** but
is no longer wired to the bar. It behaves the same way (dmenu cards, Enter
applies via `yasb-theme.exe`) but needs a Python + PyQt6 install and pulls
in a large dependency tree, which is why the Rust twin exists.

To go back to it, point the `palette` widget's `on_left` at
`palette-picker.py` (or the prebuilt `palette-picker.exe` PyInstaller
artifact) instead of the Rust release exe.

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
| `palette-picker/` | **primary** picker: Rust crate (lib-less, 4 modules) + `Cargo.toml` |
| `palette-picker/capture-screen.py` | `BitBlt`+`CAPTUREBLT` screenshot that works on layered windows |
| `palette-picker/capture-window.py` | per-window capture helper |
| `palette-themes.json` | theme catalog shown as cards (11 dark + 11 light) |
| `palette-picker.py` | fallback picker source (PyQt6) |
| `palette-picker-build.ps1` / `.spec` / `.bat` | PyInstaller build wrapper for the fallback |
| `palette-picker.exe` | prebuilt PyInstaller artifact of the fallback |
| `yasb-font.spec` / `yasb-font.exe` | font picker spec + binary (source missing, see above) |

Binaries and `target/` are gitignored; the Rust sources and this document
are the authoritative record of the tool's behaviour.