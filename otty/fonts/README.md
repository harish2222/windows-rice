# Otty — User Fonts

Drop font files into **this folder** and Otty will pick them up automatically.

## Accepted formats

- `.ttf`  — TrueType
- `.otf`  — OpenType
- `.ttc`  — TrueType Collection
- `.otc`  — OpenType Collection

Other extensions are ignored.

## Adding a font

1. Move or copy a `.ttf` / `.otf` (etc.) file into this folder.
2. Open **Settings → Appearance** in Otty. The next time the panel opens,
   the font appears in the **Font Family** picker.
3. If Otty was already running with Settings open when you dropped the
   file, close and reopen the Settings panel to trigger a rescan.

Fonts dropped here are registered process-wide via CoreText, so you do
**not** need to install them in macOS Font Book.

## Choosing a font

In **Settings → Appearance → Font Family**:

- **Global** — saves the choice to `~/.config/otty/config.toml`. Applies
  everywhere unless a theme overrides it.
- **Theme** — saves into the active theme TOML. Travels with the theme
  if you share it.
- **Fallback** — comma-separated list used when the primary font lacks a
  glyph (e.g. CJK characters, Powerline icons).

You can also edit `~/.config/otty/config.toml` directly:

```toml
font-family = "Iosevka Term"
font-size   = 13
```

## More

Full docs: <https://docs.otty.sh/customization/fonts>
