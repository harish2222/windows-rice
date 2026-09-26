// hotkeys-overlay — Omarchy-style hotkey reference overlay for the
// komorebi + GlazeWM(key layer) rice. Zero external crates: raw Win32.
//
// Toggle: GlazeWM binds alt+slash to this exe. First launch shows the
// overlay; launching again finds the existing window and closes it.
// Close: Esc, Enter, any letter key, or click.

#![allow(
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    unsafe_op_in_unsafe_fn
)]

use std::ffi::c_void;
use std::ptr;

type BYTE = u8;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;
type LONG = i32;
type COLORREF = u32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type HANDLE = *mut c_void;
type HWND = HANDLE;
type HDC = HANDLE;
type HBRUSH = HANDLE;
type HFONT = HANDLE;
type HPEN = HANDLE;
type HINSTANCE = HANDLE;
type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
struct POINT {
    x: LONG,
    y: LONG,
}
#[repr(C)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}
#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
    lPrivate: DWORD,
}
#[repr(C)]
struct WNDCLASSEXW {
    cbSize: UINT,
    style: UINT,
    lpfnWndProc: WNDPROC,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HANDLE,
    hCursor: HANDLE,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: HANDLE,
}
#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint: RECT,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [u8; 32],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HANDLE,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    fn GetMessageW(
        lpMsg: *mut MSG,
        hWnd: HWND,
        wMsgFilterMin: UINT,
        wMsgFilterMax: UINT,
    ) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn LoadCursorW(hInstance: HANDLE, lpCursorName: *const u16) -> HANDLE;
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    fn SetLayeredWindowAttributes(
        hWnd: HWND,
        crKey: COLORREF,
        bAlpha: BYTE,
        dwFlags: DWORD,
    ) -> BOOL;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    fn DrawTextW(
        hDC: HDC,
        lpchText: *const u16,
        nCount: i32,
        lpRect: *mut RECT,
        uFormat: UINT,
    ) -> i32;
    fn SystemParametersInfoW(
        uiAction: UINT,
        uiParam: UINT,
        pvParam: *mut c_void,
        fWinIni: UINT,
    ) -> BOOL;
    fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: UINT,
    ) -> BOOL;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    fn CreatePen(style: i32, width: i32, color: COLORREF) -> HPEN;
    fn CreateFontW(
        nHeight: i32,
        nWidth: i32,
        nEscapement: i32,
        nOrientation: i32,
        fnWeight: i32,
        fdwItalic: DWORD,
        fdwUnderline: DWORD,
        fdwStrikeOut: DWORD,
        fdwCharSet: DWORD,
        fdwOutputPrecision: DWORD,
        fdwClipPrecision: DWORD,
        fdwQuality: DWORD,
        fdwPitchAndFamily: DWORD,
        lpszFace: *const u16,
    ) -> HFONT;
    fn SelectObject(hdc: HDC, h: HANDLE) -> HANDLE;
    fn DeleteObject(ho: HANDLE) -> BOOL;
    fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn RoundRect(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HANDLE;
}

// ---- Win32 constants ----
const WM_DESTROY: UINT = 0x0002;
const WM_PAINT: UINT = 0x000F;
const WM_CLOSE: UINT = 0x0010;
const WM_KEYDOWN: UINT = 0x0100;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_RBUTTONDOWN: UINT = 0x0204;
const VK_ESCAPE: WPARAM = 0x1B;
const VK_SHIFT: WPARAM = 0x10;
const VK_CONTROL: WPARAM = 0x11;
const VK_MENU: WPARAM = 0x12;
const VK_LWIN: WPARAM = 0x5B;
const VK_RWIN: WPARAM = 0x5C;

const WS_POPUP: DWORD = 0x8000_0000;
const WS_EX_TOPMOST: DWORD = 0x0000_0008;
const WS_EX_TOOLWINDOW: DWORD = 0x0000_0080;
const WS_EX_LAYERED: DWORD = 0x0008_0000;
const LWA_ALPHA: DWORD = 0x0000_0002;
const SW_SHOW: i32 = 5;
const SPI_GETWORKAREA: UINT = 0x0030;
const SWP_NOMOVE: UINT = 0x0002;
const SWP_NOZORDER: UINT = 0x0004;
const TRANSPARENT: i32 = 1;
const DT_LEFT: UINT = 0x0000;
const DT_SINGLELINE: UINT = 0x0020;
const DT_NOCLIP: UINT = 0x0100;
const DT_CALCRECT: UINT = 0x0400;
const FW_BOLD: i32 = 700;
const FW_NORMAL: i32 = 400;
const DEFAULT_CHARSET: DWORD = 1;
const OUT_TT_PRECIS: DWORD = 4;
const CLIP_DEFAULT_PRECIS: DWORD = 0;
const CLEARTYPE_QUALITY: DWORD = 5;
const DEFAULT_PITCH: DWORD = 0;
const IDC_ARROW: *const u16 = 32512 as *const u16;
const PS_SOLID: i32 = 0;

// ---- Catppuccin Mocha palette ----
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as COLORREF) | ((g as COLORREF) << 8) | ((b as COLORREF) << 16)
}
const CLR_BG: COLORREF = rgb(0x18, 0x18, 0x25); // mantle
const CLR_MAUVE: COLORREF = rgb(0xcb, 0xa6, 0xf7); // accent
const CLR_TEXT: COLORREF = rgb(0xcd, 0xd6, 0xf4); // text
const CLR_SUB: COLORREF = rgb(0xa6, 0xad, 0xc8); // subtext
const CLR_CAP_BG: COLORREF = rgb(0x31, 0x32, 0x44); // surface0
const CLR_CAP_BORDER: COLORREF = rgb(0x45, 0x47, 0x5a); // surface1
const CLR_TITLE2: COLORREF = rgb(0x94, 0x9c, 0xb9); // overlay1-ish

const CLASS_NAME: &str = "HotkeysOverlayWndClass";
const WINDOW_TITLE: &str = "Hotkeys Overlay";

#[derive(Clone, Copy)]
enum Item {
    Header(&'static str),
    Row(&'static str, &'static str),
}

use Item::{Header, Row};

fn items() -> Vec<Item> {
    vec![
        Header("FOCUS & MOVE"),
        Row("Alt+H / J / K / L", "Focus window left/down/up/right (arrows too)"),
        Row("Alt+Shift+H / J / K / L", "Move window left/down/up/right"),
        Header("RESIZE MODE"),
        Row("Alt+R", "Enter resize mode"),
        Row("H / J / K / L", "Shrink left / grow down / shrink up / grow right"),
        Row("Esc or Enter", "Exit resize mode"),
        Header("WINDOW STATE"),
        Row("Alt+Q or Alt+X", "Close window"),
        Row("Alt+M", "Minimize"),
        Row("Win+F", "Toggle maximize"),
        Row("Alt+T", "Toggle tiling on/off"),
        Row("Alt+F", "Monocle / fullscreen"),
        Row("Alt+Shift+Space", "Toggle float"),
        Row("Alt+Shift+T", "Float at 1200x700 centered"),
        Header("LAYOUT & STACKS"),
        Row("Alt+D", "Cycle layout next"),
        Row("Alt+Shift+D", "Cycle layout previous"),
        Row("Alt+W", "Stack focused window"),
        Row("Alt+U", "Cycle through stack"),
        Row("Alt+Shift+U", "Unstack"),
        Row("Alt+P", "Promote in stack"),
        Row("Alt+Shift+O", "Toggle workspace layer"),
        Row("Alt+;", "Cycle focus (monocle-aware)"),
        Header("WORKSPACES"),
        Row("Alt+1 .. 9", "Focus workspace"),
        Row("Alt+Shift+1 .. 9", "Move window to workspace + follow"),
        Row("Alt+Ctrl+Left/Right", "Cycle workspace"),
        Row("Alt+\\", "Back to last workspace"),
        Header("MONITORS & PADDING"),
        Row("Alt+Shift+, / .", "Move window to prev/next monitor"),
        Row("Alt+Shift+Ctrl+L/R", "Move workspace to monitor"),
        Row("Alt+- / Alt+=", "Container padding -/+"),
        Row("Alt+Shift+- / =", "Workspace padding -/+"),
        Header("LAUNCHERS"),
        Row("Alt+Return", "Terminal (Nushell)"),
        Row("Win+Return", "PowerShell 7"),
        Row("Win+Shift+Return", "Zellij session"),
        Row("Alt+Shift+Return", "Xonsh"),
        Row("Alt+O / Alt+V", "VS Code"),
        Row("Alt+E", "Emacs"),
        Row("Alt+Shift+E", "File explorer"),
        Row("Alt+Shift+B / Alt+Z", "Default browser (Zen)"),
        Row("Alt+B / Alt+A", "Brave / Arc"),
        Header("SYSTEM"),
        Row("Alt+Shift+R", "Reload komorebi config"),
        Row("Alt+Shift+W", "Retile now"),
        Row("Alt+Shift+P", "Pause komorebi"),
        Row("Alt+PrtSc / Win+Shift+S", "Snipping Tool / screen clip"),
        Row("Alt+Ctrl+F1/F2/F3", "Volume down / up / mute"),
        Row("Alt+Shift+F2 / F3", "Brightness -/+"),
        Row("Alt+/", "Toggle this overlay"),
    ]
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn weight_of(item: &Item) -> f32 {
    match item {
        Header(_) => 1.7,
        Row(..) => 1.0,
    }
}

// Layout metrics
const MARGIN: i32 = 36;
const GAP: i32 = 40;
const ROW_H: i32 = 26;
const HEADER_H: i32 = 34;
const TITLE_H: i32 = 84;
const WIN_W: i32 = 980;

fn column_height(items: &[Item]) -> i32 {
    items
        .iter()
        .map(|it| match it {
            Header(_) => HEADER_H,
            Row(..) => ROW_H,
        })
        .sum()
}

// Split into two columns at section boundaries, balanced by weight.
fn split_columns(all: &[Item]) -> (Vec<Item>, Vec<Item>) {
    let total: f32 = all.iter().map(weight_of).sum();
    let half = total / 2.0;
    let mut acc = 0.0;
    let mut cut = all.len();
    for (i, it) in all.iter().enumerate() {
        acc += weight_of(it);
        if acc >= half && matches!(it, Header(_)) && i > 0 {
            cut = i;
            break;
        }
    }
    // never cut mid-header-run: walk back if we ended mid-section, ok as-is
    // because we only cut before a Header.
    (all[..cut].to_vec(), all[cut..].to_vec())
}

struct Gdi {
    title: HFONT,
    header: HFONT,
    row: HFONT,
    key: HFONT,
    bg: HBRUSH,
    cap_brush: HBRUSH,
    cap_pen: HPEN,
}

fn create_gdi() -> Gdi {
    let face = w("Segoe UI");
    let mono = w("Cascadia Mono");
    unsafe {
        let font = |h: i32, weight: i32, f: *const u16| {
            CreateFontW(
                h, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET, OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, f,
            )
        };
        Gdi {
            title: font(-30, FW_BOLD, face.as_ptr()),
            header: font(-21, FW_BOLD, face.as_ptr()),
            row: font(-20, FW_NORMAL, face.as_ptr()),
            key: font(-19, FW_BOLD, mono.as_ptr()),
            bg: CreateSolidBrush(CLR_BG),
            cap_brush: CreateSolidBrush(CLR_CAP_BG),
            cap_pen: CreatePen(PS_SOLID, 1, CLR_CAP_BORDER),
        }
    }
}

unsafe fn text_w(hdc: HDC, s: &str, x: i32, y: i32, color: COLORREF, font: HFONT, fmt: UINT) -> i32 {
    let buf: Vec<u16> = s.encode_utf16().collect();
    let mut rc = RECT {
        left: x,
        top: y,
        right: x,
        bottom: y,
    };
    SelectObject(hdc, font);
    SetTextColor(hdc, color);
    SetBkMode(hdc, TRANSPARENT);
    DrawTextW(hdc, buf.as_ptr(), buf.len() as i32, &mut rc, fmt);
    rc.right
}

unsafe fn measure_w(hdc: HDC, s: &str, font: HFONT) -> i32 {
    let buf: Vec<u16> = s.encode_utf16().collect();
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    SelectObject(hdc, font);
    DrawTextW(
        hdc,
        buf.as_ptr(),
        buf.len() as i32,
        &mut rc,
        DT_CALCRECT | DT_SINGLELINE,
    );
    rc.right - rc.left
}

unsafe fn paint_column(hdc: HDC, gdi: &Gdi, col: &[Item], x: i32, y0: i32) -> i32 {
    let mut y = y0;
    for item in col {
        match item {
            Header(t) => {
                text_w(hdc, t, x, y, CLR_MAUVE, gdi.header, DT_SINGLELINE | DT_NOCLIP);
                y += HEADER_H;
            }
            Row(keys, desc) => {
                // keycap
                let kw = measure_w(hdc, keys, gdi.key) + 18;
                let old_brush = SelectObject(hdc, gdi.cap_brush);
                let old_pen = SelectObject(hdc, gdi.cap_pen);
                RoundRect(hdc, x, y + 1, x + kw, y + ROW_H - 3, 6, 6);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                text_w(
                    hdc,
                    keys,
                    x + 9,
                    y + 1,
                    CLR_TEXT,
                    gdi.key,
                    DT_SINGLELINE | DT_NOCLIP,
                );
                // description
                text_w(
                    hdc,
                    desc,
                    x + kw + 12,
                    y + 1,
                    CLR_SUB,
                    gdi.row,
                    DT_SINGLELINE | DT_NOCLIP,
                );
                y += ROW_H;
            }
        }
    }
    y
}

unsafe fn do_paint(hwnd: HWND, gdi: &Gdi, all: &[Item]) {
    let mut ps = PAINTSTRUCT {
        hdc: ptr::null_mut(),
        fErase: 0,
        rcPaint: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        fRestore: 0,
        fIncUpdate: 0,
        rgbReserved: [0; 32],
    };
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    GetClientRect(hwnd, &mut rc);
    FillRect(hdc, &rc, gdi.bg);

    // Title
    text_w(hdc, "HOTKEYS", MARGIN, 22, CLR_MAUVE, gdi.title, DT_SINGLELINE | DT_NOCLIP);
    text_w(
        hdc,
        "komorebi + GlazeWM key layer  —  press Alt+/ or Esc to close",
        MARGIN + 218,
        40,
        CLR_TITLE2,
        gdi.row,
        DT_SINGLELINE | DT_NOCLIP,
    );

    let (c1, c2) = split_columns(all);
    let colw = (WIN_W - 2 * MARGIN - GAP) / 2;
    let _ = colw;
    paint_column(hdc, gdi, &c1, MARGIN, TITLE_H + 6);
    let x2 = MARGIN + (WIN_W - 2 * MARGIN - GAP) / 2 + GAP;
    paint_column(hdc, gdi, &c2, x2, TITLE_H + 6);

    EndPaint(hwnd, &ps);
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Lazy per-thread init (single UI thread; raw GDI handles aren't Sync).
    thread_local! {
        static GDI: Gdi = create_gdi();
        static ITEMS: Vec<Item> = items();
    }

    match msg {
        WM_PAINT => {
            GDI.with(|g| ITEMS.with(|it| do_paint(hwnd, g, it)));
            0
        }
        WM_KEYDOWN => {
            match wparam {
                VK_SHIFT | VK_CONTROL | VK_MENU | VK_LWIN | VK_RWIN => 0,
                _ => {
                    DestroyWindow(hwnd);
                    0
                }
            }
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            DestroyWindow(hwnd);
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    let class: Vec<u16> = w(CLASS_NAME);

    unsafe {
        // Toggle: an instance is already showing -> ask it to close, then exit.
        let existing = FindWindowW(class.as_ptr(), ptr::null());
        if !existing.is_null() {
            PostMessageW(existing, WM_CLOSE, 0, 0);
            return;
        }

        let hInstance = GetModuleHandleW(ptr::null());
        let cursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        let bg = CreateSolidBrush(CLR_BG);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance,
            hIcon: ptr::null_mut(),
            hCursor: cursor,
            hbrBackground: bg,
            lpszMenuName: ptr::null(),
            lpszClassName: class.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        if RegisterClassExW(&wc) == 0 {
            return;
        }

        // Window height: max of the two columns + title, clamped to work area.
        let all = items();
        let (c1, c2) = split_columns(&all);
        let body = column_height(&c1).max(column_height(&c2));
        let mut want_h = TITLE_H + body + MARGIN + 10;

        let mut work = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
        let work_w = work.right - work.left;
        let work_h = work.bottom - work.top;
        if want_h > work_h - 8 {
            want_h = work_h - 8;
        }
        let win_w = WIN_W.min(work_w - 16);
        let x = work.left + (work_w - win_w) / 2;
        let y = work.top + ((work_h - want_h) / 2).max(4);

        let title_w = w(WINDOW_TITLE);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED,
            class.as_ptr(),
            title_w.as_ptr(),
            WS_POPUP,
            x,
            y,
            win_w,
            want_h,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        );
        if hwnd.is_null() {
            return;
        }

        // Near-opaque (alpha 244): acrylic feel without any blur.
        SetLayeredWindowAttributes(hwnd, 0, 244, LWA_ALPHA);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetWindowPos(hwnd, ptr::null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOZORDER);

        let mut msg = MSG {
            hwnd: ptr::null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
            lPrivate: 0,
        };
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
