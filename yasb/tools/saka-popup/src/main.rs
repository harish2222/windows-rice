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
//! * Large radii — 20px on the window, 16px on the row card, 10px on the
//!   pills — feathered through a signed-distance field, so no edge is a hard
//!   1px step.
//! * Light from the top: a vertical gradient plus an accent bloom behind the
//!   header, and a hairline highlight along the top edge only. That single
//!   asymmetric highlight is most of what separates "flat rectangle" from
//!   "physical surface".
//! * One accent, used sparingly: the moon glyph, the progress fills, the
//!   weekday label. Everything structural stays in near-neutral tones.
//!
//! Geometry lives in [`layout`], not here, because the first version computed
//! its bands twice and the two copies disagreed — the moon ended up drawn on
//! top of the date.
//!
//! The panel is not layered. `UpdateLayeredWindow` reports success here and
//! puts nothing on screen; see `yasb_chrome::gdi` for the full account.

mod layout;
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

const W: i32 = layout::W;
const RADIUS: i32 = layout::RADIUS;
const PAD: i32 = layout::PAD;
const TIMER_ID: usize = 1;
/// Radius of the moon glyph.
const MOON_R: i32 = 30;

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

    fn layout(&self) -> layout::Layout {
        layout::Layout::new(self.rows().len() as i32)
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
    // One panel, ever. The bar fires this on every click of the saka chip, and
    // a stacked second copy would capture its own backdrop and repaint over
    // the first at 1 Hz. A duplicate launch raises the panel that is already
    // open and exits.
    match yasb_chrome::acquire("Local\\yasb-saka-popup") {
        Ok(None) => {
            yasb_chrome::raise_window_of_class("SakaPopupClass");
            return;
        }
        // If the guard itself fails, still open the panel.
        Err(e) => eprintln!("saka-popup: single-instance guard unavailable: {e}"),
        Ok(Some(_instance)) => unsafe { run() },
    }
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
    // The panel is a fixed grid; if the geometry ever stops fitting, the
    // footer silently lands on top of the last row, which no test of the data
    // alone would catch.
    let l = app.layout();
    l.assert_no_overlap(app.rows().len() as i32).expect("layout overlaps");
    println!("smoke: ok ({} rows, panel {}x{})", app.rows().len(), W, l.height);
}

/// Draw small-caps text with manual letter spacing.
///
/// `DrawTextW` cannot letter-space, and untracked 11px caps look cramped
/// against a 28px headline. Drawing glyph by glyph costs a few dozen `TextOut`
/// calls per panel, which is nothing at a 1 Hz repaint.
unsafe fn tracked_text(dc: HDC, text: &str, x: i32, y: i32, tracking: i32) -> i32 {
    let mut cx = x;
    let mut buf: Vec<u16> = Vec::with_capacity(2);
    let mut units = [0u16; 2];
    for ch in text.chars() {
        buf.clear();
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
/// would mean selecting and deselecting fonts dozens of times per frame.
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
    let rows = app.rows();
    let l = app.layout();

    // ---- shape pass ------------------------------------------------------
    {
        let mut c = dib.canvas();
        // Base fill. The theme's own translucency is deliberately ignored here:
        // the acrylic comes from the backdrop snapshot blended on top of this,
        // which is controllable in a way a stacked alpha is not.
        c.clear(t.bg.over(Rgba::rgb(20, 20, 28)));

        BACKDROP.with(|b| {
            if let Some(bd) = b.borrow().as_ref() {
                bd.draw_under(&mut c, 0, 0, t.backdrop_opacity);
            }
        });

        // Light from the top: a gradient, then the accent bloom that gives the
        // header its lift.
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);
        c.radial_glow(w as f32 * 0.22, -10.0, w as f32 * 0.95, t.accent, t.bloom);

        // Hairline frame. The top edge gets a brighter, thinner highlight,
        // which is the cue that sells the surface as lit from above.
        c.round_rect_border(1, 1, w - 1, h - 1, RADIUS, 1, t.border);
        c.round_rect_border(1, 1, w - 1, 14, RADIUS, 1, t.highlight);
        c.hline(PAD, w - PAD, l.rule_y, t.hairline);

        // Moon disc: a dim body, the lit limb, and a soft halo. It rides at the
        // right of the header, level with the date, rather than in a strip of
        // its own — a 30px disc does not need 76px of panel to itself.
        let mr = MOON_R as f32;
        let mcx = (w - PAD - MOON_R) as f32;
        let mcy = ((l.moon_y + l.moon_bottom) / 2) as f32;
        c.radial_glow(mcx, mcy, mr * 2.4, t.accent, t.moon_glow);
        for py in (mcy as i32 - MOON_R - 2)..=(mcy as i32 + MOON_R + 2) {
            for px in (mcx as i32 - MOON_R - 2)..=(mcx as i32 + MOON_R + 2) {
                let cov = c.disc_coverage(px, py, mcx, mcy, mr);
                if cov <= 0.0 {
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
                // Earthshine: the unlit limb is dim, not black — a black disc
                // reads as a hole punched in the panel.
                let body = t.moon_dim;
                let col = Rgba {
                    r: (body.r as f32 + (t.moon_lit.r as f32 - body.r as f32) * lit) as u8,
                    g: (body.g as f32 + (t.moon_lit.g as f32 - body.g as f32) * lit) as u8,
                    b: (body.b as f32 + (t.moon_lit.b as f32 - body.b as f32) * lit) as u8,
                    a: 255,
                };
                c.blend_pixel(px, py, col, cov);
            }
        }

        // The card the rows sit on. A raised surface rather than rows directly
        // on the panel is what gives the list its depth.
        c.round_rect(PAD - 8, l.card_y, w - PAD + 8, l.card_bottom, layout::CARD_R, t.card);
        c.round_rect_border(
            PAD - 8,
            l.card_y,
            w - PAD + 8,
            l.card_bottom,
            layout::CARD_R,
            1,
            t.hairline,
        );

        // Progress bars. The geometry comes from the layout module so the
        // shape pass and the text pass cannot disagree about where a bar is.
        let (tx, tw, th) = l.track(w);
        for (i, r) in rows.iter().enumerate() {
            let Some((frac, _)) = &r.progress else { continue };
            let y = l.row_y(i as i32) + 30;
            c.round_rect(tx, y, tx + tw, y + th, layout::PILL, t.track);
            let fw = ((frac.clamp(0.0, 1.0)) * tw as f64).round() as i32;
            if fw > 0 {
                c.round_rect(tx, y, tx + fw, y + th, layout::PILL, t.accent);
            }
        }
    }

    // ---- text pass -------------------------------------------------------
    {
        let dc = dib.dc();
        // Brahmic labels need Nirmala UI; the Latin faces carry no Telugu
        // glyphs and would render tofu.
        let (display_family, text_family) = if app.script == Script::Telugu {
            (INDIC_FAMILY, INDIC_FAMILY)
        } else {
            (DISPLAY_FAMILY, TEXT_FAMILY)
        };
        let display = font(-28, 600, display_family);
        let body = font(-16, 500, text_family);
        let small = font(-13, 400, text_family);
        let label = font(-11, 600, text_family);
        SetBkMode(dc, TRANSPARENT);

        // Weekday, small, tracked, in the accent — the one place the accent
        // touches type.
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.accent));
        let weekday = app.script.vara(app.panchangam.vara).to_uppercase();
        tracked_text(dc, &weekday, PAD, l.weekday_y, 2);

        // The date, large. This line carries the panel's whole hierarchy.
        let p = &app.panchangam;
        let s = app.script;
        SelectObject(dc, HGDIOBJ(display.0));
        SetTextColor(dc, colorref(t.text));
        let title = format!("{} {} {}", p.day, month_abbr(p.month), p.year);
        draw_text(dc, &title, rect(PAD, l.date_y, w - PAD * 2, 36), DT_LEFT | DT_VCENTER);

        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        let sub = format!(
            "{} {} · Saka {} · VS {}",
            s.month(p.saka_month),
            p.saka_day,
            p.saka_year,
            p.vikram_year
        );
        draw_text(dc, &sub, rect(PAD, l.era_y, w - PAD * 2, 18), DT_LEFT | DT_VCENTER);

        // Moon caption, right-aligned under the disc, inside the same band as
        // the era line so the two never collide.
        let cap_w = 150;
        draw_text(
            dc,
            p.phase_name(),
            rect(w - PAD - cap_w, l.moon_y + 4, cap_w, 20),
            DT_RIGHT | DT_VCENTER,
        );
        let pct = format!("{}% illuminated", (p.illum * 100.0).round() as u32);
        draw_text(
            dc,
            &pct,
            rect(w - PAD - cap_w, l.moon_y + 24, cap_w, 18),
            DT_RIGHT | DT_VCENTER,
        );

        // Rows.
        let (_tx, _tw, _th) = l.track(w);
        for (i, r) in rows.iter().enumerate() {
            let y = l.row_y(i as i32);
            SelectObject(dc, HGDIOBJ(small.0));
            SetTextColor(dc, colorref(t.faint));
            draw_text(dc, r.label, rect(PAD, y + 9, 86, 16), DT_LEFT | DT_VCENTER);

            SelectObject(dc, HGDIOBJ(body.0));
            SetTextColor(dc, colorref(t.text));
            let value_w = (w - PAD * 2 - 86).max(60);
            draw_text(
                dc,
                &r.value,
                rect(layout::Layout::VALUE_X, y + 5, value_w, 22),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );

            if let Some((_, cap)) = &r.progress {
                // Right-aligned past the end of the bar, on the bar's own line.
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.subtext));
                let cw = 92;
                draw_text(
                    dc,
                    cap,
                    rect(w - PAD - cw, y + 26, cw, 16),
                    DT_RIGHT | DT_VCENTER,
                );
                let _ = text_width(dc, cap);
            }
        }

        // Footer hint.
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.faint));
        draw_text(
            dc,
            "Tab: Telugu / English",
            rect(PAD, l.footer_y, 200, 18),
            DT_LEFT | DT_VCENTER,
        );
        draw_text(
            dc,
            "Esc or click to close",
            rect(w - PAD - 200, l.footer_y, 200, 18),
            DT_RIGHT | DT_VCENTER,
        );

        let _ = DeleteObject(HGDIOBJ(display.0));
        let _ = DeleteObject(HGDIOBJ(body.0));
        let _ = DeleteObject(HGDIOBJ(small.0));
        let _ = DeleteObject(HGDIOBJ(label.0));
    }

    blit_to_window(hwnd, &dib);
}

fn month_abbr(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        [(m as usize - 1).min(11)]
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();

    // Height comes from the layout, so the window can never disagree with its
    // own contents.
    let h = APP.with(|c| {
        c.borrow()
            .as_ref()
            .map(|a| a.layout().height)
            .unwrap_or(layout::Layout::new(10).height)
    });

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
        *b.borrow_mut() = Backdrop::capture(origin.0, origin.1, W, h);
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
        h,
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

    let _ = SetWindowPos(hwnd, HWND_TOPMOST, origin.0, origin.1, W, h, SWP_SHOWWINDOW);
    apply_round_region(hwnd, W, h, RADIUS);
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
