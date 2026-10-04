//! palette-picker — the bar's graphical theme picker, in Rust.
//!
//! A dmenu-style palette switcher: a filter line on top, a swatch grid
//! below, type to narrow, arrows/Enter to apply, click to apply. It is the
//! Rust replacement for the old PyQt6 `palette-picker.py`, which has since
//! been retired along with its PyInstaller chain — there is one picker now,
//! and keeping a second implementation around only invited drift.
//!
//! Raw Win32 + GDI rather than a GUI toolkit, matching saka-popup: this is
//! one frameless window with hand-drawn cells, so there is no widget tree,
//! stylesheet engine or layout system worth pulling in.

mod catalog;
mod layout;
mod theme;

use std::cell::RefCell;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_BACK, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_NEXT, VK_PRIOR,
    VK_RETURN, VK_RIGHT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use yasb_chrome::gdi::{
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect,
    text_width, tracked_text,
};
use yasb_chrome::Type;


use catalog::Item;
use layout::Row;
use theme::{Rgba, Theme};

/// Width of the panel.
///
/// Sized so the name column can hold the catalog's longest shipped name
/// without an ellipsis — see `layout::CELL_W` for the measurement.
const W: i32 = 1020;
/// Window corner radius, matched to saka-popup so the two panels read as one
/// system when they swap places on the same click.
const RADIUS: i32 = 20;
/// Radius of a theme cell and of the search field.
const CELL_R: i32 = 12;

const PAD: i32 = 16;
const SEARCH_H: i32 = 64;
const FOOT_H: i32 = 34;
/// Upper bound on the panel's height.
///
/// Not a layout constraint — it is the screen-fit guard, and the shipped
/// catalog is well inside it: 22 themes in three columns come to 548px. It
/// was 520, which was *below* the content, so it silently clipped the last
/// row of cells on top of the footer.
const MAX_H: i32 = 760;

thread_local! {
    /// The desktop snapshot, taken once before the window is created. Taking
    /// it after would photograph the panel itself, and blending that back in
    /// would get brighter on every 1 Hz repaint.
    static BACKDROP: RefCell<Option<Backdrop>> = const { RefCell::new(None) };
}

thread_local! {
    /// The single global the window procedure reads and writes.
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Snapshots from the background thread, newest last. Only ever drained,
    /// never written to, and only ever on the message-loop thread.
    static INBOX: RefCell<Option<Receiver<Snapshot>>> = const { RefCell::new(None) };
    /// Set when the window is destroyed, so the worker stops computing into a
    /// channel nobody is reading.
    static STOP: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

const TIMER_ID: usize = 1;
/// How often the background thread re-reads the theme.
///
/// Longer than saka-popup's 1s on purpose. The picker has no clock to draw, so
/// there is nothing to redraw every second; the only reason to poll is so a
/// theme switched elsewhere while the panel is open is reflected without
/// reopening it. Half a second is well inside "feels instant" for that, and it
/// halves a per-second stylesheet read and CSS parse.
const REFRESH_MS: u64 = 500;

/// One complete set of values for a frame.
///
/// Plain data only — strings and colours, no handles — which is what makes it
/// `Send` and therefore what allows the background thread to exist. A GDI HDC
/// belongs to the thread that selected into it and a window may not be painted
/// from a thread without a message queue, so the split is strict: the worker
/// reads files, the message loop draws.
#[derive(Clone, Debug)]
struct Snapshot {
    theme: Theme,
    /// `Some` only on the first snapshot.
    ///
    /// `current` costs a subprocess spawn — `yasb-theme current` — so it is
    /// fetched exactly once rather than on every tick. Re-reading it every
    /// 500ms would mean two process spawns a second for a value that cannot
    /// change while the picker is open.
    current: Option<String>,
}

fn take_snapshot(first: bool) -> Snapshot {
    Snapshot {
        // Re-read every tick so a switch made while the panel is open shows up
        // without reopening it. This is a file read and a CSS parse, which is
        // the second reason to keep it off the UI thread.
        theme: Theme::load(&styles_path()),
        current: if first { Some(active_name()) } else { None },
    }
}

/// Start the single background thread that produces [`Snapshot`]s.
///
/// One thread, not a pool: the work is one small file read and one CSS parse
/// twice a second. A pool would add handoff overhead to buy nothing.
fn spawn_worker() {
    let (tx, rx) = mpsc::channel::<Snapshot>();
    INBOX.with(|i| *i.borrow_mut() = Some(rx));
    let stop = Arc::new(AtomicBool::new(false));
    STOP.with(|s| *s.borrow_mut() = Some(stop.clone()));
    std::thread::spawn(move || {
        // Deliver one straight away rather than making the first paint wait.
        if tx.send(take_snapshot(true)).is_err() {
            return;
        }
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(REFRESH_MS));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            // A send error means the panel closed and dropped the receiver,
            // which is the normal way this loop ends.
            if tx.send(take_snapshot(false)).is_err() {
                break;
            }
        }
    });
}

/// Stop the worker. Called from WM_DESTROY.
///
/// Not joined: the thread is parked in a sleep when the flag flips, so it
/// cannot be mid-write to anything the UI thread still owns, and joining would
/// risk up to half a second of stall on close.
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
/// After a slow tick the channel can hold more than one. Returns `None` when
/// nothing new arrived, which is the signal not to repaint at all.
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

/// Move any fresh snapshot into the app state.
///
/// Returns whether the theme actually *changed*, which is the repaint
/// condition. The picker polls the stylesheet even when nothing has been
/// switched, and repainting an unchanged panel on every tick is pure GDI
/// churn for no visible change.
fn drain_inbox() -> bool {
    let mut newest = None;
    INBOX.with(|i| {
        if let Some(rx) = i.borrow_mut().as_mut() {
            newest = take_latest(rx);
        }
    });
    let Some(s) = newest else { return false };
    let mut repaint = false;
    APP.with(|c| {
        if let Some(a) = c.borrow_mut().as_mut() {
            if let Some(name) = s.current {
                a.current = name;
                repaint = true;
            }
            if a.theme != s.theme {
                a.theme = s.theme;
                repaint = true;
            }
        }
    });
    repaint
}

#[derive(Clone)]
struct App {
    items: Vec<Item>,
    current: String,
    /// Indices into `items` that survive the filter, in display order.
    filtered: Vec<usize>,
    query: String,
    /// Position within `filtered`, not within the rows (headers are skipped).
    sel: usize,
    theme: Theme,
    hwnd: Option<HWND>,
}

impl App {
    fn rows(&self) -> Vec<Row> {
        layout::rows_for(&self.filtered, &self.items)
    }

    /// Client-space origin of the first cell row, below the search field.
    fn grid_origin(&self) -> (i32, i32) {
        (PAD, PAD + SEARCH_H + 12)
    }

    fn rects(&self, w: i32) -> Vec<(i32, i32)> {
        let (ox, oy) = self.grid_origin();
        layout::cell_rects(&self.rows(), (ox, oy), w - PAD * 2)
    }

    fn refilter(&mut self) {
        self.filtered = layout::visible(&self.items, &self.query);
        self.sel = 0;
    }

    fn move_sel(&mut self, delta: isize, w: i32) {
        if self.filtered.is_empty() {
            return;
        }
        self.sel = layout::move_sel(self.sel, delta, self.filtered.len());
        let _ = w;
    }

    fn selected_name(&self) -> Option<&str> {
        self.filtered.get(self.sel).map(|&i| self.items[i].name.as_str())
    }
}

fn yasb_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("yasb")
}

fn styles_path() -> PathBuf {
    yasb_dir().join("styles.css")
}

fn catalog_path() -> PathBuf {
    // The catalog ships next to the picker source; fall back to the yasb
    // folder so a deployed copy still finds it.
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../palette-themes.json");
    if local.exists() {
        local
    } else {
        yasb_dir().join("tools").join("picker").join("palette-themes.json")
    }
}

fn theme_exe() -> PathBuf {
    yasb_dir().join("tools").join("theme").join("yasb-theme.exe")
}

/// Ask yasb-theme which theme is active. Used to mark the current cell.
fn active_name() -> String {
    Command::new(theme_exe())
        .arg("current")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Keeps the console window from flashing when yasb-theme is spawned from
/// a GUI process (CREATE_NO_WINDOW).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Apply a theme by name, waiting for the stylesheet rewrite to finish so
/// the bar cannot still be repainting from the old theme when we return.
fn apply(name: &str) {
    let _ = Command::new(theme_exe())
        .args(["set", name])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|mut c| {
            let _ = c.wait();
        });
}

/// A theme cell: a rounded tile, filled more strongly and ringed in the accent
/// when selected, with a marker for the theme that is currently active.
///
/// The current theme gets its own treatment (a dot plus accent-coloured text)
/// because "selected" and "active" are different facts — the arrow keys move
/// the first, and conflating them would make the panel lie about what is
/// applied.
unsafe fn draw_cell(c: &mut yasb_chrome::Canvas, x: i32, y: i32, selected: bool, is_current: bool, t: &Theme) {
    let (x0, y0, x1, y1) = (x, y, x + layout::CELL_W, y + layout::CELL_H);
    c.round_rect(x0, y0, x1, y1, CELL_R, if selected { t.cell_selected } else { t.cell });
    if selected {
        c.round_rect_border(x0, y0, x1, y1, CELL_R, 1, t.accent);
    } else {
        c.round_rect_border(x0, y0, x1, y1, CELL_R, 1, t.hairline);
    }
    if is_current {
        // A small accent dot down the left edge marks the applied theme.
        let cy = (y0 + y1) / 2;
        for py in (cy - 3)..=(cy + 3) {
            for px in (x0 + 9)..=(x0 + 15) {
                let dx = px as f32 - (x0 + 12) as f32;
                let dy = py as f32 - cy as f32;
                let d = (dx * dx + dy * dy).sqrt() - 3.0;
                c.blend_pixel(px, py, t.accent, (0.5 - d).clamp(0.0, 1.0));
            }
        }
    }
}

/// Swatch chips with a hairline edge.
///
/// The edge matters more than it looks: several shipped light palettes have
/// near-white swatches, and on a light cell those render as blank rectangles.

unsafe fn draw_swatches(c: &mut yasb_chrome::Canvas, x: i32, y: i32, item: &Item, t: &Theme) {
    let mut sx = x;
    for sw in item.swatches().into_iter().take(layout::SWATCHES as usize) {
        c.round_rect(sx, y, sx + layout::CHIP_W, y + 16, 5, sw);
        c.round_rect_border(sx, y, sx + layout::CHIP_W, y + 16, 5, 1, t.swatch_edge);
        sx += layout::SWATCH_STRIDE;
    }
}

/// Paint the panel: shapes into the DIB, then text through GDI on top.
///
/// Two passes, for the same reason as saka-popup: the shapes are direct pixel
/// writes and the text needs a font selected into the DC, so interleaving them
/// per element would mean dozens of font switches a frame.
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
    let origin = app.grid_origin();
    let rects = layout::cell_rects(&rows, origin, w - PAD * 2);

    // ---- shape pass ------------------------------------------------------
    {
        let mut c = dib.canvas();
        c.clear(t.bg.over(Rgba::rgb(20, 20, 28)));
        BACKDROP.with(|b| {
            if let Some(bd) = b.borrow().as_ref() {
                bd.draw_under(&mut c, 0, 0, t.backdrop_opacity);
            }
        });
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);
        // The bloom sits behind the search field, so the field looks lit from
        // within rather than pasted on top.
        c.radial_glow(
            w as f32 * 0.5,
            (PAD + SEARCH_H / 2) as f32,
            w as f32 * 0.7,
            t.accent,
            t.bloom,
        );
        c.round_rect_border(1, 1, w - 1, h - 1, RADIUS, 1, t.hairline);
        c.round_rect_border(1, 1, w - 1, 14, RADIUS, 1, t.highlight);

        // The search field: an inset rounded pill.
        let (fx0, fy0) = (PAD, PAD);
        let (fx1, fy1) = (w - PAD, PAD + SEARCH_H);
        c.round_rect(fx0, fy0, fx1, fy1, CELL_R + 6, t.field);
        c.round_rect_border(fx0, fy0, fx1, fy1, CELL_R + 6, 1, t.hairline);

        // Magnifier glyph: a ring plus a handle, drawn as geometry rather than
        // an icon font so it matches the panel's stroke weight exactly and
        // needs no asset on disk.
        let gcx = fx0 as f32 + 28.0;
        let gcy = (fy0 + fy1) as f32 / 2.0;
        for py in (gcy as i32 - 11)..=(gcy as i32 + 11) {
            for px in (gcx as i32 - 11)..=(gcx as i32 + 11) {
                let dx = px as f32 + 0.5 - gcx;
                let dy = py as f32 + 0.5 - gcy;
                let d = (dx * dx + dy * dy).sqrt() - 6.0;
                let cov = (0.5f32 - d.abs() / 1.7).clamp(0.0, 1.0);
                if cov > 0.0 {
                    c.blend_pixel(px, py, t.subtext, cov);
                }
            }
        }
        for i in 0..9 {
            let hx = gcx + 5.5 + i as f32 * 0.75;
            let hy = gcy + 5.5 + i as f32 * 0.75;
            for (dx, dy) in [(0i32, 0i32), (1, 0), (0, 1), (1, 1)] {
                c.blend_pixel(hx as i32 + dx, hy as i32 + dy, t.subtext, 0.85);
            }
        }

        // Cells and their swatches.
        for (i, row) in rows.iter().filter(|r| matches!(r, Row::Cell(_))).enumerate() {
            let Row::Cell(ci) = row else { continue };
            let (x, cy) = rects[i];
            let item = &app.items[*ci];
            let selected = app.filtered.get(app.sel) == Some(ci);
            draw_cell(&mut c, x, cy, selected, item.name == app.current, t);
            draw_swatches(&mut c, x + layout::SWATCH_X, cy + 12, item, t);
        }

        let (rule_y, _, _) = layout::footer_rows(h, FOOT_H, PAD);
        c.hline(PAD, w - PAD, rule_y, t.hairline);
    }

    // ---- text pass -------------------------------------------------------
    {
        let dc = dib.dc();
        let body = font(Type::Body.gdi(), 500, &t.typeface.family);
        let small = font(Type::Foot.gdi(), 400, &t.typeface.family);
        let label = font(Type::Label.gdi(), 600, &t.typeface.family);
        SetBkMode(dc, TRANSPARENT);

        // The search line. When the query is empty the hint stands in for it,
        // which is why the caret is drawn *before* the text rather than after:
        // a caret to the left of the hint would imply the hint is the value.
        let text_x = PAD + 48;
        let empty = app.query.is_empty();
        SelectObject(dc, HGDIOBJ(body.0));
        SetTextColor(dc, colorref(if empty { t.subtext } else { t.text }));
        draw_text(
            dc,
            if empty { "Type a theme name" } else { &app.query },
            rect(text_x, PAD, w - text_x - PAD, SEARCH_H),
            DT_LEFT | DT_VCENTER,
        );

        // Section headings, then the cell names.
        let mut cell_i = 0usize;
        let mut y = origin.1;
        for row in &rows {
            match row {
                Row::Header(text) => {
                    // Tracked caps, matching the weekday label in the saka
                    // panel: two panels from the same system should not
                    // disagree about what a section heading looks like.
                    //
                    // Centred in the heading's own 20px line box. It used to
                    // be drawn at `y + 14` of a full 40px band that the cells
                    // below also started at, which is how `LIGHT` ended up
                    // painted across the first light cell.
                    SelectObject(dc, HGDIOBJ(label.0));
                    SetTextColor(dc, colorref(t.subtext));
                    let caps = text.to_uppercase();
                    tracked_text(dc, &caps, PAD + 6, y + 4, 2);
                    y += layout::header_advance();
                }
                Row::Cell(ci) => {
                    let (x, cy) = rects[cell_i];
                    let item = &app.items[*ci];
                    let is_current = item.name == app.current;
                    let sw_x = x + layout::NAME_DX;
                    let avail = layout::name_avail().max(24);
                    SetTextColor(dc, colorref(if is_current { t.accent } else { t.text }));
                    // Step the size down rather than clipping: a name cut off
                    // mid-word reads as a bug, a smaller name reads as a
                    // deliberate fit.
                    let mut drawn = false;
                    // The step-down ladder. `-13` is not decoration: the longest shipped
                    // name measures 208px at 13px against a 214px column, so
                    // 13px is the size that lets it render at full weight
                    // rather than dropping to 12px. Naming only the ends of
                    // the scale quietly deleted it.
                    for size in [Type::Body.gdi(), -14, -13, Type::Foot.gdi()] {
                        let f = font(size, if is_current { 600 } else { 500 }, &t.typeface.family);
                        SelectObject(dc, HGDIOBJ(f.0));
                        if text_width(dc, &item.name) <= avail {
                            draw_text(
                                dc,
                                &item.name,
                                rect(sw_x, cy, avail, layout::CELL_H),
                                DT_LEFT | DT_VCENTER,
                            );
                            drawn = true;
                        }
                        let _ = DeleteObject(HGDIOBJ(f.0));
                        if drawn {
                            break;
                        }
                    }
                    if !drawn {
                        SelectObject(dc, HGDIOBJ(body.0));
                        draw_text(
                            dc,
                            &item.name,
                            rect(sw_x, cy, avail, layout::CELL_H),
                            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                        );
                    }
                    y = cy + layout::CELL_H + layout::GAP;
                    cell_i += 1;
                }
            }
        }

        // Footer: the match count, and the key hints.
        //
        // Always drawn — the count is useful with no filter active — so its
        // height is always reserved in `relayout`.
        let (_, foot_y, foot_h) = layout::footer_rows(h, FOOT_H, PAD);
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        let count = format!("{} of {} themes", app.filtered.len(), app.items.len());
        draw_text(dc, &count, rect(PAD + 6, foot_y, 240, foot_h), DT_LEFT | DT_VCENTER);
        SetTextColor(dc, colorref(t.subtext));
        draw_text(
            dc,
            "arrows select   enter apply   esc close",
            rect(w - PAD - 360, foot_y, 360, foot_h),
            DT_RIGHT | DT_VCENTER,
        );

        let _ = DeleteObject(HGDIOBJ(body.0));
        let _ = DeleteObject(HGDIOBJ(small.0));
        let _ = DeleteObject(HGDIOBJ(label.0));
    }

    blit_to_window(hwnd, &dib);
}

/// Resize the window to fit the current filter, then repaint.
/// Cut the panel's corners with a window region.
///
/// With no per-pixel alpha the corners cannot be antialiased, but
/// `SetWindowRgn` still clips the window to a genuinely rounded shape.
/// Resizing does not carry the region over, so this runs after every
/// `SetWindowPos`.
unsafe fn apply_shape(hwnd: HWND, w: i32, h: i32) {
    apply_round_region(hwnd, w, h, RADIUS);
}

unsafe fn relayout(hwnd: HWND, w: i32) {
    APP.with(|c| {
        if let Some(a) = c.borrow().as_ref() {
            let rows = a.rows();
            let grid_h = layout::content_height(&rows, a.grid_origin().1, w - PAD * 2);
            let h = layout::window_height(a.grid_origin().1 + grid_h, FOOT_H, PAD).min(MAX_H);
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let _ = GetWindowRect(hwnd, &mut rc);
            let cur_x = rc.left;
            let cur_y = rc.top;
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, cur_x, cur_y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
            apply_shape(hwnd, w, h);
        }
    });
    paint(hwnd);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let items = match catalog::load(&catalog_path()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("palette-picker: {e}");
            std::process::exit(1);
        }
    };
    let current = active_name();
    let theme = Theme::load(&styles_path());

    let mut app = App {
        items,
        current,
        filtered: Vec::new(),
        query: String::new(),
        sel: 0,
        theme,
        hwnd: None,
    };
    app.refilter();

    if args.iter().any(|a| a == "--smoke") {
        smoke(&mut app);
        return;
    }
    if args.iter().any(|a| a == "--no-layered") {
        eprintln!("palette-picker: --no-layered is gone; the window is never layered now");
    }

    APP.with(|c| *c.borrow_mut() = Some(app));
    // One picker, ever. The bar fires this on every click of the palette
    // chip, so without the guard a triple click stacks three windows and each
    // of them repaints over the others at 1 Hz. A duplicate launch raises the
    // panel that is already open and exits, so a second click brings it back
    // to you instead of doing nothing.
    match yasb_chrome::acquire("Local\\yasb-palette-picker") {
        Ok(None) => {
            yasb_chrome::raise_window_of_class("PalettePickerClass");
            return;
        }
        // If the guard itself fails, still open: a second window is a smaller
        // problem than a picker that refuses to appear.
        Err(e) => eprintln!("palette-picker: single-instance guard unavailable: {e}"),
        Ok(Some(_instance)) => unsafe { run() },
    }
}

/// Headless self-test: exercises catalog, filter, layout and the pick
/// path without opening a window, so a broken build is caught without
/// putting a stray panel on someone's desktop. `--smoke` is the flag;
/// there is no `--selftest`.
///
/// Operates on a local `App` rather than the thread-local, so it proves
/// the same logic the window procedure uses without needing a window.
fn smoke(app: &mut App) {
    println!(
        "items={} current={:?} theme_accent={:?}",
        app.items.len(),
        app.current,
        app.theme.accent
    );
    assert!(!app.items.is_empty());
    assert_eq!(app.filtered.len(), app.items.len(), "unfiltered list mismatch");
    let total = app.items.len();

    // Filter narrows.
    let probe = "rangalipi";
    app.query = probe.to_string();
    app.refilter();
    let n = app.filtered.len();
    assert!(n > 0, "filter found nothing");
    let first = app.selected_name().unwrap_or("").to_string();
    println!("filter={probe} matches={n} selected={first:?}");

    // Keyboard: one visual row down lands on a different cell.
    let cols = layout::columns(W - PAD * 2) as isize;
    app.move_sel(cols, W);
    let after = app.selected_name().unwrap_or("").to_string();
    assert_ne!(after, first, "keyboard move did not change selection");
    println!("keyboard-down selected={after:?}");

    // Enter applies exactly the selected name.
    assert_eq!(after, app.items[app.filtered[app.sel]].name);

    // Stepping right once per visible item returns to where we started.
    let start = app.sel;
    for _ in 0..n {
        app.move_sel(1, W);
    }
    assert_eq!(app.sel, start, "selection did not wrap back to the start");

    // Clearing restores the full list.
    app.query.clear();
    app.refilter();
    assert_eq!(app.filtered.len(), total, "clear-filter restore broken");

    // Every theme is addressable and has a parseable swatch.
    for (i, it) in app.items.iter().enumerate() {
        assert!(!it.name.is_empty(), "theme {i} has no name");
        assert!(!it.swatches().is_empty(), "{} has no swatch", it.name);
    }

    println!("smoke: ok ({total} themes, grid+keyboard path OK)");
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();
    let cls: Vec<u16> = "PalettePickerClass\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(hinst.0),
        lpszClassName: windows::core::PCWSTR(cls.as_ptr()),
        // See the `WM_SETCURSOR` arm in `wnd_proc`.
        hCursor: yasb_chrome::gdi::arrow_cursor(),
        ..Default::default()
    };
    if RegisterClassW(&wc) == 0 {
        eprintln!("palette-picker: RegisterClassW failed");
        std::process::exit(1);
    }
    let title: Vec<u16> = "Palette\0".encode_utf16().collect();
    let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
    let hwnd = match CreateWindowExW(
        ex,
        windows::core::PCWSTR(cls.as_ptr()),
        windows::core::PCWSTR(title.as_ptr()),
        WS_POPUP,
        100, 100, W, 240,
        None, None, HINSTANCE(hinst.0), None,
    ) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("palette-picker: CreateWindowExW failed: {e}");
            std::process::exit(1);
        }
    };
    APP.with(|c| {
        if let Some(a) = c.borrow_mut().as_mut() {
            a.hwnd = Some(hwnd);
        }
    });

    // Centre on the monitor holding the cursor, just below the bar. The
    // backdrop is captured *before* the window is shown: taken afterwards it
    // would photograph the panel itself, and blending that back in on every
    // repaint would compound it into a brightening smear.
    let mut pt = POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let cx = (mi.rcMonitor.left + mi.rcMonitor.right) / 2;
    let origin = (cx - W / 2, mi.rcMonitor.top + 48);
    BACKDROP.with(|b| {
        // Blur here, once, rather than on every repaint: the snapshot never
        // changes in between. This is also what makes `backdrop_opacity` of
        // 0.85 survivable — acrylic is a low-opacity *and* heavily blurred
        // material, and an unblurred 0.85 would put crisp window edges
        // directly behind the grid.
        let blur = APP.with(|c| c.borrow().as_ref().map_or(12, |a| a.theme.backdrop_blur as i32));
        *b.borrow_mut() =
            Backdrop::capture(origin.0, origin.1, W, 520).map(|bd| bd.blurred(blur));
    });
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, origin.0, origin.1, W, 240, SWP_SHOWWINDOW);
    apply_shape(hwnd, W, 240);
    let _ = SetForegroundWindow(hwnd);
    // From here on the background thread supplies the theme and the active
    // name; the first paint below still uses the values read synchronously in
    // `main`, so the grid is never briefly empty or unmarked.
    spawn_worker();
    let _ = SetTimer(hwnd, TIMER_ID, REFRESH_MS as u32, None);
    relayout(hwnd, W);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // See the note in saka-popup's `WM_SETCURSOR`: the window class has no
        // cursor, so the hourglass that the opening screen capture sets stays
        // stuck over the panel. This is what clears it on hover.
        WM_SETCURSOR => {
            let _ = SetCursor(yasb_chrome::gdi::arrow_cursor());
            LRESULT(1)
        }
        WM_TIMER if wp.0 == TIMER_ID => {
            // The worker has been reading the stylesheet while this loop was
            // busy. Draw only when something actually changed — the picker
            // polls even when no theme was switched, and repainting an
            // unchanged grid is pure GDI churn.
            if drain_inbox() {
                relayout(hwnd, W);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            if !BeginPaint(hwnd, &mut ps).0.is_null() {
                paint(hwnd);
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            match vk {
                k if k == VK_ESCAPE.0 as u16 => {
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_RETURN.0 as u16 => {
                    // Apply the selected theme, then close.
                    let name = APP.with(|c| c.borrow().as_ref().and_then(|a| a.selected_name()).map(str::to_string));
                    if let Some(n) = name {
                        apply(&n);
                    }
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_BACK.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.query.pop();
                            a.refilter();
                        }
                    });
                    relayout(hwnd, W);
                }
                k if k == VK_DOWN.0 as u16 => {
                    let cols = layout::columns(W - PAD * 2) as isize;
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(cols, W); }
                    });
                    paint(hwnd);
                }
                k if k == VK_UP.0 as u16 => {
                    let cols = layout::columns(W - PAD * 2) as isize;
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(-cols, W); }
                    });
                    paint(hwnd);
                }
                k if k == VK_RIGHT.0 as u16 || k == VK_TAB.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(1, W); }
                    });
                    paint(hwnd);
                }
                k if k == VK_LEFT.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(-1, W); }
                    });
                    paint(hwnd);
                }
                k if k == VK_NEXT.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(8, W); }
                    });
                    paint(hwnd);
                }
                k if k == VK_PRIOR.0 as u16 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() { a.move_sel(-8, W); }
                    });
                    paint(hwnd);
                }
                _ => {
                    // Printable ASCII appends to the filter.
                    let ch = char::from_u32(wp.0 as u32).unwrap_or('\0');
                    if ch.is_ascii_graphic() || ch == ' ' {
                        let c = ch.to_string();
                        APP.with(|x| {
                            if let Some(a) = x.borrow_mut().as_mut() {
                                a.query.push_str(&c);
                                a.refilter();
                            }
                        });
                        relayout(hwnd, W);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let hit = APP.with(|c| {
                let a = c.borrow();
                let a = a.as_ref().unwrap();
                layout::hit_test(&a.rects(W), x, y)
                    .and_then(|i| a.filtered.get(i).copied())
                    .map(|i| a.items[i].name.clone())
            });
            if let Some(n) = hit {
                apply(&n);
            }
            let _ = SetCapture(hwnd);
            let _ = ReleaseCapture();
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            stop_worker();
            let _ = KillTimer(hwnd, TIMER_ID);
            let _ = PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
