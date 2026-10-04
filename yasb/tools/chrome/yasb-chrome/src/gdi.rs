//! Display-side plumbing: the DIB the canvas draws into, the GDI text pass
//! that goes on top of it, and the blit that puts the result on screen.
//!
//! # Why the blit and not `UpdateLayeredWindow`
//!
//! Both popups used to be `WS_EX_LAYERED` and hand a 32-bit DIB to
//! `UpdateLayeredWindow` with `AC_SRC_ALPHA`. On this machine that call
//! returns `TRUE`, the window reports itself visible, topmost and uncloaked,
//! and the DIB provably holds the right pixels — and nothing reaches the
//! screen. A pixel diff of two screen captures, one with the popup up and one
//! without, was *identical*. So layered compositing is not available here.
//!
//! What is available is an ordinary blit. Translucency is then done the way a
//! compositor would: snapshot the desktop behind the window
//! ([`Backdrop::capture`]), alpha-blend the theme over that snapshot, and blit
//! the composed result. The visual result is the same acrylic panel, minus
//! per-pixel edge alpha — which [`apply_round_region`] recovers as a clipped
//! rounded window shape.

use std::path::PathBuf;

use windows::Win32::Foundation::{COLORREF, HWND, RECT, SIZE};
use windows::Win32::Graphics::Gdi::*;

use crate::canvas::Canvas;

/// Convert a theme colour to the `COLORREF` GDI wants (0x00BBGGRR).
pub fn colorref(c: yasb_theme::Rgba) -> COLORREF {
    COLORREF(c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16))
}

/// A top-down 32-bit BGRA DIB section plus the memory DC to draw through.
///
/// The pixel buffer is exposed as a slice so [`Canvas`] can work on it
/// directly — no per-pixel GDI calls, no round trip through `SetPixel`.
pub struct Dib {
    dc: HDC,
    screen: HDC,
    bmp: HBITMAP,
    old_bmp: HGDIOBJ,
    pub w: i32,
    pub h: i32,
    bits: *mut core::ffi::c_void,
}

impl Dib {
    /// Allocate a `w` x `h` top-down 32-bit DIB. Panics if GDI refuses,
    /// which at these sizes means the process is out of memory.
    pub fn new(w: i32, h: i32) -> Dib {
        assert!(w > 0 && h > 0, "bad DIB size {w}x{h}");
        let (screen, dc, bmp, old_bmp, bits) = unsafe { create_dib(w, h) };
        Dib { dc, screen, bmp, old_bmp, w, h, bits }
    }

    /// The memory DC, for the text pass and for blitting.
    pub fn dc(&self) -> HDC {
        self.dc
    }

    /// The screen DC the DIB is compatible with.
    pub fn screen_dc(&self) -> HDC {
        self.screen
    }

    /// The raw pixels, as a BGRA buffer.
    ///
    /// # Safety
    /// The slice aliases the DIB's own memory. It must not be held across
    /// anything that reallocates or reads the DC, and it must not be aliased
    /// twice at once.
    pub unsafe fn pixels(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.bits.cast::<u8>(), (self.w * self.h * 4) as usize)
    }

    /// Borrow the pixels as a [`Canvas`] for the shape pass.
    pub fn canvas(&mut self) -> Canvas<'_> {
        let (w, h) = (self.w, self.h);
        // SAFETY: the slice is uniquely borrowed for the lifetime of the
        // Canvas, and `Canvas::new` asserts it is the right length for w*h.
        unsafe { Canvas::new(self.pixels(), w, h) }
    }
}

/// SAFETY: `w` and `h` are checked positive by `Dib::new`.
unsafe fn create_dib(w: i32, h: i32) -> (HDC, HDC, HBITMAP, HGDIOBJ, *mut core::ffi::c_void) {
    let screen = GetDC(None);
    assert!(!screen.0.is_null(), "GetDC(NULL) failed");
    let dc = CreateCompatibleDC(screen);
    assert!(!dc.0.is_null(), "CreateCompatibleDC failed");
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // negative height == top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let bmp = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        .expect("CreateDIBSection failed");
    let old_bmp = SelectObject(dc, HGDIOBJ(bmp.0));
    (screen, dc, bmp, old_bmp, bits)
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old_bmp);
            let _ = DeleteObject(HGDIOBJ(self.bmp.0));
            let _ = DeleteDC(self.dc);
            let _ = DeleteDC(self.screen);
        }
    }
}

/// A snapshot of the desktop behind a window, used as the acrylic backdrop.
///
/// The copy must be taken *before* the window is shown, or the snapshot
/// contains the panel itself and the blend becomes a feedback loop that gets
/// brighter on every repaint.
pub struct Backdrop {
    pixels: Vec<u8>,
    pub w: i32,
    pub h: i32,
}

impl Backdrop {
    /// Capture the screen region at `(x, y)` size `w` x `h`, in physical
    /// virtual-screen coordinates.
    ///
    /// `CAPTUREBLT` is set so that anything layered on top of the desktop —
    /// the YASB bar's own acrylic, other tooltips — is included; without it
    /// the capture shows through them.
    pub fn capture(x: i32, y: i32, w: i32, h: i32) -> Option<Backdrop> {
        if w <= 0 || h <= 0 {
            return None;
        }
        let mut dib = Dib::new(w, h);
        let ok = unsafe {
            // SAFETY: the DIB was just allocated at this exact size.
            BitBlt(dib.dc(), 0, 0, w, h, dib.screen_dc(), x, y, SRCCOPY | CAPTUREBLT).is_ok()
        };
        if !ok {
            return None;
        }
        Some(Backdrop {
            // SAFETY: read once, immediately, into an owned buffer.
            pixels: unsafe { dib.pixels() }.to_vec(),
            w,
            h,
        })
    }

    /// A blurred copy of this snapshot: three box passes, which lands close
    /// enough to a gaussian to be indistinguishable through a light tint.
    ///
    /// This is what makes a strongly transparent panel survivable. Real
    /// acrylic is only 15-20% opaque *and* heavily blurred, so what comes
    /// through is a wash of the wallpaper's colour rather than its detail.
    /// Blending an unblurred snapshot at the same strength puts crisp icons
    /// and window edges directly behind body text, and no amount of
    /// material tuning rescues readability after that.
    ///
    /// Edges clamp rather than wrap, so a panel near the screen edge does
    /// not pick up the far side of the desktop.
    pub fn blurred(&self, radius: i32) -> Backdrop {
        if radius <= 0 {
            return Backdrop { pixels: self.pixels.clone(), w: self.w, h: self.h };
        }
        let (w, h) = (self.w as usize, self.h as usize);
        if w == 0 || h == 0 {
            return Backdrop { pixels: self.pixels.clone(), w: self.w, h: self.h };
        }
        let r = radius as usize;
        let mut a = self.pixels.clone();
        let mut b = vec![0u8; a.len()];
        for _ in 0..3 {
            box_blur_h(&a, &mut b, w, h, r);
            box_blur_v(&b, &mut a, w, h, r);
        }
        Backdrop { pixels: a, w: self.w, h: self.h }
    }

    /// Blend this snapshot into `canvas` at `(x, y)`, clipped to it.
    ///
    /// `opacity` is how much of the desktop shows through: 0.0 replaces it
    /// entirely, 1.0 leaves it untouched. The panels sit high, around 0.85,
    /// which is the acrylic look — but only because they blend a
    /// [`Backdrop::blurred`] copy, not the raw capture.
    pub fn draw_under(&self, canvas: &mut Canvas, x: i32, y: i32, opacity: f32) {
        let k = opacity.clamp(0.0, 1.0);
        if k <= 0.0 {
            return;
        }
        for row in 0..canvas.h {
            if row >= self.h {
                break;
            }
            let dy = y + row;
            if dy < 0 || dy >= canvas.h {
                continue;
            }
            for col in 0..canvas.w {
                let dx = x + col;
                if dx < 0 || dx >= canvas.w || dx >= self.w {
                    continue;
                }
                let so = ((row * self.w + col) * 4) as usize;
                let dofs = (dy * canvas.w + dx) as usize * 4;
                // The canvas already holds the theme fill; pull it towards
                // the desktop by `k` on each channel.
                for ch in 0..3 {
                    let d = canvas.px[dofs + ch] as f32;
                    let s = self.pixels[so + ch] as f32;
                    canvas.px[dofs + ch] = (d + (s - d) * k + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

/// One horizontal box-blur pass, `src` into `dst`.
///
/// Uses per-row prefix sums rather than a sliding window: at these sizes the
/// extra memory traffic is irrelevant and the prefix form has no edge case
/// where the running sum drifts out of sync with the window bounds.
fn box_blur_h(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let mut pre = vec![0u32; (w + 1) * 4];
    for y in 0..h {
        for ch in 0..4 {
            pre[ch] = 0;
        }
        for x in 0..w {
            let so = (y * w + x) * 4;
            for ch in 0..4 {
                pre[(x + 1) * 4 + ch] = pre[x * 4 + ch] + src[so + ch] as u32;
            }
        }
        for x in 0..w {
            let lo = x.saturating_sub(r);
            let hi = (x + r).min(w - 1);
            // Round-half-up: without the +n/2 every box average truncates
            // down and a large flat region very slowly loses value.
            let n = (hi - lo + 1) as u32;
            let dofs = (y * w + x) * 4;
            for ch in 0..4 {
                let sum = pre[(hi + 1) * 4 + ch] - pre[lo * 4 + ch];
                dst[dofs + ch] = ((sum + n / 2) / n) as u8;
            }
        }
    }
}

/// One vertical box-blur pass, `src` into `dst`. The transpose of
/// [`box_blur_h`], and correct for `src != dst`.
fn box_blur_v(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let mut pre = vec![0u32; (h + 1) * 4];
    for x in 0..w {
        for ch in 0..4 {
            pre[ch] = 0;
        }
        for y in 0..h {
            let so = (y * w + x) * 4;
            for ch in 0..4 {
                pre[(y + 1) * 4 + ch] = pre[y * 4 + ch] + src[so + ch] as u32;
            }
        }
        for y in 0..h {
            let lo = y.saturating_sub(r);
            let hi = (y + r).min(h - 1);
            let n = (hi - lo + 1) as u32;
            let dofs = (y * w + x) * 4;
            for ch in 0..4 {
                let sum = pre[(hi + 1) * 4 + ch] - pre[lo * 4 + ch];
                dst[dofs + ch] = ((sum + n / 2) / n) as u8;
            }
        }
    }
}

/// The standard arrow, shared so both panels can name the same cursor.
///
/// Cached rather than loaded per call: `LoadCursorW` with a null module is a
/// shared system resource, but this is asked for on every `WM_SETCURSOR`,
/// which fires on every pointer movement over the window.
pub fn arrow_cursor() -> windows::Win32::UI::WindowsAndMessaging::HCURSOR {
    use windows::Win32::UI::WindowsAndMessaging::{HCURSOR, IDC_ARROW, LoadCursorW};
    static ARROW: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let raw = *ARROW.get_or_init(|| unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default().0 as usize });
    HCURSOR(raw as *mut core::ffi::c_void)
}

/// Build a UI font at a given pixel height, weight and family.
///
/// ## Why not ClearType
///
/// `CLEARTYPE_QUALITY` is the usual answer for UI text, and it is wrong for
/// these panels. ClearType renders subpixel RGB: it assumes an opaque
/// background it can blend against and deliberately colours the edges of
/// every glyph. These panels do not have one — they snapshot the desktop,
/// alpha-blend a translucent theme over it, and `BitBlt` the result — so the
/// subpixel fringes are composited against whatever was behind the window and
/// show up as coloured halos on 11-13px caps, which is the worst size for it.
///
/// `ANTIALIASED_QUALITY` is grayscale: smooth, neutral, and correct for a
/// surface you have already composited yourself.
pub fn font(size: i32, weight: i32, family: &str) -> HFONT {
    let wide: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateFontW(
            size, 0, 0, 0, weight, 0, 0, 0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            ANTIALIASED_QUALITY.0 as u32,
            DEFAULT_PITCH.0 as u32,
            windows::core::PCWSTR(wide.as_ptr()),
        )
    }
}

/// Draw letter-spaced text and return the width it occupied.
///
/// `DrawTextW` cannot letter-space, and untracked 11-13px caps look cramped
/// against a 16-28px headline.
///
/// The obvious implementation draws glyph by glyph with `TextOutW`, adding
/// `GetTextExtentPoint32W`'s advance plus the tracking after each one. That
/// looks equivalent and is not: the extent call rounds every advance to a
/// whole pixel, so a 9.75px advance alternates between rounding up and down
/// and the gaps come out visibly uneven. Letting GDI do it with
/// `SetTextCharacterExtra` keeps one exact extra for every gap and keeps the
/// glyphs on GDI's own hinted positions, which is also what makes the caps
/// sit on the pixel grid instead of shimmering.
///
/// ## The anchor changed
///
/// `TextOutW` takes `y` as a **baseline**; `DrawTextW` takes the rect's top as
/// the top of the line. `y` here is a **top**, matching [`draw_text`], so every
/// call site that was tuned against the old baseline has to be re-checked —
/// it is not a drop-in swap. The width is generous but the height is not
/// zero: a rect with `bottom == top` is empty, and `DrawTextW` will happily
/// draw nothing into one.
///
/// The extra is reset afterwards because it is DC state, not font state: a
/// caller that forgets to clear it silently letter-spaces the rest of the
/// panel.
pub fn tracked_text(dc: HDC, text: &str, x: i32, y: i32, tracking: i32) -> i32 {
    // Tall enough for any face these panels use, with no vertical centring:
    // the top of the rect is the anchor.
    let r = rect(x, y, 4096, 48);
    if tracking == 0 {
        return draw_text(dc, text, r, DT_LEFT);
    }
    unsafe {
        SetTextCharacterExtra(dc, tracking);
        let w = draw_text(dc, text, r, DT_LEFT);
        SetTextCharacterExtra(dc, 0);
        w
    }
}

/// Width in pixels of `text` under the font currently selected into `dc`.
pub fn text_width(dc: HDC, text: &str) -> i32 {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    buf.push(0);
    let mut size = SIZE { cx: 0, cy: 0 };
    unsafe {
        let _ = GetTextExtentPoint32W(dc, &buf, &mut size);
    }
    size.cx
}

/// Draw `text` into `dc` with the currently selected font and text colour.
///
/// Long strings are ellipsised (`DT_END_ELLIPSIS`) rather than wrapped or
/// hard-clipped: a clipped label reads as a bug, an ellipsis reads as
/// truncation.
pub fn draw_text(dc: HDC, text: &str, rc: RECT, flags: DRAW_TEXT_FORMAT) -> i32 {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    buf.push(0);
    let mut rc = rc;
    let flags = flags | DT_NOPREFIX | DT_SINGLELINE | DT_END_ELLIPSIS;
    unsafe { DrawTextW(dc, &mut buf, &mut rc, flags) }
}

/// A rect for `draw_text`, since most call sites lay out on a column grid.
pub fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

/// Clip the window to a rounded rectangle of the given radius.
///
/// Without per-pixel alpha the corners cannot be feathered, but this still
/// gives a genuinely rounded, correctly clipped shape. Resizing does not carry
/// the region over, so every caller has to re-apply this after a
/// `SetWindowPos`.
///
/// # Safety
/// `hwnd` must be a live window handle.
pub unsafe fn apply_round_region(hwnd: HWND, w: i32, h: i32, radius: i32) {
    use windows::Win32::Foundation::BOOL;
    let region = CreateRoundRectRgn(0, 0, w + 1, h + 1, radius * 2, radius * 2);
    if region.0.is_null() {
        return;
    }
    // SetWindowRgn takes ownership on success; on failure we must free it.
    if SetWindowRgn(hwnd, region, BOOL(1)) == 0 {
        let _ = DeleteObject(HGDIOBJ(region.0));
    }
}

/// Blit a memory DC onto a window's DC.
///
/// This is the only composite step, and it is deliberately `SRCCOPY` with no
/// alpha: it is the one path on this machine that demonstrably reaches the
/// screen.
///
/// # Safety
/// `hwnd` must be a live window handle and `dib` must match its size.
pub unsafe fn blit_to_window(hwnd: HWND, dib: &Dib) -> bool {
    let dc_win = GetWindowDC(hwnd);
    if dc_win.0.is_null() {
        return false;
    }
    let ok = BitBlt(dc_win, 0, 0, dib.w, dib.h, dib.dc, 0, 0, SRCCOPY).is_ok();
    let _ = ReleaseDC(hwnd, dc_win);
    ok
}

/// The user's `styles.css`, shared by every panel.
pub fn styles_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("yasb").join("styles.css")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both text helpers must actually put ink on the DC.
    ///
    /// This is the regression fence for a bug that shipped to the desktop:
    /// [`tracked_text`] was changed from per-glyph `TextOutW` to `DrawTextW`
    /// to even out letter spacing, and it was handed `rect(x, y, 4096, 0)` —
    /// a rect whose `bottom` equals its `top`. `DrawTextW` silently draws
    /// nothing into an empty rect, whereas `TextOutW` never looked at the rect
    /// at all, so the same arguments worked before the swap and produced
    /// completely blank output after it. Every row label and the weekday in
    /// the panchangam panel vanished; only the values, drawn by the other
    /// helper, survived.
    ///
    /// Asserting the returned width is not enough — `DrawTextW` returns 0 for
    /// an empty rect too, which is easy to mistake for "drew fine". This
    /// checks that the DC has lit pixels afterwards.
    #[test]
    fn both_text_helpers_put_ink_on_the_dc() {
        const W: i32 = 220;
        const H: i32 = 60;
        unsafe {
            for tracked in [false, true] {
                let mut dib = Dib::new(W, H);
                let dc = dib.dc();
                {
                    let mut c = dib.canvas();
                    c.clear(yasb_theme::Rgba::rgb(0, 0, 0));
                }
                let f = font(-16, 500, "Segoe UI");
                let old = SelectObject(dc, HGDIOBJ(f.0));
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, colorref(yasb_theme::Rgba::rgb(255, 255, 255)));
                let w = if tracked {
                    tracked_text(dc, "Purnimanta", 4, 4, 2)
                } else {
                    draw_text(dc, "Purnimanta", rect(4, 4, W - 8, 24), DT_LEFT)
                };
                let px = dib.pixels();
                let lit = (0..(W * H) as usize)
                    .filter(|&i| px[i * 4] > 60)
                    .count();
                let _ = SelectObject(dc, old);
                let _ = DeleteObject(HGDIOBJ(f.0));

                let which = if tracked { "tracked_text" } else { "draw_text" };
                assert!(w > 0, "{which} reported a width of {w}");
                assert!(
                    lit > 100,
                    "{which} drew {lit} lit pixels — it is drawing nothing. A rect \
                     with bottom == top is empty and DrawTextW writes nothing into \
                     it, which is how the row labels went blank once already."
                );
            }
        }
    }

    /// [`font`] must ask for grayscale antialiasing, not ClearType.
    ///
    /// Checked by reading the `lfQuality` back out of the created `HFONT`
    /// with `GetObjectW`, which is the only assertion that actually
    /// discriminates here. An earlier version of this test rendered white text
    /// onto a flat black memory DC and looked for coloured glyph edges — and it
    /// passed with ClearType too, because subpixel rendering needs a real LCD
    /// surface and a memory DC silently falls back to grayscale. A test that
    /// passes either way is worse than no test, because it looks like
    /// evidence.
    #[test]
    fn font_asks_for_greyscale_antialiasing_not_cleartype() {
        unsafe {
            let f = font(-16, 500, "Segoe UI");
            let mut lf = LOGFONTW::default();
            let got = GetObjectW(HGDIOBJ(f.0), std::mem::size_of::<LOGFONTW>() as i32, Some(&mut lf as *mut _ as *mut _));
            let _ = DeleteObject(HGDIOBJ(f.0));
            assert!(got != 0, "GetObjectW refused the font handle");
            let quality = lf.lfQuality.0;
            assert_eq!(
                quality, ANTIALIASED_QUALITY.0,
                "text is being rendered with quality {quality} — ClearType on a \
                 surface this crate composites by hand puts coloured halos on \
                 every glyph edge, worst on the 11-13px caps these panels are \
                 full of"
            );
            assert_ne!(quality, CLEARTYPE_QUALITY.0);
        }
    }

    /// The backdrop blend is plain per-channel interpolation, so it can be
    /// checked without a display.
    #[test]
    fn backdrop_interpolates_towards_the_snapshot() {
        let mut px = vec![0u8; 2 * 2 * 4];
        for b in px.iter_mut() {
            *b = 0;
        }
        let mut c = Canvas::new(&mut px, 2, 2);
        c.clear(yasb_theme::Rgba::rgb(0, 0, 0));
        let bd = Backdrop { pixels: vec![200u8; 2 * 2 * 4], w: 2, h: 2 };
        bd.draw_under(&mut c, 0, 0, 1.0);
        // Full opacity hands the canvas over to the snapshot.
        assert_eq!(c.px[0], 200, "red channel not taken from the backdrop");

        let mut px2 = vec![0u8; 2 * 2 * 4];
        let mut c2 = Canvas::new(&mut px2, 2, 2);
        c2.clear(yasb_theme::Rgba::rgb(0, 0, 0));
        bd.draw_under(&mut c2, 0, 0, 0.5);
        assert_eq!(c2.px[0], 100, "half opacity should land halfway");
    }

    #[test]
    fn backdrop_is_clipped_to_the_canvas() {
        // A snapshot positioned entirely off-canvas must not write anything.
        let mut px = vec![7u8; 2 * 2 * 4];
        let before = px.clone();
        let mut c = Canvas::new(&mut px, 2, 2);
        let bd = Backdrop { pixels: vec![0u8; 2 * 2 * 4], w: 2, h: 2 };
        bd.draw_under(&mut c, 50, 50, 1.0);
        assert_eq!(px, before);
    }

    /// A checkerboard with `radius <= 0` must come back untouched — callers
    /// rely on radius 0 meaning "no blur", not "blur by an accident of
    /// clamping".
    #[test]
    fn zero_radius_is_the_identity() {
        let bd = checkerboard(8, 8);
        let out = bd.blurred(0);
        assert_eq!(out.pixels, bd.pixels);
        assert_eq!((out.w, out.h), (8, 8));
    }

    /// A flat image has no detail to smear, so any correct blur is a no-op
    /// on it. This is the cheap guard against a prefix-sum bug that darkens
    /// or lightens the wallpaper.
    #[test]
    fn flat_image_survives_the_blur_unchanged() {
        let bd = Backdrop { pixels: vec![137u8; 16 * 16 * 4], w: 16, h: 16 };
        let out = bd.blurred(4);
        assert!(
            out.pixels.chunks(4).all(|p| p[0] == 137 && p[1] == 137 && p[2] == 137),
            "a flat backdrop must not gain or lose value"
        );
    }

    /// The point of the whole exercise: high-frequency detail has to actually
    /// go away, or a 0.85 acrylic tint puts crisp window edges behind body
    /// text.
    #[test]
    fn blur_spreads_an_impulse_and_preserves_the_mean() {
        let (w, h) = (21usize, 21usize);
        let mut px = vec![0u8; w * h * 4];
        // One white pixel in the middle of black.
        px[(10 * w + 10) * 4] = 255;
        px[(10 * w + 10) * 4 + 1] = 255;
        px[(10 * w + 10) * 4 + 2] = 255;
        let bd = Backdrop { pixels: px, w: w as i32, h: h as i32 };
        let out = bd.blurred(3);

        let centre = out.pixels[(10 * w + 10) * 4] as u32;
        assert!(centre > 0, "the impulse was erased instead of spread");
        assert!(centre < 255, "the impulse was not spread at all");

        // Energy has to land on the neighbours, and it must be somewhere
        // near the centre — a blur that leaked to a corner would be wrapping
        // or indexing wrong.
        let left = out.pixels[(10 * w + 8) * 4] as u32;
        assert!(left > 0, "no blur energy reached the neighbouring column");

        let sum: u32 = out.pixels.chunks(4).map(|p| p[0] as u32).sum();
        // Energy is conserved in exact arithmetic, but every one of the six
        // passes rounds a mean back down into a u8, so a little is lost. A
        // real leak would show up as a factor, not as a few percent.
        assert!(
            (230..=281).contains(&sum),
            "blur lost or invented energy: {sum} vs 255"
        );
    }

    /// A checkerboard's peak-to-trough range has to shrink measurably. Any
    /// correct blur does this; this catches a blur that only runs on one
    /// axis, which is the easy way to get it wrong.
    #[test]
    fn blur_attenuates_a_checkerboard_in_both_axes() {
        let bd = checkerboard(32, 32);
        let out = bd.blurred(2);
        let span = |px: &[u8]| {
            let mut lo = 255u8;
            let mut hi = 0u8;
            for p in px.chunks(4) {
                lo = lo.min(p[0]);
                hi = hi.max(p[0]);
            }
            (hi - lo) as u32
        };
        let before = span(&bd.pixels);
        let after = span(&out.pixels);
        assert!(before > 200, "test input is not a checkerboard: {before}");
        assert!(
            after * 4 < before * 3,
            "checkerboard range barely moved: {before} -> {after}"
        );
    }

    /// Edges must clamp, not wrap. Wrapping is the classic separable-blur
    /// bug and it is invisible on a uniform desktop but very visible on a
    /// real one: a panel near the screen edge would blend in whatever is on
    /// the opposite edge of the display.
    ///
    /// The test is direct rather than statistical — perturb the far border
    /// and require the near border's output to be bit-identical.
    #[test]
    fn blur_clamps_at_the_edges_instead_of_wrapping() {
        let (w, h) = (48usize, 48usize);
        let a = checkerboard(w, h);
        let mut b_px = a.pixels.clone();
        // Repaint the last row and the last column white. Wrapping would drag
        // this into the first row and the first column.
        for x in 0..w {
            for ch in 0..3 {
                b_px[((h - 1) * w + x) * 4 + ch] = 255;
            }
        }
        for y in 0..h {
            for ch in 0..3 {
                b_px[(y * w + (w - 1)) * 4 + ch] = 255;
            }
        }
        let b = Backdrop { pixels: b_px, w: w as i32, h: h as i32 };
        let radius = 3;
        let (oa, ob) = (a.blurred(radius), b.blurred(radius));
        // Three box passes of radius 3 add up, so a sample influences
        // everything within 3*3 = 9 pixels. The perturbed border sits at
        // index n-1 and reaches down to n-1-9, so the last index that is
        // still provably untouched is n-10 — hence the `- 1` on the range
        // end as well as the inset.
        const PASSES: usize = 3;
        let inset = radius as usize * PASSES;
        for y in inset..h - inset - 1 {
            for x in inset..w - inset - 1 {
                let o = (y * w + x) * 4;
                assert_eq!(
                    oa.pixels[o..o + 3],
                    ob.pixels[o..o + 3],
                    "pixel ({x}, {y}) saw the far border — the blur wrapped"
                );
            }
        }
        // And the near border must be clamped, not left sharp: the first row
        // cannot still be alternating.
        let first_row: Vec<u8> = (0..w).map(|x| oa.pixels[x * 4]).collect();
        let spread = first_row.iter().copied().max().unwrap() as i32
            - first_row.iter().copied().min().unwrap() as i32;
        assert!(spread < 100, "first row is still sharp: range {spread}");
        // Clamping smears the border inwards; it must not have darkened it
        // into a shadow the panel would then show as a hard rim.
        let first_row_mean =
            first_row.iter().map(|&v| v as i32).sum::<i32>() / w as i32;
        assert!(
            (100..=156).contains(&first_row_mean),
            "first row mean drifted off the checkerboard's 127: {first_row_mean}"
        );
    }

    fn checkerboard(w: usize, h: usize) -> Backdrop {
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = if (x + y) % 2 == 0 { 255u8 } else { 0u8 };
                let o = (y * w + x) * 4;
                px[o] = v;
                px[o + 1] = v;
                px[o + 2] = v;
            }
        }
        Backdrop { pixels: px, w: w as i32, h: h as i32 }
    }
}
