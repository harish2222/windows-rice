// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: term-launch - maximized + focused terminal launcher
// ============================================================================
// GlazeWM hotkeys bind here instead of the terminal exe. Komorebi owns all
// tiling (GlazeWM ignores every window), so a maximized terminal start needs
// an external SW_MAXIMIZE + SetForegroundWindow once the real window is
// realized - otherwise the tile layer absorbs the startup state (and otty has
// no native on-launch keys at all).
//
// usage:   term-launch.exe <proc-name> [exe-path]
//          term-launch.exe otty
//
// - windows_subsystem = "windows": never flashes an empty console (that flash
//   is the hidden pwsh host; a native tool has nothing to hide).
// - A terminal's visible 16x16 pre-realization window is ignored; only a
//   surface larger than 200x200 counts.
// - Single instance aware: if a matching terminal window already exists it is
//   only maximized + focused, never duplicated.
// ============================================================================
#![windows_subsystem = "windows"]

use std::env;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, IsZoomed,
    SetForegroundWindow, ShowWindow, SW_MAXIMIZE,
};

// Populated per enumeration; the WNDENUMPROC callback cannot capture state.
static CAND_PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static HITS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

// Target exe paths when no explicit path is passed and the name is known.
const KNOWN_EXES: &[(&str, &str)] = &[("otty", "C:\\Program Files\\otty\\bin\\otty.exe")];

fn candidates(target: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    let want = target.to_ascii_lowercase();
    let want_exe = format!("{}.exe", want);
    unsafe {
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut entry: PROCESSENTRY32W = PROCESSENTRY32W::default();
            entry.dwSize = u32::try_from(std::mem::size_of::<PROCESSENTRY32W>()).unwrap_or(0);
            if Process32FirstW(snap, &mut entry).is_ok() {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len])
                        .to_ascii_lowercase();
                    if name == want || name == want_exe {
                        pids.push(entry.th32ProcessID);
                    }
                    if !Process32NextW(snap, &mut entry).is_ok() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
    }
    pids
}

unsafe extern "system" fn window_cb(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let cands = CAND_PIDS.lock().unwrap();
    if !cands.contains(&pid) {
        return BOOL(1);
    }
    drop(cands);
    let mut r: RECT = RECT::default();
    if GetWindowRect(hwnd, &mut r).is_ok() {
        let w = r.right - r.left;
        let h = r.bottom - r.top;
        if w >= 200 && h >= 200 {
            HITS.lock().unwrap().push(hwnd.0 as usize);
        }
    }
    BOOL(1)
}

unsafe fn real_window(cands: &[u32]) -> Option<HWND> {
    CAND_PIDS.lock().unwrap().clear();
    CAND_PIDS.lock().unwrap().extend_from_slice(cands);
    HITS.lock().unwrap().clear();
    let _ = EnumWindows(Some(window_cb), LPARAM(0));
    let first = *HITS.lock().unwrap().first()?;
    Some(HWND(first as *mut core::ffi::c_void))
}

unsafe fn maximize_focus(hwnd: HWND) {
    let _ = ShowWindow(hwnd, SW_MAXIMIZE);
    let _ = SetForegroundWindow(hwnd);
    // Belt-and-braces: if something de-maximized during the settle, again.
    thread::sleep(Duration::from_millis(150));
    if !IsZoomed(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_MAXIMIZE);
    }
}

fn exe_path(target: &str, explicit: Option<&String>) -> Option<String> {
    if let Some(p) = explicit {
        if Path::new(p).exists() {
            return Some(p.clone());
        }
        return None;
    }
    for (name, path) in KNOWN_EXES {
        if name.eq_ignore_ascii_case(target) && Path::new(path).exists() {
            return Some((*path).to_string());
        }
    }
    // Generic fallback: scoop shim style install (<scoop>/apps/<name>/current).
    if let Ok(home) = env::var("USERPROFILE") {
        let scoop = format!("{}\\scoop\\apps\\{}\\current\\{}.exe", home, target, target);
        if Path::new(&scoop).exists() {
            return Some(scoop);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let target = args.get(1).cloned().unwrap_or_default();
    if target.is_empty() {
        return;
    }
    let explicit = args.get(2).cloned();

    // Single instance aware: raise + maximize an existing window.
    let cands = candidates(&target);
    if !cands.is_empty() {
        unsafe {
            if let Some(hwnd) = real_window(&cands) {
                maximize_focus(hwnd);
            }
        }
        return;
    }

    // Fresh launch.
    let Some(exe) = exe_path(&target, explicit.as_ref()) else {
        return;
    };
    let child = std::process::Command::new(&exe).spawn();

    // Wait up to 15s for a real window, then maximize + focus it.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if Instant::now() > deadline {
            break;
        }
        thread::sleep(Duration::from_millis(200));
        let cands = candidates(&target);
        if cands.is_empty() {
            continue;
        }
        unsafe {
            if let Some(hwnd) = real_window(&cands) {
                thread::sleep(Duration::from_millis(400));
                maximize_focus(hwnd);
                let _ = child;
                return;
            }
        }
    }
}
