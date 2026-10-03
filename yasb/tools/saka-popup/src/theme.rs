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

/// Parsed colour, straight (non-premultiplied) RGBA in 0..=255.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Rgba { r, g, b, a: 255 }
    }
    /// Compose this over `under`, returning an opaque colour.
    pub fn over(self, under: Rgba) -> Rgba {
        if self.a == 255 {
            return self;
        }
        let a = self.a as u32;
        let ia = 255 - a;
        Rgba {
            r: ((self.r as u32 * a + under.r as u32 * ia) / 255) as u8,
            g: ((self.g as u32 * a + under.g as u32 * ia) / 255) as u8,
            b: ((self.b as u32 * a + under.b as u32 * ia) / 255) as u8,
            a: 255,
        }
    }
}

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

/// Extract the declaration map of the active theme block.
///
/// Returns a flat `name -> raw value` list (values are raw CSS text, not
/// resolved) for the active block only.
pub fn active_theme_vars(css: &str) -> Vec<(String, String)> {
    // The active block is bare: it begins with its own header comment and runs
    // until the next `/* ... */` comment, which is where the next (wrapped)
    // theme block starts. Mirrors the picker's regex so both agree.
    let re_start = " - active */";
    let Some(start) = css.find(re_start) else {
        return parse_decls(whole_root(css).unwrap_or(css));
    };
    // Begin at the opening of that header comment.
    let head = css[..start].rfind("/*").unwrap_or(0);
    let tail = &css[start + re_start.len()..];
    let end = tail.find("/*").unwrap_or(tail.len());
    parse_decls(&css[head..start + re_start.len() + end])
}

/// Fall back to the whole file when no active marker is present.
fn whole_root(css: &str) -> Option<&str> {
    css.find(":root").map(|i| &css[i..])
}

/// Pull `--name: value;` pairs out of a chunk of CSS.
fn parse_decls(chunk: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes: Vec<char> = chunk.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        // Find the next "--".
        if bytes[i] != '-' || i + 1 >= bytes.len() || bytes[i + 1] != '-' {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i + 2;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || bytes[j] == '-')
        {
            j += 1;
        }
        let name: String = bytes[start..j].iter().collect();
        // Expect ':' then the value up to ';'.
        while j < bytes.len() && bytes[j].is_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == ':' {
            j += 1;
            let vs = j;
            while j < bytes.len() && bytes[j] != ';' && bytes[j] != '\n' {
                j += 1;
            }
            let value: String = bytes[vs..j].iter().collect();
            let value = value.trim().to_string();
            if !name.is_empty() && !value.is_empty() {
                out.push((name, value));
            }
        }
        i = j.max(i + 1);
    }
    out
}

/// Parse a CSS colour: `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(...)`, `rgba(...)`.
/// Returns `None` for anything else (gradients, `var()`, `none`, ...).
pub fn parse_color(value: &str) -> Option<Rgba> {
    let v = value.trim().to_ascii_lowercase();
    if let Some(hex) = v.strip_prefix('#') {
        let h: Vec<u8> = hex
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();
        return match h.len() {
            3 => Some(Rgba::rgb(h[0] * 17, h[1] * 17, h[2] * 17)),
            6 => Some(Rgba::rgb(h[0] * 16 + h[1], h[2] * 16 + h[3], h[4] * 16 + h[5])),
            8 => Some(Rgba {
                r: h[0] * 16 + h[1],
                g: h[2] * 16 + h[3],
                b: h[4] * 16 + h[5],
                a: h[6] * 16 + h[7],
            }),
            _ => None,
        };
    }
    let (fn_name, inner) = if v.starts_with("rgba(") {
        ("rgba", &v[5..])
    } else if v.starts_with("rgb(") {
        ("rgb", &v[4..])
    } else {
        return None;
    };
    let inner = inner.strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty() && *s != "/")
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let comp = |s: &str| -> Option<u8> {
        if let Some(p) = s.strip_suffix('%') {
            let v: f64 = p.trim().parse().ok()?;
            Some((v / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8)
        } else {
            let v: f64 = s.trim().parse().ok()?;
            Some(v.round().clamp(0.0, 255.0) as u8)
        }
    };
    let r = comp(parts[0])?;
    let g = comp(parts[1])?;
    let b = comp(parts[2])?;
    let a = match parts.get(3) {
        None => 255,
        Some(s) => {
            if let Some(p) = s.strip_suffix('%') {
                let v: f64 = p.trim().parse().ok()?;
                (v / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8
            } else {
                let v: f64 = s.trim().parse().ok()?;
                (v * 255.0).round().clamp(0.0, 255.0) as u8
            }
        }
    };
    let _ = fn_name;
    Some(Rgba { r, g, b, a })
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