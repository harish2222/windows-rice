//! Stylesheet parsing shared by the theme switcher and the palette picker.
//!
//! The themes are `/* <Name> */` blocks of CSS variables inside `:root` in
//! styles.css; exactly one is active. See `../theme.md` for the block
//! contract these invariants depend on.

use std::fs;
use std::path::{Path, PathBuf};

/// One `/* Name */` block inside `:root`.
pub struct Region {
    pub name: String,
    /// Line index of the header comment.
    pub header_idx: usize,
    /// Whether the header carries the `- active` suffix.
    pub marked_active: bool,
    /// Line indices of this region's variable declarations.
    pub vars: Vec<usize>,
}

/// The parsed `:root` block: the raw lines plus the theme regions in them.
pub struct Stylesheet {
    pub lines: Vec<String>,
    pub newline: String,
    pub ends_with_newline: bool,
    pub regions: Vec<Region>,
}

/// Recolor the colour values that live in config.yaml and cannot be reached by
/// CSS, using the active theme block, so those widgets follow the palette too.
///
/// Only the listed keys inside the listed sections are touched; everything else
/// stays byte-identical. Two groups today:
///
/// * `cava` — the visualizer's four colours, which YASB reads straight from
///   the config rather than from the stylesheet.
/// * `pomodoro` — the ring drawn inside the native widget's popup. The widget
///   paints that progress circle itself, so a hex in the config is the only
///   way to theme it.
pub fn sync_config_colors(config: &Path, sheet: &Stylesheet, theme: &str) -> Result<(), String> {
    let vars = sheet.theme_vars(theme);
    let get = |k: &str, fallback: &str| {
        vars.get(k).cloned().unwrap_or_else(|| fallback.to_string())
    };
    let sections: Vec<(&str, Vec<(&str, String)>)> = vec![
        (
            "cava",
            vec![
                ("foreground:", get("teal", "#89b4fa")),
                ("gradient_color_1:", get("blue", "#89b4fa")),
                ("gradient_color_2:", get("mauve", "#cba6f7")),
                ("gradient_color_3:", get("peach", "#fab387")),
            ],
        ),
        (
            "pomodoro",
            vec![
                ("circle_work_progress_color:", get("mauve", "#a6e3a1")),
                ("circle_break_progress_color:", get("teal", "#89b4fa")),
            ],
        ),
    ];
    let raw = fs::read(config).map_err(|e| format!("cannot read {}: {e}", config.display()))?;
    let text =
        String::from_utf8(raw).map_err(|e| format!("{} is not valid UTF-8: {e}", config.display()))?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" }.to_string();
    let ends_with_newline = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();

    for (section, targets) in &sections {
        let Some(start) = lines.iter().position(|l| l.trim() == format!("{section}:")) else {
            continue; // widget not configured; nothing to do
        };
        // The section runs until the next line at the same indent level.
        let mut end = lines.len();
        for (j, line) in lines.iter().enumerate().skip(start + 1) {
            let t = line.trim_start();
            if t.is_empty() {
                continue;
            }
            if !line.starts_with(' ') && !line.starts_with('\t') {
                end = j;
                break;
            }
            if line.starts_with("  ") && !line.starts_with("   ") {
                end = j;
                break;
            }
        }
        for line in &mut lines[start + 1..end] {
            let t = line.trim_start().to_string();
            for (key, val) in targets {
                if t.starts_with(key) {
                    if let Some(q1) = t.find('"') {
                        if let Some(q2) = t[q1 + 1..].find('"') {
                            let indent = &line[..line.len() - line.trim_start().len()];
                            *line = format!("{indent}{key} \"{val}\"{}", &t[q1 + 1 + q2 + 1..]);
                        }
                    }
                    break;
                }
            }
        }
    }

    let mut out = lines.join(&newline);
    if ends_with_newline {
        out.push_str(&newline);
    }
    write_atomic(config, out.as_bytes())
}

pub fn read_styles(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    String::from_utf8(bytes).map_err(|e| format!("{} is not valid UTF-8: {e}", path.display()))
}

/// Write via temp file + rename so a concurrent reader (YASB's
/// watch_stylesheet reload, the palette's 10s `current` poll, another switch
/// click) never sees a truncated or half-written file: an in-place
/// `fs::write` fires the change event on truncate, so YASB could reload an
/// empty stylesheet mid-switch and drop every theme variable. Rename is one
/// event with complete content; retries cover a destination briefly held open
/// by the reader.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    let mut last_err = String::from("unknown error");
    for attempt in 0..5u64 {
        match fs::write(&tmp, bytes) {
            Ok(()) => match fs::rename(&tmp, path) {
                Ok(()) => return Ok(()),
                Err(e) => last_err = e.to_string(),
            },
            Err(e) => {
                last_err = e.to_string();
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(30 * (attempt + 1)));
    }
    let _ = fs::remove_file(&tmp);
    Err(format!("cannot write {}: {last_err}", path.display()))
}

pub fn parse(text: &str) -> Result<Stylesheet, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" }.to_string();
    let ends_with_newline = text.ends_with('\n');
    let lines: Vec<String> = text.lines().map(str::to_string).collect();

    let root_start = lines
        .iter()
        .position(|l| l.trim() == ":root {")
        .ok_or("could not locate :root block")?;
    let root_end = lines[root_start..]
        .iter()
        .position(|l| l.trim() == "}")
        .map(|p| p + root_start)
        .ok_or("could not locate end of :root block")?;
    if !lines[root_start..=root_end]
        .iter()
        .any(|l| l.contains("/*colors*/"))
    {
        return Err("could not locate /*colors*/ marker".to_string());
    }

    let mut regions: Vec<Region> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(root_end).skip(root_start) {
        if let Some((name, marked)) = parse_header(line) {
            if name.eq_ignore_ascii_case("colors") {
                continue;
            }
            regions.push(Region {
                name,
                header_idx: idx,
                marked_active: marked,
                vars: Vec::new(),
            });
        } else if let Some(region) = regions.last_mut() {
            if is_var_line(line) {
                region.vars.push(idx);
            }
        }
    }
    if regions.is_empty() {
        return Err("no theme blocks found".to_string());
    }

    Ok(Stylesheet {
        lines,
        newline,
        ends_with_newline,
        regions,
    })
}

/// `/* Name */` or `/* Name - active */` -> (name, marked_active).
///
/// Only name-like inners qualify (ASCII letters/digits/spaces/hyphen after an
/// optional `- active` suffix). Declaration lines (`--acrylic: ...`) and prose
/// annotations (`1% acrylic`, `festival red`) must never become regions: one
/// stray single-line comment inside a block used to split it into fake themes
/// and silently corrupt every later switch. Non-matching comments are simply
/// not headers; multi-line comments never match either side of this pattern.
pub fn parse_header(line: &str) -> Option<(String, bool)> {
    let t = line.trim();
    let inner = t.strip_prefix("/*")?.strip_suffix("*/")?.trim();
    if inner.is_empty() {
        return None;
    }
    let lower = inner.to_lowercase();
    let active = lower.ends_with("- active");
    let name = if active {
        inner[..inner.len() - "- active".len()].trim()
    } else {
        inner
    };
    let name_ok = !name.is_empty()
        && name.chars().any(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
    if !name_ok {
        return None;
    }
    Some((name.to_string(), active))
}

/// A CSS variable declaration, optionally wrapped in a `/* */` comment.
pub fn is_var_line(line: &str) -> bool {
    let mut t = line.trim_start();
    if let Some(rest) = t.strip_prefix("/*") {
        t = rest.trim_start();
    }
    let Some(rest) = t.strip_prefix("--") else {
        return false;
    };
    rest.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && t.contains(':')
}

impl Stylesheet {
    /// (name, active) for every theme block, in file order.
    pub fn themes(&self) -> Vec<(String, bool)> {
        self.regions
            .iter()
            .map(|r| {
                let bare = r
                    .vars
                    .first()
                    .is_some_and(|&v| !self.lines[v].trim_start().starts_with("/*"));
                (r.name.clone(), r.marked_active || bare)
            })
            .collect()
    }

    /// `--name: value` pairs of one theme block (names lowercased, no dashes).
    pub fn theme_vars(&self, theme: &str) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Some(region) = self.regions.iter().find(|r| r.name.eq_ignore_ascii_case(theme.trim())) {
            for &v in &region.vars {
                let line = self.lines[v].trim();
                let line = line.strip_prefix("/*").unwrap_or(line).trim_start();
                let line = line.strip_suffix("*/").unwrap_or(line).trim_end();
                if let Some((k, rest)) = line.split_once(':') {
                    let k = k.trim().trim_start_matches("--").to_lowercase();
                    let val = rest.trim().trim_end_matches(';').trim().to_string();
                    if !k.is_empty() && !val.is_empty() {
                        map.insert(k, val);
                    }
                }
            }
        }
        map
    }

    pub fn active_name(&self) -> Option<String> {
        for r in &self.regions {
            let bare = r
                .vars
                .first()
                .is_some_and(|&v| !self.lines[v].trim_start().starts_with("/*"));
            if r.marked_active || bare {
                return Some(r.name.clone());
            }
        }
        None
    }

    pub fn available(&self) -> String {
        self.regions
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Activate `name` (case-insensitive); returns the canonical name.
    pub fn set(&mut self, name: &str) -> Result<String, String> {
        let want = name.trim();
        let target = self
            .regions
            .iter()
            .position(|r| r.name.eq_ignore_ascii_case(want))
            .ok_or_else(|| format!("unknown theme '{want}'. Available: {}", self.available()))?;
        for i in 0..self.regions.len() {
            let (first, last, header_idx, activate) = {
                let r = &self.regions[i];
                if r.vars.is_empty() {
                    continue;
                }
                (
                    r.vars[0],
                    r.vars[r.vars.len() - 1],
                    r.header_idx,
                    i == target,
                )
            };
            if activate {
                uncomment_first(&mut self.lines, first);
                uncomment_last(&mut self.lines, last);
                mark_header(&mut self.lines, header_idx, true);
            } else {
                comment_first(&mut self.lines, first);
                comment_last(&mut self.lines, last);
                mark_header(&mut self.lines, header_idx, false);
            }
        }
        Ok(self.regions[target].name.clone())
    }

    /// Move one step through the file order (wraps around).
    pub fn step(&mut self, delta: isize) -> Result<String, String> {
        let current = self.active_name().unwrap_or_default();
        let mut idx = self
            .regions
            .iter()
            .position(|r| r.name == current)
            .unwrap_or(0) as isize;
        let n = self.regions.len() as isize;
        idx = (idx + delta).rem_euclid(n);
        let name = self.regions[idx as usize].name.clone();
        self.set(&name)
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let mut text = self.lines.join(&self.newline);
        if self.ends_with_newline {
            text.push_str(&self.newline);
        }
        write_atomic(path, text.as_bytes())
    }
}

/// `    /* --x: ...` -> `    --x: ...` (idempotent).
fn uncomment_first(lines: &mut [String], idx: usize) {
    let line = &lines[idx];
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);
    if let Some(after) = rest.strip_prefix("/*") {
        lines[idx] = format!("{indent}{}", after.trim_start());
    }
}

/// `    --x: ... */` -> `    --x: ...` (idempotent).
fn uncomment_last(lines: &mut [String], idx: usize) {
    let line = &lines[idx];
    if let Some(pos) = line.rfind("*/") {
        lines[idx] = line[..pos].trim_end().to_string();
    }
}

/// `    --x: ...` -> `    /* --x: ...` (idempotent).
fn comment_first(lines: &mut [String], idx: usize) {
    if lines[idx].trim_start().starts_with("/*") {
        return;
    }
    let line = &lines[idx];
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);
    lines[idx] = format!("{indent}/* {rest}");
}

/// `    --x: ...` -> `    --x: ... */` (idempotent).
fn comment_last(lines: &mut [String], idx: usize) {
    if lines[idx].trim_end().ends_with("*/") {
        return;
    }
    lines[idx] = format!("{} */", lines[idx].trim_end());
}

/// Toggle the `- active` suffix on a `/* Name */` header (idempotent).
fn mark_header(lines: &mut [String], idx: usize, active: bool) {
    let line = &lines[idx];
    let Some(pos) = line.rfind("*/") else {
        return;
    };
    let head = line[..pos].trim_end();
    let already = head.to_lowercase().ends_with("- active");
    if active && !already {
        lines[idx] = format!("{head} - active */");
    } else if !active && already {
        let stripped = head[..head.len() - "- active".len()].trim_end();
        lines[idx] = format!("{stripped} */");
    }
}

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

