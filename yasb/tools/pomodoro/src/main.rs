// Rangalipi pomodoro — 55 min work / 5 min break focus timer for the YASB bar.
//
// Pure Rust, zero dependencies, no background process. The bar widget polls
// `pomodoro.exe tick` once per second; the exe folds elapsed wall-clock time
// into a tiny state file and prints the label. Bar clicks run one-shot
// commands, so nothing has to stay resident:
//
//   pomodoro.exe tick     advance the state machine to now, print bar label
//   pomodoro.exe toggle   left click   -> start / pause / resume
//   pomodoro.exe skip     middle click -> jump straight into the next phase
//   pomodoro.exe reset    right click  -> stop and return to idle (55:00)
//   pomodoro.exe status   human-readable one-liner for terminal checks
//
// State lives in pomodoro-state.txt at this crate's root, so a YASB reload,
// a reboot, or a crashed bar never loses a running timer.

use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

/// Deep-work block length (55 minutes).
const WORK_SECS: u64 = 55 * 60;
/// Tea-break length (5 minutes).
const BREAK_SECS: u64 = 5 * 60;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Work,
    Break,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Work => "work",
            Phase::Break => "break",
        }
    }
}

struct State {
    phase: Phase,
    /// Epoch second the current phase ends (valid while running).
    ends: u64,
    /// Seconds left at pause time (valid while paused).
    remaining: u64,
    paused: bool,
    /// Completed work blocks since the last reset.
    round: u32,
}

impl Default for State {
    fn default() -> Self {
        State { phase: Phase::Idle, ends: 0, remaining: 0, paused: false, round: 0 }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The state file sits at the crate root: target/release/pomodoro.exe walks
/// two ancestors up. Falls back to beside the exe if the binary ever moves.
fn state_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(root) = dir.ancestors().nth(2) {
                return root.join("pomodoro-state.txt");
            }
            return dir.join("pomodoro-state.txt");
        }
    }
    PathBuf::from("pomodoro-state.txt")
}

fn load() -> State {
    let mut st = State::default();
    let Ok(text) = fs::read_to_string(state_path()) else {
        return st; // no file yet: fresh idle state
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match (k.trim(), v.trim()) {
            ("phase", "work") => st.phase = Phase::Work,
            ("phase", "break") => st.phase = Phase::Break,
            ("ends", n) => st.ends = n.parse().unwrap_or(0),
            ("remaining", n) => st.remaining = n.parse().unwrap_or(0),
            ("paused", "1") => st.paused = true,
            ("round", n) => st.round = n.parse().unwrap_or(0),
            _ => {}
        }
    }
    st
}

fn save(st: &State) {
    let body = format!(
        "phase={}\nends={}\nremaining={}\npaused={}\nround={}\n",
        st.phase.name(),
        st.ends,
        st.remaining,
        if st.paused { 1 } else { 0 },
        st.round,
    );
    let _ = fs::write(state_path(), body);
}

fn mmss(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Fold elapsed wall-clock time into the state: once a running phase's end
/// has passed, chain straight into the next phase (work -> break -> work).
/// Paused and idle states are frozen and never advance on their own.
fn advance(st: &mut State, now: u64) {
    if st.paused || st.phase == Phase::Idle || now < st.ends {
        return;
    }
    match st.phase {
        Phase::Work => {
            st.round += 1;
            st.phase = Phase::Break;
            st.ends = now + BREAK_SECS;
            eprintln!("[pomodoro] work block done -> break (round {})", st.round);
        }
        Phase::Break => {
            st.phase = Phase::Work;
            st.ends = now + WORK_SECS;
            eprintln!("[pomodoro] break over -> work");
        }
        Phase::Idle => {}
    }
}

/// The bar label. Compact state prefixes: "b " = break, "p " = paused.
fn label(st: &State, now: u64) -> String {
    match st.phase {
        Phase::Idle => mmss(WORK_SECS),
        Phase::Work | Phase::Break => {
            let rem = if st.paused {
                st.remaining
            } else {
                st.ends.saturating_sub(now)
            };
            let tag = if st.paused {
                "p "
            } else if st.phase == Phase::Break {
                "b "
            } else {
                ""
            };
            format!("{tag}{}", mmss(rem))
        }
    }
}

fn start_work(st: &mut State, now: u64) {
    st.phase = Phase::Work;
    st.ends = now + WORK_SECS;
    st.remaining = 0;
    st.paused = false;
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "status".to_string());
    let now = now_secs();

    match cmd.as_str() {
        // Bar poll (every 1s): advance state, print label.
        "tick" => {
            let mut st = load();
            advance(&mut st, now);
            save(&st);
            println!("{}", label(&st, now));
        }
        // Left click: start -> pause -> resume.
        "toggle" => {
            let mut st = load();
            advance(&mut st, now);
            match (st.phase, st.paused) {
                (Phase::Idle, _) => {
                    start_work(&mut st, now);
                    eprintln!("[pomodoro] work started (55:00)");
                }
                (_, false) => {
                    st.paused = true;
                    st.remaining = st.ends.saturating_sub(now).max(1);
                    eprintln!("[pomodoro] paused");
                }
                (_, true) => {
                    let secs = if st.remaining > 0 { st.remaining } else { 1 };
                    st.ends = now + secs;
                    st.remaining = 0;
                    st.paused = false;
                    eprintln!("[pomodoro] resumed");
                }
            }
            save(&st);
            println!("{}", label(&st, now));
        }
        // Middle click: jump into the next phase right now.
        "skip" => {
            let mut st = load();
            match st.phase {
                Phase::Idle => start_work(&mut st, now),
                Phase::Work => {
                    st.round += 1;
                    st.phase = Phase::Break;
                    st.ends = now + BREAK_SECS;
                }
                Phase::Break => start_work(&mut st, now),
            }
            save(&st);
            println!("{}", label(&st, now));
        }
        // Right click: full stop, back to a fresh idle chip.
        "reset" => {
            save(&State::default());
            println!("{}", mmss(WORK_SECS));
        }
        "status" => {
            let mut st = load();
            advance(&mut st, now);
            save(&st);
            let when = if st.paused {
                format!("{} left (paused)", mmss(st.remaining))
            } else if st.phase == Phase::Idle {
                format!("{} ready", mmss(WORK_SECS))
            } else {
                format!("{} left", mmss(st.ends.saturating_sub(now)))
            };
            println!("pomodoro: {} · {} · {} work blocks done", st.phase.name(), when, st.round);
        }
        _ => {
            eprintln!("usage: pomodoro [tick|toggle|skip|reset|status]");
            exit(2);
        }
    }
}
