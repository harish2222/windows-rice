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

    /// Blend this snapshot into `canvas` at `(x, y)`, clipped to it.
    ///
    /// `opacity` is how much of the desktop shows through: 0.0 replaces it
    /// entirely, 1.0 leaves it untouched. The panels sit around 0.55-0.7,
    /// which is roughly what a real acrylic material does.
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

/// Build a UI font at a given pixel height, weight and family.
pub fn font(size: i32, weight: i32, family: &str) -> HFONT {
    let wide: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateFontW(
            size, 0, 0, 0, weight, 0, 0, 0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            DEFAULT_PITCH.0 as u32,
            windows::core::PCWSTR(wide.as_ptr()),
        )
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
}
