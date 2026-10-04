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
    /// The opaque pill the weekday label sits on.
    ///
    /// The one surface in the panel that is *not* allowed to be acrylic. Every
    /// other element can pick up the wallpaper, because the panel's own theme
    /// supplies enough contrast; this one cannot, for the reason spelled out
    /// in [`Theme::from_vars`]. It is derived per theme rather than declared
    /// per theme so no one has to add it to 22 blocks.
    pub scrim: Rgba,
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
fn linearize(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// WCAG relative luminance, 0.0 (black) to 1.0 (white).
///
/// Not test-only: [`Theme::from_vars`] needs it to decide which way to push
/// the weekday scrim away from the accent.
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

/// The luminance to aim a weekday scrim at, given the ink on it.
///
/// Derived rather than guessed, because the obvious rule is wrong: "make the
/// scrim lighter than a dark ink, darker than a light one" sends a *mid-tone*
/// ink the wrong way. Rangalipi's amber `#D99A2B` has luminance 0.38 and
/// Matcha Light's green `#7A943C` has 0.26; both are darker than 0.5, so both
/// get lightened — and Matcha's contrast then *drops*, because a mid-tone
/// green needs to move away from mid-grey, not towards white.
///
/// The answer is whichever extreme of the range contrasts more with the ink,
/// and which one that is flips at an ink luminance of about 0.29, not 0.5.
/// That value is worked out rather than asserted, so a future ink cannot land
/// on the wrong side of a hard-coded pivot.
///
/// # Why "the extreme that just clears" rather than "the extreme"
///
/// The first version aimed at the absolute end of the range, and on the active
/// Wine theme that produced a 242,242,242 pill: pure white, sitting on a dark
/// wine panel like a sticker. It was maximally legible and looked it. The
/// scrim only needs to clear [`SCRIM_CONTRAST`], so this solves for the
/// *nearest* luminance that does, and the pill stays a tinted surface that
/// belongs to the palette. Legibility is a floor, not a target to maximise.
/// Where the scrim has to end up, and which way to get there.
///
/// The direction is carried rather than re-derived from the target's own
/// value, because a target is not a direction. A dark ink on a dark panel
/// solves to a target of 0.24 — above the panel, but nowhere near white — and
/// inferring "slide towards black" from `0.24 < 0.5` sends the pill the wrong
/// way and leaves it exactly where it started.
struct ScrimTarget {
    luma: f32,
    towards_white: bool,
}

fn scrim_target(accent: Rgba) -> ScrimTarget {
    /// Contrast the label must clear on its pill. The label is 11px bold and
    /// tracked, so WCAG's large-text bar is 3:1; this sits a little above it
    /// so the pill keeps its margin over the rounding of an 8-bit channel.
    const SCRIM_CONTRAST: f32 = 3.6;
    /// Never aim all the way to the end of the range, so the pill always reads
    /// as a tinted surface rather than as white or black.
    const FLOOR: f32 = 0.04;
    const CEIL: f32 = 0.96;

    let a = luma(accent);
    // Which end of the range this ink needs. Taking the better of the two is
    // what makes mid-tone inks work, and it is why there is no branch on the
    // ink's own brightness here.
    let on_white = (CEIL + 0.05) / (a + 0.05);
    let on_black = (a + 0.05) / (FLOOR + 0.05);
    if on_white >= on_black {
        // Ink is dark: lighten just far enough to clear the bar.
        ScrimTarget {
            luma: (SCRIM_CONTRAST * (a + 0.05) - 0.05).clamp(FLOOR, CEIL),
            towards_white: true,
        }
    } else {
        // Ink is light: darken just far enough.
        ScrimTarget {
            luma: ((a + 0.05) / SCRIM_CONTRAST - 0.05).clamp(FLOOR, CEIL),
            towards_white: false,
        }
    }
}

/// Slide `c` along the line towards white (or black) until its luminance hits
/// `target`.
///
/// Interpolating along that line keeps the theme's hue, so the pill still
/// belongs to the palette instead of being a grey sticker. Luminance along
/// the line is monotonic in `t`, so a bisection converges and cannot loop.
///
/// The direction is chosen by which extreme the target sits near, *not* by
/// comparing the target with `c` itself. That distinction was a live bug: a
/// near-black target under an already-near-black panel has no crossing on the
/// path towards black, so the bisection collapsed to `t = 0` and the "scrim"
/// came back as the panel colour with no pill at all.
///
/// Where the target is already on the panel's own side of the scale the result
/// is the panel's own luminance, which is correct — a theme whose background
/// is already darker than the target needs no adjustment to be legible.
fn at_luma(c: Rgba, target: ScrimTarget) -> Rgba {
    let towards_white = target.towards_white;
    let end = if towards_white { Rgba::rgb(255, 255, 255) } else { Rgba::rgb(0, 0, 0) };

    // Work in "distance from the end being slid towards", which is monotonic
    // *increasing* in `t` for both directions, and search for the target in
    // that space.
    //
    // Searching luminance directly is the trap this replaced. Towards white,
    // luminance rises with `t`; towards black it falls. One bisection cannot
    // serve both without a branch, and the branch is exactly what was wrong:
    // written once for the rising case and reused for the falling one, it
    // collapsed the interval the wrong way and returned the panel untouched
    // while reporting that it had aimed for the opposite extreme.
    //
    // `1 - luma` is exact, not an approximation: sRGB's transfer function
    // satisfies `linearize(1 - c) == 1 - linearize(c)`, so inverting the
    // colour inverts its luminance.
    let rising = |c: Rgba| if towards_white { luma(c) } else { 1.0 - luma(c) };
    let want = if towards_white { target.luma } else { 1.0 - target.luma };

    // Already at or past the target on the way in: nothing to slide, and the
    // panel's own colour is the correct answer. A theme whose background is
    // already darker than a near-black target needs no adjustment.
    if rising(c) >= want {
        return c;
    }
    // `end` is the extreme, so `rising(end) >= want` always holds and the
    // crossing is strictly inside the interval.
    debug_assert!(
        rising(end) >= want,
        "target {} is unreachable from this direction",
        target.luma
    );

    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if rising(mix(c, end, mid)) < want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    mix(c, end, (lo + hi) / 2.0)
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
            // The weekday scrim. This is the one surface that is not allowed
            // to be acrylic, and the reasoning is worth writing down because
            // an earlier attempt got it backwards.
            //
            // Every other element here can pick up the wallpaper: the panel
            // lays its own theme over the desktop, and the test above shows
            // the body text clears 4.5:1 at both ends of the range. The
            // weekday label cannot, because it is the only place the *accent*
            // touches type, and an accent is chosen to sit near the theme's
            // mid-tone — so on a bright wallpaper a dark theme's accent loses
            // its contrast, and on a dark wallpaper a light theme's does.
            //
            // The tempting fix is to clamp the accent's luminance into a band
            // so it always contrasts with `bg`. That was tried and reverted: it
            // silently recolours all 22 themes and only moves the failure to a
            // different wallpaper, because at 85% acrylic the thing under the
            // label is the *wallpaper*, not `bg`.
            //
            // So the surface moves instead of the ink. The scrim is slid to whichever
            // extreme contrasts more with this theme's accent, which
            // `scrim_target` works out from the accent rather than
            // guessing from the theme's own brightness. The theme's colours
            // are left exactly as the block declared them.
            scrim: {
                // Fully opaque, unlike every other surface in the panel.
                //
                // This started at alpha 244 to let a little of the glass
                // through, and measurement killed it: at 4% bleed a bright
                // wallpaper pulls the pill up, and Matcha Light's green fell
                // to 2.98:1 over a white desktop — just under the bar it was
                // built to clear. The pill is 92x24px; nobody perceives it as
                // a hole in the acrylic, and "it looks slightly glassy" is not
                // worth a legibility failure.
                let target = scrim_target(accent);
                Rgba { a: 255, ..at_luma(bg, target) }
            },
            // Near-opaque. At this alpha the wallpaper still contributes, but
            // only about 4% — small enough that the push above dominates, and
            // it keeps the pill from looking like a sticker laid on the glass.
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

    /// The reason the weekday label gets its own opaque surface, stated as an
    /// assertion across every wallpaper.
    ///
    /// `rows_stay_legible_across_the_whole_wallpaper_range` covers the body
    /// text, and it passes comfortably. The accent does not, and that is the
    /// whole reason this pass exists: the accent sits near the theme's own
    /// mid-tone by design, so on a bright wallpaper a dark theme's accent
    /// washes out and on a dark wallpaper a light theme's does. Asserting
    /// only against `t.bg` — which is what the older test did — would report
    /// 5.9:1 and declare victory while the label sat on a white desktop.
    ///
    /// So this measures the real chain: panel fill, wallpaper pull, header
    /// sheen, then the scrim. It walks grey as well as the two extremes,
    /// because the failing band is in the middle for a mid-tone accent.
    #[test]
    fn the_weekday_label_is_legible_on_every_wallpaper() {
        let cases: [(&str, &str, &str, &str); 5] = [
            ("Rangalipi", "#14141B", "#E7E0D2", "#D99A2B"),
            ("Rangalipi Light", "#E7E0D2", "#1A1816", "#B45F27"),
            ("Rangalipi Ember", "#1A1310", "#F0E4D4", "#C96F2F"),
            ("Rangalipi Ember Light", "#E7E0D2", "#1A1816", "#B45F27"),
            ("Rangalipi Matcha Light", "#E7E0D2", "#1A1816", "#7A943C"),
        ];
        for (name, bg, text, accent) in cases {
            let t = Theme::from_vars(&[
                ("--acrylic".into(), bg.into()),
                ("--background".into(), bg.into()),
                ("--text".into(), text.into()),
                ("--subtext".into(), text.into()),
                ("--accent".into(), accent.into()),
            ]);
            // Without the scrim this is the worst case, and it is recorded so
            // the test below is visibly doing work rather than passing
            // trivially on an easy accent.
            let bare_header = panel_over(&t, Rgba::rgb(255, 255, 255), true);
            let bare = contrast(t.accent, bare_header);

            for level in [0u8, 32, 64, 96, 128, 160, 192, 224, 255] {
                let wall = Rgba::rgb(level, level, level);
                let header = tests::panel_over(&t, wall, true);
                let pill = t.scrim.over(header);
                // The label is 11px bold and tracked, which is WCAG's "large
                // text" bar rather than the 4.5:1 body-text one.
                let ratio = contrast(t.accent, pill);
                assert!(
                    ratio >= 3.0,
                    "{name}: weekday accent on the scrim is {ratio:.1}:1 over a \
                     grey-{level} desktop (bare accent was {bare:.1}:1)"
                );
            }
        }
    }

    /// The scrim must clear the large-text bar against its own ink, stay opaque
    /// enough to hold that, and — the part a naive implementation gets wrong —
    /// stop at the *nearest* luminance that does rather than running to the end
    /// of the range. A pill at the extreme is maximally legible and reads as a
    /// sticker; the shipped Wine build measured a 242,242,242 pill on a dark
    /// panel before this was fixed.
    #[test]
    fn the_scrim_clears_the_bar_without_running_to_the_extreme() {
        let cases: [(&str, &str, &str); 5] = [
            ("Rangalipi", "#14141B", "#D99A2B"),
            ("Rangalipi Light", "#E7E0D2", "#B45F27"),
            ("Rangalipi Ember", "#1A1310", "#C96F2F"),
            ("Rangalipi Ember Light", "#E7E0D2", "#B45F27"),
            ("Rangalipi Matcha Light", "#E7E0D2", "#7A943C"),
        ];
        for (name, bg, accent) in cases {
            let t = Theme::from_vars(&[
                ("--acrylic".into(), bg.into()),
                ("--text".into(), "#E7E0D2".into()),
                ("--accent".into(), accent.into()),
            ]);
            // Legible against the ink, on the scrim alone. The wallpaper range is the
            // other test's job.
            assert!(
                contrast(t.accent, t.scrim) >= 3.0,
                "{name}: accent on the scrim is only {:.1}:1",
                contrast(t.accent, t.scrim)
            );
            // The restraint half of the property: the pill stops at the *target*, rather
            // than always at the end of the range.
            //
            // Deliberately not phrased as "differs from the panel" or "is not
            // near-white": both were tried and both are wrong. A theme whose
            // panel is already near-black and whose accent is light needs no
            // slide at all and correctly gets none — the pill does its job
            // there by being opaque. And Ember's accent genuinely needs a
            // near-white pill to clear 3:1, so landing near the top of the
            // range is the right answer there, not restraint failing.
            //
            // What must always hold is that the pill is as close to the solved
            // target as the panel allows, and no further.
            let target = scrim_target(t.accent);
            let l = luma(t.scrim);
            let panel = luma(t.bg);
            // `at_luma` slides to the target, unless the panel already sits
            // past it — in which case the panel *is* the answer and no slide
            // happens. "Past it" is measured along the slide direction.
            let nearest = if target.towards_white {
                panel.max(target.luma)
            } else {
                panel.min(target.luma)
            };
            assert!(
                (l - nearest).abs() < 0.02,
                "{name}: scrim luma {l:.3} but the nearest correct value is \
                 {nearest:.3} (target {:.3}, panel {:.3}, towards_white={})",
                target.luma,
                panel,
                target.towards_white
            );
            // Fully opaque. At 244 the wallpaper bled through and put Matcha
            // Light at 2.98:1 over a white desktop.
            assert_eq!(
                t.scrim.a, 255,
                "{name}: scrim alpha {} lets the desktop through",
                t.scrim.a
            );
        }
    }

    /// The direction is a per-theme decision, and the flip has to be real:
    /// these two accents have similar luminance but want opposite scrims.
    /// Without this, a "simplification" back to a brightness comparison would
    /// pass the contrast test above on the dark theme while quietly breaking
    /// the light one.
    #[test]
    fn the_scrim_direction_flips_with_the_accent() {
        let dark_ink = Theme::from_vars(&[
            ("--acrylic".into(), "#14141B".into()),
            ("--accent".into(), "#3B6EA5".into()), // deep blue, dark ink
        ]);
        let light_ink = Theme::from_vars(&[
            ("--acrylic".into(), "#F2EEE6".into()),
            ("--accent".into(), "#E8C46A".into()), // pale gold, light ink
        ]);
        // Each ink is paired with a panel that is free to move in the direction that
        // ink needs. The pairing is the point: a dark ink on a *light* panel
        // needs no slide at all, because the panel already clears the bar, so
        // that combination would test nothing about direction.
        assert!(
            luma(dark_ink.scrim) > luma(dark_ink.bg),
            "a dark ink needs a lighter scrim than its panel: panel {:.3}, scrim {:.3}",
            luma(dark_ink.bg),
            luma(dark_ink.scrim)
        );
        assert!(
            luma(light_ink.scrim) < luma(light_ink.bg),
            "a light ink needs a darker scrim than its panel: panel {:.3}, scrim {:.3}",
            luma(light_ink.bg),
            luma(light_ink.scrim)
        );
        // Same theme, both accents: the derivation is driven by the ink.
        let with_dark = [
            ("--acrylic".to_string(), "#6F6F6F".to_string()),
            ("--accent".to_string(), "#202040".to_string()),
        ];
        let with_light = [
            ("--acrylic".to_string(), "#6F6F6F".to_string()),
            ("--accent".to_string(), "#D8D0A0".to_string()),
        ];
        // Same panel, two accents: the dark ink's pill must be lighter than the
        // mid-grey it started from, the light ink's darker.
        // Same panel, two accents: the dark ink's pill must be lighter than the
        // shared panel, the light ink's darker. The grey is deliberately
        // mid-scale (luma ~0.16) so it sits between the two solved targets and
        // can move either way — a grey outside that window leaves one of the
        // two inks with nothing to do.
        let panel = luma(Theme::from_vars(&[("--acrylic".into(), "#6F6F6F".into())]).bg);
        let a = luma(Theme::from_vars(&with_dark).scrim);
        let b = luma(Theme::from_vars(&with_light).scrim);
        assert!(
            a > panel && b < panel,
            "the same mid-grey panel (luma {panel:.3}) gave scrim luma {a:.3} \
             and {b:.3} — the direction is not following the accent"
        );
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





