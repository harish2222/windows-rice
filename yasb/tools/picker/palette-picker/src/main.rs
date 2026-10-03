//! palette-picker — the bar's graphical theme picker, in Rust.
//!
//! A dmenu-style palette switcher: a filter line on top, a swatch grid
//! below, type to narrow, arrows/Enter to apply, click to apply. It is the
//! Rust replacement for `palette-picker.py`, which stays in the tree as a
//! documented fallback.
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

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_BACK, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_NEXT, VK_PRIOR,
    VK_RETURN, VK_RIGHT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use catalog::Item;
use layout::Row;
use theme::{Rgba, Theme};

const W: i32 = 680;

/// Set by `--debug-paint`; makes the paint path report each step so a
/// blank window can be diagnosed without guessing.
static DEBUG_PAINT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set by `--no-layered`: paints through an ordinary blit instead of
/// `UpdateLayeredWindow`, so the panel becomes a normal capturable
/// window. Only for verifying layout on a machine where screen capture
/// cannot see layered windows; the real picker always layers so it can
/// have per-pixel alpha.
static NO_LAYERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const PAD: i32 = 12;
const SEARCH_H: i32 = 30;
const FOOT_H: i32 = 22;
const MAX_H: i32 = 420;

thread_local! {
    /// The single global the window procedure reads and writes.
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
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

    /// Client-space origin of the first cell row, below the filter line.
    fn grid_origin(&self) -> (i32, i32) {
        (PAD, PAD + SEARCH_H + 8)
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

/// Widest visible name, so the font can be sized to fit the whole grid.
fn longest_visible_name(app: &App) -> String {
    app.filtered
        .iter()
        .map(|&i| app.items[i].name.as_str())
        .max_by_key(|n| n.chars().count())
        .unwrap_or("")
        .to_string()
}

/// Smallest font size (within a sane range) whose rendering of the
/// longest visible name still fits a cell. Measuring with
/// `GetTextExtentPoint32W` under the actual font is reliable in a way
/// that guessing from character counts is not.
unsafe fn fit_name_font(dc: HDC, app: &App) -> HFONT {
    let longest = longest_visible_name(app);
    // Widest cell's available text width: cell - left pad - swatches - gap.
    let swatch_w = 3 * 14 + 8;
    let avail = layout::CELL_W - 8 - swatch_w - 4 - 6;
    for size in [-11i32, -12, -13, -14] {
        let f = make_font(size, 600);
        let old = SelectObject(dc, HGDIOBJ(f.0));
        let mut buf: Vec<u16> = longest.encode_utf16().collect();
        buf.push(0);
        let mut sz = SIZE { cx: 0, cy: 0 };
        let got = GetTextExtentPoint32W(dc, &buf, &mut sz).as_bool();
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(f.0));
        if got && sz.cx <= avail {
            return make_font(size, 600);
        }
    }
    make_font(-11, 600)
}

/// Build a UI font at a given pixel height and weight.
unsafe fn make_font(size: i32, weight: i32) -> HFONT {
    let family: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
    CreateFontW(
        size, 0, 0, 0, weight, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    )
}

fn color(c: Rgba) -> COLORREF {
    COLORREF(c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16))
}

unsafe fn fill(dc: HDC, r: RECT, c: Rgba) {
    let b = CreateSolidBrush(color(c));
    let _ = FillRect(dc, &r, HBRUSH(b.0));
    let _ = DeleteObject(HGDIOBJ(b.0));
}

/// Rounded-ish cell: Win32 has no cheap rounded fill in GDI, so the
/// selection is drawn as a filled rect plus an accent border, which reads
/// cleanly at this size and matches the flat bar aesthetic.
unsafe fn draw_cell(dc: HDC, x: i32, y: i32, selected: bool, is_current: bool, t: &Theme) {
    let r = RECT { left: x, top: y, right: x + layout::CELL_W, bottom: y + layout::CELL_H };
    if selected {
        fill(dc, r, t.surface);
        let pen = CreatePen(PS_SOLID, 1, color(t.accent));
        let old = SelectObject(dc, HGDIOBJ(pen.0));
        let old_br = SelectObject(dc, GetStockObject(NULL_BRUSH));
        FrameRect(dc, &r, HBRUSH(pen.0));
        SelectObject(dc, old_br);
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    } else {
        let pen = CreatePen(PS_SOLID, 1, color(t.border));
        let old = SelectObject(dc, HGDIOBJ(pen.0));
        let old_br = SelectObject(dc, GetStockObject(NULL_BRUSH));
        FrameRect(dc, &r, HBRUSH(pen.0));
        SelectObject(dc, old_br);
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
    let _ = is_current;
}

/// Paint the whole window into a 32-bit DIB and hand it to
/// `UpdateLayeredWindow` for per-pixel alpha, as in saka-popup.
unsafe fn paint(hwnd: HWND) {
    let Some(app) = APP.with(|c| c.borrow().clone()) else { return };
    let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if GetWindowRect(hwnd, &mut rc).is_err() {
        return;
    }
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
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
            biHeight: -h,
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
        eprintln!("debug: CreateDIBSection failed");
        return;
    };
    let dbg = DEBUG_PAINT.load(std::sync::atomic::Ordering::Relaxed);
    if dbg {
        eprintln!("debug: window {w}x{h} rect=({}..{}, {}..{}) bits={:p}",
            rc.left, rc.right, rc.top, rc.bottom, bits);
    }
    let old_bmp = SelectObject(dc_mem, HGDIOBJ(bmp.0));

    // Panel fill.
    fill(dc_mem, RECT { left: 0, top: 0, right: w, bottom: h }, app.theme.bg);

    // Fonts.
    let family: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
    let font = CreateFontW(
        -15, 0, 0, 0, 400, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    );
    let small = CreateFontW(
        -12, 0, 0, 0, 400, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    );
    let bold = CreateFontW(
        -14, 0, 0, 0, 600, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        windows::core::PCWSTR(family.as_ptr()),
    );
    let old_font = SelectObject(dc_mem, HGDIOBJ(font.0));
    SetBkMode(dc_mem, TRANSPARENT);

    // Filter line: typed text, or the placeholder hint.
    let hint = "palette — type to filter, arrows to move, enter to apply";
    let (text, muted) = if app.query.is_empty() {
        (hint.to_string(), true)
    } else {
        (app.query.clone(), false)
    };
    SetTextColor(dc_mem, color(if muted { app.theme.subtext } else { app.theme.text }));
    draw_text(dc_mem, &text, PAD, PAD, w - PAD * 2);

    // Divider under the filter line.
    fill(
        dc_mem,
        RECT { left: PAD, top: PAD + SEARCH_H - 6, right: w - PAD, bottom: PAD + SEARCH_H - 5 },
        app.theme.border,
    );

    // Grid. One font size for every name, sized to the longest visible name
    // so nothing is clipped and no cell differs from its neighbour.
    let name_font = unsafe { fit_name_font(dc_mem, &app) };
    let old_name_font = SelectObject(dc_mem, HGDIOBJ(name_font.0));

    let rows = app.rows();
    let rects = layout::cell_rects(&rows, app.grid_origin(), w - PAD * 2);
    let mut cell_i = 0usize;
    let mut y = app.grid_origin().1;
    for row in &rows {
        match row {
            Row::Header(label) => {
                SelectObject(dc_mem, HGDIOBJ(small.0));
                SetTextColor(dc_mem, color(app.theme.subtext));
                draw_text(dc_mem, label, PAD, y, w - PAD * 2);
                y += layout::CELL_H;
            }
            Row::Cell(ci) => {
                let (x, cy) = rects[cell_i];
                let item = &app.items[*ci];
                let selected = app.filtered.get(app.sel) == Some(ci);
                let is_current = item.name == app.current;
                draw_cell(dc_mem, x, cy, selected, is_current, &app.theme);

                // Swatches.
                let mut sx = x + 8;
                for c in item.swatches() {
                    fill(dc_mem, RECT { left: sx, top: cy + 8, right: sx + 11, bottom: cy + 19 }, c);
                    sx += 14;
                }

                // Name; the current theme is painted in the accent. Shrink the font
                // rather than clip the text: the longest names
                // ("Rangalipi Mossfern Light") do not fit at body size.
                let name_x = sx + 4;
                let name_w = x + layout::CELL_W - name_x - 6;
                SetTextColor(dc_mem, color(if is_current { app.theme.accent } else { app.theme.text }));
                draw_text(dc_mem, &item.name, name_x, cy + 6, name_w);

                y = cy + layout::CELL_H + layout::GAP;
                cell_i += 1;
            }
        }
    }

    // Footer hint when a filter is active.
    if !app.filtered.is_empty() && !app.query.is_empty() {
        SelectObject(dc_mem, HGDIOBJ(small.0));
        SetTextColor(dc_mem, color(app.theme.subtext));
        let msg = format!("{} of {} themes", app.filtered.len(), app.items.len());
        draw_text(dc_mem, &msg, PAD, h - FOOT_H, w - PAD * 2);
    }

    SelectObject(dc_mem, old_font);
    SelectObject(dc_mem, old_name_font);
    let _ = DeleteObject(HGDIOBJ(font.0));
    let _ = DeleteObject(HGDIOBJ(small.0));
    let _ = DeleteObject(HGDIOBJ(bold.0));
    let _ = DeleteObject(HGDIOBJ(name_font.0));

    // GDI left every alpha byte at0, which would make UpdateLayeredWindow
    // render the whole panel transparent.
    yasb_theme::force_opaque_alpha(bits.cast::<u8>(), w, h);

    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let ulw = if NO_LAYERED.load(std::sync::atomic::Ordering::Relaxed) {
        // Debug path: blit straight to the window so it is capturable.
        let dc_win = GetWindowDC(hwnd);
        let _ = BitBlt(dc_win, 0, 0, w, h, dc_mem, 0, 0, SRCCOPY);
        let _ = ReleaseDC(hwnd, dc_win);
        Ok(())
    } else {
        UpdateLayeredWindow(
            hwnd, dc_screen, None, None, dc_mem, Some(&src), COLORREF(0), Some(&blend),
            ULW_ALPHA,
        )
    };
    if dbg {
        // Sample the top-left, centre and bottom-right pixels of the DIB we
        // just handed over: proves whether anything was actually drawn.
        let px = |x: i32, y: i32| -> (u8, u8, u8, u8) {
            let o = ((y * w + x) * 4) as usize;
            unsafe {
                let p = bits.cast::<u8>().add(o);
                (*p, *p.add(1), *p.add(2), *p.add(3))
            }
        };
        eprintln!("debug: dib px(2,2)={:?} px({},{})={:?} px({},{})={:?}",
            px(2, 2), w / 2, h / 2, px(w / 2, h / 2), w - 3, h - 3, px(w - 3, h - 3));
        eprintln!("debug: UpdateLayeredWindow -> {ulw:?}");
    }

    SelectObject(dc_mem, old_bmp);
    let _ = DeleteObject(HGDIOBJ(bmp.0));
    let _ = DeleteDC(dc_mem);
    let _ = DeleteDC(dc_screen);
}

unsafe fn draw_text(dc: HDC, text: &str, x: i32, y: i32, max_w: i32) {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    buf.push(0);
    let mut rc = RECT { left: x, top: y, right: x + max_w, bottom: y + 400 };
    DrawTextW(dc, &mut buf, &mut rc, DT_LEFT | DT_TOP | DT_NOPREFIX | DT_SINGLELINE);
}

/// Resize the window to fit the current filter, then repaint.
unsafe fn relayout(hwnd: HWND, w: i32) {
    APP.with(|c| {
        if let Some(a) = c.borrow().as_ref() {
            let rows = a.rows();
            let grid_h = layout::content_height(&rows, a.grid_origin().1, w - PAD * 2);
            // The footer hint is only drawn while a filter is active, so reserve
            // its height only then; otherwise the window ends up taller
            // than its content with a gap under the last row.
            let foot = if a.query.is_empty() { 0 } else { FOOT_H };
            let h = (a.grid_origin().1 + grid_h + PAD + foot).min(MAX_H);
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let _ = GetWindowRect(hwnd, &mut rc);
            let cur_x = rc.left;
            let cur_y = rc.top;
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, cur_x, cur_y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
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
    if args.iter().any(|a| a == "--debug-paint") {
        DEBUG_PAINT.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if args.iter().any(|a| a == "--no-layered") {
        NO_LAYERED.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    APP.with(|c| *c.borrow_mut() = Some(app));
    unsafe { run() }
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
        ..Default::default()
    };
    if RegisterClassW(&wc) == 0 {
        eprintln!("palette-picker: RegisterClassW failed");
        std::process::exit(1);
    }
    let title: Vec<u16> = "Palette\0".encode_utf16().collect();
    let ex = if NO_LAYERED.load(std::sync::atomic::Ordering::Relaxed) {
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW
    } else {
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED
    };
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

    // Centre on the monitor holding the cursor, just below the bar.
    let mut pt = POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let cx = (mi.rcMonitor.left + mi.rcMonitor.right) / 2;
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, cx - W / 2, mi.rcMonitor.top + 48, W, 240, SWP_SHOWWINDOW);
    let _ = SetForegroundWindow(hwnd);
    relayout(hwnd, W);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
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
            let _ = PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
