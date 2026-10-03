//! Tests for the shared theme library. Included from `lib.rs` as
//! `#[cfg(test)] #[path = "tests.rs"] mod tests;`.

use super::*;

/// A miniature stylesheet that follows the real block contract exactly: the
/// active block is bare, and every inactive block is ONE comment chunk whose
/// opener sits on the first variable line and whose closer on the last.
const CSS: &str = r##":root {
  /*colors*/
  /* Rangalipi Base */
  /* --acrylic: #1a1015; */
  /* --text: #e6dfe3; */
  /* --accent: #c04e68; */
  /* --mauve: #a08ac4; */
  /* --teal: #4f8f9d; */

  /* Rangalipi Wine - active */
  --acrylic: #15101d;
  --text: #ecdfe6;
  --accent: #c04e68;

  /* Rangalipi Moss */
  /* --acrylic: #10150f; */
  /* --text: #dfe6d9; */
  /* --accent: #7aa85f; */
  /* --mauve: #8fae6b; */
  /* --teal: #5f8f7a; */
}
"##;

// ---- block contract ----------------------------------------------------

#[test]
fn parses_every_theme_block() {
    let s = parse(CSS).unwrap();
    let all = s.themes();
    let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Rangalipi Base", "Rangalipi Wine", "Rangalipi Moss"]);
}

#[test]
fn exactly_one_block_is_active() {
    let s = parse(CSS).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Wine"));
    let all = s.themes();
    let actives: Vec<&str> = all.iter().filter(|(_, a)| *a).map(|(n, _)| n.as_str()).collect();
    assert_eq!(actives, ["Rangalipi Wine"]);
}

#[test]
fn headers_reject_non_theme_comments() {
    // A stray single-line comment inside a block used to split it into fake
    // themes and silently corrupt every later switch, so anything that is
    // not a bare name has to be rejected.
    for bad in [
        "/* --acrylic: #fff; */",
        "/* 1% acrylic */",
        "/* 100% opaque */",
        "/* festival: red */",
        "/* url(a.png) */",
        "/* */",
        "--accent: #fff;",
    ] {
        assert_eq!(parse_header(bad), None, "{bad} should not be a header");
    }
}

#[test]
fn the_colors_marker_is_not_counted_as_a_theme() {
    // parse_header accepts "colors" (it is name-like), so parse() is what has
    // to drop it -- otherwise /*colors*/ would become theme #1 and every
    // index-based switch would be off by one.
    assert_eq!(parse_header("/*colors*/"), Some(("colors".into(), false)));
    let s = parse(CSS).unwrap();
    let all = s.themes();
    assert!(!all.iter().any(|(n, _)| n.eq_ignore_ascii_case("colors")));
}

#[test]
fn headers_accept_names_with_spaces_digits_hyphens() {
    assert_eq!(
        parse_header("/* Rangalipi Wine */"),
        Some(("Rangalipi Wine".into(), false))
    );
    assert_eq!(
        parse_header("/* Rangalipi 2 */"),
        Some(("Rangalipi 2".into(), false))
    );
    assert_eq!(
        parse_header("/* Rangalipi Base - active */"),
        Some(("Rangalipi Base".into(), true))
    );
}

#[test]
fn var_lines_recognised_wrapped_and_bare() {
    assert!(is_var_line("  --accent: #fff;"));
    assert!(is_var_line("  /* --accent: #fff; */"));
    assert!(!is_var_line("  /* Rangalipi Moss */"));
    assert!(!is_var_line("  --1bad: x;"));
    assert!(!is_var_line(""));
}

#[test]
fn theme_vars_are_scoped_to_their_own_block() {
    let s = parse(CSS).unwrap();
    assert_eq!(
        s.theme_vars("Rangalipi Wine").get("acrylic").map(String::as_str),
        Some("#15101d")
    );
    assert_eq!(
        s.theme_vars("Rangalipi Moss").get("accent").map(String::as_str),
        Some("#7aa85f")
    );
    assert_eq!(
        s.theme_vars("rANGALIPI mOSS").get("teal").map(String::as_str),
        Some("#5f8f7a")
    );
    // An unknown theme yields nothing rather than borrowing another block's.
    assert!(s.theme_vars("No Such Theme").is_empty());
}

// ---- switching ---------------------------------------------------------

#[test]
fn set_round_trips_byte_identically() {
    let mut s = parse(CSS).unwrap();
    s.set("Rangalipi Moss").unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Moss"));
    s.set("Rangalipi Wine").unwrap();
    assert_eq!(s.lines.join("\n"), CSS.trim_end(), "round trip changed the source");
}

#[test]
fn set_is_case_insensitive_and_lists_themes_on_failure() {
    let mut s = parse(CSS).unwrap();
    assert_eq!(s.set("rANGALIPI mOSS").unwrap(), "Rangalipi Moss");
    let err = s.set("Nope").unwrap_err();
    assert!(err.contains("unknown theme"), "{err}");
    assert!(err.contains("Rangalipi Base"), "error should list themes: {err}");
}

#[test]
fn repeated_set_of_the_same_theme_is_idempotent() {
    let mut s = parse(CSS).unwrap();
    s.set("Rangalipi Moss").unwrap();
    let once = s.lines.clone();
    s.set("Rangalipi Moss").unwrap();
    s.set("Rangalipi Moss").unwrap();
    assert_eq!(s.lines, once);
}

#[test]
fn active_name_tracks_the_switch_within_one_process() {
    // Regression: set() rewrote the header line but left the cached
    // marked_active flag stale, so a second switch in the same process
    // started from the wrong theme.
    let mut s = parse(CSS).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Wine"));
    s.set("Rangalipi Base").unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Base"));
    s.set("Rangalipi Moss").unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Moss"));
    // Two steps in one process must walk file order (Base, Wine, Moss),
    // not restart from the theme that was active when the file was read.
    s.step(1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Base"));
    s.step(1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Wine"));
}

#[test]
fn step_wraps_around_both_ends() {
    let mut s = parse(CSS).unwrap();
    s.step(1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Moss"));
    s.step(1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Base"));
    s.step(1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Wine"));
    s.step(-1).unwrap();
    assert_eq!(s.active_name().as_deref(), Some("Rangalipi Base"));
}

// ---- colours -----------------------------------------------------------

#[test]
fn parses_every_css_colour_form() {
    assert_eq!(parse_color("#fff"), Some(Rgba::rgb(255, 255, 255)));
    assert_eq!(parse_color("#C04E68"), Some(Rgba::rgb(192, 78, 104)));
    assert_eq!(parse_color("  #1a1015  "), Some(Rgba::rgb(26, 16, 21)));
    assert_eq!(
        parse_color("#00000080"),
        Some(Rgba { r: 0, g: 0, b: 0, a: 128 })
    );
    assert_eq!(parse_color("rgb(26, 16, 21)"), Some(Rgba::rgb(26, 16, 21)));
    assert_eq!(
        parse_color("rgba(26,16,21,0.5)"),
        Some(Rgba { r: 26, g: 16, b: 21, a: 128 })
    );
    assert_eq!(parse_color("rgb(0%, 40%, 100%)"), Some(Rgba::rgb(0, 102, 255)));
}

#[test]
fn rejects_things_that_are_not_colours() {
    for bad in ["none", "var(--accent)", "transparent", "url(a.png)", "#12345", "", "red"] {
        assert_eq!(parse_color(bad), None, "{bad} should not parse");
    }
}

#[test]
fn alpha_composites_over_an_underlay() {
    // 255*128/255 = 128 exactly; the blend rounds rather than truncates.
    let half = Rgba { r: 255, g: 255, b: 255, a: 128 };
    assert_eq!(half.over(Rgba::rgb(0, 0, 0)), Rgba::rgb(128, 128, 128));
    // An opaque source short-circuits.
    assert_eq!(
        Rgba::rgb(1, 2, 3).over(Rgba::rgb(9, 9, 9)),
        Rgba::rgb(1, 2, 3)
    );
}

#[test]
fn active_theme_vars_reads_only_the_active_block() {
    let vars = active_theme_vars(CSS);
    let map: std::collections::HashMap<_, _> = vars.into_iter().collect();
    // Keys keep their leading dashes: both consumers (the picker's theme.rs
    // and saka-popup's) look up "--text" / "--accent" verbatim.
    assert_eq!(map.get("--acrylic").map(String::as_str), Some("#15101d"));
    assert_eq!(map.get("--accent").map(String::as_str), Some("#c04e68"));
    // Moss is inactive and wrapped, so nothing of it may appear.
    assert!(
        !map.contains_key("--mauve"),
        "wrapped inactive vars leaked: {map:?}"
    );
}

// ---- GDI alpha ---------------------------------------------------------

#[test]
fn force_opaque_alpha_sets_only_the_alpha_byte() {
    let (w, h) = (3i32, 2i32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    // Distinct RGB in every pixel so a stride mistake shows up.
    for (i, px) in buf.chunks_mut(4).enumerate() {
        px[0] = i as u8;
        px[1] = 0xAA;
        px[2] = 0x55;
    }
    force_opaque_alpha(buf.as_mut_ptr(), w, h);
    assert!(
        buf.chunks(4).all(|px| px[3] == 255),
        "alpha byte not set everywhere"
    );
    assert_eq!(&buf[0..4], &[0, 0xAA, 0x55, 255]);
    assert_eq!(&buf[4..8], &[1, 0xAA, 0x55, 255]);
}

#[test]
fn force_opaque_alpha_tolerates_degenerate_input() {
    // A null DIB is a real possibility when CreateDIBSection fails and the
    // caller wants to bail out quietly rather than fault.
    force_opaque_alpha(std::ptr::null_mut(), 4, 4);
    let mut buf = vec![0u8; 16];
    force_opaque_alpha(buf.as_mut_ptr(), 0, 4);
    force_opaque_alpha(buf.as_mut_ptr(), 4, -1);
    assert!(
        buf.iter().all(|&b| b == 0),
        "degenerate sizes must not write"
    );
}

// ---- config sync -------------------------------------------------------

fn temp(name: &str, body: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("yasb-theme-test-{}-{name}", std::process::id()));
    fs::write(&p, body).unwrap();
    p
}

#[test]
fn sync_rewrites_cava_and_pomodoro_from_the_theme() {
    let config = temp(
        "sync.yaml",
        concat!(
            "bar:\n",
            "  cava:\n",
            "    foreground: \"#000000\"\n",
            "    gradient_color_1: \"#000000\"\n",
            "    gradient_color_2: \"#000000\"\n",
            "    gradient_color_3: \"#000000\"\n",
            "  pomodoro:\n",
            "    options:\n",
            "      circle_work_progress_color: \"#000000\"\n",
            "      circle_break_progress_color: \"#000000\"\n",
            "  volume:\n",
            "    keep_me: \"yes\"\n",
        ),
    );
    let sheet = parse(CSS).unwrap();
    sync_config_colors(&config, &sheet, "Rangalipi Moss").unwrap();
    let out = fs::read_to_string(&config).unwrap();
    // mauve/teal come from the Moss block, not the built-in fallbacks.
    assert!(
        out.contains("circle_work_progress_color: \"#8fae6b\""),
        "{out}"
    );
    assert!(
        out.contains("circle_break_progress_color: \"#5f8f7a\""),
        "{out}"
    );
    // cava: teal/blue/mauve/peach. Moss defines teal and mauve only, so the
    // other two keep the fallbacks.
    assert!(out.contains("foreground: \"#5f8f7a\""), "{out}");
    assert!(out.contains("gradient_color_2: \"#8fae6b\""), "{out}");
    assert!(out.contains("gradient_color_1: \"#89b4fa\""), "{out}");
    assert!(out.contains("gradient_color_3: \"#fab387\""), "{out}");
    // Everything outside the two sections is untouched.
    assert!(out.contains("keep_me: \"yes\""), "{out}");
    fs::remove_file(&config).ok();
}

#[test]
fn sync_is_a_no_op_when_the_widget_is_absent() {
    let body = "bar:\n  clock:\n    label: \"x\"\n";
    let config = temp("absent.yaml", body);
    let sheet = parse(CSS).unwrap();
    sync_config_colors(&config, &sheet, "Rangalipi Wine").unwrap();
    assert_eq!(fs::read_to_string(&config).unwrap(), body);
    fs::remove_file(&config).ok();
}