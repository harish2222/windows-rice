//! Omarchy-style Keybindings cheatsheet popup with fuzzy search and Apple design.
//! Built with yasb-chrome and yasb-theme for seamless design integration.

mod layout;
mod model;
mod theme;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use model::Keybind;
use theme::Theme;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use yasb_chrome::Type;
use yasb_chrome::gdi::{
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect, text_width, tracked_text,
};

const CLASS_NAME: &str = "YasbKeybindClass";
const REFRESH_MS: u64 = 400;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static BACKDROP: RefCell<Option<Backdrop>> = const { RefCell::new(None) };
    static INBOX: RefCell<Option<mpsc::Receiver<Snapshot>>> = const { RefCell::new(None) };
    static STOP: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
struct Snapshot {
    theme: Theme,
}

fn take_snapshot() -> Snapshot {
    let styles = styles_path();
    Snapshot {
        theme: Theme::load(&styles),
    }
}

fn spawn_worker() {
    let (tx, rx) = mpsc::channel::<Snapshot>();
    INBOX.with(|i| *i.borrow_mut() = Some(rx));
    let stop = Arc::new(AtomicBool::new(false));
    STOP.with(|s| *s.borrow_mut() = Some(stop.clone()));
    std::thread::spawn(move || {
        if tx.send(take_snapshot()).is_err() {
            return;
        }
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(REFRESH_MS));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if tx.send(take_snapshot()).is_err() {
                break;
            }
        }
    });
}

fn drain_inbox() -> bool {
    let mut repaint = false;
    let mut latest: Option<Snapshot> = None;
    INBOX.with(|i| {
        if let Some(rx) = i.borrow().as_ref() {
            while let Ok(s) = rx.try_recv() {
                latest = Some(s);
            }
        }
    });
    if let Some(s) = latest {
        APP.with(|c| {
            if let Some(a) = c.borrow_mut().as_mut() {
                if a.theme != s.theme {
                    a.theme = s.theme;
                    repaint = true;
                }
            }
        });
    }
    repaint
}

#[derive(Clone)]
pub struct App {
    all: Vec<Keybind>,
    categories: Vec<&'static str>,
    selected_category: usize,
    filtered: Vec<usize>,
    query: String,
    sel: usize,
    scroll: usize,
    theme: Theme,
    hwnd: Option<HWND>,
}

impl App {
    pub fn new(styles_path: &Path) -> Self {
        let all = model::all_keybinds();
        let mut categories = vec!["All"];
        categories.extend(model::categories(&all));
        let filtered = (0..all.len()).collect();
        let theme = Theme::load(styles_path);
        Self {
            all,
            categories,
            selected_category: 0,
            filtered,
            query: String::new(),
            sel: 0,
            scroll: 0,
            theme,
            hwnd: None,
        }
    }

    pub fn refilter(&mut self) {
        let cat = self.categories[self.selected_category];
        self.filtered = model::filter_keybinds_with_category(&self.all, &self.query, Some(cat));
        self.sel = 0;
        self.scroll = 0;
    }

    pub fn set_category(&mut self, idx: usize) {
        if idx < self.categories.len() {
            self.selected_category = idx;
            self.refilter();
        }
    }

    pub fn next_category(&mut self) {
        self.selected_category = (self.selected_category + 1) % self.categories.len();
        self.refilter();
    }

    pub fn prev_category(&mut self) {
        let n = self.categories.len();
        self.selected_category = (self.selected_category + n - 1) % n;
        self.refilter();
    }

    pub fn move_sel(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as isize;
        self.sel = ((self.sel as isize + delta).rem_euclid(len)) as usize;
        self.scroll = layout::calculate_scroll(
            self.sel,
            self.scroll,
            self.filtered.len(),
            layout::VISIBLE_ROWS,
        );
    }

    pub fn scroll_by(&mut self, rows: isize) {
        let total = self.filtered.len();
        if total <= layout::VISIBLE_ROWS {
            self.scroll = 0;
            return;
        }
        let max_scroll = total - layout::VISIBLE_ROWS;
        let new_s = (self.scroll as isize - rows).clamp(0, max_scroll as isize);
        self.scroll = new_s as usize;
    }

    pub fn selected_cmd(&self) -> Option<&'static str> {
        self.filtered.get(self.sel).map(|&idx| self.all[idx].cmd)
    }

    pub fn category_count(&self, cat: &str) -> usize {
        if cat == "All" {
            self.all.len()
        } else {
            self.all.iter().filter(|k| k.category == cat).count()
        }
    }
}

fn styles_path() -> PathBuf {
    yasb_theme::default_styles_path()
}

fn execute_command(cmd: &str) {
    if cmd.is_empty() {
        return;
    }
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", "cmd", "/c", cmd])
        .spawn();
}

fn parse_keycaps(keys: &str) -> Vec<&str> {
    if keys.contains(" + ") {
        keys.split(" + ").collect()
    } else if keys.contains(" / ") {
        keys.split(" / ").collect()
    } else {
        vec![keys]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut app = App::new(&styles_path());

    if args.iter().any(|a| a == "--smoke") {
        smoke(&mut app);
        return;
    }

    APP.with(|c| *c.borrow_mut() = Some(app));

    match yasb_chrome::acquire("Local\\yasb-keybind") {
        Ok(None) => {
            yasb_chrome::raise_window_of_class(CLASS_NAME);
        }
        Err(e) => eprintln!("keybind: single-instance guard error: {e}"),
        Ok(Some(_instance)) => unsafe { run() },
    }
}

fn smoke(app: &mut App) {
    println!("keybind smoke: total={}", app.all.len());
    assert!(!app.all.is_empty());
    app.query = "focus".into();
    app.refilter();
    assert!(!app.filtered.is_empty());
    println!("keybind smoke: filter 'focus' matches={}", app.filtered.len());
    app.query.clear();
    app.set_category(1); // Navigation
    assert_eq!(app.filtered.len(), 8);
    println!("keybind smoke: category Navigation matches={}", app.filtered.len());
    println!("smoke: ok");
}

unsafe fn run() {
    let hinst = GetModuleHandleW(None).unwrap_or_default();
    let cls: Vec<u16> = format!("{CLASS_NAME}\0").encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(hinst.0),
        lpszClassName: windows::core::PCWSTR(cls.as_ptr()),
        hCursor: yasb_chrome::gdi::arrow_cursor(),
        ..Default::default()
    };
    if RegisterClassW(&wc) == 0 {
        eprintln!("keybind: RegisterClassW failed");
        std::process::exit(1);
    }

    let title: Vec<u16> = "Keyboard Shortcuts\0".encode_utf16().collect();
    let total_rows = APP.with(|c| c.borrow().as_ref().map_or(layout::VISIBLE_ROWS, |a| a.filtered.len()));
    let h = layout::total_height(total_rows);
    let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW;

    let hwnd = match CreateWindowExW(
        ex,
        windows::core::PCWSTR(cls.as_ptr()),
        windows::core::PCWSTR(title.as_ptr()),
        WS_POPUP,
        100,
        100,
        layout::W,
        h,
        None,
        None,
        HINSTANCE(hinst.0),
        None,
    ) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("keybind: CreateWindowExW failed: {e}");
            std::process::exit(1);
        }
    };

    APP.with(|c| {
        if let Some(a) = c.borrow_mut().as_mut() {
            a.hwnd = Some(hwnd);
        }
    });

    let mut pt = POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut pt);
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(mon, &mut mi);
    let cx = (mi.rcMonitor.left + mi.rcMonitor.right) / 2;
    let cy = (mi.rcMonitor.top + mi.rcMonitor.bottom) / 2;
    let x = cx - layout::W / 2;
    let y = (cy - h / 2).max(mi.rcMonitor.top + 32);

    BACKDROP.with(|b| {
        let blur = APP.with(|c| c.borrow().as_ref().map_or(12, |a| a.theme.backdrop_blur as i32));
        *b.borrow_mut() = Backdrop::capture(x, y, layout::W, h).map(|bd| bd.blurred(blur));
    });

    apply_round_region(hwnd, layout::W, h, layout::RADIUS);
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, layout::W, h, SWP_SHOWWINDOW);
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(hwnd);

    spawn_worker();
    let _ = SetTimer(hwnd, 1, REFRESH_MS as u32, None);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_ACTIVATE => {
            if (wp.0 & 0xFFFF) == 0 {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let _ = SetCursor(yasb_chrome::gdi::arrow_cursor());
            LRESULT(1)
        }
        WM_TIMER => {
            if wp.0 == 1 && drain_inbox() {
                paint(hwnd);
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
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) as i16) as i32;
            let rows_to_scroll = (delta / 120) * 2;
            APP.with(|c| {
                if let Some(a) = c.borrow_mut().as_mut() {
                    a.scroll_by(rows_to_scroll as isize);
                }
            });
            paint(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            match vk {
                k if k == VK_ESCAPE.0 => {
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_RETURN.0 => {
                    let cmd = APP.with(|c| {
                        c.borrow()
                            .as_ref()
                            .and_then(|a| a.selected_cmd())
                            .unwrap_or("")
                    });
                    if !cmd.is_empty() {
                        execute_command(cmd);
                    }
                    let _ = DestroyWindow(hwnd);
                }
                k if k == VK_TAB.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.next_category();
                        }
                    });
                    relayout(hwnd);
                }
                k if k == VK_LEFT.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.prev_category();
                        }
                    });
                    relayout(hwnd);
                }
                k if k == VK_RIGHT.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.next_category();
                        }
                    });
                    relayout(hwnd);
                }
                k if k == VK_BACK.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.query.pop();
                            a.refilter();
                        }
                    });
                    relayout(hwnd);
                }
                k if k == VK_DOWN.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.move_sel(1);
                        }
                    });
                    paint(hwnd);
                }
                k if k == VK_UP.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.move_sel(-1);
                        }
                    });
                    paint(hwnd);
                }
                k if k == VK_NEXT.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.move_sel(5);
                        }
                    });
                    paint(hwnd);
                }
                k if k == VK_PRIOR.0 => {
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.move_sel(-5);
                        }
                    });
                    paint(hwnd);
                }
                _ => {
                    let ch = char::from_u32(wp.0 as u32).unwrap_or('\0');
                    if ch.is_ascii_graphic() || ch == ' ' {
                        let s = ch.to_string();
                        APP.with(|c| {
                            if let Some(a) = c.borrow_mut().as_mut() {
                                a.query.push_str(&s);
                                a.refilter();
                            }
                        });
                        relayout(hwnd);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;

            // Search clear button
            if layout::hit_search_clear(layout::W, layout::PAD, layout::SEARCH_H, x, y) {
                APP.with(|c| {
                    if let Some(a) = c.borrow_mut().as_mut() {
                        if !a.query.is_empty() {
                            a.query.clear();
                            a.refilter();
                        }
                    }
                });
                relayout(hwnd);
                return LRESULT(0);
            }

            // Click on sidebar category
            let (sx0, sy0, sx1, sy1) = layout::sidebar_rect(layout::PAD, layout::SEARCH_H);
            if x >= sx0 && x <= sx1 && y >= sy0 && y <= sy1 {
                let cat_y_start = sy0 + 20;
                if y >= cat_y_start {
                    let cat_idx = ((y - cat_y_start) / 32) as usize;
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            a.set_category(cat_idx);
                        }
                    });
                    relayout(hwnd);
                    return LRESULT(0);
                }
            }

            // Click on shortcut row in right list
            let (lx0, ly0, lx1, ly1) = layout::list_rect(layout::W, layout::PAD, layout::SEARCH_H);
            let rows_y_start = ly0 + 20;
            if x >= lx0 && x <= lx1 - 10 && y >= rows_y_start && y <= ly1 {
                let row_idx = ((y - rows_y_start) / (layout::ROW_H + layout::GAP)) as usize;
                APP.with(|c| {
                    if let Some(a) = c.borrow_mut().as_mut() {
                        let actual_idx = a.scroll + row_idx;
                        if actual_idx < a.filtered.len() {
                            a.sel = actual_idx;
                            let cmd = a.all[a.filtered[actual_idx]].cmd;
                            if !cmd.is_empty() {
                                execute_command(cmd);
                            }
                        }
                    }
                });
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }

            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, 1);
            STOP.with(|s| {
                if let Some(flag) = s.borrow().as_ref() {
                    flag.store(true, Ordering::Relaxed);
                }
            });
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn relayout(hwnd: HWND) {
    let h = APP.with(|c| {
        let a = c.borrow();
        let a = a.as_ref().unwrap();
        layout::total_height(a.filtered.len())
    });
    let mut r = RECT::default();
    let _ = GetWindowRect(hwnd, &mut r);
    apply_round_region(hwnd, layout::W, h, layout::RADIUS);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        r.left,
        r.top,
        layout::W,
        h,
        SWP_NOMOVE | SWP_NOZORDER,
    );
    paint(hwnd);
}

unsafe fn paint(hwnd: HWND) {
    let Some(app) = APP.with(|c| c.borrow().clone()) else {
        return;
    };
    let mut rc_win = RECT::default();
    if GetWindowRect(hwnd, &mut rc_win).is_err() {
        return;
    }
    let w = rc_win.right - rc_win.left;
    let h = rc_win.bottom - rc_win.top;
    if w <= 0 || h <= 0 {
        return;
    }

    let mut dib = Dib::new(w, h);
    render(&mut dib, &app, w, h);
    blit_to_window(hwnd, &dib);
}

fn render(dib: &mut Dib, app: &App, w: i32, h: i32) {
    let t = &app.theme;
    let total_filtered = app.filtered.len();
    let (fx0, fy0, fx1, fy1) = layout::search_box(w, layout::PAD, layout::SEARCH_H);
    let (sx0, sy0, sx1, sy1) = layout::sidebar_rect(layout::PAD, layout::SEARCH_H);
    let (lx0, ly0, lx1, _ly1) = layout::list_rect(w, layout::PAD, layout::SEARCH_H);
    let list_rows_y = ly0 + 20;

    // ---- shape pass ----
    {
        let mut c = dib.canvas();
        c.clear(t.bg.over(yasb_theme::Rgba::rgb(20, 20, 28)));
        BACKDROP.with(|b| {
            if let Some(bd) = b.borrow().as_ref() {
                bd.draw_under(&mut c, 0, 0, t.backdrop_opacity);
            }
        });
        c.v_gradient(0, 0, w, h, t.sheen_top, t.sheen_bottom);

        // Radiant subtle glow behind search field
        c.radial_glow(
            w as f32 * 0.5,
            (fy0 + layout::SEARCH_H / 2) as f32,
            w as f32 * 0.75,
            t.apple_accent,
            t.bloom,
        );

        // Outer window border
        c.round_rect_border(1, 1, w - 1, h - 1, layout::RADIUS, 1, t.subtle_border);
        c.round_rect_border(1, 1, w - 1, 14, layout::RADIUS, 1, t.highlight);

        // Inset search field
        c.round_rect(fx0, fy0, fx1, fy1, 10, t.field);
        c.round_rect_border(fx0, fy0, fx1, fy1, 10, 1, t.subtle_border);

        // Magnifier vector glyph
        let gcx = fx0 as f32 + 20.0;
        let gcy = (fy0 + fy1) as f32 / 2.0;
        for py in (gcy as i32 - 9)..=(gcy as i32 + 9) {
            for px in (gcx as i32 - 9)..=(gcx as i32 + 9) {
                let dx = px as f32 + 0.5 - gcx;
                let dy = py as f32 + 0.5 - gcy;
                let d = (dx * dx + dy * dy).sqrt() - 5.0;
                let cov = (0.5f32 - d.abs() / 1.7).clamp(0.0, 1.0);
                if cov > 0.0 {
                    c.blend_pixel(px, py, t.subtext, cov);
                }
            }
        }
        for i in 0..7 {
            let hx = gcx + 4.5 + i as f32 * 0.75;
            let hy = gcy + 4.5 + i as f32 * 0.75;
            for (dx, dy) in [(0i32, 0i32), (1, 0), (0, 1), (1, 1)] {
                c.blend_pixel(hx as i32 + dx, hy as i32 + dy, t.subtext, 0.85);
            }
        }

        // Left sidebar background pane
        c.round_rect(sx0, sy0, sx1, sy1, 10, t.field);
        c.round_rect_border(sx0, sy0, sx1, sy1, 10, 1, t.hairline);

        // Category pills inside sidebar
        let cat_y_start = sy0 + 20;
        for (i, _) in app.categories.iter().enumerate() {
            let cy0 = cat_y_start + i as i32 * 32;
            let cy1 = cy0 + 26;
            if cy1 > sy1 - 4 {
                break;
            }
            if i == app.selected_category {
                c.round_rect(sx0 + 6, cy0, sx1 - 6, cy1, 6, t.elevated_surface);
                c.round_rect_border(sx0 + 6, cy0, sx1 - 6, cy1, 6, 1, t.subtle_border);
            }
        }

        // Right pane shortcut rows
        let visible_count = total_filtered
            .saturating_sub(app.scroll)
            .min(layout::VISIBLE_ROWS);

        let rows_w = lx1 - 10;
        for i in 0..visible_count {
            let actual_idx = app.scroll + i;
            let ry = layout::row_y(list_rows_y, i);
            let selected = actual_idx == app.sel;

            let rx0 = lx0;
            let rx1 = rows_w;
            let ry0 = ry;
            let ry1 = ry + layout::ROW_H;

            let fill = if selected {
                t.row_selected
            } else {
                t.row_bg
            };
            c.round_rect(rx0, ry0, rx1, ry1, 8, fill);
            if selected {
                c.round_rect_border(rx0, ry0, rx1, ry1, 8, 1, t.apple_accent);
            } else {
                c.round_rect_border(rx0, ry0, rx1, ry1, 8, 1, t.subtle_border);
            }
        }

        // Scrollbar
        let rows_total_h = (layout::ROW_H + layout::GAP) * layout::VISIBLE_ROWS as i32 - layout::GAP;
        if let Some((track, thumb)) = layout::scrollbar_rects(
            w,
            list_rows_y,
            rows_total_h,
            total_filtered,
            layout::VISIBLE_ROWS,
            app.scroll,
        ) {
            c.round_rect(track.0, track.1, track.2, track.3, 3, t.scrollbar_track);
            c.round_rect(thumb.0, thumb.1, thumb.2, thumb.3, 3, t.scrollbar_thumb);
        }

        // Footer separator rule
        let footer_rule_y = h - layout::FOOT_H - 8;
        c.hline(layout::PAD, w - layout::PAD, footer_rule_y, t.hairline);
    }

    // ---- text pass ----
    unsafe {
        let dc = dib.dc();
        let title_font = font(Type::Title.gdi(), 700, &t.typeface.display);
        let body = font(Type::Body.gdi(), 500, &t.typeface.family);
        let mono = font(Type::Body.gdi(), 600, yasb_chrome::GOHU_FAMILY);
        let small = font(Type::Foot.gdi(), 400, &t.typeface.family);
        let label = font(Type::Label.gdi(), 600, &t.typeface.family);
        SetBkMode(dc, TRANSPARENT);

        // Header: Title & Subtitle
        SelectObject(dc, HGDIOBJ(title_font.0));
        SetTextColor(dc, colorref(t.text));
        draw_text(
            dc,
            "Keyboard Shortcuts",
            rect(layout::PAD + 2, layout::PAD + 2, 280, 24),
            DT_LEFT | DT_VCENTER,
        );

        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        draw_text(
            dc,
            "Search commands, apps, and window shortcuts",
            rect(layout::PAD + 2, layout::PAD + 24, 320, 16),
            DT_LEFT | DT_VCENTER,
        );

        // Search text
        let text_x = fx0 + 38;
        let empty = app.query.is_empty();
        let avail_search = (fx1 - text_x - 36).max(20);
        SelectObject(dc, HGDIOBJ(body.0));
        SetTextColor(dc, colorref(if empty { t.subtext } else { t.text }));
        draw_text(
            dc,
            if empty {
                "Search shortcuts (e.g. focus, terminal, alt+f, resize, workspace)..."
            } else {
                &app.query
            },
            rect(text_x, fy0, avail_search, layout::SEARCH_H),
            DT_LEFT | DT_VCENTER,
        );

        if !empty {
            SelectObject(dc, HGDIOBJ(label.0));
            SetTextColor(dc, colorref(t.subtext));
            draw_text(
                dc,
                "×",
                rect(fx1 - 30, fy0, 20, layout::SEARCH_H),
                DT_CENTER | DT_VCENTER,
            );
        }

        // Sidebar categories text
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.subtext));
        tracked_text(dc, "CATEGORIES", sx0 + 10, sy0 + 4, 1);

        let cat_y_start = sy0 + 20;
        for (i, &cat) in app.categories.iter().enumerate() {
            let cy0 = cat_y_start + i as i32 * 32;
            let cy1 = cy0 + 26;
            if cy1 > sy1 - 4 {
                break;
            }
            let is_sel = i == app.selected_category;

            // Category name
            SelectObject(dc, HGDIOBJ(small.0));
            SetTextColor(dc, colorref(if is_sel { t.apple_accent } else { t.subtext }));
            draw_text(
                dc,
                cat,
                rect(sx0 + 12, cy0, layout::SIDEBAR_W - 46, 26),
                DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            );

            // Category item count
            let cnt_str = format!("{}", app.category_count(cat));
            SetTextColor(dc, colorref(t.muted));
            draw_text(
                dc,
                &cnt_str,
                rect(sx1 - 36, cy0, 26, 26),
                DT_RIGHT | DT_VCENTER,
            );
        }

        // Right pane: category header
        SelectObject(dc, HGDIOBJ(label.0));
        SetTextColor(dc, colorref(t.subtext));
        let active_cat_name = app.categories[app.selected_category].to_uppercase();
        tracked_text(dc, &active_cat_name, lx0 + 4, ly0 + 4, 1);

        let count_str = format!("{} ITEMS", total_filtered);
        draw_text(
            dc,
            &count_str,
            rect(lx1 - 100, ly0 + 2, 90, 16),
            DT_RIGHT | DT_VCENTER,
        );

        // Right pane: rows
        if app.filtered.is_empty() {
            SelectObject(dc, HGDIOBJ(body.0));
            SetTextColor(dc, colorref(t.text));
            draw_text(
                dc,
                "No shortcuts found",
                rect(lx0, ly0 + 80, lx1 - lx0, 26),
                DT_CENTER | DT_VCENTER,
            );
            SelectObject(dc, HGDIOBJ(small.0));
            SetTextColor(dc, colorref(t.muted));
            draw_text(
                dc,
                "Try searching by chord (e.g. Alt, Win) or action name",
                rect(lx0, ly0 + 110, lx1 - lx0, 18),
                DT_CENTER | DT_VCENTER,
            );
        } else {
            let visible_count = total_filtered
                .saturating_sub(app.scroll)
                .min(layout::VISIBLE_ROWS);

            for i in 0..visible_count {
                let actual_idx = app.scroll + i;
                let item_idx = app.filtered[actual_idx];
                let item = &app.all[item_idx];
                let selected = actual_idx == app.sel;
                let ry = layout::row_y(list_rows_y, i);
                let rx0 = lx0;
                let rx1 = lx1 - 10;

                // Render keycaps
                let key_tokens = parse_keycaps(item.keys);
                let mut cur_kx = rx0 + 12;
                let keycap_y = ry + 8;
                let keycap_h = 26;

                for (ki, token) in key_tokens.iter().enumerate() {
                    SelectObject(dc, HGDIOBJ(mono.0));
                    let tw = text_width(dc, token).max(18);
                    let kw = tw + 14;

                    // Draw keycap pill into DIB canvas directly
                    let mut c = dib.canvas();
                    c.round_rect(cur_kx, keycap_y, cur_kx + kw, keycap_y + keycap_h, 5, t.keycap_bg);
                    c.round_rect_border(
                        cur_kx,
                        keycap_y,
                        cur_kx + kw,
                        keycap_y + keycap_h,
                        5,
                        1,
                        if selected { t.apple_accent } else { t.keycap_border },
                    );

                    // Text inside keycap
                    SetTextColor(dc, colorref(if selected { t.apple_accent } else { t.text }));
                    draw_text(
                        dc,
                        token,
                        rect(cur_kx, keycap_y, kw, keycap_h),
                        DT_CENTER | DT_VCENTER,
                    );

                    cur_kx += kw + 4;
                    if ki + 1 < key_tokens.len() {
                        SelectObject(dc, HGDIOBJ(small.0));
                        SetTextColor(dc, colorref(t.muted));
                        let sep = if item.keys.contains(" / ") { "/" } else { "+" };
                        draw_text(
                            dc,
                            sep,
                            rect(cur_kx, keycap_y, 10, keycap_h),
                            DT_CENTER | DT_VCENTER,
                        );
                        cur_kx += 12;
                    }
                }

                // Description text aligned to column
                let desc_x = rx0 + 200;
                let desc_w = (rx1 - desc_x - 120).max(60);
                SelectObject(dc, HGDIOBJ(body.0));
                SetTextColor(dc, colorref(t.text));
                draw_text(
                    dc,
                    item.desc,
                    rect(desc_x, ry, desc_w, layout::ROW_H),
                    DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                );

                // Category pill on right
                let tag_w = 100;
                let tag_x = rx1 - tag_w - 8;
                SelectObject(dc, HGDIOBJ(small.0));
                SetTextColor(dc, colorref(if selected { t.apple_accent } else { t.muted }));
                draw_text(
                    dc,
                    item.category,
                    rect(tag_x, ry, tag_w, layout::ROW_H),
                    DT_RIGHT | DT_VCENTER,
                );
            }
        }

        // Footer status text
        let footer_y = h - layout::FOOT_H - 4;
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        let footer_left = format!("{} shortcuts", total_filtered);
        draw_text(
            dc,
            &footer_left,
            rect(layout::PAD, footer_y, 160, layout::FOOT_H),
            DT_LEFT | DT_VCENTER,
        );

        draw_text(
            dc,
            "↑↓ Navigate   Tab Category   Enter Run   Esc Close",
            rect(w - layout::PAD - 360, footer_y, 360, layout::FOOT_H),
            DT_RIGHT | DT_VCENTER,
        );

        let _ = DeleteObject(HGDIOBJ(title_font.0));
        let _ = DeleteObject(HGDIOBJ(body.0));
        let _ = DeleteObject(HGDIOBJ(mono.0));
        let _ = DeleteObject(HGDIOBJ(small.0));
        let _ = DeleteObject(HGDIOBJ(label.0));
    }
}
