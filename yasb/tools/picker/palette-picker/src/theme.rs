//! Theme colours for the picker window.
//!
//! Deliberately thin: the picker is a chrome-less grid, so it needs far
//! fewer tokens than saka-popup. The parsing itself is the shared
//! `yasb_theme` code, so the picker and the switcher can never disagree
//! about which block is active or how a colour literal resolves.

use std::path::Path;

pub use yasb_theme::{Rgba, active_theme_vars, parse_color};

/// Used only when `styles.css` cannot be read at all.
fn fallback() -> Vec<(String, String)> {
    [
        ("--background", "#181825"),
        ("--surface0", "#313244"),
        ("--text", "#cdd6f4"),
        ("--subtext", "#a6adc8"),
        ("--accent", "#b4befe"),
        ("--border", "#45475a"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub bg: Rgba,
    pub text: Rgba,
    pub subtext: Rgba,
    pub accent: Rgba,

    // --- derived material (see saka-popup's theme.rs for the rationale) ----
    /// How much of the captured desktop shows through the panel.
    ///
    /// This is acrylic's defining number and it is *high*: the material is
    /// only 15% opaque, so 85% of the blurred wallpaper is what you see.
    pub backdrop_opacity: f32,
    /// Box-blur radius applied to the backdrop before it is blended.
    ///
    /// Acrylic is low-opacity *and* high-blur. Without the blur, 0.85 here
    /// would drop crisp icons and window edges straight behind the grid and
    /// no amount of material tuning would rescue readability after that.
    pub backdrop_blur: u32,
    /// Top and bottom of the panel's light gradient.
    pub sheen_top: Rgba,
    pub sheen_bottom: Rgba,
    /// Strength of the accent bloom behind the search field.
    pub bloom: f32,
    /// Top-edge highlight.
    pub highlight: Rgba,
    /// Separator rules and cell edges.
    pub hairline: Rgba,
    /// Unselected cell fill, and the selected cell's fill.
    pub cell: Rgba,
    pub cell_selected: Rgba,
    /// The search field's inset background.
    pub field: Rgba,
    /// Swatch chip border, which keeps light swatches visible on a light panel.
    pub swatch_edge: Rgba,
}

/// Linear interpolation between two colours.
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Rgba { r: f(a.r, b.r), g: f(a.g, b.g), b: f(a.b, b.b), a: f(a.a, b.a) }
}

fn lighten(c: Rgba, t: f32) -> Rgba {
    mix(c, Rgba::rgb(255, 255, 255), t)
}

fn darken(c: Rgba, t: f32) -> Rgba {
    mix(c, Rgba::rgb(0, 0, 0), t)
}

impl Theme {
    pub fn from_vars(vars: &[(String, String)]) -> Theme {
        let col = |names: &[&str], default: Rgba| -> Rgba {
            names
                .iter()
                .find_map(|n| vars.iter().find(|(k, _)| k == n))
                .and_then(|(_, v)| parse_color(v))
                .unwrap_or(default)
        };
        let mut bg = col(&["--background", "--base", "--glassmenu"], Rgba::rgb(24, 24, 37));
        // Rangalipi dark themes set `--glass*` at ~1% alpha for a
        // near-invisible acrylic bar. That is right for a bar strip sitting
        // on the desktop and wrong for a floating panel the user has to
        // read and click, so floor the alpha and keep the tint.
        if bg.a < 235 {
            bg.a = 235;
        }
        let text = col(&["--text"], Rgba::rgb(205, 214, 244));
        let border = col(&["--border", "--hairline"], Rgba::rgb(69, 71, 90));
        Theme {
            bg,
            text,
            subtext: col(&["--subtext", "--text-muted"], Rgba::rgb(166, 173, 200)),
            accent: col(&["--accent", "--mauve"], Rgba::rgb(180, 190, 254)),

            backdrop_opacity: 0.85,
            backdrop_blur: 12,
            // Lit from above, unconditionally, so the gradient does not invert
            // on a light theme. The alphas run higher than an opaque panel
            // would need: most of what is behind this window is now wallpaper,
            // and the gradient is the only wash standing between the grid and
            // a bright desktop.
            sheen_top: Rgba { a: 46, ..lighten(bg, 0.55) },
            sheen_bottom: Rgba { a: 70, ..darken(bg, 0.45) },
            bloom: 0.10,
            highlight: Rgba { a: 38, ..lighten(text, 0.25) },
            hairline: Rgba { a: 150, ..border },
            // Unselected cells are a faint lift so the grid reads as a set of
            // tiles; the selected one is a stronger lift plus an accent ring,
            // so selection survives a glance. Both are heavier than an opaque
            // panel would want: at 85% bleed the cell is what the name text
            // actually sits on, so it has to deliver the contrast the panel
            // used to.
            cell: Rgba { a: 175, ..lighten(bg, 0.07) },
            cell_selected: Rgba { a: 215, ..lighten(bg, 0.16) },
            // The search field is inset, so it is *below* the panel rather than
            // above it — the opposite direction to the raised cells. It also
            // carries the query text, which is the one string in this window
            // the user is guaranteed to read.
            field: Rgba { a: 200, ..darken(bg, 0.22) },
            // Without this, a near-white swatch on a light theme is a
            // featureless white rectangle on a near-white cell.
            swatch_edge: Rgba { a: 90, ..darken(bg, 0.35) },
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

    #[test]
    fn floors_translucent_background() {
        let t = Theme::from_vars(&[("--background".into(), "rgba(29,16,21,0.01)".into())]);
        assert_eq!(t.bg.a, 235);
        assert_eq!((t.bg.r, t.bg.g, t.bg.b), (29, 16, 21));
    }

    #[test]
    fn keeps_opaque_background_as_is() {
        let t = Theme::from_vars(&[("--background".into(), "#14141B".into())]);
        assert_eq!(t.bg, Rgba::rgb(0x14, 0x14, 0x1B));
    }

    #[test]
    fn missing_file_falls_back() {
        let t = Theme::load(Path::new("Z:/definitely/not/here.css"));
        assert_eq!(t.text, Rgba::rgb(205, 214, 244));
    }

    /// sRGB has to be linearized before luminance means anything; summing the
    /// encoded bytes overstates mid-tones by enough to change a pass into a
    /// fail.
    fn linearize(v: u8) -> f32 {
        let c = v as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    }

    fn luma(c: Rgba) -> f32 {
        0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b)
    }

    /// The material has to behave on a light theme as well as a dark one, or
    /// half the shipped palettes get a panel lit from below.
    #[test]
    fn material_inverts_correctly_for_light_themes() {
        for bg in ["#14141B", "#E7E0D2"] {
            let t = Theme::from_vars(&[("--background".into(), bg.into())]);
            assert!(
                luma(t.sheen_top) > luma(t.sheen_bottom),
                "sheen inverted on {bg}"
            );
            // The selected cell must be visibly above the unselected one, or
            // the grid has no keyboard-navigable focus at all.
            assert!(
                luma(t.cell_selected) > luma(t.cell),
                "selected cell does not stand out on {bg}"
            );
            // ...and above the panel, so it reads as raised.
            assert!(luma(t.cell_selected) > luma(t.bg), "cell not raised on {bg}");
            // The search field is inset, i.e. below the panel.
            assert!(luma(t.field) < luma(t.bg), "field not inset on {bg}");
            for (name, c) in [
                ("sheen_top", t.sheen_top),
                ("sheen_bottom", t.sheen_bottom),
                ("cell", t.cell),
                ("cell_selected", t.cell_selected),
                ("field", t.field),
                ("hairline", t.hairline),
                ("highlight", t.highlight),
                ("swatch_edge", t.swatch_edge),
            ] {
                assert!(c.a > 0 && c.a < 255, "{name} on {bg} has alpha {}", c.a);
            }
            // The acrylic value is not a free parameter: it is the material's
            // 15% opacity stated the other way round. Pin it, so a future
            // "let's calm it down" edit cannot quietly turn the glass back
            // into a sheet of plastic.
            assert_eq!(t.backdrop_opacity, 0.85, "backdrop opacity drifted on {bg}");
            // ...and the blur is what makes that number survivable. A radius
            // of zero would pass an opacity check and still be unreadable over
            // a busy desktop.
            assert!(
                t.backdrop_blur >= 8,
                "backdrop blur {} on {bg} is too small to read as acrylic",
                t.backdrop_blur
            );
        }
    }

    /// Names are drawn in `text` on `cell`; if that pair is unreadable the
    /// grid is decorative.
    #[test]
    fn names_stay_readable_on_cells() {
        for (bg, text) in [("#14141B", "#E7E0D2"), ("#E7E0D2", "#1A1816")] {
            let t = Theme::from_vars(&[
                ("--background".into(), bg.into()),
                ("--text".into(), text.into()),
            ]);
            // Approximate: the cell is partly transparent, so the real contrast
            // is between the text and a blend of cell and bg. Both are at
            // least as dark/light as bg, so bg is the conservative case.
            let (hi, lo) = {
                let (a, b) = (luma(t.text), luma(t.bg));
                if a > b { (a, b) } else { (b, a) }
            };
            assert!(
                (hi + 0.05) / (lo + 0.05) > 7.0,
                "name text on {bg} is only {:.1}:1",
                (hi + 0.05) / (lo + 0.05)
            );
        }
    }

    /// Acrylic is 15% opaque, so 85% of whatever is on the desktop ends up
    /// behind this panel. `names_stay_readable_on_cells` measures against
    /// `t.bg`, which is no longer the surface the names sit on: it is the
    /// cell, which is the cell, which is on a wash of the wallpaper.
    ///
    /// So this replays `main.rs::paint`'s chain in order — theme fill,
    /// wallpaper, sheen, cell — and measures what actually ends up under the
    /// text, across both ends of the wallpaper range. One hostile wallpaper
    /// is not enough: a dark theme loses its text to a bright wallpaper and a
    /// light theme loses its text to a dark one, so checking only one of them
    /// would declare victory while the other was unreadable.
    #[test]
    fn the_grid_stays_legible_across_the_whole_wallpaper_range() {
        let cases: [(&str, &str, &str, &str); 2] = [
            ("Rangalipi", "#14141B", "#E7E0D2", "#B8B2A7"),
            ("Rangalipi Matcha Light", "#E7E0D2", "#1A1816", "#45423C"),
        ];
        for (name, bg, text, subtext) in cases {
            let t = Theme::from_vars(&[
                ("--background".into(), bg.into()),
                ("--text".into(), text.into()),
                ("--subtext".into(), subtext.into()),
            ]);
            for wall in [Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255)] {
                let where_ = format!("{:?} wallpaper", (wall.r, wall.g, wall.b));
                // Unselected cell, at the darker end of the gradient.
                let cell = t.cell.over(panel_over(&t, wall, false));
                let name_ratio = ratio(t.text, cell);
                let sub_ratio = ratio(t.subtext, cell);
                assert!(
                    name_ratio >= 4.5,
                    "{name}: theme name is {name_ratio:.1}:1 over a {where_}"
                );
                assert!(
                    sub_ratio >= 3.0,
                    "{name}: subtitle is {sub_ratio:.1}:1 over a {where_}"
                );
                // The query in the search field, at the top of the panel.
                let field = t.field.over(panel_over(&t, wall, true));
                let query_ratio = ratio(t.text, field);
                assert!(
                    query_ratio >= 4.5,
                    "{name}: search query is {query_ratio:.1}:1 over a {where_}"
                );
            }
        }
    }

    /// Replay `main.rs::paint`'s base chain — theme fill, the backdrop pull,
    /// then one end of the vertical gradient — and return what lands there.
    fn panel_over(t: &Theme, wall: Rgba, header: bool) -> Rgba {
        let mut c = t.bg.over(Rgba::rgb(20, 20, 28));
        let k = t.backdrop_opacity;
        let pull = |from: u8, to: u8| (from as f32 + (to as f32 - from as f32) * k + 0.5) as u8;
        c = Rgba { r: pull(c.r, wall.r), g: pull(c.g, wall.g), b: pull(c.b, wall.b), a: 255 };
        if header { t.sheen_top.over(c) } else { t.sheen_bottom.over(c) }
    }

    fn ratio(a: Rgba, b: Rgba) -> f32 {
        let (hi, lo) = {
            let (x, y) = (luma(a), luma(b));
            if x > y { (x, y) } else { (y, x) }
        };
        (hi + 0.05) / (lo + 0.05)
    }
}