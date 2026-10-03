# pomodoro — 55/5 focus timer for the bar

> **Superseded.** The bar now uses YASB's **native** `PomodoroWidget`
> (`yasb.pomodoro.PomodoroWidget`). The custom Rust timer below is kept on
> disk as a historical fallback but is **no longer wired into
> `config.yaml`**. See "What the bar uses now" first.

## What the bar uses now

`config.yaml`, right section:

```yaml
pomodoro:
  type: "yasb.pomodoro.PomodoroWidget"
  options:
    class_name: "pomodoro-widget"
    label: "<span>\uf252</span> {remaining}"
    label_alt: "{session}/{total_sessions} - {remaining}"
    work_duration: 55
    break_duration: 5
    long_break_duration: 15
    long_break_interval: 4
    auto_start_breaks: true
    auto_start_work: true
    progress_bar: { enabled: true, progress_type: "circular", ... }
    menu:
      blur: true
      round_corners: true
      border_color: None
      circle_size: 170
      circle_thickness: 8
      circle_work_progress_color: "#B082C4"   # rewritten by yasb-theme
      circle_break_progress_color: "#5CA89B"  # rewritten by yasb-theme
  callbacks:
    on_left: "toggle_menu"      # open the circular-timer popup
    on_middle: "toggle_timer"   # start / pause / resume
    on_right: "toggle_label"
```

Why the native widget won:

- **No polling.** The old design ran `pomodoro.exe tick` once per second
  and spawned a process every second. The native widget keeps state in
  process.
- **A real popup.** `toggle_menu` gives a 170px circular timer instead of
  a bare bar chip, and it is styled from `.pomodoro-menu` in `styles.css`
  so it follows the active theme.
- **Theme integration is already handled.** `yasb-theme`'s
  `sync_config_colors` rewrites `circle_work_progress_color` (→ mauve) and
  `circle_break_progress_color` (→ teal) from the active theme block on
  every switch, exactly as it does for cava.

> ⚠ **`border_color: None` must be a string.** YASB validates popup
> options with pydantic (`Input should be a valid string`). A bare YAML
> `None` is the *string* `"None"` and is what YASB expects here — but
> `null`, `~`, or a blank value all parse as null and make the whole
> widget fail to load. If the bar ever comes up without the timer chip,
> grep `yasb.log` for `Failed to validate widget(s)`; it names the exact
> key.

Styling lives in `styles.css` under `.pomodoro-widget` (bar chip) and
`.pomodoro-menu` (popup), both of which take colours from the active theme
block.

## The superseded custom timer

A deep-work timer wired into the right side of the YASB bar: 55 minutes
work / 5 minutes break, driven entirely by one-shot CLI invocations — no
resident background process, tray icon, window, or PowerShell shim.

### Bar usage (old)

Left-click toggled start/pause/resume, middle-click skipped to the next
phase, right-click reset to idle.

| Display | Meaning |
|---|---|
| `⏱ 55:00` | idle, ready |
| `⏱ 54:59` | working (counting down) |
| `⏱ p 41:07` | paused |
| `⏱ b 03:12` | on a 5-minute break |

### CLI (old)

```sh
pomodoro.exe tick     # advance state machine to now, print bar label
pomodoro.exe toggle   # start / pause / resume
pomodoro.exe skip     # jump into the next phase right now
pomodoro.exe reset    # stop, back to a fresh idle chip
pomodoro.exe status   # "pomodoro: work · 41:07 left · 2 work blocks done"
```

### Design (old)

- **No daemon.** The bar polled `tick` once per second. Each call loaded
  the state file, folded in elapsed wall-clock time, saved, printed the
  label. One-shot clicks mutated the same file.
- **Crash/reload safe.** State (`phase`, end-timestamp, pause remainder,
  round counter) lived in `pomodoro-state.txt`. Because progress was
  stored as an epoch end-time — not a countdown — a running timer
  survived YASB reloads, sleep, and reboots.
- **Constants.** `WORK_SECS = 55*60`, `BREAK_SECS = 5*60` at the top of
  `src/main.rs`.

### Files (old)

| File | Role |
|---|---|
| `Cargo.toml` | manifest; `[[bin]] path = "src/main.rs"` pins the real source |
| `src/main.rs` | the whole state machine (~240 lines, zero dependencies) |
| `tests/cli.rs` | 6 integration tests driving the CLI like the bar did (isolated `POMODORO_STATE` file) |
| `pomodoro-state.txt` | runtime state (gitignored; delete to force-reset) |
| `target/release/pomodoro.exe` | build output |

### Tests (old)

```sh
cd ~/.config/yasb/tools/pomodoro && cargo test
```

Six CLI-driven tests cover idle → start → pause → resume → skip → break →
round counting → reset, against a temp state file via `POMODORO_STATE`.
Safe to run while the bar is timing a live block.