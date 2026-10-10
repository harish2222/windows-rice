# saka-popup — the panchangam panel

A frameless, always-on-top panel that shows the full Telugu panchangam: tithi
with a progress bar, nakshatra, yoga, karana, vara, the moon phase and its
illumination, the start and end of that phase as a date *and* a time, the
amanta and purnimanta months, the Saka and Vikram Samvat eras, sunrise/sunset,
and the next new and full moons. It refreshes every second and re-reads the
active theme on every tick.

Eleven rows, laid out in one place (`src/layout.rs`) so the shape pass and the
text pass cannot disagree: 496×591 Latin, 520×701 Telugu. The widths are
measured, not round numbers — see [`Layout::W`] for the arithmetic that rules
out 460.

## Why it is a separate window

YASB's `yasb.custom.CustomWidget` **has no popup support**. Its schema exposes
only `class_name`, `label`, `label_alt`, `tooltip`, `exec_options`, `keybindings`
and `callbacks` — the `*_menu` popup options exist only on the built-in widget
types (clock, volume, wifi, komorebi layout, …). So a themed panel attached to
a custom widget cannot be a native YASB popup and has to be its own top-level
window. That is what this is, and it is the same pattern the palette picker has
always used.

The window is a GDI window: the desktop behind it is captured before it is
shown, the theme colour is blended over that capture, and the result is
blitted to the window. That is how it gets a translucent material without a
compositor — matching the bar's own "no blur anywhere" policy.

## Controls

| Input | Action |
|---|---|
| **Tab** | toggle Telugu / English script |
| **Escape** | close |
| **Left or right click** | close |
| (outside click / focus loss) | close |

## Following the theme

Every colour comes from the **active theme block** in `styles.css`, read with
the same rule the palette picker uses: the active block is the one under a
header comment `/* Name - active */`, and it runs until the next `/* … */`.
That is the documented block contract in the ACRYLIC POLICY comment at the top
of `:root`, and it is why a theme switch recolours the popup with no restart.

Colours used: `--acrylic` (falling back to `--glassmenu`) for the fill,
`--text`, `--subtext`, `--text-faint`, `--border`, `--surface0`, `--accent`
(falling back to `--mauve`) and `--background2`.

Two deliberate departures from a plain QSS popup:

* **Alpha floor.** The Rangalipi themes ship `--acrylic` at 1–2% alpha, which is
  right for a YASB popup sitting on the desktop but would make this window
  essentially invisible. The floor is **derived, not constant**: it is the least
  opacity at which the theme's own `--text` and `--subtext` still clear 4.5:1
  and 4.0:1 against *both* a dark and a light desktop, found by binary search
  in `min_alpha_for_contrast` (`src/theme.rs`).

  The old rule was a flat `if bg.a < 235 { bg.a = 235 }`, tuned for the 1%
  Wine block and then applied to all 22 themes — including the light ones that
  deliberately author 50%. The result was a panel *brighter than the
  wallpaper* behind it, which is what "a patch of white cement stuck onto the
  wall" describes. The derived floor lands at 0.698 for Rangalipi Wine Light,
  with the wallpaper contributing 30% rather than 8%.
* **Fonts.** Segoe UI carries the Latin text; Nirmala UI is selected when the
  Telugu script is toggled on, because GDI does no automatic font fallback for
  missing glyphs.

## The panel is not layered (read before touching the paint code)

An earlier version of this panel was a `WS_EX_LAYERED` window presented with
`UpdateLayeredWindow` + `AC_SRC_ALPHA`. It reported success and put **nothing**
on screen: GDI writes RGB but never the alpha byte, so a fresh 32-bit DIB is
`alpha = 0` everywhere and the compositor drops the whole surface.

Rather than patch that path, the panel stopped being layered. It now captures
the desktop (`Backdrop::capture`, optionally blurred), blends the theme colour
over it into a DIB, and `blit_to_window`s the result — so every pixel carries a
real alpha byte and nothing depends on GDI writing one. `yasb-theme`'s
`force_opaque_alpha` still exists for anything that *does* present through
`UpdateLayeredWindow`, but this panel no longer calls it.

The same class of bug also blocks screenshots: `PIL.ImageGrab` and
`PrintWindow`+`PW_RENDERFULLCONTENT` both miss these windows. Use `BitBlt`
with `CAPTUREBLT` (`0x40000000`) from the screen DC — which is what
`tools/screen-shot` does.

## Building

```sh
cd tools/saka-popup
cargo build --release          # -> target/release/saka-popup.exe
```

The popup crate depends on the engine crate by path (`../saka`), so the panel
and `saka.exe --panchangam` can never disagree about the numbers. There is no
build script or install step; `config.yaml` points straight at the release
binary.

## Self-test

```sh
saka-popup.exe --smoke
```

Parses the live theme and renders every row without creating a window, so a
broken build is caught without opening something on your desktop. Output looks
like:

```
theme bg=Rgba { r: 29, g: 16, b: 21, a: 203 } text=... border=...
tithi=22 nak=5 yoga=17 karana=2 vara=6
moon phase="Krishna Panchami" western="Waxing Crescent" illum=37%
  Tithi       Krishna Chaturdashi
  Nakshatra   Ardra
  ...
  Phase       09 Oct 01:12 · 12 Oct 19:08
  ...
  fit Tithi       value  200px in 218  caption   94px in 94
  ...
smoke: ok (11 rows, panel 496x591)
```

The `fit` lines are the point: `paint` hands both the value and the caption
`DT_END_ELLIPSIS`, so a row that does not fit shows up on screen as a trailing
`...` and nowhere else. Measuring every shipped string with the same faces and
the same `Layout::row_split` the paint pass uses is what turns a truncating row
into a failed self-test instead of a silent defect. That is how the tithi end
time's `...` was found: the panel was 460px wide and the column was 16px short.

`--te` starts in Telugu instead of Latin.

## Tests

`cargo test` covers the theme layer (the active block is selected and inactive
blocks cannot leak values, every CSS colour form parses, the derived alpha
floor clears 4.5:1 and 4.0:1 over a dark *and* a light desktop, and a missing
`styles.css` falls back to a default palette instead of panicking) and the
layout layer (no band overlaps at any row count, every label fits the label
column at its drawn size, no shipped row needs an ellipsis, and a caption is
either whole or absent — never a stub). The engine's own tests live in
`../saka` (`tests/panchang.rs`), where the elements are pinned to Drik Panchang
and the phase bracket is checked against the 45° sector edges.
`src/theme.rs` re-exports `Rgba`, `parse_color` and `active_theme_vars` straight
from the shared `yasb-theme` library rather than re-implementing them.

## Known limits

* Elements are the five classical ones plus vara. Rahu Kaal, Yamaganda,
  Gulika, Abhijit and the muhurta tables are not computed — they need the local
  lagna, which is a different calculation.
* Adhika masa (the intercalary month) is not detected; see the reasoning in
  `../saka/saka.md`.
* Location is hard-coded to Hyderabad/IST in `main.rs` (`LAT`, `LON`, `TZ`).
  Sunrise, sunset and the praayana tithi are the only location-dependent values
  shown.