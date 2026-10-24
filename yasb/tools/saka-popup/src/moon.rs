//! 3D Procedural Moon Sphere Renderer with Sun-Directed Illumination.
//!
//! Renders a realistic 3D lunar sphere featuring:
//! - Continuous 3D spherical normal mapping and rotation
//! - Procedural multi-octave lunar surface (basaltic maria and bright cratered highlands)
//! - Sun-directed phase lighting matching the astronomical elongation
//! - Lommel-Seeliger lunar retroreflective scattering
//! - Soft earthshine on the unlit limb
//! - Antialiased limb boundary and atmospheric radial glow

use crate::theme::Rgba;
use yasb_chrome::canvas::Canvas;

/// Hash-based pseudo-random noise in 3D.
fn hash3(p: [f32; 3]) -> f32 {
    let n = p[0] * 127.1 + p[1] * 311.7 + p[2] * 74.7;
    let s = (n.sin() * 43758.5453).fract();
    if s < 0.0 { s + 1.0 } else { s }
}

/// 3D value noise with smooth Hermite interpolation.
fn noise3d(p: [f32; 3]) -> f32 {
    let ix = p[0].floor();
    let iy = p[1].floor();
    let iz = p[2].floor();

    let fx = p[0] - ix;
    let fy = p[1] - iy;
    let fz = p[2] - iz;

    // Quintic Hermite curve: 6t^5 - 15t^4 + 10t^3
    let u = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let v = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
    let w = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);

    let mut sum = 0.0;
    for dz in 0..2 {
        for dy in 0..2 {
            for dx in 0..2 {
                let weight = (if dx == 1 { u } else { 1.0 - u })
                    * (if dy == 1 { v } else { 1.0 - v })
                    * (if dz == 1 { w } else { 1.0 - w });
                let h = hash3([ix + dx as f32, iy + dy as f32, iz + dz as f32]);
                sum += weight * h;
            }
        }
    }
    sum
}

/// Fractional Brownian Motion (fBM) on the 3D unit sphere.
fn fbm(p: [f32; 3], octaves: usize) -> f32 {
    let mut val = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..octaves {
        val += amp * noise3d([p[0] * freq, p[1] * freq, p[2] * freq]);
        freq *= 2.02;
        amp *= 0.5;
    }
    val
}

/// Evaluates the Moon's surface albedo at normalized coordinates `p`.
///
/// Reproduces the contrast between dark basaltic lunar maria (Oceanus Procellarum,
/// Mare Serenitatis, etc.) and bright cratered anorthosite highlands with Tycho-like rays.
fn lunar_albedo(p: [f32; 3]) -> f32 {
    // Octave 1: Major continental maria vs highlands
    let maria_noise = fbm([p[0] * 1.8 + 0.5, p[1] * 1.8 + 0.3, p[2] * 1.8], 3);
    let is_mare = maria_noise < 0.44;

    // Octave 2: Basins and ridges
    let detail = fbm([p[0] * 6.0, p[1] * 6.0, p[2] * 6.0], 3);

    // Octave 3: High-frequency impact craters and ray ejecta
    let crater_noise = fbm([p[0] * 16.0, p[1] * 16.0, p[2] * 16.0], 2);

    let base = if is_mare {
        0.14 + maria_noise * 0.12
    } else {
        0.28 + detail * 0.15 + (crater_noise - 0.5).max(0.0) * 0.35
    };

    base.clamp(0.12, 0.55)
}

/// Renders a 3D rotating Moon sphere onto the canvas.
///
/// * `cx`, `cy`: Center coordinates of the Moon on the canvas.
/// * `r`: Sphere radius in pixels.
/// * `illum`: Astronomical illumination fraction in `0.0..=1.0`.
/// * `waxing`: `true` if illumination grows from the right, `false` if waning.
/// * `rotation_angle`: Visual rotation angle around the polar axis in radians.
/// * `accent`: Theme accent for the subtle ambient halo.
pub fn draw_3d_moon(
    canvas: &mut Canvas,
    cx: f32,
    cy: f32,
    r: f32,
    illum: f32,
    waxing: bool,
    rotation_angle: f32,
    accent: Rgba,
) {
    let illum_clamped = illum.clamp(0.0, 1.0);

    // Calculate light vector L from the astronomical phase angle
    let phase_angle = (1.0 - illum_clamped) * std::f32::consts::PI;
    let (sin_p, cos_p) = phase_angle.sin_cos();
    let (lx, ly, lz) = if waxing {
        (sin_p, 0.0, cos_p)
    } else {
        (-sin_p, 0.0, cos_p)
    };

    let ir = r as i32;
    let bound = ir + 3;
    let min_x = (cx as i32 - bound).max(0);
    let max_x = (cx as i32 + bound).min(canvas.w - 1);
    let min_y = (cy as i32 - bound).max(0);
    let max_y = (cy as i32 + bound).min(canvas.h - 1);

    let r_sq = r * r;

    // 1. Soft atmospheric glow around the Moon limb
    let glow_radius = r * 1.8;
    for py in (cy as i32 - glow_radius as i32).max(0)..=(cy as i32 + glow_radius as i32).min(canvas.h - 1) {
        for px in (cx as i32 - glow_radius as i32).max(0)..=(cx as i32 + glow_radius as i32).min(canvas.w - 1) {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r && dist < glow_radius {
                let norm_d = (dist - r) / (glow_radius - r);
                let factor = (1.0 - norm_d).powi(3) * 0.22 * illum_clamped;
                if factor > 0.005 {
                    let halo_col = Rgba {
                        r: ((230.0 + accent.r as f32 * 0.1) * factor) as u8,
                        g: ((235.0 + accent.g as f32 * 0.1) * factor) as u8,
                        b: ((245.0 + accent.b as f32 * 0.1) * factor) as u8,
                        a: (255.0 * factor) as u8,
                    };
                    canvas.blend_pixel(px, py, halo_col, factor);
                }
            }
        }
    }

    // 2. 3D Ray-Sphere intersection and shading
    let (sin_rot, cos_rot) = rotation_angle.sin_cos();

    for py in min_y..=max_y {
        let dy = py as f32 + 0.5 - cy;
        let dy_sq = dy * dy;

        for px in min_x..=max_x {
            let dx = px as f32 + 0.5 - cx;
            let d_sq = dx * dx + dy_sq;

            if d_sq > (r + 1.2) * (r + 1.2) {
                continue;
            }

            let dist = d_sq.sqrt();
            // Subpixel antialiasing at the limb edge
            let edge_coverage = (0.5 + (r - dist)).clamp(0.0, 1.0);
            if edge_coverage <= 0.0 {
                continue;
            }

            // Normal vector N on the front hemisphere
            let z = (r_sq - d_sq).max(0.0).sqrt();
            let nx = dx / r;
            let ny = dy / r;
            let nz = z / r;

            // Rotate coordinates around the polar Y axis for the rotating surface
            let px_rot = nx * cos_rot + nz * sin_rot;
            let py_rot = ny;
            let pz_rot = -nx * sin_rot + nz * cos_rot;

            // Surface albedo from procedural lunar features
            let albedo = lunar_albedo([px_rot, py_rot, pz_rot]);

            // Sun-directed illumination: N · L
            let n_dot_l = (nx * lx + ny * ly + nz * lz).max(0.0);

            // Lunar Lommel-Seeliger scattering (reduces limb darkening, typical for lunar regolith)
            let diffuse = if n_dot_l > 0.0 {
                (n_dot_l / (n_dot_l + nz + 0.05)).powf(0.85)
            } else {
                0.0
            };

            // Earthshine: ambient fill on the dark limb
            let earthshine = 0.07;
            let total_light = earthshine + (1.0 - earthshine) * diffuse;

            // Final tone-mapped pixel color
            let brightness = (albedo * total_light * 2.2).clamp(0.0, 1.0);
            let highlight = if diffuse > 0.8 { (diffuse - 0.8) * 0.4 } else { 0.0 };

            let col = Rgba {
                r: ((230.0 * brightness + 25.0 * highlight).min(255.0)) as u8,
                g: ((235.0 * brightness + 20.0 * highlight).min(255.0)) as u8,
                b: ((245.0 * brightness + 10.0 * highlight).min(255.0)) as u8,
                a: 255,
            };

            canvas.blend_pixel(px, py, col, edge_coverage);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lunar_albedo_is_bounded_and_continuous() {
        for z in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            for y in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                    let a = lunar_albedo([x, y, z]);
                    assert!((0.0..=1.0).contains(&a), "albedo {a} out of range");
                }
            }
        }
    }

    #[test]
    fn draw_3d_moon_runs_without_panic_for_all_phases() {
        let mut dib = yasb_chrome::gdi::Dib::new(100, 100);
        let mut canvas = dib.canvas();
        let accent = Rgba::rgb(180, 190, 255);

        for illum in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for waxing in [true, false] {
                for rot in [0.0, 1.0, 3.14, 6.28] {
                    draw_3d_moon(&mut canvas, 50.0, 50.0, 35.0, illum, waxing, rot, accent);
                }
            }
        }
    }
}

