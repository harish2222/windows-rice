# saka-popup — the panchangam panel

A frameless, always-on-top panel that shows the full Telugu panchangam: tithi
with a progress bar, nakshatra, yoga, karana, vara, moon phase and
illumination, the amanta and purnimanta months, the Saka and Vikram Samvat
eras, sunrise/sunset, and the next new and full moons. It refreshes every
second and re-reads the active theme on every tick.

## Why it is a separate window

YASB's `yasb.custom.CustomWidget` **has no popup support**. Its schema exposes
only `class_name`, `label`, `label_alt`, `tooltip`, `exec_options`, `keybindings`
and `callbacks` — the `*_menu` popup options exist only on the built-in widget
types (clock, volume, wifi, komorebi layout, …). So a themed panel attached to
a custom widget cannot be a native YASB popup and has to be its own top-level
window. That is what this is, and it is the same pattern the palette picker has
always used.

The window is a layered Win32 window painted with GDI and presented through
`UpdateLayeredWindow`, which is how it gets per-pixel alpha without a
compositor or a blur pass — matching the bar's own "no blur anywhere" policy.

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
  essentially invisible. The fill alpha is floored at 235/255, so the panel
  keeps the theme tint and stays readable over any wallpaper.
* **Fonts.** Segoe UI carries the Latin text; Nirmala UI is selected when the
  Telugu script is toggled on, because GDI does no automatic font fallback for
  missing glyphs.

## The alpha trap (read before touching the paint code)

Windows presented with `UpdateLayeredWindow` + `AC_SRC_ALPHA` are composited
**from the alpha byte of every pixel**. GDI — `FillRect`, `DrawText`,
`FrameRect` — writes RGB but never writes that alpha byte, so a freshly
created 32-bit DIB is `alpha = 0` across the whole surface and
`UpdateLayeredWindow` renders **nothing at all**: the window exists, is
topmost, and is invisible.

The fix lives in the shared theme library so the panel and the palette picker
cannot drift:

```rust
// tools/theme/yasb-theme/src/lib.rs
pub fn force_opaque_alpha(bits: *mut u8, w: i32, h: i32)
```

Call it **after** all GDI drawing and **before** `UpdateLayeredWindow`. The
debug assertion for this is one line: after painting, a DIB pixel at (0, 0)
must read `(r, g, b, 255)` — e.g. `(21, 16, 29, 255)` for Rangalipi Wine's
`--acrylic`. If the fourth byte is 0, this is why.

The same class of bug also blocks screenshots: `PIL.ImageGrab` and
`PrintWindow`+`PW_RENDERFULLCONTENT` both miss layered windows. Use `BitBlt`
with `CAPTUREBLT` (`0x40000000`) from the screen DC.

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
theme bg=Rgba { r: 29, g: 16, b: 21, a: 235 } text=... border=...
tithi=22 nak=5 yoga=17 karana=2 vara=6
  Tithi       Krishna Ashtami
  Nakshatra   Ardra
  ...
smoke: ok (10 rows)
```

`--te` starts in Telugu instead of Latin.

## Tests

`cargo test` covers the theme layer: the active block is selected and inactive
blocks cannot leak values, every CSS colour form parses, alpha composites
correctly, and a missing `styles.css` falls back to a default palette instead of
panicking. The engine's own tests live in `../saka` (`tests/panchang.rs`), where
the elements are pinned to Drik Panchang. `src/theme.rs` re-exports `Rgba`,
`parse_color` and `active_theme_vars` straight from the shared `yasb-theme`
library rather than re-implementing them.

## Known limits

* Elements are the five classical ones plus vara. Rahu Kaal, Yamaganda,
  Gulika, Abhijit and the muhurta tables are not computed — they need the local
  lagna, which is a different calculation.
* Adhika masa (the intercalary month) is not detected; see the reasoning in
  `../saka/saka.md`.
* Location is hard-coded to Hyderabad/IST in `main.rs` (`LAT`, `LON`, `TZ`).
  Sunrise, sunset and the praayana tithi are the only location-dependent values
  shown.