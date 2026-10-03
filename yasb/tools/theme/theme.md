# theme — the bar's theme-switch engine

The machinery behind the palette chip in the omega dropdown: click it and
the whole bar recolours itself between the Rangalipi themes. Two halves:

## yasb-theme (Rust crate, this folder)

`yasb-theme/src/lib.rs` — every theme lives as a `/* <Name> */` block of
CSS variables inside `:root` in `styles.css`. The tool toggles those
comment markers **byte-safely**: it preserves the file's newline style,
trailing newline, and UTF-8 no-BOM encoding.

The crate is split so other tools can reuse the parsing instead of
re-implementing it:

- `src/lib.rs` — `parse`/`parse_header`/`is_var_line`, `Stylesheet::set`
  and `::step`, `write_atomic`, `sync_config_colors`, plus the colour
  helpers `Rgba`, `parse_color` and `active_theme_vars`.
- `src/main.rs` — the thin `yasb-theme` CLI over that library.

`tools/saka-popup` links the library for `Rgba`/`parse_color`/
`active_theme_vars` so its window themes off the same active block the
switcher writes.

```sh
yasb-theme.exe list                 # all themes, * marks active
yasb-theme.exe current              # active theme name (the bar polls this)
yasb-theme.exe set "Rangalipi Wine" # activate by full name (case-insensitive)
yasb-theme.exe next                 # cycle forward  (wraps)
yasb-theme.exe prev                 # cycle backward
```

### Block contract (do not break this when editing styles.css)

Each theme block inside `:root` follows exactly one shape:

- **Header** — a single-line `/* Rangalipi <Variant> */` comment; only the
  active block carries the extra ` - active` suffix. Headers must stay
  name-like (letters/digits/spaces/hyphen): the parser rejects anything
  else, so declaration comments (`--acrylic: ...`) and prose annotations
  can never be mistaken for themes.
- **Inactive block** — *one* comment chunk: the opener sits on the
  `--acrylic` line, the closer on the `--runner` line. Everything between
  (hover, background, holiday, motifs…) is bare text inside that chunk.
- **Active block** — every variable line fully bare, no comment markers at
  all.
- **Never** add inline `/* … */` annotations to variable lines or
  standalone single-line comments inside a block: an early `*/` would cut
  the chunk in half and desync the switcher. Record palette exceptions in
  the multi-line ACRYLIC POLICY comment at the top of `:root` instead.

Writes are **atomic** (temp file + rename): YASB's `watch_stylesheet` and
the palette's 10 s `current` poll only ever see a complete stylesheet, so
a switch can no longer reload a half-written file and drop every theme
variable.

### `--motif-mandala` must exist in every block

The control-center, home and pomodoro panels paint the full-panel mandala
via `background-image: var(--motif-mandala)`. Declaring that variable in
only the *active* block meant the art vanished on every theme switch, so
**all 22 blocks now declare it**, each pointing at its own
`motif-<stem>-mandala.png`. The stem is derived from the block's existing
`--motif: url(...motif-<stem>.svg)`, so a block and its art cannot drift
apart. `add-mandala-var.py` performs the insertion (idempotent) and
`verify-mandala.py` walks all 22 themes asserting exactly one active block,
a bare `--motif-mandala` inside it, an existing PNG, and a byte-identical
round-trip afterwards:

```sh
python tools/theme/add-mandala-var.py    # add the missing declarations
python tools/theme/verify-mandala.py     # assert all 22 themes behave
```

Note when scripting edits to `styles.css` from Python: open it with
`newline=""`. The default text mode rewrites every LF as CRLF on Windows
and turns a one-line edit into a whole-file diff.

The older `--motif-corner` variable (and its `*-mandala-corner.png` art) is
unused now that the mandala fills the panel; it is dead weight but harmless,
kept so an older cached stylesheet still paints something.

## Wrappers and shell helper

| File | Role |
|---|---|
| `yasb-theme/src/` | crate source (zero deps; rescued into the repo from a scratch clone) |
| `yasb-theme-build.ps1` | rebuild + deploy: `cargo build --release`, copies the exe here, smoke-tests `current` (refreshes `silent-run` in scoop shims only if this crate ever builds one again) |
| `yasb-theme-current.bat` / `-next.bat` / `-prev.bat` | one-line wrappers used by bar callbacks and keyboard launchers |
| `yasb-theme-shell.ps1` | dot-source from your PowerShell profile to get `yt list / set / next / prev` |

## Bar wiring

The `palette` CustomWidget in `config.yaml` polls
`yasb-theme.exe current` every 10 s; left-click opens the graphical
palette picker (see `picker.md`), right-click runs `yasb-theme.exe next`.
Every `set`/`next`/`prev` also rewrites two sections of `config.yaml` from
the new theme's tokens, because YASB reads those colours straight from the
config where no CSS variable can reach them:

- `cava` — `foreground`/`gradient_color_1..3` from
  `teal`/`blue`/`mauve`/`peach`, so the visualizer follows the palette.
- `pomodoro` — `circle_work_progress_color` = `mauve`,
  `circle_break_progress_color` = `teal`. The native PomodoroWidget paints
  that ring itself, so the config hex is the only handle on it.

Rebuild after touching the crate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\tools\theme\yasb-theme-build.ps1
```
