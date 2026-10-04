//! saka-popup — a frameless, always-on-top panchangam panel for the YASB bar.
//!
//! YASB's `CustomWidget` has no popup support (its schema only exposes `*_menu`
//! on the built-in widget types), so a themed panel attached to a custom widget
//! has to be its own top-level window. That is what this is: a panel attached
//! under the bar, drawn with GDI, which reads the *active* Rangalipi theme out
//! of `styles.css` so it recolours on a theme switch.
//!
//!   saka-popup.exe            show the panel for the current instant
//!   saka-popup.exe --te       start in Telugu script
//!   saka-popup.exe --smoke    headless self-test: parse the theme and render
//!                             every row, without creating a window
//!
//! Controls: Escape / click / focus-loss dismisses. Tab toggles Telugu.
//!
//! # The look
//!
//! Modelled on macOS Spotlight rather than on a Win32 dialog, because that is
//! the reference for "a floating panel that feels expensive":
//!
//! * A real acrylic material. The desktop behind the panel is captured before
//!   the window is shown and blended back under the theme colour, so the
//!   panel picks up the wallpaper instead of sitting on it as a grey slab.
//!   (`yasb_chrome::Backdrop`; the capture has to happen *before* the window
//!   exists or it photographs the panel itself.)
//! * Large radii — 20px on the window, 10px on the pills — feathered through
//!   a signed-distance field, so no edge is a hard 1px step.
//! * Light from the top-left: a diagonal gradient plus an accent bloom behind
//!   the header, and a hairline highlight along the top edge only. That single
//!   asymmetric highlight is most of what separates "flat rectangle" from
//!   "physical surface".
//! * One accent, used sparingly: the moon glyph, the progress fills, and the
//!   live values. Everything structural stays in near-neutral tones.
//!
//! The panel is not layered. `UpdateLayeredWindow` reports success here and
//! puts nothing on screen; see `yasb_chrome::gdi` for the full account.

mod theme;

use std::cell::RefCell;
use std::path::PathBuf;

use saka::{Element, Panchang, Script, paksha_at, tithi_name_at};
use theme::{Rgba, Theme};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_ESCAPE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use yasb_chrome::canvas::moon_lit_mask;
use yasb_chrome::gdi::{
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect, text_width,
};
use yasb_chrome::{DISPLAY_FAMILY, INDIC_FAMILY, TEXT_FAMILY};

// Hyderabad / IST: the same defaults the saka CLI uses.
const LAT: f64 = 17.3850;
const LON: f64 = 78.4867;
const TZ: f64 = 5.5;

const W: i32 = 460;
/// Header block + nine rows + the moon strip + the footer hint.
const H: i32 = 604;
/// Window corner radius. Every inner radius is derived from this so the panel
/// reads as one material rather than a stack of unrelated shapes.
const RADIUS: i32 = 20;
/// Breathing room between the panel edge and its content.
const PAD: i32 = 22;
const ROW_H: i32 = 40;
const TIMER_ID: usize = 1;

/// Radius of the small pills (progress tracks, the moon's ring).
const PILL: i32 = 10;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// The desktop snapshot, taken once before the window is created. Holding
    /// it for the panel's whole life is what keeps the acrylic from feeding
    /// back on itself across the 1 Hz repaints.
    static BACKDROP: RefCell<Option<Backdrop>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct App {
    theme: Theme,
    panchangam: Panchang,
    script: Script,
    /// Where the window sits, so the backdrop can be taken before it exists.
    origin: (i32, i32),
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
                value: p.phase_name().to_string(),
                progress: Some((
                    p.illum,
                    format!(
                        "{}% · {}",
                        (p.illum * 100.0).round() as u32,
                        if p.waxing { "waxing" } else { "waning" }
                    ),
                )),
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

    APP.with(|c| *c.borrow_mut() = Some(App { theme, panchangam, script, origin: (0, 0) }));
    unsafe { run() }
}

/// Headless self-test: proves theme parsing and layout work with no window, so
/// a broken build is caught without opening a window on someone's desktop.
fn smoke(t: &Theme, p: &Panchang, script: Script) {
    let app = App { theme: t.clone(), panchangam: p.clone(), script, origin: (0, 0) };
    println!(
        "theme bg={:?} text={:?} border={:?} accent={:?}",
        t.bg, t.text, t.border, t.accent
    );
    println!("tithi={} nak={} yoga={} karana={} vara={}", p.tithi.index, p.nakshatra.index, p.yoga.index, p.karana_slot, p.vara);
    println!("moon phase={:?} western={:?} illum={:.0}%", p.phase_name(), p.phase_name_western(), p.illum * 100.0);
    for r in app.rows() {
        println!("  {:<12}{}", r.label, r.value);
    }
    // The panel is a fixed grid; if the rows ever stop fitting in H the footer
    // silently lands on top of the last row, which no test of the data alone
    // would catch.
    let used = header_height(&app) + app.rows().len() as i32 * ROW_H + MOON_STRIP_H + FOOT_H + PAD;
    assert!(used <= H, "content needs {used}px but the window is {H}px");
    println!("smoke: ok ({} rows, layout {used}/{H})", app.rows().len());
}

/// Height of the date block above the rows.
const HEADER_H: i32 = 78;

fn header_height(_app: &App) -> i32 {
    HEADER_H
}

/// The moon glyph strip: disc, phase name, illumination.
const MOON_STRIP_H: i32 = 76;
const FOOT_H: i32 = 26;

/// The fonts for one paint pass, created and destroyed together.
struct Fonts {
    display: HFONT,
    body: HFONT,
    small: HFONT,
    label: HFONT,
}

unsafe fn make_fonts(script: Script) -> Fonts {
    let display_family = if script == Script::Telugu { INDIC_FAMILY } else { DISPLAY_FAMILY };
    let text_family = if script == Script::Telugu { INDIC_FAMILY } else { TEXT_FAMILY };
    Fonts {
        // 28px display for the date: this is the one line that has to carry
        // the panel's whole hierarchy, so it is set large and tight.
        display: font(-28, 600, display_family),
        body: font(-16, 500, text_family),
        small: font(-13, 400, text_family),
        // All-caps section labels want tracking, which GDI cannot express, so
        // they are set small and letter-spaced by hand in `tracked_text`.
        label: font(-11, 600, text_family),
    }
}

/// Draw small-caps text with manual letter spacing.
///
/// `DrawTextW` cannot letter-space, and untracked 11px caps look cramped
/// against a 28px headline. Drawing glyph by glyph costs a few dozen `TextOut`
/// calls per panel, which is nothing at a 1 Hz repaint.
unsafe fn tracked_text(dc: HDC, text: &str, x: i32, y: i32, tracking: i32) -> i32 {
    let mut cx = x;
    let mut buf: Vec<u16> = Vec::with_capacity(2);
    for ch in text.chars() {
        buf.clear();
        let mut units = [0u16; 2];
        buf.extend_from_slice(ch.encode_utf16(&mut units[..]));
        let mut sz = SIZE { cx: 0, cy: 0 };
        let _ = GetTextExtentPoint32W(dc, &buf, &mut sz);
        let _ = TextOutW(dc, cx, y, &buf);
        cx += sz.cx + tracking;
    }
    cx - x
}

/// Paint the whole panel: shapes into the DIB, then text through GDI on top.
///
/// Two passes because GDI's text rendering needs the font selected into the DC,
/// while the shapes want direct pixel writes. Interleaving them per element
/// would mean selecting and deselecting fonts dozens of times per frame; one
/// switch each way is not worth avoiding.
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

    let mut dib = Dib::new(w, h);
    let t = &app.theme;

    // ---- shape pass ------------------------------------------------------
    {
        let mut c = dib.canvas();
        // Base fill. The theme's own translucency is deliberately ignored here:
        // the acrylic comes from the backdrop snapshot blended on top of this,
        // which is controllable in a way a stacked alpha is not.
        c.clear(t.bg.over(Rgba::rgb(20, 20, 28)));

        // Desktop snapshot underneath, at the opacity an acrylic material
        // would actually have.
        BACKDROP.with(|b| {
            if let Some(bd) = b.borrow().as_ref() {
                bd.draw_under(&mut c, 0, 0, t.backdrop_opacity);
            }
        });

        // Light from the top-left: a diagonal-ish gradient, then the accent
        // bloom that gives the header its lift.
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);
        c.radial_glow(
            w as f32 * 0.22,
            -10.0,
            w as f32 * 0.95,
            t.accent,
            t.bloom,
        );

        // Hairline frame. The top edge gets a brighter, thinner highlight,
        // which is the cue that sells the surface as lit from above.
        c.round_rect_border(1, 1, w - 1, h - 1, RADIUS, 1, t.border);
        c.round_rect_border(1, 1, w - 1, 14, RADIUS, 1, t.highlight);

        // Moon strip sits directly under the header divider.
        let head_h = header_height(&app);
        let moon_top = PAD - 4 + head_h;
        c.hline(PAD, w - PAD, moon_top - 10, t.hairline);

        // Moon disc: a dark body, the lit limb, and a soft halo.
        let mr = MOON_R as f32;
        let mcx = PAD as f32 + mr;
        let mcy = (moon_top + MOON_STRIP_H / 2) as f32;
        c.radial_glow(mcx, mcy, mr * 2.4, t.accent, t.moon_glow);
        for py in (mcy as i32 - mr as i32 - 2)..=(mcy as i32 + mr as i32 + 2) {
            for px in (mcx as i32 - mr as i32 - 2)..=(mcx as i32 + mr as i32 + 2) {
                let cx0 = c.disc_coverage(px, py, mcx, mcy, mr);
                if cx0 <= 0.0 {
                    continue;
                }
                let lit = moon_lit_mask(
                    px as f32 + 0.5,
                    py as f32 + 0.5,
                    mcx,
                    mcy,
                    mr,
                    app.panchangam.illum as f32,
                    app.panchangam.waxing,
                );
                // Earthshine: the unlit limb is not black, it is just dim.
                let body = t.moon_dim;
                // Compositing lit over body by hand, because the canvas has no
                // "draw with coverage onto something already drawn" that also
                // takes two colours.
                let mixf = lit;
                let col = Rgba {
                    r: (body.r as f32 + (t.moon_lit.r as f32 - body.r as f32) * mixf) as u8,
                    g: (body.g as f32 + (t.moon_lit.g as f32 - body.g as f32) * mixf) as u8,
                    b: (body.b as f32 + (t.moon_lit.b as f32 - body.b as f32) * mixf) as u8,
                    a: 255,
                };
                c.blend_pixel(px, py, col, cx0);
            }
        }

        // The search-bar-shaped card behind the rows. Rows sit on a slightly
        // raised surface rather than directly on the panel, which is what
        // gives the list its depth.
        let rows_top = moon_top + MOON_STRIP_H - 4;
        let rows_h = app.rows().len() as i32 * ROW_H;
        let card_y = rows_top + 2;
        c.round_rect(
            PAD - 8,
            card_y,
            w - PAD + 8,
            card_y + rows_h + 14,
            RADIUS - 4,
            t.card,
        );
        c.round_rect_border(
            PAD - 8,
            card_y,
            w - PAD + 8,
            card_y + rows_h + 14,
            RADIUS - 4,
            1,
            t.hairline,
        );

        // Progress pills, drawn here rather than in the text pass because a
        // pill is three rounded fills. The geometry here and the caption in
        // the text pass must agree, so both derive from these constants.
        let track_x = TRACK_X;
        let track_w = (w - PAD - track_x + PAD - 8).max(60);
        for (i, r) in app.rows().iter().enumerate() {
            let Some((frac, _)) = &r.progress else { continue };
            let ry = card_y + 12 + i as i32 * ROW_H + 26;
            c.round_rect(track_x, ry, track_x + track_w, ry + 4, PILL, t.track);
            let fw = ((frac.clamp(0.0, 1.0)) * track_w as f64).round() as i32;
            if fw > 0 {
                c.round_rect(track_x, ry, track_x + fw, ry + 4, PILL, t.accent);
            }
        }

        // Footer separator and a small accent dot, so the bottom edge reads as
        // finished rather than cut off.
        let foot_y = card_y + rows_h + 14 + (FOOT_H - 8);
        c.hline(PAD, w - PAD, foot_y, t.hairline);
    }

    // ---- text pass -------------------------------------------------------
    {
        let dc = dib.dc();
        let fonts = make_fonts(app.script);
        SetBkMode(dc, TRANSPARENT);

        let mut y = PAD + 10;
        // Tracked label: the weekday, small and spaced.
        SelectObject(dc, HGDIOBJ(fonts.label.0));
        SetTextColor(dc, colorref(t.accent));
        let weekday = app.script.vara(app.panchangam.vara).to_uppercase();
        tracked_text(dc, &weekday, PAD, y, 2);
        y += 34;

        // The date, set large. This one line carries the panel's whole
        // hierarchy, which is why it gets the display face at 28px.
        SelectObject(dc, HGDIOBJ(fonts.display.0));
        SetTextColor(dc, colorref(t.text));
        let p = &app.panchangam;
        let s = app.script;
        let title = format!("{} {} {}", p.day, month_abbr(p.month), p.year);
        draw_text(dc, &title, rect(PAD, y, w - PAD * 2, 34), DT_LEFT);
        y += 36;

        SelectObject(dc, HGDIOBJ(fonts.small.0));
        SetTextColor(dc, colorref(t.subtext));
        let sub = format!(
            "{} {} · Saka {} · VS {}",
            s.month(p.saka_month),
            p.saka_day,
            p.saka_year,
            p.vikram_year
        );
        draw_text(dc, &sub, rect(PAD, y, w - PAD * 2, 18), DT_LEFT);

        // Moon strip text.
        let moon_top = PAD - 4 + header_height(&app);
        let text_x = PAD + MOON_R * 2 + 14;
        SelectObject(dc, HGDIOBJ(fonts.body.0));
        SetTextColor(dc, colorref(t.text));
        draw_text(dc, p.phase_name(), rect(text_x, moon_top + 8, w - text_x - PAD, 22), DT_LEFT);
        SelectObject(dc, HGDIOBJ(fonts.small.0));
        SetTextColor(dc, colorref(t.subtext));
        let cap = format!(
            "{}% illuminated · {}",
            (p.illum * 100.0).round() as u32,
            p.phase_name_western()
        );
        draw_text(dc, &cap, rect(text_x, moon_top + 32, w - text_x - PAD, 18), DT_LEFT);

        // Rows.
        let rows = app.rows();
        let card_y = moon_top + MOON_STRIP_H - 4 + 2;
        let value_x = 128i32;
        for (i, r) in rows.iter().enumerate() {
            let ry = card_y + 12 + i as i32 * ROW_H;
            SelectObject(dc, HGDIOBJ(fonts.small.0));
            SetTextColor(dc, colorref(t.faint));
            draw_text(dc, r.label, rect(PAD, ry + 4, 100, 18), DT_LEFT);

            SelectObject(dc, HGDIOBJ(fonts.body.0));
            SetTextColor(dc, colorref(t.text));
            draw_text(dc, &r.value, rect(value_x, ry, 150, 24), DT_LEFT | DT_END_ELLIPSIS);

            if let Some((_, cap)) = &r.progress {
                SelectObject(dc, HGDIOBJ(fonts.small.0));
                SetTextColor(dc, colorref(t.subtext));
                let wdt = text_width(dc, cap);
                draw_text(
                    dc,
                    cap,
                    rect(w - PAD - wdt - 6, ry + 28, wdt + 6, 16),
                    DT_LEFT,
                );
            }
        }

        // Footer hint.
        SelectObject(dc, HGDIOBJ(fonts.small.0));
        SetTextColor(dc, colorref(t.faint));
        let foot_y = card_y + rows.len() as i32 * ROW_H + 14 + 10;
        draw_text(
            dc,
            "Tab: Telugu / English     Esc or click to close",
            rect(PAD, foot_y, w - PAD * 2, 18),
            DT_LEFT,
        );

        let _ = DeleteObject(HGDIOBJ(fonts.display.0));
        let _ = DeleteObject(HGDIOBJ(fonts.body.0));
        let _ = DeleteObject(HGDIOBJ(fonts.small.0));
        let _ = DeleteObject(HGDIOBJ(fonts.label.0));
    }

    blit_to_window(hwnd, &dib);
}

/// Radius of the moon glyph.
const MOON_R: i32 = 26;

/// Left edge of the progress track, shared by the pill geometry in the shape
/// pass and the caption alignment in the text pass.
const TRACK_X: i32 = 232;

fn month_abbr(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        [(m as usize - 1).min(11)]
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();

    // Centre on the monitor holding the cursor, just below the bar. The
    // position is decided *before* the window exists so the desktop can be
    // captured underneath it.
    let mut pt = POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let cx = (mi.rcMonitor.left + mi.rcMonitor.right) / 2;
    let origin = (cx - W / 2, mi.rcMonitor.top + 40);
    APP.with(|c| {
        if let Some(a) = c.borrow_mut().as_mut() {
            a.origin = origin;
        }
    });
    BACKDROP.with(|b| {
        *b.borrow_mut() = Backdrop::capture(origin.0, origin.1, W, H);
    });

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
    let hwnd = match CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        windows::core::PCWSTR(cls.as_ptr()),
        windows::core::PCWSTR(title.as_ptr()),
        WS_POPUP,
        origin.0,
        origin.1,
        W,
        H,
        None,
        None,
        HINSTANCE(hinst.0),
        None,
    ) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("saka-popup: CreateWindowExW failed: {e}");
            std::process::exit(1);
        }
    };

    let _ = SetWindowPos(hwnd, HWND_TOPMOST, origin.0, origin.1, W, H, SWP_SHOWWINDOW);
    apply_round_region(hwnd, W, H, RADIUS);
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
        // The panel has no WS_EX_LAYERED, so the system repaints exposed and
        // restyled parts through WM_PAINT rather than us blitting on a timer.
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            if !BeginPaint(hwnd, &mut ps).0.is_null() {
                paint(hwnd);
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
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
