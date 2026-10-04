//! screen-shot — screenshot and diff tool for the YASB bar's popups.
//!
//! This is the Rust replacement for `capture-screen.py`, `capture-window.py`
//! and the ad-hoc `diff.py` that were living in the picker folder.
//!
//! # Why CAPTUREBLT matters here
//!
//! A plain `BitBlt` from the screen DC silently *skips layered windows*, and
//! `PrintWindow` returns black for them. Every "why is this popup invisible?"
//! question was answered by diffing two captures, and getting that diff right
//! is what finally proved the popups were broken: the capture with the popup
//! up and the capture without it were **identical**, which meant the popup was
//! never reaching the screen at all no matter what `UpdateLayeredWindow`
//! returned.
//!
//! So `CAPTUREBLT` is not incidental here — without it the tool cannot see the
//! thing it exists to check.
//!
//! ```text
//! screen-shot out.png                       capture the screen
//! screen-shot out.png x0 y0 x1 y1            capture and crop
//! screen-shot --window "Palette" out.png    capture one window by title
//! screen-shot --diff a.png b.png x0 y0 x1 y1 how many pixels changed
//! ```

use std::path::Path;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// An RGB image, 8 bits per channel.
struct Image {
    w: i32,
    h: i32,
    px: Vec<u8>, // w * h * 3
}

impl Image {
    fn at(&self, x: i32, y: i32) -> (u8, u8, u8) {
        let o = ((y * self.w + x) * 3) as usize;
        (self.px[o], self.px[o + 1], self.px[o + 2])
    }

    /// Crop to an inclusive-exclusive box, clipped to the image.
    fn crop(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> Image {
        let x0 = x0.clamp(0, self.w);
        let x1 = x1.clamp(0, self.w);
        let y0 = y0.clamp(0, self.h);
        let y1 = y1.clamp(0, self.h);
        let (w, h) = (x1 - x0, y1 - y0);
        let mut px = Vec::with_capacity((w * h * 3) as usize);
        for y in y0..y1 {
            for x in x0..x1 {
                let (r, g, b) = self.at(x, y);
                px.push(r);
                px.push(g);
                px.push(b);
            }
        }
        Image { w, h, px }
    }

    fn save_png(&self, path: &Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc
            .write_header()
            .map_err(|e| format!("png header: {e}"))?;
        // PNG rows are top-down and our DIB is top-down (negative height), so
        // the buffer goes out as-is.
        w.write_image_data(&self.px).map_err(|e| format!("png data: {e}"))?;
        Ok(())
    }

    fn load_png(path: &Path) -> Result<Image, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let dec = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = dec.read_info().map_err(|e| format!("{}: {e}", path.display()))?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| format!("{}: {e}", path.display()))?;
        buf.truncate(info.buffer_size());
        let (w, h) = (info.width as i32, info.height as i32);
        // Normalise whatever came out of the file into 8-bit RGB.
        let px = match (info.color_type, info.bit_depth) {
            (png::ColorType::Rgb, png::BitDepth::Eight) => buf,
            (png::ColorType::Rgba, png::BitDepth::Eight) => buf
                .chunks_exact(4)
                .flat_map(|c| [c[0], c[1], c[2]])
                .collect(),
            (png::ColorType::Grayscale, png::BitDepth::Eight) => {
                buf.iter().flat_map(|g| [*g, *g, *g]).collect()
            }
            (ct, d) => {
                return Err(format!(
                    "{path:?}: unsupported PNG {ct:?}/{d:?}; re-save as 8-bit RGB"
                ));
            }
        };
        Ok(Image { w, h, px })
    }
}

/// Capture a screen region into a top-down 32-bit BGRA DIB.
///
/// # Safety
/// `x`, `y`, `w`, `h` must describe a region the DC can serve.
unsafe fn grab(x: i32, y: i32, w: i32, h: i32, layered: bool) -> Result<(Image, bool), String> {
    let screen = GetDC(None);
    if screen.0.is_null() {
        return Err("GetDC(NULL) failed".into());
    }
    let dc = CreateCompatibleDC(screen);
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let bmp = match CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
        Ok(b) => b,
        Err(e) => {
            let _ = DeleteDC(dc);
            let _ = DeleteDC(screen);
            return Err(format!("CreateDIBSection: {e}"));
        }
    };
    let old = SelectObject(dc, HGDIOBJ(bmp.0));

    // CAPTUREBLT is the whole point: without it layered windows are omitted.
    let mut rop = SRCCOPY;
    if layered {
        rop |= CAPTUREBLT;
    }
    let ok = BitBlt(dc, 0, 0, w, h, screen, x, y, rop).is_ok();

    let src = std::slice::from_raw_parts(bits.cast::<u8>(), (w * h * 4) as usize);
    let mut px = Vec::with_capacity((w * h * 3) as usize);
    for p in src.chunks_exact(4) {
        px.push(p[2]); // R from BGRA
        px.push(p[1]);
        px.push(p[0]);
    }

    SelectObject(dc, old);
    let _ = DeleteObject(HGDIOBJ(bmp.0));
    let _ = DeleteDC(dc);
    let _ = ReleaseDC(None, screen);
    Ok((Image { w, h, px }, ok))
}

/// Find the first visible top-level window whose title contains `needle`.
///
/// Windows are enumerated in z-order, so the first hit is the topmost match —
/// which is what you want when several windows share a title.
fn find_window(needle: &str) -> Option<HWND> {
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let needle = &*(lp.0 as *const String);
        // SAFETY: `hwnd` is a live window from the enumeration.
        let n = unsafe { GetWindowTextLengthW(hwnd) };
        if n > 0 {
            let mut buf = vec![0u16; n as usize + 1];
            unsafe { GetWindowTextW(hwnd, &mut buf) };
            let title = String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())]);
            if title.contains(needle.as_str()) && unsafe { IsWindowVisible(hwnd) }.as_bool() {
                FIND.with(|f| f.borrow_mut().push(hwnd));
            }
        }
        BOOL(1)
    }

    FIND.with(|f| f.borrow_mut().clear());
    let mut s = needle.to_string();
    let _ = unsafe { EnumWindows(Some(cb), LPARAM(&mut s as *mut String as isize)) };
    FIND.with(|f| f.borrow().first().copied())
}

thread_local! {
    static FIND: std::cell::RefCell<Vec<HWND>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// How many pixels of `box` differ between two images, and where.
fn diff(a: &Image, b: &Image, region: (i32, i32, i32, i32)) -> (i64, i32, i32, i32, i32) {
    let (x0, y0, x1, y1) = region;
    let mut changed = 0i64;
    let (mut mnx, mut mny) = (i32::MAX, i32::MAX);
    let (mut mxx, mut mxy) = (i32::MIN, i32::MIN);
    const THRESHOLD: i32 = 8;
    for y in y0..y1 {
        for x in x0..x1 {
            let (ar, ag, ab) = a.at(x, y);
            let (br, bg, bb) = b.at(x, y);
            let d = (ar as i32 - br as i32).abs()
                + (ag as i32 - bg as i32).abs()
                + (ab as i32 - bb as i32).abs();
            if d > THRESHOLD {
                changed += 1;
                mnx = mnx.min(x);
                mny = mny.min(y);
                mxx = mxx.max(x);
                mxy = mxy.max(y);
            }
        }
    }
    if changed == 0 {
        return (0, -1, -1, -1, -1);
    }
    (changed, mnx, mny, mxx + 1, mxy + 1)
}

fn usage() -> ! {
    eprintln!(
        "screen-shot — screenshot and diff the YASB popups\n\n\
         USAGE:\n  \
         screen-shot <out.png> [x0 y0 x1 y1]      capture the screen (optionally cropped)\n  \
         screen-shot --window <title> <out.png>   capture one window by title substring\n  \
         screen-shot --diff <a.png> <b.png> <x0> <y0> <x1> <y1>\n\n\
         The diff reports how many pixels differ. 0 changed means the thing you\n\
         are looking for is not on screen at all — that is how the popups' \
         invisible-window bug was found."
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }

    // Without this the capture comes back in the DPI-virtualised size, which
    // does not match the coordinates the popups were placed at.
    unsafe {
        let _ = SetProcessDPIAware();
    }

    match args[0].as_str() {
        "--diff" => {
            // out, a, b, x0, y0, x1, y1
            if args.len() < 7 {
                usage();
            }
            let a = Image::load_png(Path::new(&args[1])).unwrap_or_else(|e| {
                eprintln!("screen-shot: {e}");
                std::process::exit(1);
            });
            let b = Image::load_png(Path::new(&args[2])).unwrap_or_else(|e| {
                eprintln!("screen-shot: {e}");
                std::process::exit(1);
            });
            let n: Vec<i32> = args[3..7].iter().filter_map(|v| v.parse().ok()).collect();
            if n.len() != 4 {
                usage();
            }
            // An inverted box is a typo, not an empty region: normalising it
            // here means the diff reports a real "nothing changed" instead of
            // looping zero times and looking like a pass.
            let region = (n[0], n[1], n[2].max(n[0]), n[3].max(n[1]));
            let (changed, bx0, by0, bx1, by1) = diff(&a, &b, region);
            let w = region.2 - region.0;
            let h = region.3 - region.1;
            let pct = if w * h > 0 { changed * 100 / (w as i64 * h as i64) } else { 0 };
            if changed == 0 {
                println!(
                    "rect=({}, {}, {}, {}) size={}x{} pixels_differing=0 (0%) — NOTHING CHANGED",
                    region.0, region.1, region.2, region.3, w, h
                );
            } else {
                println!(
                    "rect=({}, {}, {}, {}) size={}x{} pixels_differing={} ({}%) bbox=({}, {}, {}, {})",
                    region.0, region.1, region.2, region.3, w, h, changed, pct, bx0, by0, bx1, by1
                );
            }
        }
        "--window" => {
            if args.len() < 3 {
                usage();
            }
            let Some(hwnd) = find_window(&args[1]) else {
                eprintln!("screen-shot: no visible window matching {:?}", args[1]);
                std::process::exit(1);
            };
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let _ = unsafe { GetWindowRect(hwnd, &mut r) };
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            if w <= 0 || h <= 0 {
                eprintln!("screen-shot: window has no area: {w}x{h}");
                std::process::exit(1);
            }
            // Capture the window's own screen position rather than asking it to
            // render itself: both popups blit into their window DC, which
            // PrintWindow does not reliably see.
            let (img, ok) = unsafe { grab(r.left, r.top, w, h, true) }
                .unwrap_or_else(|e| {
                    eprintln!("screen-shot: {e}");
                    std::process::exit(1);
                });
            if let Err(e) = img.save_png(Path::new(&args[2])) {
                eprintln!("screen-shot: {e}");
                std::process::exit(1);
            }
            println!("captured window {:?} {w}x{h} BitBlt={ok} -> {}", args[1], args[2]);
        }
        _ => {
            let out = &args[0];
            let (w, h) = unsafe {
                (
                    GetSystemMetrics(SM_CXSCREEN),
                    GetSystemMetrics(SM_CYSCREEN),
                )
            };
            let (img, ok) = unsafe { grab(0, 0, w, h, true) }.unwrap_or_else(|e| {
                eprintln!("screen-shot: {e}");
                std::process::exit(1);
            });
            let img = if args.len() >= 5 {
                let n: Vec<i32> = args[1..5].iter().filter_map(|v| v.parse().ok()).collect();
                if n.len() != 4 {
                    usage();
                }
                img.crop(n[0], n[1], n[2], n[3])
            } else {
                img
            };
            if let Err(e) = img.save_png(Path::new(out)) {
                eprintln!("screen-shot: {e}");
                std::process::exit(1);
            }
            let distinct = distinct_colours(&img);
            println!("captured {}x{} BitBlt={ok} distinct-colours={distinct} -> {out}", img.w, img.h);
        }
    }
}

/// Number of distinct colours, as a cheap "is anything actually in here?"
/// signal. A blank or all-one-colour capture answers immediately.
fn distinct_colours(img: &Image) -> usize {
    let mut seen = std::collections::HashSet::with_capacity(4096);
    for p in img.px.chunks_exact(3) {
        seen.insert([p[0], p[1], p[2]]);
        if seen.len() > 200_000 {
            break;
        }
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad(w: i32, h: i32) -> Image {
        let mut px = Vec::new();
        for y in 0..h {
            for x in 0..w {
                px.push((x * 2) as u8);
                px.push((y * 2) as u8);
                px.push(0);
            }
        }
        Image { w, h, px }
    }

    #[test]
    fn crop_extracts_the_right_region() {
        let im = grad(10, 10);
        let c = im.crop(2, 3, 5, 7);
        assert_eq!((c.w, c.h), (3, 4));
        // Top-left of the crop is (2,3) of the source.
        assert_eq!(c.at(0, 0), im.at(2, 3));
        assert_eq!(c.at(2, 3), im.at(4, 6));
    }

    #[test]
    fn crop_clips_to_the_image_instead_of_panicking() {
        let im = grad(10, 10);
        assert_eq!((im.crop(-5, -5, 3, 3).w, im.crop(-5, -5, 3, 3).h), (3, 3));
        assert_eq!((im.crop(8, 8, 99, 99).w, im.crop(8, 8, 99, 99).h), (2, 2));
    }

    /// The bug this tool exists to find: two identical screens must report
    /// zero changed pixels. If the threshold were off, or the comparison
    /// skipped the box, it would report a difference and hide the failure.
    #[test]
    fn identical_images_report_zero_changed_pixels() {
        let a = grad(40, 40);
        let b = grad(40, 40);
        let (changed, _, _, _, _) = diff(&a, &b, (0, 0, 40, 40));
        assert_eq!(changed, 0, "identical captures must diff to zero");
    }

    #[test]
    fn a_fully_opaque_overlay_counts_every_pixel() {
        let a = grad(40, 40);
        let mut b = grad(40, 40);
        for p in b.px.iter_mut() {
            *p = 255 - *p;
        }
        let (changed, bx0, by0, bx1, by1) = diff(&a, &b, (0, 0, 40, 40));
        assert_eq!(changed, 1600);
        assert_eq!((bx0, by0, bx1, by1), (0, 0, 40, 40));
    }

    #[test]
    fn a_single_changed_pixel_is_found_with_its_bbox() {
        let a = grad(40, 40);
        let mut b = grad(40, 40);
        b.px[(10 * 40 + 7) * 3] = b.px[(10 * 40 + 7) * 3].wrapping_add(200);
        let (changed, bx0, by0, bx1, by1) = diff(&a, &b, (0, 0, 40, 40));
        assert_eq!(changed, 1, "one pixel over threshold by 200 must register once");
        assert_eq!((bx0, by0, bx1, by1), (7, 10, 8, 11));
    }

    /// Small per-channel noise must not count: a live desktop differs by a few
    /// units between two captures, and a 0-threshold diff would drown in it.
    #[test]
    fn sub_threshold_noise_is_ignored() {
        let a = grad(40, 40);
        let mut b = grad(40, 40);
        for p in b.px.iter_mut() {
            *p = p.wrapping_add(1);
        }
        let (changed, _, _, _, _) = diff(&a, &b, (0, 0, 40, 40));
        assert_eq!(changed, 0, "a 1-per-channel wobble is not a change");
    }

    #[test]
    fn distinct_colours_counts_unique_pixels() {
        let mut img = grad(4, 4);
        assert!(distinct_colours(&img) > 1);
        for p in img.px.iter_mut() {
            *p = 0;
        }
        assert_eq!(distinct_colours(&img), 1);
    }
}
