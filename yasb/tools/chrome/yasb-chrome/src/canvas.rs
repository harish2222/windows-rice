//! Pure pixel drawing on a 32-bit BGRA buffer.
//!
//! Everything here is arithmetic on a byte slice: no GDI, no HWND, no display.
//! That is what makes the shapes testable — a unit test can fill a 40x40
//! canvas and assert on the exact pixels — and it is also what buys the
//! anti-aliasing GDI's stock `FillRect`/`FrameRect` cannot do. Those are hard
//! 1px edges, which is precisely what reads as "cheap Win32 app" at large
//! radii. Here each primitive computes a signed distance field and blends
//! using the fractional coverage, so a 14px rounded corner is smooth.
//!
//! Colour model: the buffer is BGRA, matching a 32-bit `BI_RGB` DIB section,
//! and blending is straight source-over on premultiplied-free values.

use yasb_theme::Rgba;

/// How far outside the shape the antialiasing ramp reaches. One pixel is
/// enough: coverage is clamped, so anything further is wasted work.
const FEATHER: f32 = 1.0;

/// A borrowed BGRA pixel buffer with the drawing primitives both popups use.
pub struct Canvas<'a> {
    pub px: &'a mut [u8],
    pub w: i32,
    pub h: i32,
}

impl<'a> Canvas<'a> {
    /// Wrap a buffer. `px` must hold at least `w * h * 4` bytes.
    ///
    /// # Panics
    /// If the buffer is too small, which is always a programming error in the
    /// caller (the DIB section is allocated from the window size).
    pub fn new(px: &'a mut [u8], w: i32, h: i32) -> Canvas<'a> {
        assert!(
            px.len() >= (w * h * 4) as usize,
            "canvas buffer too small for {w}x{h}"
        );
        Canvas { px, w, h }
    }

    /// Opaque fill of the whole canvas.
    pub fn clear(&mut self, c: Rgba) {
        self.fill_rect(0, 0, self.w, self.h, c);
    }

    /// Alpha-blend one pixel. Out-of-bounds writes are dropped, so callers do
    /// not have to clip; the primitives below still clip for speed.
    fn blend(&mut self, x: i32, y: i32, c: Rgba, cov: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || cov <= 0.0 {
            return;
        }
        let a = (c.a as f32 / 255.0) * cov.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let o = ((y * self.w + x) * 4) as usize;
        let dst = &mut self.px[o..o + 4];
        // BGRA, with the alpha byte left at 255: the blit is SRCCOPY and never
        // consults it, but a stale value here would confuse any later reader.
        dst[0] = mix(dst[0], c.b, a);
        dst[1] = mix(dst[1], c.g, a);
        dst[2] = mix(dst[2], c.r, a);
        dst[3] = 255;
    }

    /// Alpha-blend one pixel at an explicit coverage.
    ///
    /// Public because callers that build a shape from a per-pixel function
    /// (the moon's terminator, for one) have already done their own coverage
    /// maths and must not have it recomputed.
    pub fn blend_pixel(&mut self, x: i32, y: i32, c: Rgba, cov: f32) {
        self.blend(x, y, c, cov);
    }

    /// Axis-aligned rectangle, half-open on the right and bottom edges.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgba) {
        let (x0, x1) = clamp_span(x0, x1, self.w);
        let (y0, y1) = clamp_span(y0, y1, self.h);
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend(x, y, c, 1.0);
            }
        }
    }

    /// Anti-aliased rounded rectangle.
    ///
    /// `radius` is clamped to half the shorter side, so an over-large radius
    /// degrades to a lozenge instead of inverting the corner geometry.
    pub fn round_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, radius: i32, c: Rgba) {
        let r = (radius.max(0) as f32).min((x1 - x0) as f32 / 2.0).min((y1 - y0) as f32 / 2.0);
        let cx0 = x0.min(self.w);
        let cx1 = x1.max(0);
        let cy0 = y0.min(self.h);
        let cy1 = y1.max(0);
        // The SDF is only evaluated inside the shape's bounding box grown by
        // the feather, which is where coverage can be non-zero anyway.
        let f = FEATHER as i32 + 1;
        for y in (cy0 - f).max(0)..(cy1 + f).min(self.h) {
            for x in (cx0 - f).max(0)..(cx1 + f).min(self.w) {
                let d = sd_round_rect(x as f32 + 0.5, y as f32 + 0.5, x0, y0, x1, y1, r);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, c, cov);
                }
            }
        }
    }

    /// Rounded-rectangle outline of the given thickness, drawn *inward* from
    /// the bounds (matching how a CSS `border` sits inside `box-sizing:
    /// border-box`, which is what the panel's own frame does).
    ///
    /// This is a band, not a filled shape: each pixel's coverage is the outer
    /// field minus the same field inset by the thickness. Blitting a filled
    /// rounded rect and then a smaller filled one would erase the interior
    /// rather than hollow it out.
    pub fn round_rect_border(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        radius: i32,
        thickness: i32,
        c: Rgba,
    ) {
        let t = thickness.max(1);
        let r = (radius.max(0) as f32).min((x1 - x0) as f32 / 2.0).min((y1 - y0) as f32 / 2.0);
        let ri = (r - t as f32).max(0.0);
        let f = FEATHER as i32 + 1;
        for y in (y0 - f).max(0)..(y1 + f).min(self.h) {
            for x in (x0 - f).max(0)..(x1 + f).min(self.w) {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let outer = (0.5 - sd_round_rect(px, py, x0, y0, x1, y1, r)).clamp(0.0, 1.0);
                if outer <= 0.0 {
                    continue;
                }
                let inner = if x0 + t < x1 - t && y0 + t < y1 - t {
                    (0.5 - sd_round_rect(px, py, x0 + t, y0 + t, x1 - t, y1 - t, ri)).clamp(0.0, 1.0)
                } else {
                    // Too thin to hollow: fall back to a solid fill so the
                    // border does not vanish entirely.
                    0.0
                };
                let cov = outer * (1.0 - inner);
                if cov > 0.0 {
                    self.blend(x, y, c, cov);
                }
            }
        }
    }

    /// Vertical gradient across a rectangle, `top` at `y0` and `bottom` at
    /// `y1`. A one-pixel-tall span takes the top colour.
    pub fn v_gradient(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, top: Rgba, bottom: Rgba) {
        let (x0, x1) = clamp_span(x0, x1, self.w);
        let (y0, y1) = clamp_span(y0, y1, self.h);
        let height = (y1 - y0).max(1);
        for y in y0..y1 {
            let t = (y - y0) as f32 / (height - 1).max(1) as f32;
            let c = Rgba {
                r: mix8(top.r, bottom.r, t),
                g: mix8(top.g, bottom.g, t),
                b: mix8(top.b, bottom.b, t),
                a: top.a.max(bottom.a),
            };
            for x in x0..x1 {
                self.blend(x, y, c, 1.0);
            }
        }
    }

    /// A soft radial bloom, the accent-tinted halo that gives a panel its
    /// "lit from above" depth. `strength` scales the whole falloff.
    ///
    /// The falloff is `(1 - d)^2` rather than a linear ramp: a linear one puts
    /// a visible circular edge on the glow, the quadratic does not.
    pub fn radial_glow(&mut self, cx: f32, cy: f32, radius: f32, c: Rgba, strength: f32) {
        if radius <= 0.0 || strength <= 0.0 {
            return;
        }
        let r = radius.ceil() as i32 + 1;
        for y in ((cy as i32) - r).max(0)..((cy as i32) + r).min(self.h) {
            for x in ((cx as i32) - r).max(0)..((cx as i32) + r).min(self.w) {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt() / radius;
                if d >= 1.0 {
                    continue;
                }
                let f = (1.0 - d) * (1.0 - d) * strength;
                self.blend(x, y, c, f);
            }
        }
    }

    /// A 1px horizontal rule. Panels use these as separators under headers,
    /// and a hard 1px line is right there — it should read as a hairline.
    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: Rgba) {
        self.fill_rect(x0, y, x1, y + 1, c);
    }

    /// Coverage of a filled disc, for the moon phase glyph.
    ///
    /// Returns how much of the pixel at `(x, y)` is inside the disc of radius
    /// `r` centred at `(cx, cy)`. Shared with [`moon_lit_mask`], which applies
    /// the phase terminator on top of it.
    pub fn disc_coverage(&self, x: i32, y: i32, cx: f32, cy: f32, r: f32) -> f32 {
        let dx = x as f32 + 0.5 - cx;
        let dy = y as f32 + 0.5 - cy;
        let d = (dx * dx + dy * dy).sqrt() - r;
        (0.5 - d).clamp(0.0, 1.0)
    }
}

/// Lit fraction of the moon disc at pixel `(x, y)`, for drawing the phase
/// glyph in the panchangam popup.
///
/// `illum` is the illuminated fraction in `0.0..=1.0` and `waxing` says
/// which limb is bright. The terminator is modelled as the classic ellipse:
/// with `t = 1 - 2*illum`, the lit region is everything right of
/// `t * sqrt(1 - dy^2)` when waxing (mirrored when waning). That reproduces
/// crescent, quarter and gibbous correctly from the same formula, which a
/// lookup table of eight names cannot do — the panel repaints every second, so
/// the shape has to be continuous in illumination.
///
/// Returns `0.0` outside the disc.
pub fn moon_lit_mask(x: f32, y: f32, cx: f32, cy: f32, r: f32, illum: f32, waxing: bool) -> f32 {
    let dx = x - cx;
    let dy = (y - cy).clamp(-r, r);
    let outer = r * r - dx * dx - dy * dy;
    if outer <= 0.0 {
        return 0.0;
    }
    // The terminator is an ellipse whose *vertical* semi-axis is the full
    // radius and whose horizontal offset scales with illumination. Deriving
    // the row's half-chord from `outer` (which also subtracts dx^2) instead
    // yields a circle scaled by `t`, which is geometrically wrong: it makes a
    // full moon report ~85% lit and shifts every phase off its true shape.
    let half_chord = (r * r - dy * dy).max(0.0).sqrt();
    let t = 1.0 - 2.0 * illum.clamp(0.0, 1.0);
    let edge = t * half_chord;
    // Distance to the terminator, positive on the lit side.
    let d = if waxing { dx - edge } else { -dx - edge };
    let term = (0.5 + d / FEATHER).clamp(0.0, 1.0);
    // Feather the limb separately, or the crescent's edge aliases. This has
    // to use the radial distance from the centre: `outer.sqrt()` is the row's
    // half-chord, which shrinks towards the poles and would dim the middle of
    // a full moon to half brightness.
    let radial = (dx * dx + (y - cy) * (y - cy)).sqrt();
    term * (0.5 - ((radial - r) / FEATHER)).clamp(0.0, 1.0)
}

/// Signed distance to a rounded rectangle: negative inside, positive outside,
/// zero on the boundary. The standard `sdRoundedBox` construction.
fn sd_round_rect(px: f32, py: f32, x0: i32, y0: i32, x1: i32, y1: i32, r: f32) -> f32 {
    let hx = (x1 - x0) as f32 / 2.0;
    let hy = (y1 - y0) as f32 / 2.0;
    let mx = (x0 + x1) as f32 / 2.0;
    let my = (y0 + y1) as f32 / 2.0;
    let qx = (px - mx).abs() - (hx - r);
    let qy = (py - my).abs() - (hy - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// Blend `src` into `dst` by `a` in `0.0..=1.0`, rounding to nearest.
fn mix(dst: u8, src: u8, a: f32) -> u8 {
    let v = dst as f32 + (src as f32 - dst as f32) * a;
    (v + 0.5).clamp(0.0, 255.0) as u8
}

/// Interpolate two bytes by `t`, for the gradient.
fn mix8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t + 0.5).clamp(0.0, 255.0) as u8
}

fn clamp_span(a: i32, b: i32, limit: i32) -> (i32, i32) {
    (a.max(0).min(limit), b.max(0).min(limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPAQUE: Rgba = Rgba { r: 255, g: 0, b: 0, a: 255 };

    fn canvas(w: i32, h: i32) -> (Vec<u8>, i32, i32) {
        (vec![0u8; (w * h * 4) as usize], w, h)
    }

    fn at(px: &[u8], w: i32, x: i32, y: i32) -> (u8, u8, u8) {
        let o = ((y * w + x) * 4) as usize;
        (px[o + 2], px[o + 1], px[o]) // R, G, B
    }

    #[test]
    fn fill_rect_writes_bgra() {
        let (mut buf, w, h) = canvas(4, 4);
        let mut c = Canvas::new(&mut buf, w, h);
        c.fill_rect(1, 1, 3, 3, OPAQUE);
        assert_eq!(at(&buf, w, 1, 1), (255, 0, 0));
        assert_eq!(at(&buf, w, 2, 2), (255, 0, 0));
        // Half-open on the right/bottom.
        assert_eq!(at(&buf, w, 3, 3), (0, 0, 0));
    }

    #[test]
    fn alpha_blend_halves_towards_the_source() {
        let (mut buf, w, h) = canvas(2, 2);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        c.fill_rect(0, 0, 2, 2, Rgba { r: 255, g: 255, b: 255, a: 128 });
        let (r, g, b) = at(&buf, w, 0, 0);
        assert!((126..=129).contains(&r), "r was {r}");
        assert_eq!((r, g, b), (r, r, r));
    }

    #[test]
    fn round_rect_is_transparent_outside_and_opaque_inside() {
        let (mut buf, w, h) = canvas(40, 40);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        c.round_rect(4, 4, 36, 36, 10, OPAQUE);
        // Deep inside: painted.
        assert_eq!(at(&buf, w, 20, 20), (255, 0, 0));
        // Well outside the shape: untouched.
        assert_eq!(at(&buf, w, 0, 0), (0, 0, 0));
        assert_eq!(at(&buf, w, 39, 39), (0, 0, 0));
    }

    #[test]
    fn round_rect_corners_are_antialiased_not_staircased() {
        let (mut buf, w, h) = canvas(40, 40);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        c.round_rect(4, 4, 36, 36, 12, OPAQUE);
        // The corner arc passes through (7,7): a hard-edged fill leaves that
        // pixel either fully painted or fully untouched, which is the
        // stair-step this whole module exists to avoid.
        let (r, _, _) = at(&buf, w, 7, 7);
        assert!(r > 0 && r < 255, "corner pixel r={r}, expected a partial blend");
    }

    #[test]
    fn oversized_radius_degrades_to_a_lozenge_not_a_broken_shape() {
        let (mut buf, w, h) = canvas(20, 20);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        // Radius far beyond half the width.
        c.round_rect(2, 2, 18, 18, 999, OPAQUE);
        assert_eq!(at(&buf, w, 10, 10), (255, 0, 0));
    }

    #[test]
    fn border_sits_inside_the_bounds() {
        let (mut buf, w, h) = canvas(30, 30);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        c.round_rect_border(5, 5, 25, 25, 6, 2, OPAQUE);
        // The outer ring is painted...
        assert_eq!(at(&buf, w, 5, 15), (255, 0, 0));
        assert_eq!(at(&buf, w, 6, 15), (255, 0, 0));
        // ...and the fill two pixels in is not.
        assert_eq!(at(&buf, w, 7, 15), (0, 0, 0));
    }

    #[test]
    fn gradient_runs_from_top_to_bottom() {
        let (mut buf, w, h) = canvas(4, 20);
        let mut c = Canvas::new(&mut buf, w, h);
        c.v_gradient(0, 0, 4, 20, Rgba::rgb(0, 0, 0), Rgba::rgb(200, 200, 200));
        let (top, _, _) = at(&buf, w, 1, 0);
        let (bottom, _, _) = at(&buf, w, 1, 19);
        assert!(top < 5, "top r={top}");
        assert!(bottom > 190, "bottom r={bottom}");
    }

    #[test]
    fn glow_falls_off_and_never_hard_ends() {
        let (mut buf, w, h) = canvas(60, 60);
        let mut c = Canvas::new(&mut buf, w, h);
        c.clear(Rgba::rgb(0, 0, 0));
        c.radial_glow(30.0, 30.0, 25.0, OPAQUE, 1.0);
        let centre = at(&buf, w, 30, 30).0;
        let mid = at(&buf, w, 40, 30).0;
        let outside = at(&buf, w, 55, 30).0;
        assert!(centre > mid, "centre {centre} should exceed mid {mid}");
        assert_eq!(outside, 0, "glow leaked past its radius");
    }

    #[test]
    fn new_horizon_is_empty_and_full_moon_is_whole() {
        // Dead centre of the disc.
        let empty = moon_lit_mask(10.0, 10.0, 10.0, 10.0, 8.0, 0.0, true);
        let full = moon_lit_mask(10.0, 10.0, 10.0, 10.0, 8.0, 1.0, true);
        assert!(empty < 0.01, "new moon was partly lit: {empty}");
        assert!(full > 0.99, "full moon was not lit: {full}");
    }

    #[test]
    fn quarters_light_exactly_one_limb() {
        let r = 8.0;
        let (cx, cy) = (20.0, 20.0);
        let right = moon_lit_mask(cx + 5.0, cy, cx, cy, r, 0.5, true);
        let left = moon_lit_mask(cx - 5.0, cy, cx, cy, r, 0.5, true);
        assert!(right > 0.99, "waxing quarter: right limb {right} dark");
        assert!(left < 0.01, "waxing quarter: left limb {left} lit");
        // Mirrored when waning.
        let w_right = moon_lit_mask(cx + 5.0, cy, cx, cy, r, 0.5, false);
        let w_left = moon_lit_mask(cx - 5.0, cy, cx, cy, r, 0.5, false);
        assert!(w_left > 0.99, "waning quarter: left limb {w_left} dark");
        assert!(w_right < 0.01, "waning quarter: right limb {w_right} lit");
    }

    #[test]
    fn gibbous_lights_more_than_half_and_crescent_less() {
        let (cx, cy, r) = (20.0, 20.0, 10.0);
        // Normalise by the disc's own pixel count, not its bounding box: the
        // box is 4r^2 but the disc only covers pi*r^2 of it, and comparing
        // against the box would make every fraction read ~21% low.
        let mut disc_px = 0;
        for y in 0..40 {
            for x in 0..40 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if ((dx * dx + dy * dy).sqrt() - r) < 0.0 {
                    disc_px += 1;
                }
            }
        }
        let lit = |illum: f32| {
            let mut n = 0;
            for y in 0..40 {
                for x in 0..40 {
                    if moon_lit_mask(x as f32 + 0.5, y as f32 + 0.5, cx, cy, r, illum, true) > 0.5 {
                        n += 1;
                    }
                }
            }
            n as f32 / disc_px as f32
        };
        let gibbous = lit(0.75);
        let quarter = lit(0.5);
        let crescent = lit(0.25);
        // The mask has to be *proportional* to the illumination fraction, not
        // merely monotonic: a full moon that renders as 85% lit is a visible
        // bug, and that is exactly what a scaled-circle terminator produces.
        for (want, got) in [(0.75, gibbous), (0.5, quarter), (0.25, crescent)] {
            assert!(
                (want - got).abs() < 0.05,
                "illumination {want} rendered as {got:.3} lit"
            );
        }
        assert!(gibbous > quarter && quarter > crescent);
    }

    #[test]
    fn nothing_outside_the_disc_is_lit() {
        assert_eq!(moon_lit_mask(0.0, 0.0, 20.0, 20.0, 5.0, 1.0, true), 0.0);
    }

    /// The phase graphic has to agree with the number the panel reports beside
    /// it, at every point in the cycle — not just at the three phases the
    /// other test samples.
    ///
    /// "Authentic" for a phase glyph means exactly this: the lit area is the
    /// illuminated fraction. A terminator that is merely monotonic in
    /// illumination still looks subtly wrong at, say, 8% lit while reading
    /// "8%" next to it, and the panel repaints every second so the shape has
    /// to be right continuously, not at eight named stops.
    ///
    /// Swept at 1/64 steps against a 1% tolerance, which is finer than the
    /// pixel quantisation of a 60px disc.
    #[test]
    fn the_drawn_phase_matches_the_reported_illumination_across_the_cycle() {
        const R: f32 = 30.0;
        let cx = 100.0;
        let cy = 100.0;
        // Count disc pixels once; the mask is 0 outside, so it counts itself.
        let mut disc = 0usize;
        for y in 0..200 {
            for x in 0..200 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= R * R {
                    disc += 1;
                }
            }
        }
        assert!(disc > 2000, "the disc sampler found only {disc} pixels");

        let lit_fraction = |illum: f32, waxing: bool| {
            let mut n = 0usize;
            for y in 0..200 {
                for x in 0..200 {
                    if moon_lit_mask(
                        x as f32 + 0.5,
                        y as f32 + 0.5,
                        cx,
                        cy,
                        R,
                        illum,
                        waxing,
                    ) > 0.5
                    {
                        n += 1;
                    }
                }
            }
            n as f32 / disc as f32
        };

        let mut worst: f32 = 0.0;
        for step in 0..=64 {
            let illum = step as f32 / 64.0;
            let drawn = lit_fraction(illum, true);
            worst = worst.max((drawn - illum).abs());
            // Waxing and waning are the same moon lit from the other side, so
            // the lit *area* must be identical.
            let mirrored = lit_fraction(illum, false);
            assert!(
                (drawn - mirrored).abs() < 0.01,
                "at {illum:.3} waxing lights {drawn:.3} but waning lights {mirrored:.3}"
            );
        }
        assert!(
            worst < 0.01,
            "the terminator drifts from the reported illumination by {worst:.4}"
        );
    }

    /// New moon is dark and full moon is wholly lit, at the extremes the panel
    /// actually reaches at the quarter-degree boundaries of a lunation.
    #[test]
    fn the_cycle_ends_are_exactly_dark_and_whole() {
        let (cx, cy, r) = (50.0f32, 50.0f32, 20.0f32);
        let sum = |illum: f32| {
            let mut n = 0usize;
            for y in 0..100 {
                for x in 0..100 {
                    if moon_lit_mask(x as f32 + 0.5, y as f32 + 0.5, cx, cy, r, illum, true)
                        > 0.5
                    {
                        n += 1;
                    }
                }
            }
            n
        };
        assert_eq!(sum(0.0), 0, "a new moon must have no lit pixels at all");
        assert_eq!(sum(1.0), sum(0.999), "a full moon must be wholly lit");
    }
}
