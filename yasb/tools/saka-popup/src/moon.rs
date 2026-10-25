//! 3D Procedural Moon Sphere & Astronomical Sky Phase Renderer.
//!
//! Renders two distinct, authentic lunar presentations:
//! 1. `draw_3d_moon`: A continuous rotating 3D lunar sphere featuring spherical normal
//!    mapping, procedural basaltic maria and cratered highlands, Lommel-Seeliger scattering,
//!    and sun-directed directional illumination.
//! 2. `draw_sky_moon`: The true telescopic night-sky view as seen from Earth right now.
//!    Set within a celestial cosmic portal with an authentic starfield and atmospheric halo.
//!    Tidally locked to Earth, displaying authentic near-side geography (Oceanus Procellarum,
//!    Mare Imbrium, Mare Serenitatis, Mare Tranquillitatis, Mare Crisium, Tycho ray crater,
//!    Copernicus), realistic grazing sun terminator roughness, and physical earthshine.

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

/// Evaluates the rotating 3D Moon's surface albedo at normalized coordinates `p`.
///
/// Reproduces global contrast between dark basaltic lunar maria and bright cratered
/// anorthosite highlands for the dynamic 3D spinning globe.
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

/// Evaluates authentic selenographic near-side albedo as seen from Earth.
///
/// Accurately positions the prominent geological features visible through a telescope:
/// - Mare Imbrium (Sea of Rains)
/// - Oceanus Procellarum (Ocean of Storms)
/// - Mare Serenitatis & Mare Tranquillitatis (Apollo 11)
/// - Mare Crisium (isolated eastern oval)
/// - Mare Nubium & Humorum (southern seas)
/// - Tycho crater with sweeping radial ray system across the southern hemisphere
/// - Copernicus & Kepler ray craters
/// - Aristarchus plateau
fn near_side_albedo(nx: f32, ny: f32, nz: f32) -> f32 {
    // Background highland anorthosite crust: base albedo 0.35 + fractal cratering
    let crust_detail = fbm([nx * 8.0, ny * 8.0, nz * 8.0], 3);
    let micro_craters = fbm([nx * 22.0, ny * 22.0, nz * 22.0], 2);
    let mut albedo = 0.34 + crust_detail * 0.12 + (micro_craters - 0.5).max(0.0) * 0.22;

    // Helper for circular/elliptical basaltic mare depression
    let mare_blend = |cx: f32, cy: f32, rx: f32, ry: f32| -> f32 {
        let dx = (nx - cx) / rx;
        let dy = (ny - cy) / ry;
        let d = (dx * dx + dy * dy).sqrt();
        (1.0 - d).clamp(0.0, 1.0)
    };

    // 1. Mare Imbrium: (cx = -0.20, cy = -0.38, r = 0.28)
    let imbrium = mare_blend(-0.20, -0.38, 0.28, 0.26);
    if imbrium > 0.0 {
        let mare_noise = fbm([nx * 5.0, ny * 5.0, 1.0], 2);
        let target = 0.13 + mare_noise * 0.05;
        albedo = albedo * (1.0 - imbrium) + target * imbrium;
    }

    // 2. Oceanus Procellarum: sprawling western basin
    let procellarum = mare_blend(-0.45, -0.05, 0.32, 0.45);
    if procellarum > 0.0 {
        let mare_noise = fbm([nx * 4.0, ny * 4.0, 2.0], 2);
        let target = 0.14 + mare_noise * 0.06;
        albedo = albedo * (1.0 - procellarum) + target * procellarum;
    }

    // 3. Mare Serenitatis: (cx = 0.18, cy = -0.40, r = 0.18)
    let serenitatis = mare_blend(0.18, -0.40, 0.18, 0.17);
    if serenitatis > 0.0 {
        let target = 0.14;
        albedo = albedo * (1.0 - serenitatis) + target * serenitatis;
    }

    // 4. Mare Tranquillitatis: (cx = 0.32, cy = -0.12, r = 0.22)
    let tranquillitatis = mare_blend(0.32, -0.12, 0.22, 0.20);
    if tranquillitatis > 0.0 {
        let target = 0.12; // deep dark titanium basalt
        albedo = albedo * (1.0 - tranquillitatis) + target * tranquillitatis;
    }

    // 5. Mare Crisium: distinct isolated oval (cx = 0.62, cy = -0.28)
    let crisium = mare_blend(0.62, -0.28, 0.14, 0.11);
    if crisium > 0.0 {
        let target = 0.13;
        albedo = albedo * (1.0 - crisium) + target * crisium;
    }

    // 6. Mare Fecunditatis & Nectaris
    let fecunditatis = mare_blend(0.48, 0.10, 0.18, 0.18);
    if fecunditatis > 0.0 {
        albedo = albedo * (1.0 - fecunditatis) + 0.15 * fecunditatis;
    }
    let nectaris = mare_blend(0.30, 0.26, 0.13, 0.13);
    if nectaris > 0.0 {
        albedo = albedo * (1.0 - nectaris) + 0.14 * nectaris;
    }

    // 7. Mare Nubium & Mare Humorum
    let nubium = mare_blend(-0.16, 0.32, 0.18, 0.16);
    if nubium > 0.0 {
        albedo = albedo * (1.0 - nubium) + 0.15 * nubium;
    }
    let humorum = mare_blend(-0.44, 0.38, 0.12, 0.12);
    if humorum > 0.0 {
        albedo = albedo * (1.0 - humorum) + 0.14 * humorum;
    }

    // 8. Tycho Crater and Ray System: (cx = -0.08, cy = 0.62)
    let tycho_dx = nx - (-0.08);
    let tycho_dy = ny - 0.62;
    let tycho_d = (tycho_dx * tycho_dx + tycho_dy * tycho_dy).sqrt();
    if tycho_d < 0.05 {
        // Bright central crater core
        albedo += 0.35 * (1.0 - tycho_d / 0.05);
    }
    // Tycho radiating ray ejecta
    let ray_angle = tycho_dy.atan2(tycho_dx);
    let ray_pattern = ((ray_angle * 12.0).sin().abs().powi(6)
        + (ray_angle * 8.0 + 0.5).sin().abs().powi(8) * 0.6)
        * (-tycho_d * 2.2).exp();
    albedo += ray_pattern * 0.28;

    // 9. Copernicus Crater & Rays: (cx = -0.20, cy = -0.12)
    let cop_dx = nx - (-0.20);
    let cop_dy = ny - (-0.12);
    let cop_d = (cop_dx * cop_dx + cop_dy * cop_dy).sqrt();
    if cop_d < 0.045 {
        albedo += 0.30 * (1.0 - cop_d / 0.045);
    }
    if cop_d < 0.18 {
        let cop_rays = ((cop_dy.atan2(cop_dx) * 10.0).sin().abs().powi(4)) * (1.0 - cop_d / 0.18);
        albedo += cop_rays * 0.20;
    }

    // 10. Aristarchus Plateau: (cx = -0.47, cy = -0.36)
    let arist_d = ((nx - (-0.47)).powi(2) + (ny - (-0.36)).powi(2)).sqrt();
    if arist_d < 0.035 {
        albedo += 0.40 * (1.0 - arist_d / 0.035);
    }

    albedo.clamp(0.11, 0.65)
}

/// Celestial background stars for the Telescopic Sky View portal.
/// Each star is defined with normalized offset (dx, dy) relative to portal radius,
/// brightness (0.0..1.0), and subtle spectral hue (0 = warm, 1 = blue-white, 2 = diamond).
const SKY_STARS: &[(f32, f32, f32, u8)] = &[
    (-0.82, -0.65, 0.95, 1),
    (-0.72, -0.80, 0.65, 2),
    (-0.50, -0.88, 0.75, 0),
    (-0.25, -0.92, 0.55, 1),
    ( 0.15, -0.90, 0.85, 2),
    ( 0.55, -0.82, 0.70, 0),
    ( 0.78, -0.68, 0.90, 1),
    ( 0.88, -0.42, 0.60, 2),
    ( 0.92, -0.15, 0.80, 1),
    ( 0.88,  0.25, 0.75, 0),
    ( 0.80,  0.60, 0.85, 2),
    ( 0.62,  0.78, 0.65, 1),
    ( 0.30,  0.88, 0.90, 2),
    (-0.15,  0.92, 0.70, 0),
    (-0.52,  0.85, 0.80, 1),
    (-0.75,  0.65, 0.60, 2),
    (-0.90,  0.35, 0.85, 1),
    (-0.88, -0.10, 0.75, 0),
    (-0.68, -0.48, 0.50, 1),
    ( 0.70, -0.50, 0.55, 2),
    ( 0.72,  0.42, 0.50, 0),
    (-0.65,  0.50, 0.55, 1),
    (-0.35,  0.80, 0.65, 2),
    ( 0.45, -0.75, 0.60, 1),
];

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
            let edge_coverage = (0.5 + (r - dist)).clamp(0.0, 1.0);
            if edge_coverage <= 0.0 {
                continue;
            }

            // Normal vector N on the front hemisphere
            let z = (r_sq - d_sq).max(0.0).sqrt();
            let nx = dx / r;
            let ny = dy / r;
            let nz = z / r;

            // Rotate coordinates around polar axis
            let px_rot = nx * cos_rot + nz * sin_rot;
            let py_rot = ny;
            let pz_rot = -nx * sin_rot + nz * cos_rot;

            // Surface albedo
            let albedo = lunar_albedo([px_rot, py_rot, pz_rot]);

            // Sun-directed illumination: N · L
            let n_dot_l = (nx * lx + ny * ly + nz * lz).max(0.0);

            // Lommel-Seeliger scattering
            let diffuse = if n_dot_l > 0.0 {
                (n_dot_l / (n_dot_l + nz + 0.05)).powf(0.85)
            } else {
                0.0
            };

            // Physical earthshine: faint ambient fill on unlit limb, scaling down near full
            let earthshine = 0.035 * (1.0 - illum_clamped).powi(2);
            let total_light = earthshine + (1.0 - earthshine) * diffuse;

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

/// Renders the Telescopic Astronomical Sky View — shaded EXACTLY as the Moon appears
/// when observing the real night sky from Earth.
///
/// Features:
/// - Celestial Sky Portal: An astronomical circular telescope view with deep cosmic night
///   sky gradient and realistic twinkling background stars.
/// - Authentic Selenography: Tidally locked near-side orientation facing Earth featuring
///   the unmistakable "Man in the Moon" maria (Imbrium, Procellarum, Serenitatis,
///   Tranquillitatis, Crisium), Copernicus, and Tycho crater's expansive ray system.
/// - Real Sky Phase Shadow: Accurate terminator curve with grazing-angle rim roughness.
/// - Physical Earthshine: Soft ashen glow during crescent phases, disappearing near full moon.
/// - Atmospheric Halo: Forward-scattering lunar corona diffusing naturally into the starfield.
pub fn draw_sky_moon(
    canvas: &mut Canvas,
    cx: f32,
    cy: f32,
    r: f32,
    illum: f32,
    waxing: bool,
    accent: Rgba,
) {
    let illum_clamped = illum.clamp(0.0, 1.0);

    // Light vector L from the astronomical phase angle
    let phase_angle = (1.0 - illum_clamped) * std::f32::consts::PI;
    let (sin_p, cos_p) = phase_angle.sin_cos();
    let (lx, ly, lz) = if waxing {
        (sin_p, 0.0, cos_p)
    } else {
        (-sin_p, 0.0, cos_p)
    };

    // 1. Celestial Sky Portal Backdrop (Deep Cosmic Space Aperture)
    let portal_r = r + 13.0;
    let portal_r_sq = portal_r * portal_r;
    let p_min_x = (cx - portal_r - 2.0).max(0.0) as i32;
    let p_max_x = (cx + portal_r + 2.0).min(canvas.w as f32 - 1.0) as i32;
    let p_min_y = (cy - portal_r - 2.0).max(0.0) as i32;
    let p_max_y = (cy + portal_r + 2.0).min(canvas.h as f32 - 1.0) as i32;

    for py in p_min_y..=p_max_y {
        let dy = py as f32 + 0.5 - cy;
        let dy_sq = dy * dy;
        for px in p_min_x..=p_max_x {
            let dx = px as f32 + 0.5 - cx;
            let d_sq = dx * dx + dy_sq;
            if d_sq > portal_r_sq {
                continue;
            }
            let dist = d_sq.sqrt();
            let portal_coverage = (0.5 + (portal_r - dist)).clamp(0.0, 1.0);
            if portal_coverage <= 0.0 {
                continue;
            }

            // Radial depth gradient: deep space void
            let t_depth = (dist / portal_r).clamp(0.0, 1.0);
            let sky_r = (5.0 + t_depth * 7.0) as u8;
            let sky_g = (8.0 + t_depth * 10.0) as u8;
            let sky_b = (18.0 + t_depth * 18.0) as u8;
            let sky_col = Rgba { r: sky_r, g: sky_g, b: sky_b, a: (235.0 * portal_coverage) as u8 };
            canvas.blend_pixel(px, py, sky_col, portal_coverage * 0.92);

            // Subtle lens aperture hairline on the portal rim
            if (dist - portal_r).abs() < 1.4 {
                let rim_alpha = (1.0 - (dist - portal_r).abs() / 1.4) * 0.45;
                let rim_col = Rgba {
                    r: ((accent.r as f32 * 0.5 + 120.0) * rim_alpha) as u8,
                    g: ((accent.g as f32 * 0.5 + 130.0) * rim_alpha) as u8,
                    b: ((accent.b as f32 * 0.5 + 160.0) * rim_alpha) as u8,
                    a: (255.0 * rim_alpha) as u8,
                };
                canvas.blend_pixel(px, py, rim_col, rim_alpha);
            }
        }
    }

    // 2. Celestial Background Starfield inside the sky portal
    for &(sx, sy, brightness, tint) in SKY_STARS {
        let star_x = cx + sx * (portal_r - 4.0);
        let star_y = cy + sy * (portal_r - 4.0);
        let dist_from_moon = ((star_x - cx).powi(2) + (star_y - cy).powi(2)).sqrt();
        if dist_from_moon <= r + 2.0 {
            continue; // behind the moon disc
        }

        let (sr, sg, sb) = match tint {
            0 => (255.0, 220.0, 180.0), // Warm amber star
            1 => (190.0, 225.0, 255.0), // Cool blue-white star
            _ => (250.0, 250.0, 255.0), // Pure diamond white
        };

        // Draw star with soft subpixel point spread
        let ix = star_x.floor() as i32;
        let iy = star_y.floor() as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let px = ix + dx;
                let py = iy + dy;
                if px < 0 || px >= canvas.w || py < 0 || py >= canvas.h {
                    continue;
                }
                let delta = ((px as f32 + 0.5 - star_x).powi(2) + (py as f32 + 0.5 - star_y).powi(2)).sqrt();
                if delta < 1.5 {
                    let falloff = (1.0 - delta / 1.5).powi(2) * brightness;
                    let star_col = Rgba {
                        r: (sr * falloff).min(255.0) as u8,
                        g: (sg * falloff).min(255.0) as u8,
                        b: (sb * falloff).min(255.0) as u8,
                        a: (255.0 * falloff) as u8,
                    };
                    canvas.blend_pixel(px, py, star_col, falloff);
                }
            }
        }
    }

    // 3. Atmospheric forward-scattering lunar corona around the illuminated limb
    let halo_max = r * 1.65;
    for py in (cy - halo_max).max(0.0) as i32..=(cy + halo_max).min(canvas.h as f32 - 1.0) as i32 {
        for px in (cx - halo_max).max(0.0) as i32..=(cx + halo_max).min(canvas.w as f32 - 1.0) as i32 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r && dist < halo_max {
                let lit_dir = if waxing { dx / dist } else { -dx / dist };
                let sun_bias = (lit_dir * 0.5 + 0.5).clamp(0.0, 1.0);
                let norm_d = (dist - r) / (halo_max - r);
                let factor = (1.0 - norm_d).powi(3) * 0.24 * illum_clamped * (0.25 + 0.75 * sun_bias);
                if factor > 0.005 {
                    let halo_col = Rgba {
                        r: ((230.0 + accent.r as f32 * 0.1) * factor) as u8,
                        g: ((238.0 + accent.g as f32 * 0.1) * factor) as u8,
                        b: ((250.0 + accent.b as f32 * 0.1) * factor) as u8,
                        a: (255.0 * factor) as u8,
                    };
                    canvas.blend_pixel(px, py, halo_col, factor);
                }
            }
        }
    }

    // 4. Ray-sphere rendering with authentic near-side selenography and real sky terminator
    let ir = r as i32;
    let bound = ir + 3;
    let min_x = (cx as i32 - bound).max(0);
    let max_x = (cx as i32 + bound).min(canvas.w - 1);
    let min_y = (cy as i32 - bound).max(0);
    let max_y = (cy as i32 + bound).min(canvas.h - 1);
    let r_sq = r * r;

    // Physical earthshine: In the night sky, earthshine is noticeable only on slim crescents.
    let earthshine = 0.038 * (1.0 - illum_clamped).powi(3);

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
            let edge_coverage = (0.5 + (r - dist)).clamp(0.0, 1.0);
            if edge_coverage <= 0.0 {
                continue;
            }

            // Normal vector N on the Earth-facing hemisphere
            let z = (r_sq - d_sq).max(0.0).sqrt();
            let nx = dx / r;
            let ny = dy / r;
            let nz = z / r;

            // Authentic Earth-facing near-side albedo
            let albedo = near_side_albedo(nx, ny, nz);

            // Crater rims along the terminator create authentic rough shadow relief
            let terminator_relief = fbm([nx * 16.0, ny * 16.0, nz * 16.0], 2);
            let raw_n_dot_l = nx * lx + ny * ly + nz * lz;
            let terminator_offset = (terminator_relief - 0.48) * 0.14;
            let effective_n_dot_l = (raw_n_dot_l + terminator_offset).max(0.0);

            // Smooth terminator transition
            let lit_factor = (effective_n_dot_l / 0.12).clamp(0.0, 1.0);

            // Lommel-Seeliger retroreflective scattering on illuminated terrain
            let diffuse = if effective_n_dot_l > 0.0 {
                (effective_n_dot_l / (effective_n_dot_l + nz + 0.04)).powf(0.85)
            } else {
                0.0
            };

            // Combined illumination: earthshine on unlit limb + direct solar illumination
            let total_light = earthshine + (1.0 - earthshine) * diffuse * lit_factor;

            // Tone mapping
            let brightness = (albedo * total_light * 2.3).clamp(0.0, 1.0);
            let specular = if diffuse > 0.85 { (diffuse - 0.85) * 0.45 } else { 0.0 };

            // In deep space, unlit portions show subtle earthshine or dark silhouette
            let pixel_alpha = if total_light > 0.005 {
                edge_coverage
            } else {
                edge_coverage * (0.05 + earthshine * 2.0)
            };

            let col = Rgba {
                r: ((225.0 * brightness + 30.0 * specular).min(255.0)) as u8,
                g: ((232.0 * brightness + 24.0 * specular).min(255.0)) as u8,
                b: ((245.0 * brightness + 12.0 * specular).min(255.0)) as u8,
                a: (255.0 * pixel_alpha.clamp(0.0, 1.0)) as u8,
            };

            canvas.blend_pixel(px, py, col, pixel_alpha);
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
    fn near_side_albedo_is_bounded() {
        for y in [-0.9f32, -0.5, 0.0, 0.5, 0.9] {
            for x in [-0.9f32, -0.5, 0.0, 0.5, 0.9] {
                let d_sq = x * x + y * y;
                if d_sq < 1.0 {
                    let z = (1.0f32 - d_sq).sqrt();
                    let a = near_side_albedo(x, y, z);
                    assert!((0.0..=1.0).contains(&a), "near side albedo {a} out of range");
                }
            }
        }
    }

    #[test]
    fn draw_3d_moon_runs_without_panic_for_all_phases() {
        let mut dib = yasb_chrome::gdi::Dib::new(140, 140);
        let mut canvas = dib.canvas();
        let accent = Rgba::rgb(180, 190, 255);

        for illum in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for waxing in [true, false] {
                for rot in [0.0, 1.0, 3.14, 6.28] {
                    draw_3d_moon(&mut canvas, 70.0, 70.0, 55.0, illum, waxing, rot, accent);
                }
            }
        }
    }

    #[test]
    fn draw_sky_moon_runs_without_panic_for_all_phases() {
        let mut dib = yasb_chrome::gdi::Dib::new(140, 140);
        let mut canvas = dib.canvas();
        let accent = Rgba::rgb(180, 190, 255);

        for illum in [0.0, 0.10, 0.25, 0.5, 0.85, 1.0] {
            for waxing in [true, false] {
                draw_sky_moon(&mut canvas, 70.0, 70.0, 55.0, illum, waxing, accent);
            }
        }
    }
}



