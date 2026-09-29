# saka — Indian national (Saka) calendar for the bar

Converts a Gregorian date into the **Indian national calendar (Saka era)**
and prints it for the YASB bar chip: the `ॐ Ashwin 4, Saka 1948` widget in
the bar's centre.

## Why "1948" when it's 2026?

That is **correct, not a bug**. The Saka era is the official calendar of
India (adopted in 1957 on the Calendar Reform Committee's recommendation)
and its epoch is **78 years behind the Gregorian era**:

```
Saka year = Gregorian year − 78        (while both eras are running)
2026 − 78 = 1948
```

The Saka new year is **Chaitra 1**, fixed to **22 March in Gregorian leap
years and 23 March otherwise** — it is a solar calendar, not lunar. From
mid-March/late-March until 31 December the two years differ by 78; from
1 January until the new year the running Saka year is still the previous
one (79 years behind). So:

| Gregorian date | Saka date | Why |
|---|---|---|
| 2026-09-29 | Ashwin 4, Saka 1948 | Sep: 2026 − 78 |
| 2026-03-23 | 1 Chaitra 1948 | Saka new year (non-leap anchor) |
| 2026-03-01 | Phalguna 10, Saka 1947 | before new year: still 1947 |
| 2028-03-22 | 1 Chaitra 1950 | leap-year anchor |
| 2026-12-31 | Pausha 6, Saka 1948 | year-end sanity |

The Saka era itself started in **78 CE** (attributed to emperor
Kanishka's era), which is where the 78-year offset comes from. Compare:
the Vikram Samvat era used in many panchangs is 57 years *ahead* of the
Gregorian era — a different calendar entirely.

## Files

| File | Role |
|---|---|
| `Cargo.toml` | crate manifest; `[[bin]] path = "src/main.rs"` so cargo always compiles the real source |
| `src/main.rs` | the converter (~140 lines, zero dependencies) |
| `saka-today.ps1` | wrapper the widget calls; passes today's date, prints long form |
| `target/release/saka.exe` | build output (gitignored, rebuild with `cargo build --release`) |

> History: a stale `saka.rs` at the crate root used to shadow
> `src/main.rs` because `Cargo.toml` pointed at it — source edits never
> reached the bar. The `[[bin]] path` line pins the true source.

## CLI

```sh
saka.exe 2026-09-29          # 29 Ashwin 1948          (short form)
saka.exe 2026-09-29 --long   # Ashwin 29, Saka 1948    (era-correct form)
saka.exe 2026-09-29 --deva   # 29 आश्विन 1948          (Devanagari)
```

## Algorithm (src/main.rs)

1. **Days library** — `days_from_civil` / `civil_from_days` are Howard
   Hinnant's classic proleptic-Gregorian algorithms. Everything is done in
   absolute day numbers, so month-length math never touches day-of-month
   edge cases.
2. **Anchor** — Chaitra 1 of the current Gregorian year: 22 March in
   Gregorian leap years, 23 March otherwise.
3. **Day-of-year in Saka space** — `today − anchor` in days. Negative
   (Jan–Mar before the new year) rolls back to the previous Gregorian
   year's anchor and decrements the Saka year.
4. **Month walk** — the Gazette month lengths
   `[31,31,33,31,31,31,31,30,30,30,30,30]` (Chaitra gets +1 day in leap
   years) are subtracted in order until the remainder fits one month.
   The remainder is the day of month, the index is the month.

Because each step is pure integer math with no tz or DST inputs, the
output is deterministic for any date string.

## Bar wiring

`config.yaml` (centre section):

```yaml
saka:
  type: "yasb.custom.CustomWidget"
  options:
    class_name: "saka-widget"
    label: "<span>\u0950</span> {data}"
    exec_options:
      run_cmd: powershell -NoProfile -ExecutionPolicy Bypass -File
               C:\Users\haris\.config\yasb\tools\saka\saka-today.ps1
      run_interval: 600000   # re-check every 10 min; date rolls over on its own
```

The middle-click `toggle_label` switches between the label and its alt
form; the CSS chip lives under `.saka-widget` in `styles.css`.

## Verification

The anchor and month-boundary cases in the table above all pass on the
built exe; run any of them as a regression check after touching the
month table.
