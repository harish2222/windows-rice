// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: YASB theme/yasb-theme Module (bin/silent-run)
// ============================================================================
//! silent-run — launch a console program with no visible window.
//!
//! GlazeWM `shell-exec` gives every console child (komorebic) its own flashing
//! conhost window. Routing through here (`shell-exec silent-run komorebic …`)
//! spawns it with CREATE_NO_WINDOW, so hotkeys act with zero flash.
//! Lives on PATH (scoop shims dir or ~/.local/bin) so bare `silent-run` resolves everywhere.
#![windows_subsystem = "windows"]

use std::os::windows::process::CommandExt;
use std::process::Stdio;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(prog) = args.next() else {
        eprintln!("usage: silent-run <program> [args...]");
        std::process::exit(2);
    };
    let rest: Vec<String> = args.collect();
    let mut cmd = std::process::Command::new(prog);
    cmd.args(&rest);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    match cmd.spawn() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("silent-run: {e}");
            std::process::exit(1);
        }
    }
}
