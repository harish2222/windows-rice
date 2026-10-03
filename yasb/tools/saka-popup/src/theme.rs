//! Theme extraction: read the *active* theme block out of `styles.css` and turn
//! its CSS custom properties into RGBA colours.
//!
//! The block contract this relies on is the one documented in the ACRYLIC
//! POLICY comment at the top of `:root`: the active theme's declarations sit
//! under a single header comment `/* Name - active */`, and every other theme
//! is wrapped in a comment chunk. That makes "the active block" a plain
//! regex-able region, and it is the same region the Python palette picker
//! reads, so both follow a theme switch.

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
#[derive(Clone, Debug)]
pub struct Theme {
    /// Window fill. May be translucent; composited over `desktop`.
    pub bg: Rgba,
    /// Panel behind the progress rows.
    pub surface: Rgba,
    pub text: Rgba,
    pub subtext: Rgba,
    pub faint: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
    pub track: Rgba,
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
        Theme {
            bg,
            surface: col(&["--surface0", "--background2"], Rgba::rgb(49, 50, 68)),
            text: col(&["--text"], Rgba::rgb(205, 214, 244)),
            subtext: col(&["--subtext", "--text-muted"], Rgba::rgb(166, 173, 200)),
            faint: col(&["--text-faint"], Rgba::rgb(140, 140, 160)),
            border: col(&["--border", "--hairline"], Rgba::rgb(69, 71, 90)),
            accent: col(&["--accent", "--mauve"], Rgba::rgb(180, 190, 254)),
            track: col(&["--background2"], Rgba::rgb(49, 50, 68)),
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
        assert_eq!(
            parse_color("#C04E6880").map(|c| c.a),
            Some(128)
        );
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
}