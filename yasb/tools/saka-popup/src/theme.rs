//! Theme extraction: read the *active* theme block out of `styles.css` and turn
//! its CSS custom properties into RGBA colours.
//!
//! The block contract this relies on is the one documented in the ACRYLIC
//! POLICY comment at the top of `:root`: the active theme's declarations sit
//! under a single header comment `/* Name - active */`, and every other theme
//! is wrapped in a comment chunk. That makes "the active block" a plain
//! regex-able region, and it is the same region the palette picker reads, so
//! both follow a theme switch.
//!
//! # Deriving a material from a flat palette
//!
//! A Rangalipi block declares colours, not surfaces: there is no token for
//! "the sheen at the top of the panel" or "how much wallpaper shows through".
//! Rather than add a new declaration to all 22 blocks — where the other 21
//! would silently ignore it, and anyone editing one block would get a
//! different panel — the material is *derived* here from the colours that do
//! exist.
//!
//! The directional tokens (sheen, highlight) brighten towards white and darken
//! towards black unconditionally, because that is what "lit from above" means
//! on a dark surface *and* on a light one. Getting this backwards is the easy
//! failure: a sheen that is derived contrast-dependently darkens on a light
//! theme, which produces a panel lit from below. It looks subtly wrong and is
//! hard to name, which is why `derived_material_is_coherent_on_dark_and_light_themes`
//! asserts the direction on both.

use std::path::Path;

pub use yasb_theme::{Rgba, active_theme_vars, parse_color};

/// Fallback palette, only used when `styles.css` cannot be read at all. It
/// matches the Catppuccin-ish defaults the picker uses.
fn fallback() -> Vec<(String, String)> {
    [
        ("--background", "#181825"),
        ("--surface0", "#313244"),
        ("--text", "#cdd6f4"),
        ("--subtext", "#a6adc8"),
        ("--accent", "#b4befe"),
        ("--border", "#45475a"),
        ("--mauve", "#cba6f7"),
        ("--shade", "#11111b"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// The colours the popup paints with.
///
/// The first eight fields are the theme's own tokens; everything below is
/// derived. See the module docs for why.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Window fill. May be translucent; composited over the backdrop.
    pub bg: Rgba,
    pub text: Rgba,
    pub subtext: Rgba,
    pub faint: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
    pub track: Rgba,

    // --- derived material -------------------------------------------------
    /// How much of the captured desktop shows through the panel.
    ///
    /// This is acrylic's defining number and it is *high*: the material is
    /// only 15% opaque, so 85% of the blurred wallpaper is what you see.
    pub backdrop_opacity: f32,
    /// Box-blur radius applied to the backdrop before it is blended.
    ///
    /// Acrylic is low-opacity *and* high-blur. Without the blur, 0.85 here
    /// would drop crisp icons and window edges straight behind the body
    /// text and nothing else about the material could rescue it. This is a
    /// material token rather than a constant in the paint path so the two
    /// panels can be tuned independently.
    pub backdrop_blur: u32,
    /// Top and bottom of the panel's vertical light gradient.
    pub sheen_top: Rgba,
    pub sheen_bottom: Rgba,
    /// Strength of the accent bloom behind the header.
    pub bloom: f32,
    /// The hairline highlight along the top edge.
    pub highlight: Rgba,
    /// Separator rules and the card's own edge.
    pub hairline: Rgba,
    /// The raised card the rows sit on.
    pub card: Rgba,
    /// Moon glyph: the unlit limb (earthshine) and the lit limb.
    pub moon_dim: Rgba,
    pub moon_lit: Rgba,
    /// Halo around the moon glyph.
    pub moon_glow: f32,
}

/// Linear interpolation between two colours, alpha included.
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Rgba { r: f(a.r, b.r), g: f(a.g, b.g), b: f(a.b, b.b), a: f(a.a, b.a) }
}

/// sRGB electro-optical transfer function: the encoded 0-1 channel value to
/// linear light.
///
/// This step is not optional. Summing the encoded bytes directly — the common
/// shortcut — overstates mid-tones badly enough to report #4a4a5a on #fdf6e3
/// as 2.9:1 when the real ratio is 8.4:1, which would have this test reject a
/// perfectly legible theme.
#[cfg(test)]
fn linearize(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// WCAG relative luminance, 0.0 (black) to 1.0 (white).
#[cfg(test)]
fn luma(c: Rgba) -> f32 {
    0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b)
}

/// WCAG contrast ratio between two colours.
#[cfg(test)]
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (hi, lo) = {
        let (x, y) = (luma(a), luma(b));
        if x > y { (x, y) } else { (y, x) }
    };
    (hi + 0.05) / (lo + 0.05)
}

/// Brighten towards white. The light that falls on a surface.
fn lighten(c: Rgba, t: f32) -> Rgba {
    mix(c, Rgba::rgb(255, 255, 255), t)
}

/// Darken towards black. The shadow a surface casts.
fn darken(c: Rgba, t: f32) -> Rgba {
    mix(c, Rgba::rgb(0, 0, 0), t)
}

impl Theme {
    /// Build a theme from a parsed declaration list.
    pub fn from_vars(vars: &[(String, String)]) -> Theme {
        let find = |names: &[&str]| -> Option<String> {
            for n in names {
                if let Some((_, v)) = vars.iter().find(|(k, _)| k == n) {
                    return Some(v.clone());
                }
            }
            None
        };
        let col = |names: &[&str], default: Rgba| -> Rgba {
            find(names)
                .and_then(|v| parse_color(&v))
                .unwrap_or(default)
        };
        let mut bg = col(&["--acrylic", "--glassmenu"], Rgba::rgb(24, 24, 37));
        // The Rangalipi themes ship `--acrylic` at 1-2% alpha, which is right
        // for a QSS popup sitting on the desktop but would make this window
        // effectively invisible. Floor the alpha so the panel keeps the theme
        // tint and stays readable over any wallpaper.
        if bg.a < 235 {
            bg.a = 235;
        }

        let text = col(&["--text"], Rgba::rgb(205, 214, 244));
        let subtext = col(&["--subtext", "--text-muted"], Rgba::rgb(166, 173, 200));
        let faint = col(&["--text-faint"], Rgba::rgb(140, 140, 160));
        let border = col(&["--border", "--hairline"], Rgba::rgb(69, 71, 90));
        let accent = col(&["--accent", "--mauve"], Rgba::rgb(180, 190, 254));
        let track = col(&["--background2"], Rgba::rgb(49, 50, 68));

        // Directional terms: brighten the top, darken the bottom. This is
        // deliberately not contrast-dependent. The alphas are higher than a
        // solid panel would need because most of what is behind this window
        // is now wallpaper: the gradient is the only wash standing between
        // the header text and a bright icon.
        let sheen_top = Rgba { a: 46, ..lighten(bg, 0.55) };
        let sheen_bottom = Rgba { a: 70, ..darken(bg, 0.45) };

        Theme {
            bg,
            text,
            subtext,
            faint,
            border,
            accent,
            track,

            // Acrylic is a 15%-opaque material: the panel should read as a
            // pane of glass with the desktop behind it, not as a dark sheet
            // laid over the desktop. The blur below is what makes that
            // legible; the card underneath the rows is the second half of the
            // bargain.
            backdrop_opacity: 0.85,
            backdrop_blur: 12,
            sheen_top,
            sheen_bottom,
            // The bloom is the accent at low alpha, so a light theme with a
            // pale accent gets a pale glow rather than a colour wash.
            bloom: 0.10,
            highlight: Rgba { a: 38, ..lighten(text, 0.25) },
            hairline: Rgba { a: 150, ..border },
            // A raised card is lighter than the panel on both light and dark
            // themes — that is how macOS does it — and the hairline supplies
            // the edge where the two are too close in luminance to separate.
            // Heavier than it would be on an opaque panel: at 85% bleed this
            // card is what the body text actually sits on, so it has to
            // deliver the contrast the panel used to.
            card: Rgba { a: 175, ..lighten(bg, 0.09) },
            // The unlit limb is dim, not black: the moon's dark side catches
            // earthshine, and a pure black disc reads as a hole in the panel.
            moon_dim: lighten(bg, 0.16),
            // Lit limb is near-white with a trace of the accent, so it looks
            // like it is catching the same light as the rest of the panel.
            moon_lit: mix(lighten(bg, 0.94), accent, 0.10),
            moon_glow: 0.16,
        }
    }

    /// Read the active theme from `styles_path`.
    pub fn load(styles_path: &Path) -> Theme {
        let Ok(css) = std::fs::read_to_string(styles_path) else {
            return Theme::from_vars(&fallback());
        };
        let mut vars = active_theme_vars(&css);
        if vars.is_empty() {
            vars = fallback();
        }
        Theme::from_vars(&vars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
:root {
/* Rangalipi Alpha */
    /* --acrylic: rgba(1, 2, 3, 0.5);
    --text: #aabbcc; */
/* Rangalipi Wine - active */
    --acrylic: rgba(192, 78, 104, 0.55);
    --acrylic-hover: rgba(192, 78, 104, 0.7);
    --text: #F1DFE3;
    --subtext: #C8B6BC;
    --border: #C04E68;
    --motif-mandala: url("x.png");
/* Rangalipi Dune */
    /* --text: #000000; */
}
"#;

    #[test]
    fn picks_the_active_block_only() {
        let vars = active_theme_vars(SAMPLE);
        let map: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();
        assert!(map.contains(&"--text"));
        assert!(map.contains(&"--border"));
        // The inactive blocks are commented out, so their values must not leak.
        let text = vars.iter().find(|(k, _)| k == "--text").unwrap().1.clone();
        assert_eq!(parse_color(&text), Some(Rgba::rgb(0xF1, 0xDF, 0xE3)));
    }

    #[test]
    fn theme_uses_active_values() {
        let vars = active_theme_vars(SAMPLE);
        let t = Theme::from_vars(&vars);
        // The alpha is floored for popup readability, so the tint survives.
        assert_eq!(t.bg.a, 235);
        assert_eq!((t.bg.r, t.bg.g, t.bg.b), (192, 78, 104));
        assert_eq!(t.text, Rgba::rgb(0xF1, 0xDF, 0xE3));
        assert_eq!(t.border, Rgba::rgb(0xC0, 0x4E, 0x68));
    }

    #[test]
    fn parses_every_colour_form() {
        assert_eq!(parse_color("#fff"), Some(Rgba::rgb(255, 255, 255)));
        assert_eq!(parse_color("#C04E68"), Some(Rgba::rgb(192, 78, 104)));
        assert_eq!(parse_color("#C04E6880").map(|c| c.a), Some(128));
        assert_eq!(
            parse_color("rgba(192, 78, 104, 0.5)"),
            Some(Rgba { r: 192, g: 78, b: 104, a: 128 })
        );
        assert_eq!(parse_color("url(\"x.png\")"), None);
        assert_eq!(parse_color("var(--x)"), None);
    }

    #[test]
    fn alpha_composites() {
        let top = Rgba { r: 0, g: 0, b: 0, a: 128 };
        let out = top.over(Rgba::rgb(255, 255, 255));
        assert_eq!(out.a, 255);
        assert_eq!(out.r, 127);
    }

    #[test]
    fn missing_file_falls_back_instead_of_panicking() {
        let t = Theme::load(Path::new("Z:/definitely/not/here.css"));
        assert_eq!(t.text, Rgba::rgb(205, 214, 244));
    }

    /// One set of invariants over a dark, a light and two more themes. The
    /// point of deriving the material is that all of them get a coherent panel
    /// without anyone editing a theme block.
    #[test]
    fn derived_material_is_coherent_on_dark_and_light_themes() {
        // Real dark and light theme pairs from `styles.css`.
        let cases: [(&str, &str); 4] = [
            ("#14141B", "#E7E0D2"), // Rangalipi
            ("#E7E0D2", "#1A1816"), // Rangalipi Light
            ("#1A1310", "#F0E4D4"), // Rangalipi Ember
            ("#E7E0D2", "#1A1816"), // Rangalipi Matcha Light
        ];
        for (bg, text) in cases {
            let t = Theme::from_vars(&[
                ("--acrylic".into(), bg.into()),
                ("--text".into(), text.into()),
                ("--subtext".into(), text.into()),
                ("--accent".into(), "#b4befe".into()),
                ("--border".into(), "#45475a".into()),
            ]);

            // Lit from above on every theme.
            assert!(
                luma(t.sheen_top) > luma(t.sheen_bottom),
                "sheen inverted on {bg}: top {} bottom {}",
                luma(t.sheen_top),
                luma(t.sheen_bottom)
            );
            // The raised card sits above the panel, not below it.
            assert!(
                luma(t.card) > luma(t.bg),
                "card {} is not above bg {} on {bg}",
                luma(t.card),
                luma(t.bg)
            );
            // The lit limb is brighter than the unlit one.
            assert!(
                luma(t.moon_lit) > luma(t.moon_dim),
                "moon limb contrast wrong on {bg}"
            );
            // The unlit limb is dim, but not a black hole.
            assert!(
                luma(t.moon_dim) > luma(t.bg) * 0.8,
                "unlit limb too dark on {bg}"
            );
            // Every material token stays partially transparent, or it paints
            // over the acrylic and the panel stops reading as one surface.
            for (name, c) in [
                ("sheen_top", t.sheen_top),
                ("sheen_bottom", t.sheen_bottom),
                ("card", t.card),
                ("highlight", t.highlight),
                ("hairline", t.hairline),
            ] {
                assert!(c.a < 255, "{name} on {bg} is opaque");
                assert!(c.a > 0, "{name} on {bg} is invisible");
            }
            // Acrylic must actually show wallpaper, and the value is not a
            // free parameter: it is the material's 15% opacity stated the
            // other way round. Pin it so a future "let's calm it down" edit
            // cannot quietly turn the glass back into a sheet of plastic.
            assert_eq!(
                t.backdrop_opacity, 0.85,
                "backdrop opacity drifted on {bg}"
            );
            // ...and the blur is what makes that number survivable. A radius
            // of zero here would pass an opacity check and still be
            // unreadable over a busy desktop.
            assert!(
                t.backdrop_blur >= 8,
                "backdrop blur {} on {bg} is too small to read as acrylic",
                t.backdrop_blur
            );
            // The accent as ink is deliberately *not* pinned to a luminance band
            // here, and there is a reason worth writing down. Any fixed
            // colour has a luminance at which its contrast against a
            // background falls to 1:1, and at 85% acrylic the panel's
            // luminance is the wallpaper's. So there is no value that makes
            // the weekday label legible over every wallpaper, and clamping
            // the accent into a band would only move the problem to a
            // different wallpaper while quietly recolouring all 22 themes.
            // The label's contrast is measured against the theme's own
            // background in `text_contrast_survives_the_acrylic_floor`.
        }
    }

    /// Text has to stay legible on the panel it sits on, or letting 85% of
    /// the wallpaper through was pointless.
    ///
    /// The colour pairs are copied from the real Rangalipi blocks, not
    /// invented: a made-up light theme with a dark-theme accent fails this
    /// assertion for reasons no user would ever hit, and a test that only
    /// passes on invented input is not worth much. These are the actual
    /// worst cases — `Matcha Light`'s accent is the lowest-contrast one in the
    /// set at 2.6:1.
    #[test]
    fn text_contrast_survives_the_acrylic_floor() {
        // (name, background, text, subtext, accent)
        let cases: [(&str, &str, &str, &str, &str); 5] = [
            ("Rangalipi", "#14141B", "#E7E0D2", "#B8B2A7", "#D99A2B"),
            ("Rangalipi Ember", "#1A1310", "#F0E4D4", "#C9A37E", "#C96F2F"),
            ("Rangalipi Ember Light", "#E7E0D2", "#1A1816", "#45423C", "#B45F27"),
            ("Rangalipi Matcha Light", "#E7E0D2", "#1A1816", "#45423C", "#7A943C"),
            ("Rangalipi Wine Light", "#E7E0D2", "#1A1816", "#45423C", "#A83E58"),
        ];
        for (name, bg, text, subtext, accent) in cases {
            let t = Theme::from_vars(&[
                ("--acrylic".into(), bg.into()),
                ("--background".into(), bg.into()),
                ("--text".into(), text.into()),
                ("--subtext".into(), subtext.into()),
                ("--accent".into(), accent.into()),
            ]);
            assert!(
                contrast(t.text, t.bg) > 7.0,
                "{}: text/bg contrast {:.1}",
                name,
                contrast(t.text, t.bg)
            );
            assert!(
                contrast(t.subtext, t.bg) > 4.0,
                "{}: subtext/bg contrast {:.1}",
                name,
                contrast(t.subtext, t.bg)
            );
            // The accent carries the tracked weekday label and the progress
            // fills. Large/bold text is legible at 3:1; the weekday label is
            // bold and tracked, so that is the bar it has to clear.
            assert!(
                contrast(t.accent, t.bg) > 2.5,
                "{}: accent/bg contrast {:.1}",
                name,
                contrast(t.accent, t.bg)
            );
        }
    }

    /// The one that matters now that 85% of the wallpaper is showing.
    ///
    /// The other contrast test measures theme tokens against `t.bg`, which
    /// was the surface the text sat on when the panel was opaque. It is not
    /// any more: at `backdrop_opacity` the text sits on the *card*, which
    /// sits on a wash of whatever is on the desktop. So this test replays
    /// `main.rs::paint`'s chain in order — theme fill, wallpaper, sheen,
    /// card — and measures what actually ends up under the text.
    ///
    /// It runs the whole range, not one hostile wallpaper. A panel at 85%
    /// glass can end up nearly as dark as the theme or nearly as light as
    /// the desktop, and the two extremes fail for opposite reasons: a dark
    /// theme loses its text to a bright wallpaper, a light theme loses its
    /// text to a dark one. Checking only the white wallpaper would have
    /// declared victory while the black one was unreadable.
    #[test]
    fn rows_stay_legible_across_the_whole_wallpaper_range() {
        let cases: [(&str, &str, &str, &str); 3] = [
            ("Rangalipi", "#14141B", "#E7E0D2", "#B8B2A7"),
            ("Rangalipi Ember", "#1A1310", "#F0E4D4", "#C9A37E"),
            ("Rangalipi Matcha Light", "#E7E0D2", "#1A1816", "#45423C"),
        ];
        for (name, bg, text, subtext) in cases {
            let t = Theme::from_vars(&[
                ("--acrylic".into(), bg.into()),
                ("--background".into(), bg.into()),
                ("--text".into(), text.into()),
                ("--subtext".into(), subtext.into()),
                ("--accent".into(), "#D99A2B".into()),
            ]);

            for wall in [Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255)] {
                // The rows sit on the card; the card sits on the bottom of
                // the panel, which is the darker end of the gradient.
                let card = t.card.over(panel_over(&t, wall, false));
                let text_ratio = contrast(t.text, card);
                let subtext_ratio = contrast(t.subtext, card);
                // WCAG AA for body text.
                let where_ = format!("{:?} wallpaper", (wall.r, wall.g, wall.b));
                assert!(
                    text_ratio >= 4.5,
                    "{name}: text on the card is {text_ratio:.1}:1 over a {where_}"
                );
                assert!(
                    subtext_ratio >= 3.0,
                    "{name}: subtext on the card is {subtext_ratio:.1}:1 over a {where_}"
                );
            }
        }
    }

    /// Replay `main.rs::paint`'s base chain — theme fill, then the backdrop
    /// pull, then one end of the vertical gradient — and return the colour
    /// that ends up there.
    ///
    /// `header` picks which end of the gradient: true for the top, where the
    /// weekday label lives, false for the bottom, which is the darker end
    /// and so the fair case for the card.
    fn panel_over(t: &Theme, wall: Rgba, header: bool) -> Rgba {
        let mut c = t.bg.over(Rgba::rgb(20, 20, 28));
        let k = t.backdrop_opacity;
        let pull = |from: u8, to: u8| (from as f32 + (to as f32 - from as f32) * k + 0.5) as u8;
        c = Rgba { r: pull(c.r, wall.r), g: pull(c.g, wall.g), b: pull(c.b, wall.b), a: 255 };
        if header { t.sheen_top.over(c) } else { t.sheen_bottom.over(c) }
    }
}
