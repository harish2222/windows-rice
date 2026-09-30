# theme — the bar's theme-switch engine

The machinery behind the palette chip in the omega dropdown: click it and
the whole bar recolours itself between the Rangalipi themes. Two halves:

## yasb-theme (Rust crate, this folder)

`yasb-theme/src/main.rs` — every theme lives as a `/* <Name> */` block of
CSS variables inside `:root` in `styles.css`. The tool toggles those
comment markers **byte-safely**: it preserves the file's newline style,
trailing newline, and UTF-8 no-BOM encoding.

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
Every `set`/`next`/`prev` also rewrites the cava widget's four colour
lines in `config.yaml` from the new theme's `teal`/`blue`/`mauve`/`peach`
tokens, so the visualizer follows the palette automatically.
Rebuild after touching the crate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\tools\theme\yasb-theme-build.ps1
```
