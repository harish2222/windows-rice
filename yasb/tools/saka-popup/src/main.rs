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
mod moon;
mod state;
mod theme;

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
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
use yasb_chrome::gdi::{
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect,
    text_width, tracked_text,
};
use yasb_chrome::{INDIC_FAMILY, Type};

pub fn resolve_saka_family() -> (&'static str, &'static str) {
    if yasb_chrome::font_is_installed("Segoe UI") {
        return ("Segoe UI", "Segoe UI");
    }
    let candidates = [
        ("GohuFont 14 Nerd Font Mono", "GohuFont 11 Nerd Font Mono"),
        ("GohuFont 14 Nerd Font", "GohuFont 11 Nerd Font"),
        ("GohuFont uni14 Nerd Font Mono", "GohuFont uni11 Nerd Font Mono"),
        ("GohuFont uni14 Nerd Font", "GohuFont uni11 Nerd Font"),
    ];
    for (disp, body) in candidates {
        if yasb_chrome::font_is_installed(body) {
            return (disp, body);
        }
    }
    ("Segoe UI", "Segoe UI")
}

// Hyderabad / IST: the same defaults the saka CLI uses.
const LAT: f64 = 17.3850;
const LON: f64 = 78.4867;
const TZ: f64 = 5.5;

const RADIUS: i32 = layout::RADIUS;
const PAD: i32 = layout::PAD;
const TIMER_ID: usize = 1;
const ANIM_TIMER_ID: usize = 2;
const ANIM_MS: u32 = 100;
/// Posted by a second launch to ask the open panel to raise itself.
///
/// `WM_APP` is the range reserved for application-private window messages, so
/// this cannot collide with anything the system or another program sends.
///
/// Named `WM_TOGGLE_CLOSE` historically, when it closed the panel. It raises
/// it now; the name is kept because renaming a message constant buys nothing
/// and would only make the diff harder to read against the old behaviour.
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
    while let Ok(s) = rx.try_recv() {
        newest = Some(s);
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
    mode: layout::LayoutMode,
    rotation_angle: f32,
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
                value: if s == Script::Telugu {
                    if p.waxing { "శుక్ల పక్షం".into() } else { "కృష్ణ పక్షం".into() }
                } else {
                    p.phase_name_western().to_string()
                },
                progress: Some((
                    p.illum,
                    format!(
                        "{}% · {}",
                        (p.illum * 100.0).round() as u32,
                        if p.waxing {
                            if s == Script::Telugu { "వృద్ధి" } else { "waxing" }
                        } else {
                            if s == Script::Telugu { "క్షయ" } else { "waning" }
                        }
                    ),
                )),
            },
            // The current 45° moon sector, displaying the active phase name,
            // progress through the sector, and the exact date/time it ends.
            Row {
                label: "Phase",
                value: p.phase_name_in(s).to_string(),
                progress: Some((
                    p.phase_progress(),
                    if s == Script::Telugu {
                        format!("{} వరకు", p.day_label_short(p.phase_end_jd))
                    } else {
                        format!("ends {}", p.day_label_short(p.phase_end_jd))
                    },
                )),
            },
            // The next 45° sector that will begin immediately when the current phase ends.
            Row {
                label: "Next Phase",
                value: if s == Script::Telugu {
                    format!(
                        "{} · {} నుండి",
                        s.phase((p.phase_index + 1) % 8),
                        p.day_label_short(p.phase_end_jd)
                    )
                } else {
                    format!(
                        "{} · starts {}",
                        s.phase((p.phase_index + 1) % 8),
                        p.day_label_short(p.phase_end_jd)
                    )
                },
                progress: None,
            },
            Row {
                label: "Amanta",
                value: format!(
                    "{} · {} {}",
                    s.month(p.amanta_month),
                    // `Day` is a word, so it localises with the rest of the
                    // column; leaving it English put "భాద్ర · Day 23" in the
                    // middle of a Telugu row.
                    if s == Script::Telugu { "దిన" } else { "Day" },
                    p.amanta_tithi_day
                ),
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
                    "New {} · Full {}",
                    p.day_label_short(p.next_new_moon_jd),
                    p.day_label_short(p.next_full_moon_jd)
                ),
                progress: None,
            },
        ]
    }

    fn layout(&self) -> layout::Layout {
        layout::Layout::build_mode(self.mode, self.rows().len() as i32, self.script == Script::Telugu)
    }

    #[allow(dead_code)]
    fn panchang_rows(&self) -> Vec<Row> {
        let mut r = self.rows();
        r.truncate(4);
        r
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
    let saved_state = state::load_state(&state::state_file_path());
    let script = if args.iter().any(|a| a == "--te") {
        Script::Telugu
    } else if args.iter().any(|a| a == "--en" || a == "--latin") {
        Script::Latin
    } else {
        saved_state.script
    };

    let mut mode = saved_state.mode;
    if args.iter().any(|a| a == "--compact" || a == "-c") {
        mode = layout::LayoutMode::Compact;
    } else if args.iter().any(|a| a == "--landscape" || a == "-l") {
        mode = layout::LayoutMode::Landscape;
    } else if args.iter().any(|a| a == "--portrait" || a == "-p") {
        mode = layout::LayoutMode::Portrait;
    }

    if args.iter().any(|a| a == "--smoke") {
        smoke(&theme, &panchangam, script, mode);
        return;
    }

    state::save_state(&state::state_file_path(), &state::AppState { mode, script });

    APP.with(|c| *c.borrow_mut() = Some(App { theme, panchangam, script, mode, rotation_angle: 0.0, origin: (0, 0) }));
    // One panel, ever, and the bar chip toggles it.
    //
    // A second launch owns no window, so it cannot destroy the first one's; it
    // posts a message instead and exits. The open panel handles it in its own
    // window procedure, which is the only place destroying the window is safe.
    match yasb_chrome::acquire("Local\\yasb-saka-popup") {
        Ok(None) => {
            // The panel is already open. Ask it to come to the front and stop; it
            // is never re-created, so a second click cannot produce a second
            // panel or make the first one flicker.
            //
            // `Action::None` means the previous instance has already gone but
            // has not released the name yet — a race between closing and
            // reopening. Treat the click as a request to open, or it is
            // swallowed and the panel appears to be stuck shut.
            if yasb_chrome::notify_window_of_class("SakaPopupClass", WM_TOGGLE_CLOSE)
                == yasb_chrome::Action::None
            {
                unsafe { run() };
            }
        }
        // If the guard itself fails, still open the panel.
        Err(e) => eprintln!("saka-popup: single-instance guard unavailable: {e}"),
        Ok(Some(_instance)) => unsafe { run() },
    }
}

/// Headless self-test: proves theme parsing and layout work with no window, so
/// a broken build is caught without opening a window on someone's desktop.
fn smoke(t: &Theme, p: &Panchang, script: Script, mode: layout::LayoutMode) {
    let app = App { theme: t.clone(), panchangam: p.clone(), script, mode, rotation_angle: 0.0, origin: (0, 0) };
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

    // Fit check.
    //
    // `paint` hands the value `DT_END_ELLIPSIS`, so a row that does not fit
    // shows up on screen as a trailing "..." and nowhere else — no test, no
    // log, no exit code. Measuring the shipped strings with the same faces
    // and the same `row_split` the paint pass uses is what makes a truncating
    // row fail here instead of on someone's desktop.
    let (_, gohu_body) = resolve_saka_family();
    let text_family: &str =
        if script == Script::Telugu { INDIC_FAMILY } else { gohu_body };
    let mut bad: Vec<String> = Vec::new();
    unsafe {
        let dc = GetDC(None);
        let body = font(Type::Lead.gdi(), 500, text_family);
        let small = font(Type::Meta.gdi(), 400, text_family);
        for r in app.rows() {
            SelectObject(dc, HGDIOBJ(body.0));
            let value_need = text_width(dc, &r.value);
            let cap_need = r.progress.as_ref().map(|(_, c)| {
                SelectObject(dc, HGDIOBJ(small.0));
                text_width(dc, c)
            });
            let (value_w, cap_w) = layout::Layout::row_split(l.w(), value_need, cap_need);
            println!(
                "  fit {:<11} value {value_need:>4}px in {value_w:<4} caption {}",
                r.label,
                cap_need.map(|n| format!("{n:>4}px in {cap_w}")).unwrap_or("-- none --".into())
            );
            if value_w < value_need {
                bad.push(format!("{}'s value {value_need}px > {value_w}px", r.label));
            }
            if let Some(n) = cap_need {
                if cap_w < n {
                    bad.push(format!("{}'s caption {n}px > {cap_w}px", r.label));
                }
            }
        }
        let _ = ReleaseDC(None, dc);
        let _ = DeleteObject(HGDIOBJ(body.0));
        let _ = DeleteObject(HGDIOBJ(small.0));
    }
    if !bad.is_empty() {
        panic!("rows that need an ellipsis: {}", bad.join("; "));
    }
    println!("smoke: ok ({} rows, panel {}x{})", app.rows().len(), l.w(), l.height);
}

/// Width [`tracked_text`] would draw, without drawing it.
///
/// The weekday scrim has to be sized around its label, and the label is drawn
/// with letter spacing, so the measurement has to use the same spacing or the
/// pill clips the text or trails empty space behind it.
///
/// Both this and [`yasb_chrome::gdi::tracked_text`] set the extra on the DC and
/// let GDI measure, which is what makes them agree: the extra is applied
/// *between* characters, so the extent that comes back has no trailing gap.
/// This used to sum per-glyph `GetTextExtentPoint32W` results, which rounds
/// each advance to a whole pixel and so does not match what the draw call
/// actually puts on screen.
///
/// Requires `font` to already be selected into `dc`.
unsafe fn measure_tracked(dc: HDC, text: &str, tracking: i32) -> i32 {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    buf.push(0);
    let mut sz = SIZE { cx: 0, cy: 0 };
    unsafe {
        if tracking != 0 {
            SetTextCharacterExtra(dc, tracking);
        }
        let _ = GetTextExtentPoint32W(dc, &buf, &mut sz);
        if tracking != 0 {
            SetTextCharacterExtra(dc, 0);
        }
    }
    sz.cx
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
            let (_, gohu_body) = resolve_saka_family();
            let fam = if app.script == Script::Telugu { INDIC_FAMILY } else { gohu_body };
            let f = font(-11, 600, fam);
            let saved = SelectObject(dc, HGDIOBJ(f.0));
            let width = measure_tracked(dc, &weekday, 2);
            SelectObject(dc, saved);
            let _ = DeleteObject(HGDIOBJ(f.0));
            width
        };

        let mut c = dib.canvas();
        // Base fill.
        c.clear(Rgba::rgb(16, 18, 22));

        BACKDROP.with(|b| {
            if let Some(bd) = b.borrow().as_ref() {
                bd.draw_under(&mut c, 0, 0, t.backdrop_opacity);
            }
        });

        // Light from the top: a gradient, then the accent bloom that gives the
        // header its lift.
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);
        c.radial_glow(w as f32 * 0.22, -10.0, w as f32 * 0.95, t.apple_accent, t.bloom);

        // Frame: subtle neutral 1px border (#383D47), NO bright red outline!
        c.round_rect_border(1, 1, w - 1, h - 1, RADIUS, 1, t.subtle_border);
        c.round_rect_border(1, 1, w - 1, 14, RADIUS, 1, t.highlight);

        let (sx0, sy0, sx1, sy1) = l.weekday_pill(weekday_w);
        c.round_rect(sx0, sy0, sx1, sy1, layout::SCRIM_R, t.scrim);

        // 3D Moon sphere with soft atmospheric glow
        let mr = l.moon_r as f32;
        let mcx = l.moon_cx as f32;
        let mcy = l.moon_cy as f32;
        c.radial_glow(mcx, mcy, mr * 2.2, t.apple_accent, t.moon_glow);
        moon::draw_3d_moon(
            &mut c,
            mcx,
            mcy,
            mr,
            app.panchangam.illum as f32,
            app.panchangam.waxing,
            app.rotation_angle,
            t.apple_accent,
        );

        match app.mode {
            layout::LayoutMode::Portrait => {
                // Sunrise / Sunset timeline
                let tx0 = PAD + 64;
                let tx1 = w - PAD - 64;
                let ty = l.timeline_y + 10;
                c.round_rect(tx0, ty, tx1, ty + 2, 1, t.track);
                if let (Some(sr), Some(ss)) = (app.panchangam.sunrise_jd, app.panchangam.sunset_jd) {
                    let now = app.panchangam.jd;
                    if now >= sr && now <= ss && ss > sr {
                        let frac = ((now - sr) / (ss - sr)).clamp(0.0, 1.0) as f32;
                        let pip_x = tx0 as f32 + frac * (tx1 - tx0) as f32;
                        c.round_rect(tx0, ty, pip_x as i32, ty + 2, 1, t.solar);
                        c.radial_glow(pip_x, ty as f32 + 1.0, 7.0, t.solar, 0.50);
                        c.round_rect((pip_x - 3.0) as i32, ty - 2, (pip_x + 3.0) as i32, ty + 4, 3, t.solar);
                    }
                }

                // Section 1 hairline divider
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);

                // Card 1: Today's Panchang
                c.round_rect(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, t.card);
                c.round_rect_border(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, 1, t.hairline);

                // Progress indicators for rows 0..4
                let (tx, tw, th) = l.track(w);
                for i in 0..4 {
                    if let Some(r) = rows.get(i) {
                        if let Some((frac, _)) = &r.progress {
                            let y = l.row_y(i as i32) + l.indicator_y();
                            c.round_rect(tx, y, tx + tw, y + th, 1, t.track);
                            let fw = ((frac.clamp(0.0, 1.0)) * tw as f64).round() as i32;
                            if fw > 0 {
                                c.round_rect(tx, y, tx + fw, y + th, 1, t.apple_accent);
                            }
                        }
                    }
                }

                // Card 2: Moon & Calendar 2x2 grid tiles
                for idx in 0..4 {
                    let (x0, y0, x1, y1) = l.tile_rect(idx);
                    c.round_rect(x0, y0, x1, y1, 12, t.surface_elevated);
                    c.round_rect_border(x0, y0, x1, y1, 12, 1, t.hairline);
                }
            }
            layout::LayoutMode::Landscape => {
                // Sunrise / Sunset timeline under date on left
                let tx0 = PAD + 54;
                let tx1 = PAD + 210 - 54;
                let ty = l.timeline_y + 10;
                c.round_rect(tx0, ty, tx1, ty + 2, 1, t.track);
                if let (Some(sr), Some(ss)) = (app.panchangam.sunrise_jd, app.panchangam.sunset_jd) {
                    let now = app.panchangam.jd;
                    if now >= sr && now <= ss && ss > sr {
                        let frac = ((now - sr) / (ss - sr)).clamp(0.0, 1.0) as f32;
                        let pip_x = tx0 as f32 + frac * (tx1 - tx0) as f32;
                        c.round_rect(tx0, ty, pip_x as i32, ty + 2, 1, t.solar);
                        c.radial_glow(pip_x, ty as f32 + 1.0, 6.0, t.solar, 0.45);
                        c.round_rect((pip_x - 3.0) as i32, ty - 2, (pip_x + 3.0) as i32, ty + 4, 3, t.solar);
                    }
                }

                // Right column: Today's Panchang card
                let right_card_x0 = 420;
                let right_card_x1 = w - PAD;
                c.round_rect(right_card_x0, l.panchang_card_y, right_card_x1, l.panchang_card_bottom, layout::CARD_R, t.card);
                c.round_rect_border(right_card_x0, l.panchang_card_y, right_card_x1, l.panchang_card_bottom, layout::CARD_R, 1, t.hairline);

                let track_x0 = right_card_x0 + 85;
                let track_w = right_card_x1 - 14 - track_x0;
                let th = 2;
                for i in 0..4 {
                    if let Some(r) = rows.get(i) {
                        if let Some((frac, _)) = &r.progress {
                            let y = l.row_y(i as i32) + l.indicator_y();
                            c.round_rect(track_x0, y, track_x0 + track_w, y + th, 1, t.track);
                            let fw = ((frac.clamp(0.0, 1.0)) * track_w as f64).round() as i32;
                            if fw > 0 {
                                c.round_rect(track_x0, y, track_x0 + fw, y + th, 1, t.apple_accent);
                            }
                        }
                    }
                }

                // Hairline divider below top deck
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);

                // Bottom deck: 4 horizontal tiles
                for idx in 0..4 {
                    let (x0, y0, x1, y1) = l.tile_rect(idx);
                    c.round_rect(x0, y0, x1, y1, 10, t.surface_elevated);
                    c.round_rect_border(x0, y0, x1, y1, 10, 1, t.hairline);
                }
            }
            layout::LayoutMode::Compact => {
                let tx0 = PAD + 56;
                let tx1 = w - PAD - 56;
                let ty = l.timeline_y + 9;
                c.round_rect(tx0, ty, tx1, ty + 2, 1, t.track);
                if let (Some(sr), Some(ss)) = (app.panchangam.sunrise_jd, app.panchangam.sunset_jd) {
                    let now = app.panchangam.jd;
                    if now >= sr && now <= ss && ss > sr {
                        let frac = ((now - sr) / (ss - sr)).clamp(0.0, 1.0) as f32;
                        let pip_x = tx0 as f32 + frac * (tx1 - tx0) as f32;
                        c.round_rect(tx0, ty, pip_x as i32, ty + 2, 1, t.solar);
                        c.radial_glow(pip_x, ty as f32 + 1.0, 5.0, t.solar, 0.40);
                        c.round_rect((pip_x - 3.0) as i32, ty - 2, (pip_x + 3.0) as i32, ty + 4, 3, t.solar);
                    }
                }
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);
            }
        }
    }

    // ---- text pass -------------------------------------------------------
    {
        let dc = dib.dc();
        // The bar's own font, so the panel and the bar it hangs from are one
        // typeface. Telugu switches the whole panel to Nirmala UI: a row's
        // label and value are read as a pair, and setting one in a Latin face
        // and the other in a Brahmic one looks broken even when both are
        // individually correct.
        let telugu = app.script == Script::Telugu;
        let (gohu_disp, gohu_body) = resolve_saka_family();
        let (display_family, text_family) = if telugu {
            (INDIC_FAMILY, INDIC_FAMILY)
        } else {
            (gohu_disp, gohu_body)
        };
        let display = font(Type::Title.gdi(), 600, display_family);
        let body = font(Type::Lead.gdi(), 500, text_family);
        let small = font(Type::Meta.gdi(), 400, text_family);
        let label = font(Type::Meta.gdi(), 600, text_family);
        let section = font(Type::Meta.gdi(), 700, text_family);
        SetBkMode(dc, TRANSPARENT);

        // Weekday, small, tracked, in the accent — the one place the accent
        // touches type. It sits on the opaque scrim drawn in the shape pass.
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.accent));
        tracked_text(dc, &weekday, PAD, l.weekday_text_top(), 2);

        // The date, large. This line carries the panel's whole hierarchy.
        let p = &app.panchangam;
        let s = app.script;
        SelectObject(dc, HGDIOBJ(display.0));
        SetTextColor(dc, colorref(t.text));
        let title = format!("{} {} {}", p.day, month_abbr(p.month), p.year);
        draw_text(dc, &title, rect(PAD, l.date_y, w - PAD * 2 - 80, 36), DT_LEFT | DT_VCENTER);

        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        let sub = format!(
            "{} {} · Saka {} · VS {}",
            s.month(p.saka_month),
            p.saka_day,
            p.saka_year,
            p.vikram_year
        );
        draw_text(
            dc,
            &sub,
            rect(PAD, l.era_y, l.era_right(w) - PAD, 18),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        match app.mode {
            layout::LayoutMode::Portrait => {
                // Sunrise / Sunset labels
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(PAD, l.timeline_y + 1, 60, 20), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(w - PAD - 60, l.timeline_y + 1, 60, 20), DT_RIGHT | DT_VCENTER);

                // Section 1 Header: TODAY'S PANCHANG
                SelectObject(dc, HGDIOBJ(section.0));
                SetTextColor(dc, colorref(t.apple_accent));
                tracked_text(dc, if telugu { "నేటి పంచాంగం" } else { "TODAY'S PANCHANG" }, PAD, l.rule_y + 10, 1);

                // Rows 0..4 (Tithi, Nakshatra, Yoga, Karana)
                for i in 0..4 {
                    if let Some(r) = rows.get(i) {
                        let y = l.row_y(i as i32);

                        SelectObject(dc, HGDIOBJ(label.0));
                        SetTextColor(dc, colorref(t.faint));
                        let caps = r.label.to_uppercase();
                        tracked_text(dc, &caps, PAD + 14, y + 6, 1);

                        let value_x = layout::Layout::VALUE_X;
                        SelectObject(dc, HGDIOBJ(body.0));
                        let value_need = text_width(dc, &r.value);
                        let cap_need = r.progress.as_ref().map(|(_, c)| {
                            SelectObject(dc, HGDIOBJ(small.0));
                            text_width(dc, c)
                        });
                        let (value_w, cap_w) = layout::Layout::row_split(w, value_need, cap_need);

                        SelectObject(dc, HGDIOBJ(body.0));
                        SetTextColor(dc, colorref(t.text));
                        draw_text(
                            dc,
                            &r.value,
                            rect(value_x, y + 2, value_w, l.text_h()),
                            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                        );

                        if cap_w > 0 {
                            if let Some((_, cap)) = &r.progress {
                                SelectObject(dc, HGDIOBJ(small.0));
                                SetTextColor(dc, colorref(t.subtext));
                                draw_text(
                                    dc,
                                    cap,
                                    rect(w - PAD - 14 - cap_w, y + 2, cap_w, l.text_h()),
                                    DT_LEFT | DT_VCENTER,
                                );
                            }
                        }
                    }
                }

                // Section 2 Header: MOON & CALENDAR
                SelectObject(dc, HGDIOBJ(section.0));
                SetTextColor(dc, colorref(t.apple_accent));
                tracked_text(dc, if telugu { "చంద్రుడు & క్యాలెండర్" } else { "MOON & CALENDAR" }, PAD, l.panchang_card_bottom + 10, 1);

                // 2x2 Grid Tile texts
                paint_tiles(dc, &l, t, p, s, telugu, label, body, small);
            }
            layout::LayoutMode::Landscape => {
                // Sunrise / Sunset labels
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(PAD, l.timeline_y + 1, 50, 20), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(PAD + 210 - 50, l.timeline_y + 1, 50, 20), DT_RIGHT | DT_VCENTER);

                // Moon illumination caption under 3D moon
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.subtext));
                let illum_str = format!("{}% · {}", (p.illum * 100.0).round() as u32, if p.waxing { "Waxing" } else { "Waning" });
                draw_text(dc, &illum_str, rect(l.moon_cx - 65, l.moon_cy + l.moon_r + 6, 130, 16), DT_CENTER | DT_VCENTER);

                // Right column rows (Tithi, Nakshatra, Yoga, Karana)
                let right_card_x0 = 420;
                let right_card_x1 = w - PAD;
                for i in 0..4 {
                    if let Some(r) = rows.get(i) {
                        let y = l.row_y(i as i32);
                        SelectObject(dc, HGDIOBJ(label.0));
                        SetTextColor(dc, colorref(t.faint));
                        let caps = r.label.to_uppercase();
                        tracked_text(dc, &caps, right_card_x0 + 12, y + 6, 1);

                        let val_x = right_card_x0 + 85;
                        SelectObject(dc, HGDIOBJ(body.0));
                        SetTextColor(dc, colorref(t.text));
                        draw_text(
                            dc,
                            &r.value,
                            rect(val_x, y + 2, right_card_x1 - val_x - 12, l.text_h()),
                            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                        );
                    }
                }

                // Bottom deck: 4 horizontal tiles
                paint_tiles(dc, &l, t, p, s, telugu, label, body, small);
            }
            layout::LayoutMode::Compact => {
                // Tithi and Illumination summary row
                SelectObject(dc, HGDIOBJ(body.0));
                SetTextColor(dc, colorref(t.text));
                let tithi_str = format!(
                    "{} · {:.0}% {}",
                    rows.first().map(|r| r.value.as_str()).unwrap_or(""),
                    p.illum * 100.0,
                    if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }
                );
                draw_text(dc, &tithi_str, rect(PAD, l.era_bottom + 3, w - PAD * 2 - 50, 18), DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS);

                // Sunrise / Sunset labels
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(PAD, l.timeline_y, 52, 18), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(w - PAD - 52, l.timeline_y, 52, 18), DT_RIGHT | DT_VCENTER);

                // Highlights row
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.subtext));
                let high_str = format!(
                    "Nakshatra: {} · Yoga: {}",
                    s.nakshatra(p.nakshatra.index),
                    s.yoga(p.yoga.index)
                );
                draw_text(dc, &high_str, rect(PAD, l.rule_y + 4, w - PAD * 2, 16), DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS);
            }
        }

        // Footer hint.
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.faint));
        draw_text(
            dc,
            if app.mode == layout::LayoutMode::Compact { "m: Mode" } else { "Tab: Telugu / English · m: Mode" },
            rect(PAD, l.footer_y, 250, 18),
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
        let _ = DeleteObject(HGDIOBJ(section.0));
    }

    blit_to_window(hwnd, &dib);
}

unsafe fn paint_tiles(
    dc: HDC,
    l: &layout::Layout,
    t: &Theme,
    p: &Panchang,
    s: Script,
    telugu: bool,
    label: HFONT,
    body: HFONT,
    small: HFONT,
) {
    let tiles = [
        (
            if telugu { "చంద్ర కాంతి" } else { "ILLUMINATION" },
            format!("{}% · {}", (p.illum * 100.0).round() as u32, if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }),
            None,
        ),
        (
            if telugu { "అమాంత మాసం" } else { "AMANTA" },
            format!("{} · {} {}", s.month(p.amanta_month), if telugu { "దిన" } else { "Day" }, p.amanta_tithi_day),
            None,
        ),
        (
            if telugu { "దశ మార్పు" } else { "PHASE TRANSITIONS" },
            p.phase_name_in(s).to_string(),
            Some(if telugu { format!("{} వరకు", p.day_label_short(p.phase_end_jd)) } else { format!("ends {}", p.day_label_short(p.phase_end_jd)) }),
        ),
        (
            if telugu { "పూర్ణిమాంత మాసం" } else { "PURNIMANTA" },
            s.month(p.purnimanta_month).to_string(),
            Some(format!("New {} · Full {}", p.day_label_short(p.next_new_moon_jd), p.day_label_short(p.next_full_moon_jd))),
        ),
    ];

    for (idx, (head, val, sub)) in tiles.iter().enumerate() {
        let (x0, y0, x1, y1) = l.tile_rect(idx);
        let tw = x1 - x0;
        let pad_x = 12;

        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.subtext));
        draw_text(
            dc,
            head,
            rect(x0 + pad_x, y0 + 6, tw - pad_x * 2, 14),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        SelectObject(dc, HGDIOBJ(body.0));
        SetTextColor(dc, colorref(t.text));
        draw_text(
            dc,
            val,
            rect(x0 + pad_x, y0 + 22, tw - pad_x * 2, 18),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        if let Some(sub_text) = sub {
            if y1 - y0 >= 52 {
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.faint));
                draw_text(
                    dc,
                    sub_text,
                    rect(x0 + pad_x, y0 + 40, tw - pad_x * 2, 14),
                    DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
        }
    }
}

fn month_abbr(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        [(m as usize - 1).min(11)]
}

/// Where the panel sits: horizontally centred on the monitor holding the
/// cursor, 40px below its top edge — but never so low that the bottom runs off
/// the screen.
///
/// The clamp is the reason this is a function. The Indic panel is 100px taller
/// than the Latin one, so "monitor top + 40" is fine for one script and clips
/// the footer on a short work area for the other.
fn panel_origin(w: i32, h: i32) -> (i32, i32) {
    let mut pt = POINT { x: 0, y: 0 };
    let _ = unsafe { GetCursorPos(&mut pt) };
    let mon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { let _ = GetMonitorInfoW(mon, &mut mi); };

    let left = mi.rcMonitor.left;
    let right = mi.rcMonitor.right;
    let top = mi.rcMonitor.top;
    let bottom = mi.rcMonitor.bottom;
    // `rcWork` excludes the taskbar and the bar itself; falling back to
    // `rcMonitor` only matters on a degenerate report, where staying on screen
    // is still better than trusting a zero-height work area.
    let (work_top, work_bottom) =
        if mi.rcWork.bottom > mi.rcWork.top { (mi.rcWork.top, mi.rcWork.bottom) } else { (top, bottom) };

    let x = (left + right) / 2 - w / 2;
    let mut y = work_top + 40;
    if y + h > work_bottom {
        y = work_bottom - h;
    }
    (x, y.max(work_top))
}

/// Resize the window to whatever the current script's layout needs, move it if
/// the new size no longer fits, re-snapshot the backdrop at the new size, and
/// repaint.
///
/// The backdrop has to be re-captured, not just reused: it is a bitmap of the
/// exact rectangle the panel occupies, so at the old size it no longer covers
/// the window and the acrylic would show unblurred desktop along the bottom.
unsafe fn resize_to_script(hwnd: HWND) {
    let Some(app) = APP.with(|c| c.borrow().clone()) else { return };
    let l = app.layout();
    let (w, h) = (l.w(), l.height);

    let (x, y) = panel_origin(w, h);
    APP.with(|c| {
        if let Some(a) = c.borrow_mut().as_mut() {
            a.origin = (x, y);
        }
    });
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_SHOWWINDOW);
    apply_round_region(hwnd, w, h, RADIUS);

    let t = &app.theme;
    BACKDROP.with(|b| {
        *b.borrow_mut() = Backdrop::capture(x, y, w, h).map(|bd| bd.blurred(t.backdrop_blur as i32));
    });
    paint(hwnd);
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();

    // Height comes from the layout, so the window can never disagree with its
    // own contents.
    let (h, w) = APP.with(|c| {
        c.borrow()
            .as_ref()
            .map(|a| {
                let l = a.layout();
                (l.height, l.w())
            })
            .unwrap_or((layout::Layout::new(10).height, layout::W))
    });

    // Centre on the monitor holding the cursor, just below the bar, clamped to
    // the work area so a tall script cannot hang off the bottom. The position is
    // decided *before* the window exists so the desktop can be captured
    // underneath it.
    let origin = panel_origin(w, h);
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
            Backdrop::capture(origin.0, origin.1, w, h).map(|bd| bd.blurred(t.backdrop_blur as i32));
    });

    let cls: Vec<u16> = "SakaPopupClass\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(hinst.0),
        lpszClassName: windows::core::PCWSTR(cls.as_ptr()),
        // Set as well as handled in `WM_SETCURSOR`: the class cursor is what
        // applies before the first `WM_SETCURSOR` arrives, and a null one is
        // how the hourglass from the opening screen capture survives.
        hCursor: yasb_chrome::gdi::arrow_cursor(),
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
        w,
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

    let _ = SetWindowPos(hwnd, HWND_TOPMOST, origin.0, origin.1, w, h, SWP_SHOWWINDOW);
    apply_round_region(hwnd, w, h, RADIUS);
    let _ = SetForegroundWindow(hwnd);
    // From here on the background thread supplies the values; the first paint
    // below still uses the synchronously-computed ones so the panel is never
    // briefly empty.
    spawn_worker();
    let _ = SetTimer(hwnd, TIMER_ID, REFRESH_MS as u32, None);
    let _ = SetTimer(hwnd, ANIM_TIMER_ID, ANIM_MS, None);
    paint(hwnd);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Windows sends this every time the pointer moves onto the window and
        // expects the window to say what the cursor should be.
        //
        // Without this the class cursor is null, so the system keeps whatever
        // it was showing — and what it was showing is the hourglass, because
        // opening this panel does a full-screen `BitBlt` to snapshot the
        // backdrop. That is a long operation; the cursor goes busy for it; and
        // with nothing to reset it on hover the hourglass just stays there for
        // the life of the panel. Setting the arrow here is what makes the
        // cursor correct again the instant the pointer enters.
        WM_SETCURSOR => {
            let _ = SetCursor(yasb_chrome::gdi::arrow_cursor());
            LRESULT(1)
        }
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
            // was busy; collect whatever is waiting and draw it.
            if drain_inbox() {
                paint(hwnd);
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == ANIM_TIMER_ID => {
            APP.with(|c| {
                if let Some(a) = c.borrow_mut().as_mut() {
                    a.rotation_angle = (a.rotation_angle + 0.04) % (2.0 * std::f32::consts::PI);
                }
            });
            paint(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            match wp.0 as u16 {
                k if k == VK_ESCAPE.0 => {
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_TAB.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.script = if a.script == Script::Telugu {
                                Script::Latin
                            } else {
                                Script::Telugu
                            };
                            state::save_state(&state::state_file_path(), &state::AppState { mode: a.mode, script: a.script });
                        }
                    });
                    // Switching script changes the layout, and the layout owns
                    // the window's size. Repainting alone left the taller Indic
                    // layout painting into the shorter Latin window, so the
                    // last rows were drawn outside the window and clipped away.
                    resize_to_script(hwnd);
                }
                0x4D => {
                    // 'M' or 'm' toggles layout modes: Portrait -> Landscape -> Compact
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.mode = match a.mode {
                                layout::LayoutMode::Portrait => layout::LayoutMode::Landscape,
                                layout::LayoutMode::Landscape => layout::LayoutMode::Compact,
                                layout::LayoutMode::Compact => layout::LayoutMode::Portrait,
                            };
                            state::save_state(&state::state_file_path(), &state::AppState { mode: a.mode, script: a.script });
                        }
                    });
                    resize_to_script(hwnd);
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
            let _ = KillTimer(hwnd, ANIM_TIMER_ID);
            stop_worker();
            PostQuitMessage(0);
            LRESULT(0)
        }
        // A second click on the bar chip while this panel is already up.
        //
        // It raises the panel and does nothing else.
        //
        // This used to *toggle*: a second click destroyed the window, and a
        // third opened it again. That is a small thing to want and an
        // annoying thing to live with — the panel is a glanceable readout, so
        // clicking the chip twice in a row (which is what a double-click is)
        // used to make it vanish and come back, and a stray click while
        // reaching for something else closed it outright.
        //
        // Now the panel is opened by clicking the chip and dismissed by
        // Escape or by clicking the panel itself. Re-clicking the chip brings
        // it back to the front if it has been buried by another window, which
        // is the only thing a second click usefully needs to do.
        WM_TOGGLE_CLOSE => {
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            let _ = SetForegroundWindow(hwnd);
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

    /// The window is sized from the layout, so switching script has to change
    /// the size `paint` draws into.
    ///
    /// This is the defect behind the clipped Telugu panel: `run()` computed
    /// `w`/`h` once from the launch-time script, `VK_TAB` only flipped the
    /// script and repainted, and `paint` reads its canvas from the *window
    /// rect*. So the Indic layout painted 657px of rows into a 557px window and
    /// the last rows were clipped away. Asserting the two layouts differ is
    /// weak on its own; what matters is that a single size cannot serve both,
    /// which is exactly the case that used to break.
    #[test]
    fn the_two_scripts_need_different_window_sizes() {
        let latin = layout::Layout::new(10);
        let indic = layout::Layout::new_indic(10);
        assert_ne!(
            (latin.w(), latin.height),
            (indic.w(), indic.height),
            "one window size cannot serve both scripts"
        );
        assert!(
            indic.height > latin.height,
            "the Indic layout is the taller one, so it is the one that clips"
        );
    }

    /// The panel must fit the work area at either size.
    ///
    /// Without the clamp, `monitor top + 40` plus the taller Indic panel puts
    /// the footer below the bottom of a short work area. A 1080p screen has
    /// 1048px to play with, which both sizes fit, so the test pins the
    /// arithmetic rather than the current monitor.
    #[test]
    fn both_scripts_fit_a_work_area_that_can_just_hold_them() {
        for (name, l) in [("latin", layout::Layout::new(10)), ("indic", layout::Layout::new_indic(10))] {
            let work_h = 700;
            let y = work_h - l.height;
            assert!(
                y >= 0,
                "{name} panel of {}px cannot fit a {work_h}px work area",
                l.height
            );
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
