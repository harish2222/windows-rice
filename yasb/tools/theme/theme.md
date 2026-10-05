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
  and `::step`, `write_atomic`, `sync_config_colors`, `sync_cava_colors`,
  plus the colour helpers `Rgba`, `parse_color` and `active_theme_vars`.
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

### `--motif` — the panel artwork

The control-center, media and pomodoro panels paint a motif via
`background-image: var(--motif)`. Declaring that variable in only the
*active* block would make the art vanish on every theme switch, so **all 22
blocks declare it**.

Three motifs are drawn at random from the `motif-*.svg` files in the yasb
folder and dealt out one per theme block, so switching themes changes the
artwork without every theme looking identical:

```sh
C:\Users\haris\.config\yasb\tools\theme\yasb-theme.exe motif
```

`yasb-theme motif` discovers the SVGs by globbing `motif-*.svg`, so dropping
a new file into that folder is enough to make it eligible — there is no
hard-coded list to update. Only the `--motif:` declaration of each block is
rewritten, and the match is on the whole key: the key is stripped and the
value skipped if it continues with a `-`. That guard is not incidental —
`--motif-mandala:` shared the prefix, so a bare prefix match would have
rewritten the wrong variable and pointed a panel at art it was never meant
to draw. `assign_rewrites_motif_but_never_motif_mandala` in `motifs.rs` is
the test that holds that line.

The palette picker runs `motif` after every `set`, so applying a theme also
rerolls the three motifs.

**Opacity is baked into the asset, not the stylesheet.** Each `motif-*.svg`
carries `opacity="0.4"` on its root element. Qt paints `background-image` at
full strength, so a QSS `opacity` on the panel cannot dim the art — it has to
be in the file.

### mandala-gen — retired

`mandala-gen/` and its 22 `motif-*-mandala.png` files are **gone**. The
panels moved to the local SVG motifs above, after which nothing referenced
either the PNGs or the `--motif-mandala:` declarations that named them, so
the generator and both were removed rather than left as dead weight.

The distinctness contract it enforced — 22 byte-unique files, pairwise at
least `MIN_PATTERN_DISTANCE` bits apart on a 64-bit dHash so two designs
differing only in tint would not slip through — does not carry over, because
the motifs are now hand-drawn and are not generated per theme. Nothing
checks that the 12 SVGs are visually distinct from one another; if that
matters, it wants a test of its own rather than the deleted generator's.

The legacy `--motif-corner` variable and its `*-mandala-corner.png` art were
removed earlier, for the same reason: nothing consumed them once the panel
had a background image at all.

## Wrappers and shell helper

| File | Role |
|---|---|
| `yasb-theme/src/` | crate source (zero deps; rescued into the repo from a scratch clone) |
| `motifs.rs` | discovers `motif-*.svg`, draws 3 at random and deals them out across the 22 theme blocks (`yasb-theme motif`) |
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

- `cava` — the bar widget's `foreground`/`gradient_color_1..3` from
  `teal`/`blue`/`mauve`/`peach`.
- `pomodoro` — `circle_work_progress_color` = `mauve`,
  `circle_break_progress_color` = `teal`. The native PomodoroWidget paints
  that ring itself, so the config hex is the only handle on it.

### The standalone cava app

`~/.config/cava/config` is a *different file* from YASB's `cava` widget
block: it belongs to the cava program itself, which is the visualiser that
actually runs here. Every `set`/`next`/`prev` rewrites its `[color]` section
too, via `sync_cava_colors`:

- `foreground` — `frame`.
- `gradient_color_1..8`, bass to treble — `frame`, `red`, `peach`, `yellow`,
  `green`, `teal`, `sapphire`, `sky`.

The order is fixed across themes on purpose: a visualiser whose colours
both reshuffle *and* reorder on every switch stops being readable as a
level meter, because you learn the mapping by position.

cava runs with `live-config = 1`, so it picks the change up without a
restart. Only the nine colour values are touched — indentation, quote
style and comments in that file are left byte-for-byte alone, so a
hand-edited config does not get reformatted by a theme switch. A missing
file is not an error, and a config with no `[color]` section is skipped
rather than having one invented for it.

Override the path with `--cava PATH` if cava lives somewhere unusual.

Rebuild after touching the crate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\tools\theme\yasb-theme-build.ps1
```
