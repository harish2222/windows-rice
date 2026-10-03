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
    /// Selected-cell fill and the search field's focus tint.
    pub surface: Rgba,
    pub text: Rgba,
    pub subtext: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
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
        Theme {
            bg,
            surface: col(&["--surface0", "--surface1"], Rgba::rgb(49, 50, 68)),
            text: col(&["--text"], Rgba::rgb(205, 214, 244)),
            subtext: col(&["--subtext", "--text-muted"], Rgba::rgb(166, 173, 200)),
            border: col(&["--border", "--hairline"], Rgba::rgb(69, 71, 90)),
            accent: col(&["--accent", "--mauve"], Rgba::rgb(180, 190, 254)),
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
}