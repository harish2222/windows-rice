// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: YASB saka-popup Module (main)
// ============================================================================
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
                    if p.karana_first_half {
                        if s == Script::Telugu { "మొదటి సగం".into() } else { "first half".into() }
                    } else {
                        if s == Script::Telugu { "రెండవ సగం".into() } else { "second half".into() }
                    },
                )),
            },
            Row {
                label: "Vara",
                value: if s == Script::Telugu {
                    let lords = ["సూర్యుడు", "చంద్రుడు", "కుజుడు", "బుధుడు", "గురుడు", "శుక్రుడు", "శని"];
                    format!("{} · అధిపతి: {}", s.vara(p.vara), lords[p.vara % 7])
                } else {
                    let lords = ["Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn"];
                    format!("{} · Lord: {}", s.vara(p.vara), lords[p.vara % 7])
                },
                progress: None,
            },
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

    if let Some(pos) = args.iter().position(|a| a == "--render" || a == "--snapshot") {
        if let Some(out_path) = args.get(pos + 1) {
            let app = App { theme, panchangam, script, mode, rotation_angle: 0.0, origin: (0, 0) };
            let l = app.layout();
            let mut dib = Dib::new(l.w(), l.height);
            unsafe { render_into_dib(&app, &mut dib) };
            if let Err(e) = save_dib_bmp(&mut dib, std::path::Path::new(out_path)) {
                eprintln!("failed to save snapshot {out_path}: {e}");
                std::process::exit(1);
            }
            println!("snapshot saved: {out_path} ({}x{})", l.w(), l.height);
            return;
        }
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

/// Paint the whole panel into a DIB: shapes into canvas, then text through GDI.
unsafe fn render_into_dib(app: &App, dib: &mut Dib) {
    let w = dib.w;
    let h = dib.h;
    let t = &app.theme;
    let rows = app.rows();
    let l = app.layout();
    let weekday = app.script.vara(app.panchangam.vara).to_uppercase();

    // ---- shape pass ------------------------------------------------------
    {
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

        // Light from the top: a vertical gradient, then the theme accent bloom that gives the
        // header its atmospheric lift.
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);
        c.radial_glow(w as f32 * 0.22, -10.0, w as f32 * 0.95, t.accent, t.bloom);

        // Frame: subtle theme-aligned 1px border and top highlight rule
        c.round_rect_border(1, 1, w - 1, h - 1, RADIUS, 1, t.subtle_border);
        c.round_rect_border(1, 1, w - 1, 14, RADIUS, 1, t.highlight);

        let (sx0, sy0, sx1, sy1) = l.weekday_pill(weekday_w);
        c.round_rect(sx0, sy0, sx1, sy1, layout::SCRIM_R, t.scrim);

        let mr = l.moon_r as f32;
        let mcx1 = l.moon_cx as f32;
        let mcx2 = l.sky_moon_cx as f32;
        let mcy = l.moon_cy as f32;

        match app.mode {
            layout::LayoutMode::Portrait => {
                // Dual Moons: 3D rotating moon + sky phase moon beside it
                c.radial_glow(mcx1, mcy, mr * 1.9, t.accent, t.moon_glow * 0.7);
                c.radial_glow(mcx2, mcy, mr * 1.9, t.accent, t.moon_glow);

                // Top micro-tags above each moon with ample clearance
                c.round_rect(mcx1 as i32 - 58, l.moon_y - 22, mcx1 as i32 + 58, l.moon_y - 6, 6, t.surface_elevated);
                c.round_rect_border(mcx1 as i32 - 58, l.moon_y - 22, mcx1 as i32 + 58, l.moon_y - 6, 6, 1, t.hairline);
                c.round_rect(mcx2 as i32 - 58, l.moon_y - 22, mcx2 as i32 + 58, l.moon_y - 6, 6, t.surface_elevated);
                c.round_rect_border(mcx2 as i32 - 58, l.moon_y - 22, mcx2 as i32 + 58, l.moon_y - 6, 6, 1, t.hairline);

                // 1. 3D Rotating Moon
                moon::draw_3d_moon(
                    &mut c,
                    mcx1,
                    mcy,
                    mr,
                    app.panchangam.illum as f32,
                    app.panchangam.waxing,
                    app.rotation_angle,
                    t.accent,
                );

                // 2. Telescopic Astronomical Sky Phase Moon
                moon::draw_sky_moon(
                    &mut c,
                    mcx2,
                    mcy,
                    mr,
                    app.panchangam.illum as f32,
                    app.panchangam.waxing,
                    t.accent,
                );

                // Phase badge pill beneath both moons with >= 10px downward padding
                let badge_x0 = mcx1 as i32 - mr as i32 - 6;
                let badge_x1 = mcx2 as i32 + mr as i32 + 6;
                let badge_y0 = l.moon_bottom + 10;
                let badge_y1 = badge_y0 + 24;
                c.round_rect(badge_x0, badge_y0, badge_x1, badge_y1, 8, t.card);
                c.round_rect_border(badge_x0, badge_y0, badge_x1, badge_y1, 8, 1, t.hairline);

                // Sunrise / Sunset timeline
                let tx0 = PAD + 76;
                let tx1 = w - PAD - 76;
                let ty = l.timeline_y + 13;
                c.round_rect(tx0, ty, tx1, ty + 2, 1, t.track);
                if let (Some(sr), Some(ss)) = (app.panchangam.sunrise_jd, app.panchangam.sunset_jd) {
                    let now = app.panchangam.jd;
                    if now >= sr && now <= ss && ss > sr {
                        let frac = ((now - sr) / (ss - sr)).clamp(0.0, 1.0) as f32;
                        let pip_x = tx0 as f32 + frac * (tx1 - tx0) as f32;
                        c.round_rect(tx0, ty, pip_x as i32, ty + 2, 1, t.solar);
                        c.radial_glow(pip_x, ty as f32 + 1.0, 7.0, t.solar, 0.55);
                        c.round_rect((pip_x - 3.0) as i32, ty - 2, (pip_x + 3.0) as i32, ty + 4, 3, t.solar);
                    }
                }

                // Section 1 hairline divider
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);

                // Card 1: Today's Panchang (5 rows)
                c.round_rect(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, t.card);
                c.round_rect_border(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, 1, t.hairline);

                // Progress indicators for rows 0..5
                let (tx, tw, th) = l.track(w);
                for i in 0..5 {
                    if let Some(r) = rows.get(i) {
                        if let Some((frac, _)) = &r.progress {
                            let y = l.row_y(i as i32) + l.indicator_y();
                            c.round_rect(tx, y, tx + tw, y + th, 1, t.track);
                            let fw = ((frac.clamp(0.0, 1.0)) * tw as f64).round() as i32;
                            if fw > 0 {
                                c.round_rect(tx, y, tx + fw, y + th, 1, t.accent);
                            }
                        }
                    }
                }

                // Card 2: Moon & Calendar 2x2 grid tiles
                for idx in 0..4 {
                    let (x0, y0, x1, y1) = l.tile_rect(idx);
                    c.round_rect(x0, y0, x1, y1, 14, t.surface_elevated);
                    c.round_rect_border(x0, y0, x1, y1, 14, 1, t.hairline);
                }
            }
            layout::LayoutMode::Landscape => {
                c.radial_glow(mcx1, mcy, mr * 1.8, t.accent, t.moon_glow * 0.7);
                c.radial_glow(mcx2, mcy, mr * 1.8, t.accent, t.moon_glow);

                // Top micro-tags above dual moons
                c.round_rect(mcx1 as i32 - 58, l.moon_y - 22, mcx1 as i32 + 58, l.moon_y - 6, 6, t.surface_elevated);
                c.round_rect_border(mcx1 as i32 - 58, l.moon_y - 22, mcx1 as i32 + 58, l.moon_y - 6, 6, 1, t.hairline);
                c.round_rect(mcx2 as i32 - 58, l.moon_y - 22, mcx2 as i32 + 58, l.moon_y - 6, 6, t.surface_elevated);
                c.round_rect_border(mcx2 as i32 - 58, l.moon_y - 22, mcx2 as i32 + 58, l.moon_y - 6, 6, 1, t.hairline);

                moon::draw_3d_moon(&mut c, mcx1, mcy, mr, app.panchangam.illum as f32, app.panchangam.waxing, app.rotation_angle, t.accent);
                moon::draw_sky_moon(&mut c, mcx2, mcy, mr, app.panchangam.illum as f32, app.panchangam.waxing, t.accent);

                // Phase badge under moons
                let badge_x0 = mcx1 as i32 - mr as i32 - 6;
                let badge_x1 = mcx2 as i32 + mr as i32 + 6;
                let badge_y0 = l.moon_bottom + 10;
                let badge_y1 = badge_y0 + 24;
                c.round_rect(badge_x0, badge_y0, badge_x1, badge_y1, 8, t.card);
                c.round_rect_border(badge_x0, badge_y0, badge_x1, badge_y1, 8, 1, t.hairline);

                // Solar timeline on left under moons
                let tx0 = PAD + 56;
                let tx1 = 440 - 56;
                let ty = l.timeline_y + 13;
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
                let right_card_x0 = 470;
                let right_card_x1 = w - PAD;
                c.round_rect(right_card_x0, l.panchang_card_y, right_card_x1, l.panchang_card_bottom, layout::CARD_R, t.card);
                c.round_rect_border(right_card_x0, l.panchang_card_y, right_card_x1, l.panchang_card_bottom, layout::CARD_R, 1, t.hairline);

                let track_x0 = right_card_x0 + 130;
                let track_w = right_card_x1 - 16 - track_x0;
                let th = 2;
                for i in 0..5 {
                    if let Some(r) = rows.get(i) {
                        if let Some((frac, _)) = &r.progress {
                            let y = l.row_y(i as i32) + l.indicator_y();
                            c.round_rect(track_x0, y, track_x0 + track_w, y + th, 1, t.track);
                            let fw = ((frac.clamp(0.0, 1.0)) * track_w as f64).round() as i32;
                            if fw > 0 {
                                c.round_rect(track_x0, y, track_x0 + fw, y + th, 1, t.accent);
                            }
                        }
                    }
                }

                // Hairline divider below top deck
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);

                // Bottom deck: 4 horizontal tiles
                for idx in 0..4 {
                    let (x0, y0, x1, y1) = l.tile_rect(idx);
                    c.round_rect(x0, y0, x1, y1, 14, t.surface_elevated);
                    c.round_rect_border(x0, y0, x1, y1, 14, 1, t.hairline);
                }
            }
            layout::LayoutMode::Compact => {
                c.radial_glow(mcx1, mcy, mr * 2.2, t.accent, t.moon_glow);
                moon::draw_sky_moon(
                    &mut c,
                    mcx1,
                    mcy,
                    mr,
                    app.panchangam.illum as f32,
                    app.panchangam.waxing,
                    t.accent,
                );

                // Moon badge pill beneath moon
                let badge_x0 = l.sky_moon_cx - 60;
                let badge_x1 = l.sky_moon_cx + 60;
                let badge_y0 = l.moon_bottom + 10;
                let badge_y1 = badge_y0 + 24;
                c.round_rect(badge_x0, badge_y0, badge_x1, badge_y1, 8, t.surface_elevated);
                c.round_rect_border(badge_x0, badge_y0, badge_x1, badge_y1, 8, 1, t.hairline);

                // Hairline divider below header
                c.hline(PAD, w - PAD, l.rule_y, t.hairline);

                // Card 1: Panchang Highlights Card
                c.round_rect(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, t.card);
                c.round_rect_border(PAD, l.panchang_card_y, w - PAD, l.panchang_card_bottom, layout::CARD_R, 1, t.hairline);

                // Tithi progress rule
                let track_x0 = PAD + 16;
                let track_w = w - PAD * 2 - 32;
                let bar_y = l.panchang_card_y + 40;
                c.round_rect(track_x0, bar_y, track_x0 + track_w, bar_y + 2, 1, t.track);
                let tithi_frac = app.panchangam.tithi.progress;
                let fw = ((tithi_frac.clamp(0.0, 1.0)) * track_w as f64).round() as i32;
                if fw > 0 {
                    c.round_rect(track_x0, bar_y, track_x0 + fw, bar_y + 2, 1, t.accent);
                }

                // 4 flex chip backgrounds for Card 1 (Nakshatra, Yoga, Karana, Vara)
                let chip_gap = 12;
                let chip_w = (track_w - chip_gap) / 2;
                let chip_h = 28;
                let chip_y0 = l.panchang_card_y + 52;
                let chip_y1 = l.panchang_card_y + 86;
                for (cx0, cy0) in [
                    (track_x0, chip_y0),
                    (track_x0 + chip_w + chip_gap, chip_y0),
                    (track_x0, chip_y1),
                    (track_x0 + chip_w + chip_gap, chip_y1),
                ] {
                    c.round_rect(cx0, cy0, cx0 + chip_w, cy0 + chip_h, 8, t.surface_elevated);
                    c.round_rect_border(cx0, cy0, cx0 + chip_w, cy0 + chip_h, 8, 1, t.hairline);
                }

                // Card 2: Solar Timeline & Cycle Card
                c.round_rect(PAD, l.timeline_y, w - PAD, l.timeline_bottom, layout::CARD_R, t.card);
                c.round_rect_border(PAD, l.timeline_y, w - PAD, l.timeline_bottom, layout::CARD_R, 1, t.hairline);

                // Center day duration pill background in Card 2
                let dur_pill_w = 170;
                let dur_pill_x0 = (w - dur_pill_w) / 2;
                c.round_rect(dur_pill_x0, l.timeline_y + 10, dur_pill_x0 + dur_pill_w, l.timeline_y + 32, 6, t.surface_elevated);
                c.round_rect_border(dur_pill_x0, l.timeline_y + 10, dur_pill_x0 + dur_pill_w, l.timeline_y + 32, 6, 1, t.hairline);

                // Solar track at y = l.timeline_y + 44 (12px clearance below text)
                let tx0 = PAD + 76;
                let tx1 = w - PAD - 76;
                let ty = l.timeline_y + 44;
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

                // 4 flex chip backgrounds for Card 2 lunar cycle
                let cycle_chip_y0 = l.timeline_y + 56;
                let cycle_chip_y1 = l.timeline_y + 86;
                for (cx0, cy0) in [
                    (track_x0, cycle_chip_y0),
                    (track_x0 + chip_w + chip_gap, cycle_chip_y0),
                    (track_x0, cycle_chip_y1),
                    (track_x0 + chip_w + chip_gap, cycle_chip_y1),
                ] {
                    c.round_rect(cx0, cy0, cx0 + chip_w, cy0 + chip_h, 8, t.surface_elevated);
                    c.round_rect_border(cx0, cy0, cx0 + chip_w, cy0 + chip_h, 8, 1, t.hairline);
                }
            }
        }
    }

    // ---- text pass -------------------------------------------------------
    {
        let dc = dib.dc();
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

        // Weekday, small, tracked, in the accent
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.accent));
        tracked_text(dc, &weekday, PAD, l.weekday_text_top(), 2);

        // The date, large display face
        let p = &app.panchangam;
        let s = app.script;
        SelectObject(dc, HGDIOBJ(display.0));
        SetTextColor(dc, colorref(t.text));
        let title = format!("{} {} {}", p.day, month_abbr(p.month), p.year);
        let header_text_w = if app.mode == layout::LayoutMode::Portrait {
            (l.moon_cx - l.moon_r - 16) - PAD
        } else if app.mode == layout::LayoutMode::Landscape {
            440 - PAD
        } else {
            (l.sky_moon_cx - l.moon_r - 16) - PAD
        };
        draw_text(
            dc,
            &title,
            rect(PAD, l.date_y, header_text_w, if app.mode == layout::LayoutMode::Compact { 36 } else { 42 }),
            DT_LEFT | DT_VCENTER,
        );

        // Solar / National Calendar era
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
            rect(PAD, l.era_y, header_text_w, 22),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        // Udaya Tithi and Day Duration (Dedicated non-overlapping lines)
        let day_duration = if let (Some(sr), Some(ss)) = (p.sunrise_jd, p.sunset_jd) {
            let total_mins = ((ss - sr) * 24.0 * 60.0).round() as i64;
            let hrs = total_mins / 60;
            let mins = total_mins % 60;
            if telugu {
                format!("{hrs}గం {mins}ని")
            } else {
                format!("{hrs}h {mins}m")
            }
        } else {
            "--".into()
        };

        let udaya_name = if let Some(ut) = &p.tithi_at_sunrise {
            format!("{} {}", paksha_at(ut.index, s), tithi_name_at(ut.index, s))
        } else {
            format!("{} {}", paksha_at(p.tithi.index, s), tithi_name_at(p.tithi.index, s))
        };

        if app.mode != layout::LayoutMode::Compact {
            let udaya_str = if telugu {
                format!("ఉదయ తిథి: {udaya_name}")
            } else {
                format!("Udaya Tithi: {udaya_name}")
            };
            draw_text(
                dc,
                &udaya_str,
                rect(PAD, l.udaya_y, header_text_w, 22),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );

            let day_dur_str = if telugu {
                format!("దినమానం: {day_duration}")
            } else {
                format!("Day Duration: {day_duration}")
            };
            draw_text(
                dc,
                &day_dur_str,
                rect(PAD, l.day_duration_y, header_text_w, 22),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );
        } else {
            let udaya_str = if telugu {
                format!("ఉదయ తిథి: {udaya_name}")
            } else {
                format!("Udaya Tithi: {udaya_name}")
            };
            draw_text(
                dc,
                &udaya_str,
                rect(PAD, l.udaya_y, header_text_w, 22),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );
        }

        match app.mode {
            layout::LayoutMode::Portrait => {
                // Micro-tags above dual moons
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.accent));
                let tag1 = if telugu { "3D భ్రమణం" } else { "3D ROTATING" };
                let tag2 = if telugu { "ఆకాశ దృశ్యం" } else { "SKY VIEW" };
                draw_text(dc, tag1, rect(l.moon_cx - 58, l.moon_y - 22, 116, 16), DT_CENTER | DT_VCENTER);
                draw_text(dc, tag2, rect(l.sky_moon_cx - 58, l.moon_y - 22, 116, 16), DT_CENTER | DT_VCENTER);

                // Phase summary badge beneath both moons
                let phase_summary = format!(
                    "● {} · {:.0}% {}",
                    p.phase_name_in(s),
                    p.illum * 100.0,
                    if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }
                );
                let badge_x0 = l.moon_cx - l.moon_r - 6;
                let badge_x1 = l.sky_moon_cx + l.moon_r + 6;
                draw_text(
                    dc,
                    &phase_summary,
                    rect(badge_x0, l.moon_bottom + 10, badge_x1 - badge_x0, 24),
                    DT_CENTER | DT_VCENTER,
                );

                // Sunrise / Sunset timeline labels
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(PAD, l.timeline_y + 4, 70, 22), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(w - PAD - 70, l.timeline_y + 4, 70, 22), DT_RIGHT | DT_VCENTER);

                // Section 1 Header: TODAY'S PANCHANG
                SelectObject(dc, HGDIOBJ(section.0));
                SetTextColor(dc, colorref(t.accent));
                tracked_text(dc, if telugu { "నేటి పంచాంగం" } else { "TODAY'S PANCHANG" }, PAD, l.rule_y + 12, 1);

                // Rows 0..5 (Tithi, Nakshatra, Yoga, Karana, Vara)
                for i in 0..5 {
                    if let Some(r) = rows.get(i) {
                        let y = l.row_y(i as i32);

                        SelectObject(dc, HGDIOBJ(label.0));
                        SetTextColor(dc, colorref(t.faint));
                        let caps = r.label.to_uppercase();
                        tracked_text(dc, &caps, PAD + 16, y + 8, 1);

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
                            rect(value_x, y + 4, value_w, l.text_h()),
                            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                        );

                        if cap_w > 0 {
                            if let Some((_, cap)) = &r.progress {
                                SelectObject(dc, HGDIOBJ(small.0));
                                SetTextColor(dc, colorref(t.subtext));
                                draw_text(
                                    dc,
                                    cap,
                                    rect(w - PAD - 16 - cap_w, y + 4, cap_w, l.text_h()),
                                    DT_LEFT | DT_VCENTER,
                                );
                            }
                        }
                    }
                }

                // Section 2 Header: MOON & CALENDAR
                SelectObject(dc, HGDIOBJ(section.0));
                SetTextColor(dc, colorref(t.accent));
                tracked_text(dc, if telugu { "చంద్రుడు & క్యాలెండర్" } else { "MOON & CALENDAR" }, PAD, l.panchang_card_bottom + 14, 1);

                // 2x2 Grid Tile texts
                paint_tiles(dc, &l, t, p, s, telugu, label, body, small);
            }
            layout::LayoutMode::Landscape => {
                // Micro-tags above dual moons
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.accent));
                let tag1 = if telugu { "3D భ్రమణం" } else { "3D ROTATING" };
                let tag2 = if telugu { "ఆకాశ దృశ్యం" } else { "SKY VIEW" };
                draw_text(dc, tag1, rect(l.moon_cx - 58, l.moon_y - 22, 116, 16), DT_CENTER | DT_VCENTER);
                draw_text(dc, tag2, rect(l.sky_moon_cx - 58, l.moon_y - 22, 116, 16), DT_CENTER | DT_VCENTER);

                // Phase summary badge beneath both moons
                let phase_summary = format!(
                    "● {} · {:.0}% {}",
                    p.phase_name_in(s),
                    p.illum * 100.0,
                    if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }
                );
                let badge_x0 = l.moon_cx - l.moon_r - 6;
                let badge_x1 = l.sky_moon_cx + l.moon_r + 6;
                draw_text(
                    dc,
                    &phase_summary,
                    rect(badge_x0, l.moon_bottom + 10, badge_x1 - badge_x0, 24),
                    DT_CENTER | DT_VCENTER,
                );

                // Sunrise / Sunset labels
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(PAD, l.timeline_y + 4, 60, 22), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(440 - 60, l.timeline_y + 4, 60, 22), DT_RIGHT | DT_VCENTER);

                // Right column rows (Today's Panchang, 5 rows)
                let right_card_x0 = 470;
                let right_card_x1 = w - PAD;
                SelectObject(dc, HGDIOBJ(section.0));
                SetTextColor(dc, colorref(t.accent));
                tracked_text(dc, if telugu { "నేటి పంచాంగం" } else { "TODAY'S PANCHANG" }, right_card_x0, PAD + 8, 1);

                for i in 0..5 {
                    if let Some(r) = rows.get(i) {
                        let y = l.row_y(i as i32);
                        SelectObject(dc, HGDIOBJ(label.0));
                        SetTextColor(dc, colorref(t.faint));
                        let caps = r.label.to_uppercase();
                        tracked_text(dc, &caps, right_card_x0 + 16, y + 8, 1);

                        let val_x = right_card_x0 + 130;
                        SelectObject(dc, HGDIOBJ(body.0));
                        SetTextColor(dc, colorref(t.text));
                        draw_text(
                            dc,
                            &r.value,
                            rect(val_x, y + 4, right_card_x1 - val_x - 16, l.text_h()),
                            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                        );

                        if let Some((_, cap)) = &r.progress {
                            SelectObject(dc, HGDIOBJ(small.0));
                            SetTextColor(dc, colorref(t.subtext));
                            draw_text(
                                dc,
                                cap,
                                rect(right_card_x1 - 16 - 120, y + 4, 120, l.text_h()),
                                DT_RIGHT | DT_VCENTER,
                            );
                        }
                    }
                }

                // Bottom deck: 4 horizontal tiles
                paint_tiles(dc, &l, t, p, s, telugu, label, body, small);
            }
            layout::LayoutMode::Compact => {
                // Micro-badge below moon
                let badge_x0 = l.sky_moon_cx - 60;
                let badge_x1 = l.sky_moon_cx + 60;
                let badge_y0 = l.moon_bottom + 10;
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.subtext));
                let compact_phase_str = format!(
                    "● {:.0}% {}",
                    p.illum * 100.0,
                    if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }
                );
                draw_text(
                    dc,
                    &compact_phase_str,
                    rect(badge_x0, badge_y0, badge_x1 - badge_x0, 24),
                    DT_CENTER | DT_VCENTER,
                );

                // Card 1 Text (Panchang Highlights with Responsive Flex Chips)
                let track_x0 = PAD + 16;
                let track_w = w - PAD * 2 - 32;
                let tithi_row = &rows[0];
                SelectObject(dc, HGDIOBJ(body.0));
                SetTextColor(dc, colorref(t.text));
                let tithi_title = format!("{} · {}", if telugu { "తిథి" } else { "TITHI" }, tithi_row.value);
                draw_text(
                    dc,
                    &tithi_title,
                    rect(track_x0, l.panchang_card_y + 12, track_w - 140, 24),
                    DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                );

                if let Some((_, cap)) = &tithi_row.progress {
                    SelectObject(dc, HGDIOBJ(small.0));
                    SetTextColor(dc, colorref(t.accent));
                    draw_text(
                        dc,
                        cap,
                        rect(track_x0 + track_w - 140, l.panchang_card_y + 12, 140, 24),
                        DT_RIGHT | DT_VCENTER,
                    );
                }

                // 4 Responsive Flex Chips in Card 1
                let chip_gap = 12;
                let chip_w = (track_w - chip_gap) / 2;
                let chip_h = 28;
                let chip_y0 = l.panchang_card_y + 52;
                let chip_y1 = l.panchang_card_y + 86;

                let card1_chips = [
                    (track_x0, chip_y0, format!("{}: {} ({})", if telugu { "నక్షత్రం" } else { "Nak" }, s.nakshatra(p.nakshatra.index), bar_caption(&p.nakshatra, p).1)),
                    (track_x0 + chip_w + chip_gap, chip_y0, format!("{}: {} ({})", if telugu { "యోగం" } else { "Yoga" }, s.yoga(p.yoga.index), bar_caption(&p.yoga, p).1)),
                    (track_x0, chip_y1, format!("{}: {} ({})", if telugu { "కరణం" } else { "Karana" }, s.karana(p.karana_slot), if p.karana_first_half { if telugu { "1వ సగం" } else { "1st half" } } else { if telugu { "2వ సగం" } else { "2nd half" } })),
                    (track_x0 + chip_w + chip_gap, chip_y1, if telugu { format!("{}: {} (రవి)", s.vara(p.vara), ["సూర్యుడు", "చంద్రుడు", "కుజుడు", "బుధుడు", "గురుడు", "శుక్రుడు", "శని"][p.vara % 7]) } else { format!("Vara: {} ({})", s.vara(p.vara), ["Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn"][p.vara % 7]) }),
                ];

                for (cx0, cy0, text) in card1_chips {
                    SelectObject(dc, HGDIOBJ(small.0));
                    SetTextColor(dc, colorref(t.subtext));
                    draw_text(
                        dc,
                        &text,
                        rect(cx0 + 10, cy0 + 3, chip_w - 20, chip_h - 6),
                        DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                    );
                }

                // Card 2 Text (Solar Timeline & Lunar Cycle)
                // Row 1: Sunrise & Sunset
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.solar));
                let rise_str = format!("☀ {}", p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                let set_str = format!("☀ {}", p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or_else(|| "--:--".into()));
                draw_text(dc, &rise_str, rect(track_x0, l.timeline_y + 10, 80, 22), DT_LEFT | DT_VCENTER);
                draw_text(dc, &set_str, rect(track_x0 + track_w - 80, l.timeline_y + 10, 80, 22), DT_RIGHT | DT_VCENTER);

                // Row 1 Center: Day duration pill text (100% CLEAR - NEVER TOUCHES THE SOLAR TRACK)
                let dur_pill_w = 170;
                let dur_pill_x0 = (w - dur_pill_w) / 2;
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.text));
                let day_dur_str = format!("{}: {}", if telugu { "దినమానం" } else { "Day Duration" }, day_duration);
                draw_text(
                    dc,
                    &day_dur_str,
                    rect(dur_pill_x0, l.timeline_y + 10, dur_pill_w, 22),
                    DT_CENTER | DT_VCENTER,
                );

                // Row 2: 4 Lunar Cycle Chips in Card 2
                let cycle_chip_y0 = l.timeline_y + 56;
                let cycle_chip_y1 = l.timeline_y + 86;

                let card2_chips = [
                    (track_x0, cycle_chip_y0, if telugu { format!("● దశ: {}", p.phase_name_in(s)) } else { format!("● Phase: {}", p.phase_name_in(s)) }),
                    (track_x0 + chip_w + chip_gap, cycle_chip_y0, if telugu { format!("అమాంతం: {} (దిన {})", s.month(p.amanta_month), p.amanta_tithi_day) } else { format!("Amanta: {} (Day {})", s.month(p.amanta_month), p.amanta_tithi_day) }),
                    (track_x0, cycle_chip_y1, if telugu { format!("పూర్ణిమాంతం: {}", s.month(p.purnimanta_month)) } else { format!("Purnimanta: {}", s.month(p.purnimanta_month)) }),
                    (track_x0 + chip_w + chip_gap, cycle_chip_y1, if telugu { format!("ముగింపు: {}", p.day_label_short(p.phase_end_jd)) } else { format!("Ends: {}", p.day_label_short(p.phase_end_jd)) }),
                ];

                for (cx0, cy0, text) in card2_chips {
                    SelectObject(dc, HGDIOBJ(small.0));
                    SetTextColor(dc, colorref(t.subtext));
                    draw_text(
                        dc,
                        &text,
                        rect(cx0 + 10, cy0 + 3, chip_w - 20, chip_h - 6),
                        DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                    );
                }
            }
        }

        // Footer hint.
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.faint));
        draw_text(
            dc,
            "m: Mode · Tab: English / Telugu",
            rect(PAD, l.footer_y, 300, 20),
            DT_LEFT | DT_VCENTER,
        );
        draw_text(
            dc,
            "Esc or click to close",
            rect(w - PAD - 200, l.footer_y, 200, 20),
            DT_RIGHT | DT_VCENTER,
        );

        let _ = DeleteObject(HGDIOBJ(display.0));
        let _ = DeleteObject(HGDIOBJ(body.0));
        let _ = DeleteObject(HGDIOBJ(small.0));
        let _ = DeleteObject(HGDIOBJ(label.0));
        let _ = DeleteObject(HGDIOBJ(section.0));
    }
}

/// Paint the whole panel into the window DC.
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
    render_into_dib(&app, &mut dib);
    blit_to_window(hwnd, &dib);
}

fn save_dib_bmp(dib: &mut Dib, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let w = dib.w;
    let h = dib.h;
    let row_bytes = (w * 4) as usize;
    let image_size = (row_bytes * h as usize) as u32;
    let file_size = 54 + image_size;

    let mut file = std::fs::File::create(path)?;
    // BITMAPFILEHEADER (14 bytes)
    file.write_all(b"BM")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&54u32.to_le_bytes())?;

    // BITMAPINFOHEADER (40 bytes)
    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&w.to_le_bytes())?;
    file.write_all(&(-h).to_le_bytes())?; // negative for top-down rows
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&image_size.to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;

    let pixels = unsafe { dib.pixels() };
    file.write_all(pixels)?;
    Ok(())
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
            if telugu { "చంద్ర కాంతి" } else { "LUNAR ILLUMINATION" },
            format!("{:.0}% · {}", p.illum * 100.0, if p.waxing { if telugu { "వృద్ధి" } else { "Waxing" } } else { if telugu { "క్షయ" } else { "Waning" } }),
            Some(if telugu { format!("శక దినం: {} · VS {}", p.saka_day, p.vikram_year) } else { format!("Saka Day {} · VS {}", p.saka_day, p.vikram_year) }),
        ),
        (
            if telugu { "అమాంత మాసం" } else { "AMANTA LUNAR MONTH" },
            format!("{} · {} {}", s.month(p.amanta_month), if telugu { "దిన" } else { "Day" }, p.amanta_tithi_day),
            Some(if telugu { "తెలుగు సంప్రదాయ మాసం".into() } else { "Classical lunar cycle".into() }),
        ),
        (
            if telugu { "దశ పరివర్తన" } else { "PHASE TRANSITIONS" },
            p.phase_name_in(s).to_string(),
            Some(if telugu { format!("ముగింపు: {}", p.day_label_short(p.phase_end_jd)) } else { format!("ends {}", p.day_label_short(p.phase_end_jd)) }),
        ),
        (
            if telugu { "రాబోయే పర్వాలు" } else { "UPCOMING PHASES" },
            format!("New {} · Full {}", p.day_label_short(p.next_new_moon_jd), p.day_label_short(p.next_full_moon_jd)),
            Some(if telugu { format!("పూర్ణిమాంతం: {}", s.month(p.purnimanta_month)) } else { format!("Purnimanta: {}", s.month(p.purnimanta_month)) }),
        ),
    ];

    for (idx, (head, val, sub)) in tiles.iter().enumerate() {
        let (x0, y0, x1, y1) = l.tile_rect(idx);
        let tw = x1 - x0;
        let pad_x = 16;

        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.subtext));
        draw_text(
            dc,
            head,
            rect(x0 + pad_x, y0 + 12, tw - pad_x * 2, 16),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        SelectObject(dc, HGDIOBJ(body.0));
        SetTextColor(dc, colorref(t.text));
        draw_text(
            dc,
            val,
            rect(x0 + pad_x, y0 + 38, tw - pad_x * 2, if telugu { 28 } else { 24 }),
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
        );

        if let Some(sub_text) = sub {
            if y1 - y0 >= 60 {
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(t.faint));
                draw_text(
                    dc,
                    sub_text,
                    rect(x0 + pad_x, y0 + 72, tw - pad_x * 2, 20),
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
            let work_h = 1000;
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
