// Integration tests for the 55/5 state machine, driven through the CLI
// exactly like the bar drives it. Every test points POMODORO_STATE at an
// isolated file (so the user's live timer is never touched) and a mutex
// keeps them serial, since they share that file. Locks are deliberately
// poison-tolerant: an assertion failure in one test must not cascade
// into lock errors for the others.
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn state_file() -> std::path::PathBuf {
    std::env::temp_dir().join("pomodoro-yasb-tests-state.txt")
}

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pomodoro"))
        .args(args)
        .env("POMODORO_STATE", state_file())
        .output()
        .expect("pomodoro binary runs");
    assert!(out.status.success(), "pomodoro {args:?} exited non-zero");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Fresh idle state before each test.
fn reset() {
    let _ = std::fs::remove_file(state_file());
    run(&["reset"]);
}

#[test]
fn idle_label_is_full_block() {
    let _g = lock();
    reset();
    assert_eq!(run(&["tick"]), "55:00");
    assert!(run(&["status"]).contains("idle"));
}

#[test]
fn toggle_starts_and_runs_work() {
    let _g = lock();
    reset();
    assert_eq!(run(&["toggle"]), "55:00"); // label at the very start
    std::thread::sleep(std::time::Duration::from_millis(1300));
    let label = run(&["tick"]);
    assert!(
        label.starts_with("54:") || label.starts_with("53:"),
        "running label was {label}"
    );
    assert!(run(&["status"]).contains("work"));
}

#[test]
fn toggle_pauses_then_resumes() {
    let _g = lock();
    reset();
    run(&["toggle"]);
    let paused = run(&["toggle"]);
    assert!(paused.starts_with("p "), "paused label was {paused}");
    // Pausing in the same second as start banks a full 3300s, so the
    // resumed label may legitimately still read 55:00 for that second.
    let resumed = run(&["toggle"]);
    assert!(!resumed.starts_with("p "), "resumed label was {resumed}");
    assert!(
        resumed == "55:00" || resumed.starts_with("54:"),
        "resumed label was {resumed}"
    );
    std::thread::sleep(std::time::Duration::from_millis(1300));
    let label = run(&["tick"]);
    assert!(
        label.starts_with("54:") || label.starts_with("53:"),
        "post-resume countdown was {label}"
    );
}

#[test]
fn skip_from_idle_starts_work() {
    let _g = lock();
    reset();
    run(&["skip"]);
    assert!(run(&["status"]).contains("work"));
}

#[test]
fn skip_from_work_enters_break_and_counts_round() {
    let _g = lock();
    reset();
    run(&["toggle"]); // work
    let label = run(&["skip"]); // straight into break
    assert!(label.starts_with("b "), "break label was {label}");
    assert!(run(&["status"]).contains("break"));
    run(&["skip"]); // break -> next work block
    let status = run(&["status"]);
    assert!(status.contains("work"));
    assert!(status.contains("1 work blocks done"), "status was {status}");
}

#[test]
fn reset_stops_everything() {
    let _g = lock();
    reset();
    run(&["toggle"]);
    run(&["skip"]); // break
    assert_eq!(run(&["reset"]), "55:00");
    assert_eq!(run(&["tick"]), "55:00");
    assert!(run(&["status"]).contains("idle"));
    assert!(run(&["status"]).contains("0 work blocks done"));
}
