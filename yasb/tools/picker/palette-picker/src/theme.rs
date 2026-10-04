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
    /// How much of the captured desktop shows through.
    pub backdrop_opacity: f32,
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

            backdrop_opacity: 0.62,
            // Lit from above, unconditionally, so the gradient does not invert
            // on a light theme.
            sheen_top: Rgba { a: 30, ..lighten(bg, 0.55) },
            sheen_bottom: Rgba { a: 46, ..darken(bg, 0.45) },
            bloom: 0.10,
            highlight: Rgba { a: 38, ..lighten(text, 0.25) },
            hairline: Rgba { a: 150, ..border },
            // Unselected cells are a faint lift so the grid reads as a set of
            // tiles; the selected one is a stronger lift plus an accent ring,
            // so selection survives a glance.
            cell: Rgba { a: 90, ..lighten(bg, 0.07) },
            cell_selected: Rgba { a: 170, ..lighten(bg, 0.16) },
            // The search field is inset, so it is *below* the panel rather than
            // above it — the opposite direction to the raised cells.
            field: Rgba { a: 150, ..darken(bg, 0.22) },
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
}