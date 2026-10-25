// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: YASB keybind Module (theme)
// ============================================================================
//! Theme colors for the keybindings cheatsheet.

use std::path::Path;
pub use yasb_theme::{Rgba, active_theme_vars, parse_color};

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

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub bg: Rgba,
    pub text: Rgba,
    pub subtext: Rgba,
    pub accent: Rgba,
    pub backdrop_opacity: f32,
    pub typeface: yasb_chrome::Typeface,
    pub backdrop_blur: u32,
    pub sheen_top: Rgba,
    pub sheen_bottom: Rgba,
    pub bloom: f32,
    pub highlight: Rgba,
    pub hairline: Rgba,
    pub row_bg: Rgba,
    pub row_selected: Rgba,
    pub badge_bg: Rgba,
    pub badge_border: Rgba,
    pub field: Rgba,
    pub scrollbar_track: Rgba,
    pub scrollbar_thumb: Rgba,
    /// Subtle neutral border (#383D47) from Apple specification
    pub subtle_border: Rgba,
    /// Elevated surface (#272B33)
    pub elevated_surface: Rgba,
    /// Keycap surface (#303540)
    pub keycap_bg: Rgba,
    /// Keycap border (#4B5260)
    pub keycap_border: Rgba,
    /// Restrained Apple accent (#B8C8EE)
    pub apple_accent: Rgba,
    /// Muted metadata (#858C9A)
    pub muted: Rgba,
}

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
        if bg.a < 235 {
            bg.a = 235;
        }
        let text = col(&["--text"], Rgba::rgb(205, 214, 244));
        let border = col(&["--border", "--hairline"], Rgba::rgb(69, 71, 90));
        let accent = col(&["--accent", "--mauve", "--blue"], Rgba::rgb(180, 190, 254));
        let subtext = col(&["--subtext", "--subtext0", "--overlay1"], Rgba::rgb(166, 173, 200));

        let family = vars
            .iter()
            .find(|(k, _)| k == "--font-family" || k == "--family")
            .map(|(_, v)| v.as_str());

        Theme {
            bg,
            text,
            subtext,
            accent,
            // Match palette-picker's luxurious 70% frosted acrylic pane
            backdrop_opacity: 0.30,
            typeface: yasb_chrome::Typeface::resolve(family),
            backdrop_blur: 12,
            sheen_top: Rgba { a: 46, ..lighten(bg, 0.55) },
            sheen_bottom: Rgba { a: 70, ..darken(bg, 0.45) },
            bloom: 0.12,
            highlight: Rgba { a: 38, ..lighten(text, 0.25) },
            hairline: Rgba { a: 150, ..border },
            row_bg: Rgba { a: 175, ..lighten(bg, 0.07) },
            row_selected: Rgba { a: 215, ..lighten(bg, 0.16) },
            badge_bg: Rgba { a: 200, ..darken(bg, 0.22) },
            badge_border: Rgba { a: 140, ..border },
            field: Rgba { a: 200, ..darken(bg, 0.22) },
            scrollbar_track: Rgba { a: 35, ..darken(bg, 0.30) },
            scrollbar_thumb: Rgba { a: 180, ..accent },
            subtle_border: Rgba::rgb(56, 61, 71),
            elevated_surface: Rgba::rgb(39, 43, 51),
            keycap_bg: Rgba::rgb(48, 53, 64),
            keycap_border: Rgba::rgb(75, 82, 96),
            apple_accent: col(&["--accent-apple", "--lavender", "--sapphire"], Rgba::rgb(184, 200, 238)),
            muted: col(&["--muted", "--overlay0"], Rgba::rgb(133, 140, 154)),
        }
    }

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
