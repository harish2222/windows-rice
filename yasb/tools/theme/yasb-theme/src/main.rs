//! yasb-theme — switch the YASB bar color theme from the bar or terminal.
//!
//! The themes live as `/* <Name> */` blocks of CSS variables inside the `:root`
//! section of styles.css; exactly one block is uncommented (the active theme).
//! This tool toggles those comment markers, preserving the file's newline style,
//! trailing-newline state and UTF-8 (no BOM) encoding byte-for-byte otherwise.
//!
//! Usage:
//!   yasb-theme [--styles PATH] [--config PATH] [--cava PATH]
//!             <list|current|set <name>|next|prev|motif>
//!
//! Bar wiring (omega dropdown): run_cmd -> `yasb-theme.exe current`,
//! left-click -> `yasb-theme.exe next`, right-click -> `yasb-theme.exe prev`
//! via the .bat wrappers next to the binary. YASB reloads styles.css on save.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use yasb_theme::{motifs, parse, read_styles, sync_cava_colors, sync_config_colors};

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
    let mut cava_override: Option<PathBuf> = None;
    let mut motif_dir_override: Option<PathBuf> = None;
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
        } else if args[i] == "--cava" {
            if i + 1 >= args.len() {
                return Err("--cava needs a path".to_string());
            }
            cava_override = Some(PathBuf::from(args.remove(i + 1)));
            args.remove(i);
        } else if args[i] == "--motif-dir" {
            if i + 1 >= args.len() {
                return Err("--motif-dir needs a path".to_string());
            }
            motif_dir_override = Some(PathBuf::from(args.remove(i + 1)));
            args.remove(i);
        } else {
            i += 1;
        }
    }
    let styles = styles_override.unwrap_or_else(default_styles_path);
    let config = config_override.unwrap_or_else(default_config_path);
    let cava = cava_override.unwrap_or_else(default_cava_path);
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
            sync_cava_colors(&cava, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        "next" => {
            let mut sheet = parse(&read_styles(&styles)?)?;
            let new = sheet.step(1)?;
            sheet.write(&styles)?;
            sync_config_colors(&config, &sheet, &new)?;
            sync_cava_colors(&cava, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        "prev" | "previous" => {
            let mut sheet = parse(&read_styles(&styles)?)?;
            let new = sheet.step(-1)?;
            sheet.write(&styles)?;
            sync_config_colors(&config, &sheet, &new)?;
            sync_cava_colors(&cava, &sheet, &new)?;
            println!("{new}");
            Ok(())
        }
        "motif" => {
            // Deal fresh motifs out to the theme blocks: three drawn at random
            // from the SVG motifs on disk, cycling if there are fewer motifs
            // than themes.
            //
            // Seeded from the clock on purpose -- the point is that the art
            // changes between runs, so a fixed seed would make this a no-op
            // dressed up as a feature.
            let dir = motif_dir_override.unwrap_or_else(default_motif_dir);
            let found = motifs::discover(&dir);
            if found.is_empty() {
                return Err(format!("no motif-*.svg found in {}", dir.display()));
            }
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E3779B97F4A7C15);
            let chosen = motifs::pick(&found, 3, seed);
            let mut sheet = parse(&read_styles(&styles)?)?;
            let n = motifs::assign(&mut sheet.lines, &sheet.regions, &chosen);
            if n == 0 {
                return Err("no --motif: declarations found to rewrite".to_string());
            }
            sheet.write(&styles)?;
            let names: Vec<&str> = chosen.iter().map(|m| m.stem.as_str()).collect();
            println!("{} motifs from {} themes: {}", names.join(", "), n, found.len());
            Ok(())
        }
        other => Err(format!(
            "unknown command '{other}'; expected list|current|set|next|prev|motif"
        )),
    }
}

/// `%USERPROFILE%\.config\yasb` — where the SVG motifs live, alongside the
/// stylesheet that references them.
fn default_motif_dir() -> PathBuf {
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".config").join("yasb")
}

/// `%USERPROFILE%\.config\cava\config` — the visualiser's own INI.
///
/// Not derived from the exe's location: cava is a separate program with its
/// own config directory, and it is not always installed at all. A missing file
/// is treated as "nothing to sync" rather than an error.
fn default_cava_path() -> PathBuf {
    let home = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".config").join("cava").join("config")
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

