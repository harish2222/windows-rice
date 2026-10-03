//! yasb-theme — switch the YASB bar color theme from the bar or terminal.
//!
//! The themes live as `/* <Name> */` blocks of CSS variables inside the `:root`
//! section of styles.css; exactly one block is uncommented (the active theme).
//! This tool toggles those comment markers, preserving the file's newline style,
//! trailing-newline state and UTF-8 (no BOM) encoding byte-for-byte otherwise.
//!
//! Usage:
//!   yasb-theme [--styles PATH] <list|current|set <name>|next|prev>
//!
//! Bar wiring (omega dropdown): run_cmd -> `yasb-theme.exe current`,
//! left-click -> `yasb-theme.exe next`, right-click -> `yasb-theme.exe prev`
//! via the .bat wrappers next to the binary. YASB reloads styles.css on save.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

struct Region {
    name: String,
    header_idx: usize,
    marked_active: bool,
    vars: Vec<usize>,
}

struct Stylesheet {
    lines: Vec<String>,
    newline: String,
    ends_with_newline: bool,
    regions: Vec<Region>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("yasb-theme: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut styles_override: Option<PathBuf> = None;
    let mut config_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--styles" {
            if i + 1 >= args.len() {
                return Err("--styles needs a path".to_string());
            }
            styles_override = Some(PathBuf::from(args.remove(i + 1)));
            args.remove(i);
        } else if args[i] == "--config" {
            if i + 1 >= args.len() {
                return Err("--config needs a path".to_string());
            }
            config_override = Some(PathBuf::from(args.remove(i + 1)));
            args.remove(i);
        } else {
            i += 1;
        }
    }
    let styles = styles_override.unwrap_or_else(default_styles_path);
    let config = config_override.unwrap_or_else(default_config_path);
    let Some(cmd) = args.first() else {
        return Err(
            "usage: yasb-theme [--styles PATH] [--config PATH] <list|current|set <name>|next|prev>"
                .to_string(),
        );
    };
    match cmd.to_lowercase().as_str() {
        "list" => {
            let sheet = parse(&read_styles(&styles)?)?;
            for (name, active) in sheet.themes() {
                println!("{} {}", if active { "*" } else { " " }, name);
            }
            Ok(())
        }
        "current" => {
            let sheet = parse(&read_styles(&styles)?)?;
            match sheet.active_name() {
                Some(name) => {
                    // No trailing newline: the bar label must be exactly the name.
                    print!("{name}");
                    Ok(())
                }
                None => Err("no active theme found".to_string()),
            }
        }
        "set" => {
            let Some(name) = args.get(1) else {
                return Err("usage: yasb-theme set <name>".to_string());
            };
            let mut sheet = parse(&read_styles(&styles)?)?;
            let new = sheet.set(name)?;
            sheet.write(&styles)?;
            sync_config_colors(&config, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        "next" => {
            let mut sheet = parse(&read_styles(&styles)?)?;
            let new = sheet.step(1)?;
            sheet.write(&styles)?;
            sync_config_colors(&config, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        "prev" | "previous" => {
            let mut sheet = parse(&read_styles(&styles)?)?;
            let new = sheet.step(-1)?;
            sheet.write(&styles)?;
            sync_config_colors(&config, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        other => Err(format!(
            "unknown command '{other}'; expected list|current|set|next|prev"
        )),
    }
}

/// styles.css next to the binary, falling back to %USERPROFILE%\.config\yasb.
fn default_styles_path() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_exe = dir.join("styles.css");
            if next_to_exe.is_file() {
                return next_to_exe;
            }
        }
    }
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".config").join("yasb").join("styles.css")
}

/// config.yaml next to the binary, falling back to %USERPROFILE%\.config\yasb.
fn default_config_path() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_exe = dir.join("config.yaml");
            if next_to_exe.is_file() {
                return next_to_exe;
            }
        }
    }
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".config").join("yasb").join("config.yaml")
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
fn sync_config_colors(config: &Path, sheet: &Stylesheet, theme: &str) -> Result<(), String> {
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

fn read_styles(path: &Path) -> Result<String, String> {
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
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
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

fn parse(text: &str) -> Result<Stylesheet, String> {
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
fn parse_header(line: &str) -> Option<(String, bool)> {
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
fn is_var_line(line: &str) -> bool {
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
    fn themes(&self) -> Vec<(String, bool)> {
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
    fn theme_vars(&self, theme: &str) -> std::collections::HashMap<String, String> {
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

    fn active_name(&self) -> Option<String> {
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

    fn available(&self) -> String {
        self.regions
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Activate `name` (case-insensitive); returns the canonical name.
    fn set(&mut self, name: &str) -> Result<String, String> {
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
    fn step(&mut self, delta: isize) -> Result<String, String> {
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

    fn write(&self, path: &Path) -> Result<(), String> {
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
