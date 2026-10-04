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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

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
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect,
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
/// Posted by a second launch to ask the open panel to close. `WM_APP` is the
/// range reserved for application-private window messages, so this cannot
/// collide with anything the system or another program sends.
const WM_TOGGLE_CLOSE: u32 = WM_APP + 1;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// The desktop snapshot, taken once before the window is created. Holding
    /// it for the panel's whole life is what keeps the acrylic from feeding
    /// back on itself across the 1 Hz repaints.
    static BACKDROP: RefCell<Option<Backdrop>> = const { RefCell::new(None) };
    /// Snapshots from the background thread, newest last. `None` until
    /// [`spawn_worker`] runs. Only ever drained, never written to, and only
    /// ever on the message-loop thread.
    static INBOX: RefCell<Option<Receiver<Snapshot>>> = const { RefCell::new(None) };
    /// Set when the window is destroyed, so the worker stops computing into a
    /// channel nobody is reading.
    static STOP: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

/// How often the background thread recomputes. Matches the WM_TIMER period:
/// the timer exists to *draw* the newest snapshot, not to produce one.
const REFRESH_MS: u64 = 1000;

/// One complete set of values for a frame.
///
/// Plain data only — numbers, strings and colours, no handles and no
/// pointers. That is what makes it `Send`, and it is the whole reason the
/// background thread is allowed to exist: nothing here can name a window, a
/// DC or any other piece of per-thread GDI state.
///
/// A GDI HDC is owned by the thread that selected into it, and a window may
/// not be painted from a thread that does not own its message queue. So the
/// split is strict: the worker computes, the message loop paints. Both halves
/// are useless alone.
#[derive(Clone, Debug)]
struct Snapshot {
    panchangam: Panchang,
    theme: Theme,
}

fn take_snapshot() -> Snapshot {
    Snapshot {
        panchangam: Panchang::now(LAT, LON, TZ),
        // Re-read the theme every tick so a switch made while the popup is
        // open shows up without reopening it. This is a file read and a CSS
        // parse, which is a second reason to keep it off the UI thread.
        theme: Theme::load(&styles_path()),
    }
}

/// Start the single background thread that produces [`Snapshot`]s.
///
/// One thread, not a pool: the work is a calendar computation and one small
/// file read once a second. A pool would add handoff overhead to buy nothing,
/// and every extra thread would only widen the window in which a stale frame
/// can be painted.
fn spawn_worker() {
    let (tx, rx) = mpsc::channel::<Snapshot>();
    INBOX.with(|i| *i.borrow_mut() = Some(rx));
    let stop = Arc::new(AtomicBool::new(false));
    STOP.with(|s| *s.borrow_mut() = Some(stop.clone()));
    std::thread::spawn(move || {
        // Deliver one straight away rather than making the first open wait a
        // full second on an empty inbox.
        if tx.send(take_snapshot()).is_err() {
            return;
        }
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(REFRESH_MS));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            // A send error means the panel closed and dropped the receiver,
            // which is the normal way this loop ends.
            if tx.send(take_snapshot()).is_err() {
                break;
            }
        }
    });
}

/// Stop the worker. Called from WM_DESTROY.
///
/// The thread is deliberately not joined. It is parked in a sleep when the
/// flag flips, so it cannot be mid-write to anything the UI thread still owns;
/// joining it would only risk blocking the message loop for up to a second on
/// close, which is exactly the sort of input lag this whole change exists to
/// remove.
fn stop_worker() {
    STOP.with(|s| {
        if let Some(flag) = s.borrow_mut().take() {
            flag.store(true, Ordering::Relaxed);
        }
    });
    INBOX.with(|i| *i.borrow_mut() = None);
}

/// Collapse everything queued into the single newest snapshot.
///
/// After a slow tick the channel can hold more than one, and painting the
/// oldest would be strictly worse than painting nothing — the whole point of
/// the panel is that it shows *now*. Returns `None` when nothing new arrived,
/// which is the signal not to repaint at all.
fn take_latest(rx: &mut Receiver<Snapshot>) -> Option<Snapshot> {
    let mut newest = None;
    loop {
        match rx.try_recv() {
            Ok(s) => newest = Some(s),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
        }
    }
    newest
}

/// Move any fresh snapshot into the app state. Returns whether there was one.
fn drain_inbox() -> bool {
    let mut newest = None;
    INBOX.with(|i| {
        if let Some(rx) = i.borrow_mut().as_mut() {
            newest = take_latest(rx);
        }
    });
    match newest {
        Some(s) => {
            APP.with(|c| {
                if let Some(a) = c.borrow_mut().as_mut() {
                    a.panchangam = s.panchangam;
                    a.theme = s.theme;
                }
            });
            true
        }
        None => false,
    }
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
    // One panel, ever, and the bar chip toggles it.
    //
    // A second launch owns no window, so it cannot destroy the first one's; it
    // posts a message instead and exits. The open panel handles it in its own
    // window procedure, which is the only place destroying the window is safe.
    match yasb_chrome::acquire("Local\\yasb-saka-popup") {
        Ok(None) => {
            // `Action::None` means the previous instance is already gone but
            // has not released the name yet — a race between closing and
            // reopening. Treat the click as a request to open, or it is
            // swallowed and the panel appears to be stuck shut.
            if yasb_chrome::notify_window_of_class("SakaPopupClass", WM_TOGGLE_CLOSE)
                == yasb_chrome::Action::None
            {
                unsafe { run() };
            }
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

/// Width [`tracked_text`] would draw, without drawing it.
///
/// The weekday scrim has to be sized around its label, and the label is drawn
/// with the manual tracking above — which `DrawTextW`'s `DT_CALCRECT` does not
/// model, because the tracking is applied here rather than in GDI. Measuring
/// glyph by glyph the same way is the only way to get a pill that is neither
/// clipping the text nor trailing empty space behind it.
///
/// Requires `font` to already be selected into `dc`.
unsafe fn measure_tracked(dc: HDC, text: &str, tracking: i32) -> i32 {
    let mut total = 0i32;
    let mut buf: Vec<u16> = Vec::with_capacity(2);
    let mut units = [0u16; 2];
    for ch in text.chars() {
        buf.clear();
        buf.extend_from_slice(ch.encode_utf16(&mut units[..]));
        let mut sz = SIZE { cx: 0, cy: 0 };
        let _ = GetTextExtentPoint32W(dc, &buf, &mut sz);
        total += sz.cx + tracking;
    }
    // `tracked_text` adds `tracking` after the final glyph too, so the
    // trailing one is not part of the visible width.
    (total - tracking).max(0)
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
    // Hoisted out of both passes: the shape pass sizes the scrim around this
    // string and the text pass draws it.
    let weekday = app.script.vara(app.panchangam.vara).to_uppercase();

    // ---- shape pass ------------------------------------------------------
    {
        // Measured before the canvas is taken: `dib.canvas()` borrows the DIB
        // mutably for the whole block, so the DC needed to size the text has
        // to be borrowed first and released.
        let weekday_w = {
            let dc = dib.dc();
            let fam = if app.script == Script::Telugu { INDIC_FAMILY } else { TEXT_FAMILY };
            let f = font(-11, 600, fam);
            let saved = SelectObject(dc, HGDIOBJ(f.0));
            let width = measure_tracked(dc, &weekday, 2);
            SelectObject(dc, saved);
            let _ = DeleteObject(HGDIOBJ(f.0));
            width
        };

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

        // The weekday's scrim, drawn before the moon so the glyph's halo can
        // still bleed over it if the two ever approach.
        //
        // This is the only opaque surface in the panel. The weekday label is
        // the only place the accent touches type, and the accent is chosen to
        // sit near the theme's own mid-tone — so on a wallpaper brighter than
        // the theme it loses contrast, and at 85% acrylic the wallpaper is
        // most of what is behind it. `t.scrim` is derived away from the
        // accent's luminance (see `Theme::from_vars`) so the pair holds at
        // both ends of the range without recolouring any theme.
        let (sx0, sy0, sx1, sy1) = l.weekday_pill(weekday_w);
        c.round_rect(sx0, sy0, sx1, sy1, layout::SCRIM_R, t.scrim);

        // Moon disc: a dim body, the lit limb, and a soft halo. It rides at the
        // right of the header, level with the date. No caption: the phase name
        // and the illumination are already the `Moon` row's value and caption.
        let mr = layout::MOON_R as f32;
        let mcx = l.moon_center_x(w) as f32;
        let mcy = ((l.moon_y + l.moon_bottom) / 2) as f32;
        c.radial_glow(mcx, mcy, mr * 2.4, t.accent, t.moon_glow);
        for py in (mcy as i32 - layout::MOON_R - 2)..=(mcy as i32 + layout::MOON_R + 2) {
            for px in (mcx as i32 - layout::MOON_R - 2)..=(mcx as i32 + layout::MOON_R + 2) {
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
            let y = l.row_y(i as i32) + 32;
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
        // touches type. It sits on the opaque scrim drawn in the shape pass.
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.accent));
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

        // The moon glyph carries no caption. An earlier version drew the phase
        // name and the illumination beneath it, which was wrong twice over:
        // both already appear on the `Moon` row, and a caption centred on a
        // 60px disc lands in the middle of the date.

        // Rows: one line of text, then the bar under it.
        //
        // The label is set as tracked caps and the value in the body face, so
        // the two are told apart by weight and case rather than by size alone.
        // The caption is right-aligned to the same edge as the bar ends.
        let cap_w = 96;
        for (i, r) in rows.iter().enumerate() {
            let y = l.row_y(i as i32);

            SelectObject(dc, HGDIOBJ(label.0));
            SetTextColor(dc, colorref(t.faint));
            let caps = r.label.to_uppercase();
            tracked_text(dc, &caps, PAD, y + 13, 1);

            SelectObject(dc, HGDIOBJ(body.0));
            SetTextColor(dc, colorref(t.text));
            let value_x = layout::Layout::VALUE_X;
            let value_w = (w - PAD * 2 - (value_x - PAD) - cap_w).max(40);
            draw_text(
                dc,
                &r.value,
                rect(value_x, y + 7, value_w, 22),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );

            if let Some((_, cap)) = &r.progress {
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.subtext));
                draw_text(
                    dc,
                    cap,
                    rect(w - PAD - cap_w, y + 7, cap_w, 22),
                    DT_RIGHT | DT_VCENTER,
                );
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
        // Blur here, once, rather than on every repaint: the panel repaints
        // at least once a second and the snapshot never changes in between.
        // This is also what makes `backdrop_opacity` of 0.85 survivable —
        // acrylic is a low-opacity *and* heavily blurred material.
        let t = &APP.with(|c| c.borrow().clone()).expect("app alive here").theme;
        *b.borrow_mut() =
            Backdrop::capture(origin.0, origin.1, W, h).map(|bd| bd.blurred(t.backdrop_blur as i32));
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
    // From here on the background thread supplies the values; the first paint
    // below still uses the synchronously-computed ones so the panel is never
    // briefly empty.
    spawn_worker();
    let _ = SetTimer(hwnd, TIMER_ID, REFRESH_MS as u32, None);
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
            // The worker has been producing values while this message loop
            // was busy; collect whatever is waiting and draw it. Painting is
            // skipped when nothing new arrived — there is no animation on this
            // panel, so a repaint with identical inputs is pure GDI churn.
            if drain_inbox() {
                paint(hwnd);
            }
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
            stop_worker();
            let _ = PostQuitMessage(0);
            LRESULT(0)
        }
        // A second click on the bar chip: close this panel.
        WM_TOGGLE_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(n: u32) -> Snapshot {
        Snapshot {
            panchangam: Panchang::now(LAT, LON, TZ),
            theme: Theme::from_vars(&[("--accent".into(), format!("#{n:06x}"))]),
        }
    }

    /// After a slow tick more than one snapshot can be queued, and the panel
    /// has to show the newest. Painting the oldest would mean the popup could
    /// go *backwards* in time after a stall, which is the one failure mode
    /// that makes a clock untrustworthy.
    #[test]
    fn a_backlog_collapses_to_the_newest_snapshot() {
        let (tx, mut rx) = mpsc::channel();
        tx.send(snap(0x111111)).unwrap();
        tx.send(snap(0x222222)).unwrap();
        tx.send(snap(0x333333)).unwrap();
        let got = take_latest(&mut rx).expect("three snapshots queued");
        assert_eq!(
            (got.theme.accent.r, got.theme.accent.g, got.theme.accent.b),
            (0x33, 0x33, 0x33),
            "took an older snapshot than the newest"
        );
    }

    /// Nothing new means nothing to draw. Repainting identical inputs is pure
    /// GDI churn, and on a panel with no animation it is the difference
    /// between 1 Hz of work and 1 Hz of nothing.
    #[test]
    fn an_empty_inbox_reports_nothing_to_draw() {
        let (_tx, mut rx) = mpsc::channel();
        assert!(take_latest(&mut rx).is_none());
    }

    /// Draining must consume, not peek: a second drain with no new work has
    /// to find nothing, or the same snapshot would be painted on every tick
    /// forever.
    #[test]
    fn draining_consumes_the_snapshot() {
        let (tx, mut rx) = mpsc::channel();
        tx.send(snap(0x444444)).unwrap();
        assert!(take_latest(&mut rx).is_some());
        assert!(take_latest(&mut rx).is_none());
    }

    /// A disconnected sender means the worker has gone. That is not an error
    /// to surface — it is how the panel learns to stop asking — so the drain
    /// reports "nothing new" rather than looping or panicking.
    #[test]
    fn a_dead_worker_is_not_an_error() {
        let (tx, mut rx) = mpsc::channel::<Snapshot>();
        tx.send(snap(0x555555)).unwrap();
        drop(tx);
        assert!(take_latest(&mut rx).is_some(), "the queued value is still valid");
        assert!(take_latest(&mut rx).is_none(), "then it reads as empty, forever");
    }
}
