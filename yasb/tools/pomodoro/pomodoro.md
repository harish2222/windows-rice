# pomodoro — 55/5 focus timer for the bar

A deep-work timer wired into the **right side of the YASB bar**: 55 minutes
work / 5 minutes break, driven entirely by one-shot CLI invocations — there
is no resident background process, tray icon, or window.

## Bar usage

The chip sits left of the systray. Left-click toggles
start/pause/resume, middle-click skips to the next phase, right-click
resets to idle.

| Display | Meaning |
|---|---|
| `⏱ 55:00` | idle, ready |
| `⏱ 54:59` | working (counting down) |
| `⏱ p 41:07` | paused |
| `⏱ b 03:12` | on a 5-minute break |

Automatic chaining: work ends → break starts → break ends → next work
block. Completed work blocks are counted in `status` until you reset.

## CLI

```sh
pomodoro.exe tick     # advance state machine to now, print bar label
pomodoro.exe toggle   # start / pause / resume
pomodoro.exe skip     # jump into the next phase right now
pomodoro.exe reset    # stop, back to a fresh idle chip
pomodoro.exe status   # "pomodoro: work · 41:07 left · 2 work blocks done"
```

Terminal helper (long form):

```powershell
C:\Users\haris\.config\yasb\tools\pomodoro\pomodoro.exe status
```

## Design

- **No daemon.** The bar polls `tick` once per second. Each call loads the
  state file, folds in elapsed wall-clock time, saves, prints the label.
  One-shot clicks (`toggle`/`skip`/`reset`) mutate the same file.
- **Crash/reload safe.** State (`phase`, end-timestamp, pause remainder,
  round counter) lives in `pomodoro-state.txt` next to the crate. Because
  progress is stored as an epoch end-time — not a countdown — a running
  timer survives YASB reloads, sleep, and reboots: after any interruption
  the next `tick` fast-forwards through however many phases elapsed.
- **Constants.** `WORK_SECS = 55*60`, `BREAK_SECS = 5*60` at the top of
  `src/main.rs`; change and rebuild to retune.

## Files

| File | Role |
|---|---|
| `Cargo.toml` | manifest; `[[bin]] path = "src/main.rs"` pins the real source |
| `src/main.rs` | the whole state machine (~230 lines, zero dependencies) |
| `pomodoro.ps1` | 1s wrapper the widget polls; prints the label, safe idle fallback |
| `pomodoro-state.txt` | runtime state (gitignored; delete to force-reset) |
| `target/release/pomodoro.exe` | build output — rebuild with `cargo build --release` |

## Bar wiring

`config.yaml` right section: `pomodoro, lines, systray, ...`; widget
definition under `widgets:` (`run_interval: 1000`, clicks run the exe
one-shot via `silent-run`). Styling: `.pomodoro-widget` block in
`styles.css` — 11px subtext, tabular numerals so the countdown doesn't
wobble, accent-coloured clock icon.
