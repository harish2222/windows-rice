//! mandala-gen — procedural Rangalipi mandala art for all 22 themes.
//!
//! The control-center, media and pomodoro panels paint `var(--motif-mandala)`
//! as a full-panel watermark. The first pass only ever had eleven PNGs: the
//! dark/light halves of a palette shared one image, `Rangalipi Light` borrowed
//! `Rangalipi`'s, and the Wine/Wine Light pair was a byte-identical copy under
//! two names — so switching themes rarely changed the art at all.
//!
//! This tool draws **22 distinct designs** — a different ornament *pattern*
//! per theme, built from one vocabulary of primitives (petals, rays, waves,
//! scallops, lattices, spirograph weaves, …) and tinted from that theme's own
//! palette tokens in styles.css — then points every block's `--motif-mandala`
//! at its own file:
//!
//!   mandala-gen generate   rewrite the 22 urls (inserting any that are
//!                          missing) + draw the 22 PNGs, then self-check
//!   mandala-gen check      assert the urls, and that every file is unique
//!                          both byte-wise and as a *pattern* (dHash)
//!
//! Run it from the `yasb/` directory, or pass `--styles <path>`.
//!
//! The block contract in `../theme.md` still applies: rewriting touches only
//! the `url(...)` inside each `--motif-mandala` line, never the comment
//! markers that delimit an inactive block.

use std::collections::HashMap;
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_6, TAU};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use yasb_theme::{Rgba, Stylesheet, parse, parse_color, read_styles, write_atomic};

/// Output edge length in pixels — the size the stylesheet was written for
/// (`background-size: cover` scales it from here).
const SIZE: usize = 420;
/// Supersampling factor: each output pixel averages SS×SS subsamples, so the
/// strokes stay smooth instead of stair-stepping.
const SS: usize = 2;
/// Two designs whose dHash is closer than this are the same drawing recoloured,
/// which is exactly the regression this tool exists to prevent.
const MIN_PATTERN_DISTANCE: u32 = 10;

// ---------------------------------------------------------------------------
// Palette roles: which styles.css token tints a layer
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Role {
    Frame,
    Mauve,
    Peach,
    Teal,
    Blue,
    Lavender,
    Green,
    Yellow,
    Pink,
    Maroon,
    Rosewater,
}

impl Role {
    fn key(self) -> &'static str {
        match self {
            Role::Frame => "frame",
            Role::Mauve => "mauve",
            Role::Peach => "peach",
            Role::Teal => "teal",
            Role::Blue => "blue",
            Role::Lavender => "lavender",
            Role::Green => "green",
            Role::Yellow => "yellow",
            Role::Pink => "pink",
            Role::Maroon => "maroon",
            Role::Rosewater => "rosewater",
        }
    }

    fn all() -> [Role; 11] {
        [
            Role::Frame,
            Role::Mauve,
            Role::Peach,
            Role::Teal,
            Role::Blue,
            Role::Lavender,
            Role::Green,
            Role::Yellow,
            Role::Pink,
            Role::Maroon,
            Role::Rosewater,
        ]
    }
}

fn palette(vars: &HashMap<String, String>) -> HashMap<Role, Rgba> {
    let fallback = vars
        .get("frame")
        .and_then(|v| parse_color(v))
        .unwrap_or(Rgba::rgb(128, 96, 128));
    Role::all()
        .into_iter()
        .map(|role| {
            let c = vars
                .get(role.key())
                .and_then(|v| parse_color(v))
                .unwrap_or(fallback);
            (role, c)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The design vocabulary: what a layer can draw
// ---------------------------------------------------------------------------

/// A design-time layer: a primitive, the palette role that tints it, and its
/// watermark alpha. The PNG carries low alpha on purpose — it sits under text.
#[derive(Clone, Copy)]
enum Layer {
    Disc {
        r: f64,
        role: Role,
        alpha: f64,
    },
    Ring {
        r: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Dots {
        ring: f64,
        n: usize,
        phase: f64,
        size: f64,
        role: Role,
        alpha: f64,
    },
    Wave {
        r: f64,
        w: f64,
        lobes: usize,
        amp: f64,
        role: Role,
        alpha: f64,
    },
    Petal {
        n: usize,
        phase: f64,
        base: f64,
        amp: f64,
        k: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Ray {
        n: usize,
        phase: f64,
        from: f64,
        to: f64,
        halfw: f64,
        role: Role,
        alpha: f64,
    },
    Scallop {
        n: usize,
        phase: f64,
        ring: f64,
        rad: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Diamond {
        size: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Chevron {
        base: f64,
        amp: f64,
        lobes: usize,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Star {
        n: usize,
        inner: f64,
        outer: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Pinwheel {
        n: usize,
        twist: f64,
        from: f64,
        to: f64,
        halfw: f64,
        role: Role,
        alpha: f64,
    },
    Tri {
        size: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
    Spiro {
        big: f64,
        small: f64,
        off: f64,
        w: f64,
        role: Role,
        alpha: f64,
    },
}

/// Render-ready layer: colours resolved, the spirograph pre-sampled.
enum Kind {
    Disc {
        r: f64,
    },
    Ring {
        r: f64,
        w: f64,
    },
    Dots {
        ring: f64,
        n: usize,
        phase: f64,
        size: f64,
    },
    Wave {
        r: f64,
        w: f64,
        lobes: usize,
        amp: f64,
    },
    Petal {
        n: usize,
        phase: f64,
        base: f64,
        amp: f64,
        k: f64,
        w: f64,
    },
    Ray {
        n: usize,
        phase: f64,
        from: f64,
        to: f64,
        halfw: f64,
    },
    Scallop {
        n: usize,
        phase: f64,
        ring: f64,
        rad: f64,
        w: f64,
    },
    Diamond {
        size: f64,
        w: f64,
    },
    Chevron {
        base: f64,
        amp: f64,
        lobes: usize,
        w: f64,
    },
    Star {
        n: usize,
        inner: f64,
        outer: f64,
        w: f64,
    },
    Pinwheel {
        n: usize,
        twist: f64,
        from: f64,
        to: f64,
        halfw: f64,
    },
    Tri {
        size: f64,
        w: f64,
    },
    Spiro {
        w: f64,
        pts: Vec<(f64, f64)>,
        rmin: f64,
        rmax: f64,
    },
}

struct Prim {
    kind: Kind,
    color: [f64; 3],
    alpha: f64,
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Linear 0..1 ramp: 1 inside `half - aa`, 0 past `half + aa`, smooth between.
#[inline]
fn ramp(half: f64, dist: f64, aa: f64) -> f64 {
    (1.0 - (dist - (half - aa)) / (2.0 * aa)).clamp(0.0, 1.0)
}

/// Soft 1 inside `[from, to]` — keeps the ends of rays from aliasing.
#[inline]
fn band(from: f64, to: f64, rr: f64, aa: f64) -> f64 {
    ((rr - from) / (2.0 * aa)).clamp(0.0, 1.0) * ((to - rr) / (2.0 * aa)).clamp(0.0, 1.0)
}

/// Fade out past `r` — clips lattices to the medallion.
#[inline]
fn clip_disc(rr: f64, r: f64, aa: f64) -> f64 {
    ((r - rr) / (2.0 * aa)).clamp(0.0, 1.0)
}

/// Fraction in 0..1 for any real x (Rust's `fract` is negative below zero,
/// and `th` runs from -π).
#[inline]
fn frac(x: f64) -> f64 {
    x - x.floor()
}

fn spiro_pts(big: f64, small: f64, off: f64, n: usize) -> Vec<(f64, f64)> {
    // Seven revolutions: for the irrational-looking ratios used here that
    // closes the weave densely without repeating exactly.
    let turns = 7.0;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = TAU * turns * i as f64 / n as f64;
        let rr = big - small;
        let x = rr * t.cos() + off * (rr / small * t).cos();
        let y = rr * t.sin() - off * (rr / small * t).sin();
        pts.push((x, y));
    }
    pts
}

fn prepare(layer: &Layer, pal: &HashMap<Role, Rgba>) -> Prim {
    let rgb = |role: Role| -> [f64; 3] {
        let c = pal[&role];
        [
            c.r as f64 / 255.0,
            c.g as f64 / 255.0,
            c.b as f64 / 255.0,
        ]
    };
    let (kind, role, alpha) = match *layer {
        Layer::Disc { r, role, alpha } => (Kind::Disc { r }, role, alpha),
        Layer::Ring { r, w, role, alpha } => (Kind::Ring { r, w }, role, alpha),
        Layer::Dots { ring, n, phase, size, role, alpha } => (
            Kind::Dots { ring, n, phase, size },
            role,
            alpha,
        ),
        Layer::Wave { r, w, lobes, amp, role, alpha } => {
            (Kind::Wave { r, w, lobes, amp }, role, alpha)
        }
        Layer::Petal { n, phase, base, amp, k, w, role, alpha } => (
            Kind::Petal { n, phase, base, amp, k, w },
            role,
            alpha,
        ),
        Layer::Ray { n, phase, from, to, halfw, role, alpha } => (
            Kind::Ray { n, phase, from, to, halfw },
            role,
            alpha,
        ),
        Layer::Scallop { n, phase, ring, rad, w, role, alpha } => (
            Kind::Scallop { n, phase, ring, rad, w },
            role,
            alpha,
        ),
        Layer::Diamond { size, w, role, alpha } => (Kind::Diamond { size, w }, role, alpha),
        Layer::Chevron { base, amp, lobes, w, role, alpha } => (
            Kind::Chevron { base, amp, lobes, w },
            role,
            alpha,
        ),
        Layer::Star { n, inner, outer, w, role, alpha } => {
            (Kind::Star { n, inner, outer, w }, role, alpha)
        }
        Layer::Pinwheel { n, twist, from, to, halfw, role, alpha } => (
            Kind::Pinwheel { n, twist, from, to, halfw },
            role,
            alpha,
        ),
        Layer::Tri { size, w, role, alpha } => (Kind::Tri { size, w }, role, alpha),
        Layer::Spiro { big, small, off, w, role, alpha } => {
            let pts = spiro_pts(big, small, off, 720);
            let (mut rmin, mut rmax) = (f64::INFINITY, 0.0f64);
            for &(x, y) in &pts {
                let r = (x * x + y * y).sqrt();
                rmin = rmin.min(r);
                rmax = rmax.max(r);
            }
            (
                Kind::Spiro { w, pts, rmin, rmax },
                role,
                alpha,
            )
        }
    };
    Prim { kind, color: rgb(role), alpha }
}

/// Stroke coverage of one prim at (u, v); `rr`/`th` are the polar forms.
#[allow(clippy::too_many_arguments)]
fn coverage(u: f64, v: f64, rr: f64, th: f64, aa: f64, k: &Kind) -> f64 {
    match k {
        Kind::Disc { r } => ramp(*r, rr, aa),
        Kind::Ring { r, w } => ramp(*w, (rr - r).abs(), aa),
        Kind::Dots { ring, n, phase, size } => {
            let mut best = 0.0;
            for i in 0..*n {
                let a = phase + TAU * i as f64 / *n as f64;
                let dx = u - ring * a.cos();
                let dy = v - ring * a.sin();
                let c = ramp(*size, (dx * dx + dy * dy).sqrt(), aa);
                if c > best {
                    best = c;
                }
            }
            best
        }
        Kind::Wave { r, w, lobes, amp } => {
            let f = r + amp * (th * *lobes as f64).sin();
            ramp(*w, (rr - f).abs(), aa)
        }
        Kind::Petal { n, phase, base, amp, k, w } => {
            let lobe = (0.5 + 0.5 * (*n as f64 * (th - phase)).cos()).powf(*k);
            ramp(*w, (rr - (base + amp * lobe)).abs(), aa)
        }
        Kind::Ray { n, phase, from, to, halfw } => {
            let step = TAU / *n as f64;
            let a = th - phase;
            let k = (a / step).round();
            let da = (a - k * step).abs();
            ramp(*halfw, da * rr, aa) * band(*from, *to, rr, aa)
        }
        Kind::Scallop { n, phase, ring, rad, w } => {
            let mut best = 0.0;
            for i in 0..*n {
                let a = phase + TAU * i as f64 / *n as f64;
                let dx = u - ring * a.cos();
                let dy = v - ring * a.sin();
                let dist = ((dx * dx + dy * dy).sqrt() - rad).abs();
                let c = ramp(*w, dist, aa);
                if c > best {
                    best = c;
                }
            }
            best
        }
        Kind::Diamond { size, w } => {
            let p = (u + v) / size;
            let q = (u - v) / size;
            let dp = (p - p.round()).abs();
            let dq = (q - q.round()).abs();
            ramp(*w, dp.min(dq) * size * FRAC_1_SQRT_2, aa) * clip_disc(rr, 0.96, aa)
        }
        Kind::Tri { size, w } => {
            let mut d = f64::INFINITY;
            for ang in [FRAC_PI_6, FRAC_PI_2, FRAC_PI_6 * 5.0] {
                let t = (u * ang.cos() + v * ang.sin()) / size;
                d = d.min((t - t.round()).abs() * size);
            }
            ramp(*w, d, aa) * clip_disc(rr, 0.96, aa)
        }
        Kind::Chevron { base, amp, lobes, w } => {
            let f = base + amp * (1.0 - 2.0 * frac(th * *lobes as f64 / TAU)).abs();
            ramp(*w, (rr - f).abs(), aa)
        }
        Kind::Star { n, inner, outer, w } => {
            let t = frac(th * *n as f64 / TAU);
            let f = inner + (outer - inner) * (2.0 * t - 1.0).abs();
            ramp(*w, (rr - f).abs(), aa)
        }
        Kind::Pinwheel { n, twist, from, to, halfw } => {
            let a = th + twist * rr * rr;
            let step = TAU / *n as f64;
            let k = (a / step).round();
            let da = (a - k * step).abs();
            ramp(*halfw, da * rr, aa) * band(*from, *to, rr, aa)
        }
        Kind::Spiro { w, pts, rmin, rmax } => {
            if rr < rmin - w - 2.0 * aa || rr > rmax + w + 2.0 * aa {
                return 0.0;
            }
            let mut best2 = f64::INFINITY;
            for &(px, py) in pts {
                let dx = u - px;
                let dy = v - py;
                let d2 = dx * dx + dy * dy;
                if d2 < best2 {
                    best2 = d2;
                }
            }
            ramp(*w, best2.sqrt(), aa)
        }
    }
}

// ---------------------------------------------------------------------------
// The 22 designs — one distinct pattern per theme, keyed by file stem
// ---------------------------------------------------------------------------

fn stem_of(theme: &str) -> String {
    let t = theme.trim();
    let rest = t.strip_prefix("Rangalipi").map(str::trim).unwrap_or(t);
    if rest.is_empty() {
        "base".to_string()
    } else {
        rest.to_ascii_lowercase().replace(' ', "-")
    }
}

fn design_for(stem: &str) -> Result<(&'static str, Vec<Layer>), String> {
    use Layer::*;
    let d = match stem {
        "base" => (
            "lotus crown",
            vec![
                Ring { r: 0.93, w: 0.010, role: Role::Frame, alpha: 0.50 },
                Petal { n: 12, phase: 0.0, base: 0.55, amp: 0.30, k: 1.6, w: 0.008, role: Role::Mauve, alpha: 0.45 },
                Ring { r: 0.50, w: 0.008, role: Role::Peach, alpha: 0.42 },
                Dots { ring: 0.75, n: 24, phase: 0.0, size: 0.016, role: Role::Teal, alpha: 0.45 },
                Ring { r: 0.20, w: 0.007, role: Role::Mauve, alpha: 0.40 },
                Disc { r: 0.06, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "light" => (
            "star and dots",
            vec![
                Star { n: 9, inner: 0.25, outer: 0.85, w: 0.009, role: Role::Blue, alpha: 0.45 },
                Ring { r: 0.60, w: 0.008, role: Role::Teal, alpha: 0.45 },
                Dots { ring: 0.92, n: 36, phase: 0.10, size: 0.013, role: Role::Peach, alpha: 0.45 },
                Dots { ring: 0.40, n: 12, phase: 0.26, size: 0.018, role: Role::Frame, alpha: 0.45 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "ember" => (
            "flame burst",
            vec![
                Ray { n: 16, phase: 0.0, from: 0.15, to: 0.95, halfw: 0.006, role: Role::Frame, alpha: 0.50 },
                Star { n: 8, inner: 0.30, outer: 0.75, w: 0.008, role: Role::Peach, alpha: 0.45 },
                Ring { r: 0.97, w: 0.010, role: Role::Maroon, alpha: 0.45 },
                Ring { r: 0.35, w: 0.007, role: Role::Yellow, alpha: 0.40 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "ember-light" => (
            "petal scallops",
            vec![
                Scallop { n: 10, phase: 0.0, ring: 0.55, rad: 0.30, w: 0.008, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.92, w: 0.009, role: Role::Mauve, alpha: 0.45 },
                Scallop { n: 6, phase: 0.5, ring: 0.30, rad: 0.16, w: 0.007, role: Role::Peach, alpha: 0.42 },
                Dots { ring: 0.78, n: 18, phase: 0.17, size: 0.014, role: Role::Teal, alpha: 0.42 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "mossfern" => (
            "fern spiral",
            vec![
                Pinwheel { n: 9, twist: 1.6, from: 0.12, to: 0.92, halfw: 0.006, role: Role::Green, alpha: 0.50 },
                Ring { r: 0.93, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.55, n: 9, phase: 0.35, size: 0.017, role: Role::Yellow, alpha: 0.45 },
                Ring { r: 0.30, w: 0.007, role: Role::Green, alpha: 0.42 },
                Disc { r: 0.045, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "mossfern-light" => (
            "honeycomb lattice",
            vec![
                Tri { size: 0.16, w: 0.006, role: Role::Teal, alpha: 0.42 },
                Ring { r: 0.90, w: 0.009, role: Role::Green, alpha: 0.45 },
                Ring { r: 0.45, w: 0.007, role: Role::Frame, alpha: 0.40 },
                Dots { ring: 0.68, n: 14, phase: 0.0, size: 0.014, role: Role::Blue, alpha: 0.40 },
            ],
        ),
        "wine" => (
            "spirograph weave",
            vec![
                Spiro { big: 0.62, small: 0.23, off: 0.52, w: 0.006, role: Role::Mauve, alpha: 0.50 },
                Ring { r: 0.96, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.18, w: 0.007, role: Role::Peach, alpha: 0.40 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "wine-light" => (
            "paisley rosette",
            vec![
                Petal { n: 8, phase: 0.196, base: 0.50, amp: 0.34, k: 2.6, w: 0.008, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.30, n: 8, phase: 0.196, size: 0.020, role: Role::Mauve, alpha: 0.45 },
                Ring { r: 0.94, w: 0.009, role: Role::Peach, alpha: 0.45 },
                Ray { n: 8, phase: 0.589, from: 0.62, to: 0.88, halfw: 0.005, role: Role::Mauve, alpha: 0.40 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "dune" => (
            "dune waves",
            vec![
                Wave { r: 0.72, w: 0.009, lobes: 6, amp: 0.06, role: Role::Yellow, alpha: 0.45 },
                Wave { r: 0.52, w: 0.007, lobes: 9, amp: 0.05, role: Role::Peach, alpha: 0.45 },
                Ring { r: 0.92, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.30, n: 12, phase: 0.0, size: 0.015, role: Role::Maroon, alpha: 0.42 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "dune-light" => (
            "chevron fan",
            vec![
                Chevron { base: 0.60, amp: 0.05, lobes: 12, w: 0.008, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.30, w: 0.007, role: Role::Peach, alpha: 0.45 },
                Ray { n: 24, phase: 0.13, from: 0.75, to: 0.92, halfw: 0.0045, role: Role::Blue, alpha: 0.42 },
                Dots { ring: 0.45, n: 16, phase: 0.0, size: 0.012, role: Role::Teal, alpha: 0.40 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "matcha" => (
            "tea whisk rings",
            vec![
                Ring { r: 0.88, w: 0.013, role: Role::Green, alpha: 0.50 },
                Ring { r: 0.62, w: 0.008, role: Role::Frame, alpha: 0.42 },
                Dots { ring: 0.75, n: 16, phase: 0.196, size: 0.020, role: Role::Yellow, alpha: 0.45 },
                Wave { r: 0.36, w: 0.007, lobes: 4, amp: 0.03, role: Role::Green, alpha: 0.45 },
                Disc { r: 0.06, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "matcha-light" => (
            "diamond grid",
            vec![
                Diamond { size: 0.14, w: 0.006, role: Role::Teal, alpha: 0.42 },
                Ring { r: 0.90, w: 0.009, role: Role::Green, alpha: 0.45 },
                Ring { r: 0.48, w: 0.007, role: Role::Pink, alpha: 0.40 },
                Dots { ring: 0.68, n: 8, phase: 0.39, size: 0.018, role: Role::Frame, alpha: 0.45 },
            ],
        ),
        "espresso" => (
            "layered star polygon",
            vec![
                Star { n: 14, inner: 0.35, outer: 0.92, w: 0.0075, role: Role::Peach, alpha: 0.45 },
                Star { n: 7, inner: 0.15, outer: 0.50, w: 0.007, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.70, n: 28, phase: 0.11, size: 0.010, role: Role::Yellow, alpha: 0.40 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "espresso-light" => (
            "coffee bloom",
            vec![
                Wave { r: 0.74, w: 0.010, lobes: 5, amp: 0.07, role: Role::Peach, alpha: 0.48 },
                Scallop { n: 5, phase: 0.10, ring: 0.44, rad: 0.26, w: 0.008, role: Role::Frame, alpha: 0.45 },
                Scallop { n: 5, phase: 0.72, ring: 0.44, rad: 0.18, w: 0.006, role: Role::Maroon, alpha: 0.42 },
                Dots { ring: 0.22, n: 5, phase: 0.10, size: 0.020, role: Role::Peach, alpha: 0.50 },
                Ring { r: 0.96, w: 0.009, role: Role::Teal, alpha: 0.42 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "noir" => (
            "art deco fan",
            vec![
                Ray { n: 27, phase: 0.0, from: 0.35, to: 0.95, halfw: 0.0045, role: Role::Rosewater, alpha: 0.45 },
                Ring { r: 0.95, w: 0.010, role: Role::Frame, alpha: 0.50 },
                Ring { r: 0.72, w: 0.006, role: Role::Lavender, alpha: 0.45 },
                Scallop { n: 12, phase: 0.26, ring: 0.34, rad: 0.14, w: 0.006, role: Role::Frame, alpha: 0.45 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "noir-light" => (
            "strong pinwheel",
            vec![
                Pinwheel { n: 6, twist: 3.2, from: 0.10, to: 0.90, halfw: 0.007, role: Role::Blue, alpha: 0.45 },
                Ring { r: 0.93, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.50, n: 6, phase: 0.52, size: 0.022, role: Role::Pink, alpha: 0.45 },
                Ring { r: 0.22, w: 0.007, role: Role::Lavender, alpha: 0.45 },
            ],
        ),
        "aubergine" => (
            "peacock eyes",
            vec![
                Dots { ring: 0.80, n: 18, phase: 0.0, size: 0.055, role: Role::Lavender, alpha: 0.42 },
                Dots { ring: 0.80, n: 18, phase: 0.0, size: 0.026, role: Role::Mauve, alpha: 0.50 },
                Ring { r: 0.95, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.55, w: 0.008, role: Role::Pink, alpha: 0.42 },
                Dots { ring: 0.30, n: 10, phase: 0.31, size: 0.018, role: Role::Teal, alpha: 0.45 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "aubergine-light" => (
            "kaleidoscope wedges",
            vec![
                Ray { n: 18, phase: 0.0, from: 0.10, to: 0.55, halfw: 0.012, role: Role::Frame, alpha: 0.42 },
                Star { n: 9, inner: 0.50, outer: 0.90, w: 0.008, role: Role::Blue, alpha: 0.45 },
                Ring { r: 0.95, w: 0.009, role: Role::Maroon, alpha: 0.42 },
                Dots { ring: 0.70, n: 9, phase: 0.35, size: 0.016, role: Role::Teal, alpha: 0.42 },
            ],
        ),
        "clay" => (
            "pot rim scallops",
            vec![
                Scallop { n: 14, phase: 0.0, ring: 0.62, rad: 0.24, w: 0.007, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.94, w: 0.010, role: Role::Peach, alpha: 0.50 },
                Ring { r: 0.34, w: 0.008, role: Role::Maroon, alpha: 0.45 },
                Dots { ring: 0.48, n: 14, phase: 0.22, size: 0.013, role: Role::Yellow, alpha: 0.45 },
                Disc { r: 0.05, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "clay-light" => (
            "kolam weave",
            vec![
                Scallop { n: 8, phase: 0.0, ring: 0.45, rad: 0.30, w: 0.006, role: Role::Teal, alpha: 0.45 },
                Scallop { n: 8, phase: 0.393, ring: 0.45, rad: 0.30, w: 0.006, role: Role::Frame, alpha: 0.45 },
                Ring { r: 0.90, w: 0.008, role: Role::Frame, alpha: 0.40 },
                Dots { ring: 0.78, n: 16, phase: 0.10, size: 0.014, role: Role::Pink, alpha: 0.45 },
            ],
        ),
        "olive" => (
            "lance leaves",
            vec![
                Petal { n: 7, phase: 0.45, base: 0.45, amp: 0.45, k: 3.2, w: 0.008, role: Role::Green, alpha: 0.45 },
                Ray { n: 7, phase: 0.45, from: 0.30, to: 0.88, halfw: 0.004, role: Role::Green, alpha: 0.40 },
                Ring { r: 0.94, w: 0.009, role: Role::Frame, alpha: 0.45 },
                Dots { ring: 0.22, n: 7, phase: 0.45, size: 0.016, role: Role::Yellow, alpha: 0.45 },
                Disc { r: 0.04, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        "olive-light" => (
            "hairline sunburst",
            vec![
                Ray { n: 64, phase: 0.0, from: 0.35, to: 0.90, halfw: 0.0035, role: Role::Frame, alpha: 0.40 },
                Ring { r: 0.93, w: 0.009, role: Role::Green, alpha: 0.45 },
                Ring { r: 0.33, w: 0.008, role: Role::Pink, alpha: 0.45 },
                Dots { ring: 0.62, n: 48, phase: 0.03, size: 0.008, role: Role::Teal, alpha: 0.45 },
                Disc { r: 0.045, role: Role::Frame, alpha: 0.50 },
            ],
        ),
        other => {
            return Err(format!(
                "no design for stem '{other}' — add one in design_for()"
            ));
        }
    };
    Ok(d)
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(design: &[Layer], pal: &HashMap<Role, Rgba>) -> Vec<u8> {
    let prims: Vec<Prim> = design.iter().map(|l| prepare(l, pal)).collect();
    let hi = SIZE * SS;
    let ss = (SS * SS) as f64;
    let aa = 2.0 / SIZE as f64;
    let mut out = vec![0u8; SIZE * SIZE * 4];

    for oy in 0..SIZE {
        for ox in 0..SIZE {
            let (mut pr, mut pg, mut pb, mut pa) = (0.0f64, 0.0, 0.0, 0.0);
            for sy in 0..SS {
                for sx in 0..SS {
                    let y = oy * SS + sy;
                    let x = ox * SS + sx;
                    let u = ((x as f64 + 0.5) / hi as f64) * 2.0 - 1.0;
                    let v = ((y as f64 + 0.5) / hi as f64) * 2.0 - 1.0;
                    let rr = (u * u + v * v).sqrt();
                    let th = v.atan2(u);

                    // Premultiplied "over", starting from transparent.
                    let (mut sr, mut sg, mut sb, mut sa) = (0.0f64, 0.0, 0.0, 0.0);
                    for p in &prims {
                        let cov = coverage(u, v, rr, th, aa, &p.kind);
                        if cov <= 0.0 {
                            continue;
                        }
                        let a = cov * p.alpha;
                        let keep = 1.0 - a;
                        sr = p.color[0] * a + sr * keep;
                        sg = p.color[1] * a + sg * keep;
                        sb = p.color[2] * a + sb * keep;
                        sa = a + sa * keep;
                    }
                    pr += sr;
                    pg += sg;
                    pb += sb;
                    pa += sa;
                }
            }

            let a = pa / ss;
            if a <= 1e-6 {
                continue; // stays fully transparent
            }
            let o = (oy * SIZE + ox) * 4;
            out[o] = ((pr / ss / a) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[o + 1] = ((pg / ss / a) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[o + 2] = ((pb / ss / a) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[o + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

fn png_write(path: &Path, rgba: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), SIZE as u32, SIZE as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_filter(png::FilterType::Paeth);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header for {}: {e}", path.display()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("png data for {}: {e}", path.display()))
}

fn png_read(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png info for {}: {e}", path.display()))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png frame for {}: {e}", path.display()))?;
    buf.truncate(info.buffer_size());
    if info.color_type != png::ColorType::Rgba {
        return Err(format!("{} is not RGBA", path.display()));
    }
    Ok((info.width, info.height, buf))
}

/// 64-bit difference hash over a 9×8 grayscale grid — two files with the same
/// *drawing* hash alike even when their colours differ, which is the failure
/// mode (a recoloured copy) that byte comparison cannot see.
fn dhash(rgba: &[u8], w: usize, h: usize) -> u64 {
    let mut cells = [0u64; 72];
    for j in 0..8usize {
        for i in 0..9usize {
            let x0 = i * w / 9;
            let x1 = ((i + 1) * w / 9).max(x0 + 1);
            let y0 = j * h / 8;
            let y1 = ((j + 1) * h / 8).max(y0 + 1);
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let o = (y * w + x) * 4;
                    // Transparent background reads as black: the silhouette of
                    // the mandala is part of what makes a pattern itself.
                    sum += (rgba[o] as u64 * 299 + rgba[o + 1] as u64 * 587 + rgba[o + 2] as u64 * 114) / 1000;
                    n += 1;
                }
            }
            cells[j * 9 + i] = sum / n.max(1);
        }
    }
    let mut hash = 0u64;
    for j in 0..8usize {
        for i in 0..8usize {
            let idx = j * 9 + i;
            if cells[idx] > cells[idx + 1] {
                hash |= 1u64 << (j * 8 + i);
            }
        }
    }
    hash
}

// ---------------------------------------------------------------------------
// styles.css
// ---------------------------------------------------------------------------

/// Absolute, forward-slash directory the PNGs live in, so the rewritten urls
/// match the format the stylesheet already uses (`C:/Users/.../yasb/`).
fn prefix_of(styles: &Path) -> Result<String, String> {
    let abs = fs::canonicalize(styles).map_err(|e| format!("cannot resolve {}: {e}", styles.display()))?;
    let dir = abs
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", styles.display()))?;
    let raw = dir.display().to_string().replace('\\', "/");
    // canonicalize hands back the verbatim `\\?\C:\...` form on Windows;
    // the stylesheet uses plain `C:/...` paths.
    let mut s = raw.strip_prefix("//?/").unwrap_or(&raw).to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    Ok(s)
}

fn url_of(line: &str) -> Option<&str> {
    let rest = &line[line.find("url(")?..];
    let inner_start = rest.find('"')? + 1;
    let inner_end = rest[inner_start..].find('"')? + inner_start;
    Some(&rest[inner_start..inner_end])
}

fn mandala_line(sheet: &Stylesheet, region: &yasb_theme::Region) -> Option<usize> {
    region
        .vars
        .iter()
        .copied()
        .find(|&v| sheet.lines[v].contains("--motif-mandala:"))
}

fn find_styles() -> PathBuf {
    for cand in ["styles.css", "yasb/styles.css"] {
        let p = PathBuf::from(cand);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("styles.css")
}

/// Point every block at `motif-<stem>-mandala.png`, preserving everything else
/// byte-for-byte (line endings, comment markers, trailing newline).
///
/// A block that has no `--motif-mandala` at all gets one inserted right after
/// its `--motif:` line, so adding a 23rd theme only means running `generate`.
/// Insertions are applied back-to-front: the region indices stay valid because
/// later lines move before earlier ones, never after.
fn rewrite_urls(sheet: &mut Stylesheet, prefix: &str) -> Result<bool, String> {
    let regions: Vec<(String, String)> = sheet
        .regions
        .iter()
        .map(|r| (r.name.clone(), stem_of(&r.name)))
        .collect();
    let mut changed = false;
    // (insert-after index, line to add) — collected, applied in reverse below.
    let mut insertions: Vec<(usize, String)> = Vec::new();
    for (name, stem) in &regions {
        let region = sheet
            .regions
            .iter()
            .find(|r| &r.name == name)
            .expect("region just listed");
        let want = format!("url(\"{prefix}motif-{stem}-mandala.png\")");
        match mandala_line(sheet, region) {
            Some(idx) => {
                let line = sheet.lines[idx].clone();
                let start = line
                    .find("url(")
                    .ok_or_else(|| format!("theme '{name}': --motif-mandala has no url(...)"))?;
                let end = line[start..]
                    .find(';')
                    .map(|i| start + i)
                    .unwrap_or(line.len());
                let new_line = format!("{}{}{}", &line[..start], want, &line[end..]);
                if new_line != sheet.lines[idx] {
                    sheet.lines[idx] = new_line;
                    changed = true;
                }
            }
            None => {
                let idx = region
                    .vars
                    .iter()
                    .copied()
                    .find(|&v| sheet.lines[v].contains("--motif:"))
                    .ok_or_else(|| {
                        format!("theme '{name}' declares neither --motif-mandala nor --motif")
                    })?;
                let indent: String =
                    sheet.lines[idx].chars().take_while(|c| c.is_whitespace()).collect();
                insertions.push((idx, format!("{indent}--motif-mandala: {want};")));
                changed = true;
            }
        }
    }
    insertions.sort_by_key(|(i, _)| std::cmp::Reverse(*i));
    for (idx, line) in insertions {
        sheet.lines.insert(idx + 1, line);
    }
    Ok(changed)
}

fn assemble(sheet: &Stylesheet) -> String {
    let mut out = sheet.lines.join(&sheet.newline);
    if sheet.ends_with_newline {
        out.push_str(&sheet.newline);
    }
    out
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn generate(styles: &Path) -> Result<(), String> {
    let text = read_styles(styles)?;
    let mut sheet = parse(&text)?;
    let prefix = prefix_of(styles)?;
    let dir = styles
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    if rewrite_urls(&mut sheet, &prefix)? {
        write_atomic(styles, assemble(&sheet).as_bytes())?;
        println!("styles.css: --motif-mandala urls rewritten");
    } else {
        println!("styles.css: urls already correct");
    }

    let names: Vec<String> = sheet.regions.iter().map(|r| r.name.clone()).collect();
    for name in names {
        let stem = stem_of(&name);
        let (design_name, design) = design_for(&stem)?;
        let vars = sheet.theme_vars(&name);
        if vars.is_empty() {
            return Err(format!("theme '{name}' exposes no variables"));
        }
        let pal = palette(&vars);
        let rgba = render(&design, &pal);
        let png_name = format!("motif-{stem}-mandala.png");
        png_write(&dir.join(&png_name), &rgba)?;
        println!("ok  {name:<26} {design_name:<22} -> {png_name}");
    }

    check(styles)
}

fn check(styles: &Path) -> Result<(), String> {
    let text = read_styles(styles)?;
    let sheet = parse(&text)?;
    if sheet.regions.len() != 22 {
        return Err(format!("expected 22 themes, found {}", sheet.regions.len()));
    }
    let prefix = prefix_of(styles)?;
    let dir = styles
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut files: Vec<(String, String, Vec<u8>)> = Vec::new(); // (theme, png name, bytes)
    for region in &sheet.regions {
        let stem = stem_of(&region.name);
        let idx = mandala_line(&sheet, region)
            .ok_or_else(|| format!("theme '{}' has no --motif-mandala", region.name))?;
        // `url_of` yields the bare path inside the quotes.
        let want = format!("{prefix}motif-{stem}-mandala.png");
        match url_of(&sheet.lines[idx]) {
            Some(got) if got == want => {}
            Some(got) => {
                return Err(format!(
                    "[{}] --motif-mandala points at {got}, expected {want}",
                    region.name
                ));
            }
            None => return Err(format!("[{}] --motif-mandala has no url(...)", region.name)),
        }
        let png_name = format!("motif-{stem}-mandala.png");
        let bytes = fs::read(dir.join(&png_name))
            .map_err(|e| format!("[{}] missing {png_name}: {e}", region.name))?;
        files.push((region.name.clone(), png_name, bytes));
    }

    for i in 0..files.len() {
        for j in i + 1..files.len() {
            if files[i].2 == files[j].2 {
                return Err(format!(
                    "{} and {} are byte-identical ({})",
                    files[i].0, files[j].0, files[i].1
                ));
            }
        }
    }

    let mut hashes: Vec<(String, u64)> = Vec::new();
    for (name, png_name, _) in &files {
        let (w, h, rgba) = png_read(&dir.join(png_name))?;
        if w != SIZE as u32 || h != SIZE as u32 {
            return Err(format!("{png_name} is {w}×{h}, expected {SIZE}×{SIZE}"));
        }
        hashes.push((name.clone(), dhash(&rgba, w as usize, h as usize)));
    }
    let (mut min_d, mut min_i, mut min_j) = (u32::MAX, 0usize, 0usize);
    for i in 0..hashes.len() {
        for j in i + 1..hashes.len() {
            let d = (hashes[i].1 ^ hashes[j].1).count_ones();
            if d < min_d {
                min_d = d;
                min_i = i;
                min_j = j;
            }
        }
    }
    if min_d < MIN_PATTERN_DISTANCE {
        return Err(format!(
            "patterns too alike: '{}' vs '{}' differ by only {min_d} dHash bits (need >= {MIN_PATTERN_DISTANCE})",
            hashes[min_i].0, hashes[min_j].0
        ));
    }

    for (name, png_name, _) in &files {
        println!("ok  {name:<26} -> {png_name}");
    }
    println!(
        "ok  22 themes: all PNGs byte-unique; closest pattern pair {} / {} at {min_d} bits (>= {MIN_PATTERN_DISTANCE})",
        hashes[min_i].0, hashes[min_j].0
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut styles: Option<PathBuf> = None;
    let mut cmd = String::from("generate");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--styles" => {
                i += 1;
                styles = args.get(i).cloned().map(PathBuf::from);
            }
            "generate" | "check" => cmd = args[i].clone(),
            other => {
                eprintln!("unknown argument {other:?} (expected generate | check | --styles PATH)");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let styles = styles.unwrap_or_else(find_styles);
    let result = match cmd.as_str() {
        "generate" => generate(&styles),
        "check" => check(&styles),
        _ => unreachable!(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
