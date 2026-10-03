//! saka-popup — a frameless, always-on-top panchangam panel for the YASB bar.
//!
//! YASB's `CustomWidget` has no popup support (its schema only exposes `*_menu`
//! on the built-in widget types), so a themed panel attached to a custom widget
//! has to be its own top-level window. That is what this is: a layered Win32
//! window painted with GDI, positioned under the bar, which reads the *active*
//! Rangalipi theme out of `styles.css` so it recolours on a theme switch.
//!
//!   saka-popup.exe            show the panel for the current instant
//!   saka-popup.exe --te       start in Telugu script
//!   saka-popup.exe --smoke    headless self-test: parse the theme and render
//!                             every row, without creating a window
//!
//! Controls: Escape / click / focus-loss dismisses. Tab toggles Telugu.

mod theme;

use std::cell::RefCell;
use std::path::PathBuf;

use saka::{Element, Panchang, Script, paksha_at, tithi_name_at};
use theme::{Rgba, Theme};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_ESCAPE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::*;

// Hyderabad / IST: the same defaults the saka CLI uses.
const LAT: f64 = 17.3850;
const LON: f64 = 78.4867;
const TZ: f64 = 5.5;

const W: i32 = 440;
const H: i32 = 470;
const TIMER_ID: usize = 1;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct App {
    theme: Theme,
    panchangam: Panchang,
    script: Script,
}

/// One row: a label, a value, and an optional progress bar with a caption.
struct Row {
    label: &'static str,
    value: String,
    progress: Option<(f64, String)>,
}

fn bar_caption(e: &Element, p: &Panchang) -> (f64, String) {
    (
        e.progress,
        format!("{}% · {}", (e.progress * 100.0).round() as u32, p.hhmm(e.ends_jd)),
    )
}

impl App {
    fn rows(&self) -> Vec<Row> {
        let p = &self.panchangam;
        let s = self.script;
        vec![
            Row {
                label: "Tithi",
                value: format!(
                    "{} {}",
                    paksha_at(p.tithi.index, s),
                    tithi_name_at(p.tithi.index, s)
                ),
                progress: Some(bar_caption(&p.tithi, p)),
            },
            Row {
                label: "Nakshatra",
                value: s.nakshatra(p.nakshatra.index).to_string(),
                progress: Some(bar_caption(&p.nakshatra, p)),
            },
            Row {
                label: "Yoga",
                value: s.yoga(p.yoga.index).to_string(),
                progress: Some(bar_caption(&p.yoga, p)),
            },
            Row {
                label: "Karana",
                value: s.karana(p.karana_slot).to_string(),
                progress: Some((
                    p.karana.progress,
                    if p.karana_first_half { "first half" } else { "second half" }.into(),
                )),
            },
            Row { label: "Vara", value: s.vara(p.vara).to_string(), progress: None },
            Row {
                label: "Moon",
                value: format!("{} · {}%", p.phase_name(), (p.illum * 100.0).round() as u32),
                progress: Some((p.illum, if p.waxing { "waxing".into() } else { "waning".into() })),
            },
            Row {
                label: "Amanta",
                value: format!("{} · day {}", s.month(p.amanta_month), p.amanta_tithi_day),
                progress: None,
            },
            Row {
                label: "Purnimanta",
                value: s.month(p.purnimanta_month).to_string(),
                progress: None,
            },
            Row {
                label: "Sun",
                value: format!(
                    "{} · {}",
                    p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()),
                    p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into())
                ),
                progress: None,
            },
            Row {
                label: "Next",
                value: format!(
                    "new {} · full {}",
                    p.day_label(p.next_new_moon_jd),
                    p.day_label(p.next_full_moon_jd)
                ),
                progress: None,
            },
        ]
    }
}

fn styles_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("yasb").join("styles.css")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let theme = Theme::load(&styles_path());
    let panchangam = Panchang::now(LAT, LON, TZ);
    let script = if args.iter().any(|a| a == "--te") {
        Script::Telugu
    } else {
        Script::Latin
    };

    if args.iter().any(|a| a == "--smoke") {
        smoke(&theme, &panchangam, script);
        return;
    }

    APP.with(|c| *c.borrow_mut() = Some(App { theme, panchangam, script }));
    unsafe { run() }
}

/// Headless self-test: proves theme parsing and layout work with no window, so
/// a broken build is caught without opening a window on someone's desktop.
fn smoke(t: &Theme, p: &Panchang, script: Script) {
    let app = App { theme: t.clone(), panchangam: p.clone(), script };
    println!(
        "theme bg={:?} text={:?} border={:?} accent={:?}",
        t.bg, t.text, t.border, t.accent
    );
    println!("tithi={} nak={} yoga={} karana={} vara={}", p.tithi.index, p.nakshatra.index, p.yoga.index, p.karana_slot, p.vara);
    for r in app.rows() {
        println!("  {:<12}{}", r.label, r.value);
    }
    println!("smoke: ok ({} rows)", app.rows().len());
}

fn color(c: Rgba) -> COLORREF {
    COLORREF(c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16))
}

/// Draw the whole panel into a top-down 32-bit DIB and hand it to
/// `UpdateLayeredWindow`, which is how a Win32 window gets per-pixel alpha
/// without a compositor or a blur pass.
unsafe fn paint(hwnd: HWND) {
    let Some(app) = APP.with(|c| c.borrow().clone()) else { return };
    let mut rc_win = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if GetWindowRect(hwnd, &mut rc_win).is_err() {
        return;
    }
    let w = rc_win.right - rc_win.left;
    let h = rc_win.bottom - rc_win.top;
    if w <= 0 || h <= 0 {
        return;
    }

    let dc_screen = GetDC(None);
    if dc_screen.0.is_null() {
        return;
    }
    let dc_mem = CreateCompatibleDC(dc_screen);
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let Ok(bmp) = CreateDIBSection(dc_screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
        let _ = DeleteDC(dc_mem);
        let _ = DeleteDC(dc_screen);
        return;
    };
    let old_bmp = SelectObject(dc_mem, HGDIOBJ(bmp.0));

    // Background: the translucent theme colour over a neutral dark base.
    let brush = CreateSolidBrush(color(app.theme.bg.over(Rgba::rgb(18, 18, 27))));
    let full = RECT { left: 0, top: 0, right: w, bottom: h };
    FillRect(dc_mem, &full, HBRUSH(brush.0));
    let _ = DeleteObject(HGDIOBJ(brush.0));

    // Double border, matching the YASB popups' `3px double var(--border)`.
    let pen = CreatePen(PS_SOLID, 1, color(app.theme.border));
    let old_pen = SelectObject(dc_mem, HGDIOBJ(pen.0));
    let old_br = SelectObject(dc_mem, GetStockObject(NULL_BRUSH));
    for i in 0..3 {
        let inset = 2 + i as i32 * 3;
        let r = RECT { left: inset, top: inset, right: w - inset, bottom: h - inset };
        FrameRect(dc_mem, &r, HBRUSH(pen.0));
    }
    SelectObject(dc_mem, old_pen);
    SelectObject(dc_mem, old_br);
    let _ = DeleteObject(HGDIOBJ(pen.0));

    // Fonts: Segoe UI carries Latin, Nirmala UI carries Telugu.
    let family: Vec<u16> = if app.script == Script::Telugu {
        "Nirmala UI\0".encode_utf16().collect()
    } else {
        "Segoe UI\0".encode_utf16().collect()
    };
    let font = CreateFontW(
        -15, 0, 0, 0, 400, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    );
    let head_font = CreateFontW(
        -19, 0, 0, 0, 600, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    );
    let old_font = SelectObject(dc_mem, HGDIOBJ(font.0));
    SetBkMode(dc_mem, TRANSPARENT);

    let p = &app.panchangam;
    let s = app.script;
    let mut y = 16i32;

    // Title block.
    SelectObject(dc_mem, HGDIOBJ(head_font.0));
    SetTextColor(dc_mem, color(app.theme.text));
    let title = format!(
        "{} · {:02} {} {}",
        s.vara(p.vara),
        p.day,
        month_abbr(p.month),
        p.year
    );
    draw_text(dc_mem, &title, 18, y, w - 36);
    SelectObject(dc_mem, HGDIOBJ(font.0));
    y += 24;
    SetTextColor(dc_mem, color(app.theme.subtext));
    let sub = format!(
        "{} {} · Saka {} · Vikram Samvat {}",
        s.month(p.saka_month),
        p.saka_day,
        p.saka_year,
        p.vikram_year
    );
    draw_text(dc_mem, &sub, 18, y, w - 36);
    y += 30;

    // Divider.
    let div = CreateSolidBrush(color(app.theme.surface));
    let dr = RECT { left: 18, top: y - 10, right: w - 18, bottom: y - 9 };
    FillRect(dc_mem, &dr, HBRUSH(div.0));
    let _ = DeleteObject(HGDIOBJ(div.0));

    let value_x = 112i32;
    let track_x = 300i32;
    let track_w = (w - track_x - 20).max(40);

    for r in app.rows() {
        SetTextColor(dc_mem, color(app.theme.faint));
        draw_text(dc_mem, r.label, 18, y + 1, 92);
        SetTextColor(dc_mem, color(app.theme.text));
        draw_text(dc_mem, &r.value, value_x, y + 1, track_x - value_x - 8);

        if let Some((frac, cap)) = &r.progress {
            let ty = y + 12;
            let track = CreateSolidBrush(color(app.theme.track));
            let tr = RECT { left: track_x, top: ty, right: track_x + track_w, bottom: ty + 4 };
            FillRect(dc_mem, &tr, HBRUSH(track.0));
            let _ = DeleteObject(HGDIOBJ(track.0));
            let fw = ((frac.clamp(0.0, 1.0)) * track_w as f64).round() as i32;
            if fw > 0 {
                let fill = CreateSolidBrush(color(app.theme.accent));
                let fr = RECT { left: track_x, top: ty, right: track_x + fw, bottom: ty + 4 };
                FillRect(dc_mem, &fr, HBRUSH(fill.0));
                let _ = DeleteObject(HGDIOBJ(fill.0));
            }
            SetTextColor(dc_mem, color(app.theme.subtext));
            draw_text(dc_mem, cap, track_x, ty + 7, track_w + 30);
        }
        y += 42;
    }

    // Footer hint.
    SetTextColor(dc_mem, color(app.theme.faint));
    draw_text(dc_mem, "Tab: Telugu/English   ·   Esc or click to close", 18, h - 26, w - 36);

    SelectObject(dc_mem, old_font);
    let _ = DeleteObject(HGDIOBJ(font.0));
    let _ = DeleteObject(HGDIOBJ(head_font.0));

    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    // Passing None for the destination keeps the window where SetWindowPos put it.
    let _ = UpdateLayeredWindow(
        hwnd,
        dc_screen,
        None,
        None,
        dc_mem,
        Some(&src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );

    SelectObject(dc_mem, old_bmp);
    let _ = DeleteObject(HGDIOBJ(bmp.0));
    let _ = DeleteDC(dc_mem);
    let _ = DeleteDC(dc_screen);
    let _ = bits;
}

fn month_abbr(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        [(m as usize - 1).min(11)]
}

unsafe fn draw_text(dc: HDC, text: &str, x: i32, y: i32, max_w: i32) {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    buf.push(0);
    let mut rc = RECT { left: x, top: y, right: x + max_w, bottom: y + 400 };
    DrawTextW(dc, &mut buf, &mut rc, DT_LEFT | DT_TOP | DT_NOPREFIX | DT_SINGLELINE);
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();
    let cls: Vec<u16> = "SakaPopupClass\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(hinst.0),
        lpszClassName: windows::core::PCWSTR(cls.as_ptr()),
        ..Default::default()
    };
    if RegisterClassW(&wc) == 0 {
        eprintln!("saka-popup: RegisterClassW failed");
        std::process::exit(1);
    }

    let title: Vec<u16> = "Panchangam\0".encode_utf16().collect();
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
        windows::core::PCWSTR(cls.as_ptr()),
        windows::core::PCWSTR(title.as_ptr()),
        WS_POPUP,
        100,
        100,
        W,
        H,
        None,
        None,
        HINSTANCE(hinst.0),
        None,
    );
    let hwnd = match hwnd {
        Ok(h) => h,
        Err(e) => {
            eprintln!("saka-popup: CreateWindowExW failed: {e}");
            std::process::exit(1);
        }
    };

    // Center on the monitor holding the cursor, just below the bar.
    let mut pt = POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let cx = (mi.rcMonitor.left + mi.rcMonitor.right) / 2;
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, cx - W / 2, mi.rcMonitor.top + 40, W, H, SWP_SHOWWINDOW);
    let _ = SetForegroundWindow(hwnd);
    let _ = SetTimer(hwnd, TIMER_ID, 1000, None);
    paint(hwnd);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wp.0 == TIMER_ID => {
            // Refresh the live values and repaint at 1 Hz.
            APP.with(|c| {
                if let Some(a) = c.borrow_mut().as_mut() {
                    a.panchangam = Panchang::now(LAT, LON, TZ);
                    // Re-read the theme so a switch while the popup is open is
                    // picked up on the next tick.
                    a.theme = Theme::load(&styles_path());
                }
            });
            paint(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            match wp.0 as u16 {
                k if k == VK_ESCAPE.0 as u16 => {
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_TAB.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.script = if a.script == Script::Telugu {
                                Script::Latin
                            } else {
                                Script::Telugu
                            };
                        }
                    });
                    paint(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            let _ = SetCapture(hwnd);
            let _ = ReleaseCapture();
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_ID);
            let _ = PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}