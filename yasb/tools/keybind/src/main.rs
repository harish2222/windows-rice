//! Omarchy-style Keybindings cheatsheet popup with fuzzy search.
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
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect,
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
        let filtered = (0..all.len()).collect();
        let theme = Theme::load(styles_path);
        Self {
            all,
            filtered,
            query: String::new(),
            sel: 0,
            scroll: 0,
            theme,
            hwnd: None,
        }
    }

    pub fn refilter(&mut self) {
        self.filtered = model::filter_keybinds(&self.all, &self.query);
        self.sel = 0;
        self.scroll = 0;
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

    let title: Vec<u16> = "Keybindings\0".encode_utf16().collect();
    let total_rows = APP.with(|c| c.borrow().as_ref().map_or(11, |a| a.filtered.len()));
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

    // Center on monitor with cursor
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
            let origin_y = layout::PAD + layout::SEARCH_H + 12;

            let clicked_row = if y >= origin_y && x >= layout::PAD && x <= layout::W - layout::PAD - 10 {
                let row_idx = ((y - origin_y) / (layout::ROW_H + layout::GAP)) as usize;
                Some(row_idx)
            } else {
                None
            };

            if let Some(row_idx) = clicked_row {
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
            } else if x >= layout::W - layout::PAD - 8 && y >= origin_y {
                // Clicked on scrollbar area - page scroll
                let total_h = (layout::ROW_H + layout::GAP) * layout::VISIBLE_ROWS as i32;
                if y <= origin_y + total_h {
                    let rel_y = y - origin_y;
                    let frac = (rel_y as f32 / total_h as f32).clamp(0.0, 1.0);
                    APP.with(|c| {
                        if let Some(a) = c.borrow_mut().as_mut() {
                            if a.filtered.len() > layout::VISIBLE_ROWS {
                                let max_scroll = (a.filtered.len() - layout::VISIBLE_ROWS) as f32;
                                a.scroll = (frac * max_scroll) as usize;
                            }
                        }
                    });
                    paint(hwnd);
                }
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
    let has_scrollbar = total_filtered > layout::VISIBLE_ROWS;
    let rows_w = if has_scrollbar {
        w - layout::PAD - layout::SCROLLBAR_W - 8
    } else {
        w - layout::PAD
    };

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

        // Radiant glow behind search field for luxurious illuminated aesthetic
        c.radial_glow(
            w as f32 * 0.5,
            (layout::PAD + layout::SEARCH_H / 2) as f32,
            w as f32 * 0.75,
            t.accent,
            t.bloom,
        );

        // Subtle borders with anti-aliasing
        c.round_rect_border(1, 1, w - 1, h - 1, layout::RADIUS, 1, t.hairline);
        c.round_rect_border(1, 1, w - 1, 14, layout::RADIUS, 1, t.highlight);

        // Inset search field
        let (fx0, fy0, fx1, fy1) = layout::search_box(w, layout::PAD, layout::SEARCH_H);
        c.round_rect(fx0, fy0, fx1, fy1, 14, t.field);
        c.round_rect_border(fx0, fy0, fx1, fy1, 14, 1, t.hairline);

        // Magnifier glyph: crisp geometric vector
        let gcx = fx0 as f32 + 24.0;
        let gcy = (fy0 + fy1) as f32 / 2.0;
        for py in (gcy as i32 - 9)..=(gcy as i32 + 9) {
            for px in (gcx as i32 - 9)..=(gcx as i32 + 9) {
                let dx = px as f32 + 0.5 - gcx;
                let dy = py as f32 + 0.5 - gcy;
                let d = (dx * dx + dy * dy).sqrt() - 5.5;
                let cov = (0.5f32 - d.abs() / 1.7).clamp(0.0, 1.0);
                if cov > 0.0 {
                    c.blend_pixel(px, py, t.subtext, cov);
                }
            }
        }
        for i in 0..8 {
            let hx = gcx + 5.0 + i as f32 * 0.75;
            let hy = gcy + 5.0 + i as f32 * 0.75;
            for (dx, dy) in [(0i32, 0i32), (1, 0), (0, 1), (1, 1)] {
                c.blend_pixel(hx as i32 + dx, hy as i32 + dy, t.subtext, 0.85);
            }
        }

        // Visible rows
        let origin_y = layout::PAD + layout::SEARCH_H + 12;
        let visible_count = total_filtered
            .saturating_sub(app.scroll)
            .min(layout::VISIBLE_ROWS);

        for i in 0..visible_count {
            let actual_idx = app.scroll + i;
            let ry = layout::row_y(origin_y, i);
            let selected = actual_idx == app.sel;

            let rx0 = layout::PAD;
            let rx1 = rows_w;
            let ry0 = ry;
            let ry1 = ry + layout::ROW_H;

            // Row background
            let fill = if selected {
                t.row_selected
            } else {
                t.row_bg
            };
            c.round_rect(rx0, ry0, rx1, ry1, 8, fill);
            if selected {
                c.round_rect_border(rx0, ry0, rx1, ry1, 8, 1, t.accent);
            } else {
                c.round_rect_border(rx0, ry0, rx1, ry1, 8, 1, t.hairline);
            }

            // Key badge hardware keycap pill
            let (bx0, by0, bx1, by1) = layout::key_badge_rect(rx0, ry0);
            c.round_rect(bx0, by0, bx1, by1, 6, t.badge_bg);
            c.round_rect_border(
                bx0,
                by0,
                bx1,
                by1,
                6,
                1,
                if selected { t.accent } else { t.badge_border },
            );
            // Subtle top sheen on keycap for hardware depth
            c.hline(bx0 + 4, bx1 - 4, by0 + 1, t.highlight);

            // Category tag pill on right
            let tag_w = 120;
            let tag_x1 = rx1 - 8;
            let tag_x0 = tag_x1 - tag_w;
            let tag_y0 = ry0 + 7;
            let tag_y1 = ry1 - 7;
            c.round_rect(tag_x0, tag_y0, tag_x1, tag_y1, 6, t.field);
            c.round_rect_border(tag_x0, tag_y0, tag_x1, tag_y1, 6, 1, t.hairline);
        }

        // Visual Scrollbar Track and Thumb
        let rows_total_h = (layout::ROW_H + layout::GAP) * layout::VISIBLE_ROWS as i32 - layout::GAP;
        if let Some((track, thumb)) = layout::scrollbar_rects(
            w,
            origin_y,
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

    // ---- text pass (ClearType subpixel antialiasing) ----
    unsafe {
        let dc = dib.dc();
        let body = font(Type::Body.gdi(), 500, &t.typeface.family);
        let mono = font(Type::Body.gdi(), 600, yasb_chrome::GOHU_FAMILY);
        let small = font(Type::Foot.gdi(), 500, &t.typeface.family);
        SetBkMode(dc, TRANSPARENT);

        // Search text
        let (fx0, _fy0, fx1, _fy1) = layout::search_box(w, layout::PAD, layout::SEARCH_H);
        let text_x = fx0 + 42;
        let empty = app.query.is_empty();
        SelectObject(dc, HGDIOBJ(body.0));
        SetTextColor(dc, colorref(if empty { t.subtext } else { t.text }));
        draw_text(
            dc,
            if empty {
                "Search shortcuts (e.g. focus, terminal, alt+f, resize, workspace)..."
            } else {
                &app.query
            },
            rect(text_x, layout::PAD, fx1 - text_x - 12, layout::SEARCH_H),
            DT_LEFT | DT_VCENTER,
        );

        // Visible rows
        let origin_y = layout::PAD + layout::SEARCH_H + 12;
        let visible_count = total_filtered
            .saturating_sub(app.scroll)
            .min(layout::VISIBLE_ROWS);

        for i in 0..visible_count {
            let actual_idx = app.scroll + i;
            let item_idx = app.filtered[actual_idx];
            let item = &app.all[item_idx];
            let selected = actual_idx == app.sel;
            let ry = layout::row_y(origin_y, i);
            let rx0 = layout::PAD;
            let rx1 = rows_w;
            let (bx0, by0, bx1, by1) = layout::key_badge_rect(rx0, ry);

            // Key badge text (centered in hardware keycap)
            SelectObject(dc, HGDIOBJ(mono.0));
            SetTextColor(dc, colorref(if selected { t.accent } else { t.text }));
            draw_text(
                dc,
                item.keys,
                rect(bx0, by0, bx1 - bx0, by1 - by0),
                DT_CENTER | DT_VCENTER,
            );

            // Description text
            SelectObject(dc, HGDIOBJ(body.0));
            SetTextColor(dc, colorref(if selected { t.text } else { t.text }));
            let desc_x = bx1 + 16;
            let tag_w = 120;
            let desc_w = (rx1 - 8 - tag_w - desc_x - 12).max(40);
            draw_text(
                dc,
                item.desc,
                rect(desc_x, ry, desc_w, layout::ROW_H),
                DT_LEFT | DT_VCENTER,
            );

            // Category tag (pill-centered on right)
            SelectObject(dc, HGDIOBJ(small.0));
            SetTextColor(dc, colorref(if selected { t.accent } else { t.subtext }));
            let tag_x1 = rx1 - 8;
            let tag_x0 = tag_x1 - tag_w;
            draw_text(
                dc,
                item.category,
                rect(tag_x0, ry, tag_w, layout::ROW_H),
                DT_CENTER | DT_VCENTER,
            );
        }

        // Footer status text
        let footer_y = h - layout::FOOT_H - 4;
        SelectObject(dc, HGDIOBJ(small.0));
        SetTextColor(dc, colorref(t.subtext));
        let footer_text = format!(
            "Showing {} of {} keybindings · ↑↓ or Wheel to scroll · Enter to run · Esc to close",
            total_filtered,
            app.all.len()
        );
        draw_text(
            dc,
            &footer_text,
            rect(layout::PAD, footer_y, w - layout::PAD * 2, layout::FOOT_H),
            DT_CENTER | DT_VCENTER,
        );
    }
}
