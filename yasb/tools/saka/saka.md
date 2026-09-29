# saka — Indian national (Saka) calendar for the bar

Converts a Gregorian date into the **Indian national calendar (Saka era)**
and prints it for the YASB bar chip: the `ॐ Asvina 7, Saka 1948` widget in
the bar's centre.

## Why "1948" when it's 2026?

That is **correct, not a bug**. The Saka era is the official calendar of
India (adopted in 1957 on the Calendar Reform Committee's recommendation)
and its epoch is **78 years behind the Gregorian era** (79 from January
until the March new year):

```
Saka year = Gregorian year − 78        (March–December)
2026 − 78 = 1948
```

The Saka new year is **Chaitra 1**: 22 March in ordinary years, 21 March
in leap years. It is a *tropical solar* calendar — every other month then
starts on a fixed Gregorian date. Compare: the Vikram Samvat era used in
many panchangs is 57 years *ahead* — a different calendar entirely; and
the lunisolar panchang months (Ashwin, Shravana, …) drift against this
fixed solar scheme.

## Official month table (Gazette of India / Rashtriya Panchang)

| # | Month | Days | Starts (Gregorian) |
|---|---|---|---|
| 1 | Chaitra | 30 (31 leap) | 22 Mar (21 Mar leap) |
| 2 | Vaisakha | 31 | 21 Apr |
| 3 | Jyaishtha | 31 | 22 May |
| 4 | Ashadha | 31 | 22 Jun |
| 5 | Sravana | 31 | 23 Jul |
| 6 | Bhadra | 31 | 23 Aug |
| 7 | Asvina | 30 | 23 Sep |
| 8 | Kartika | 30 | 23 Oct |
| 9 | Agrahayana | 30 | 22 Nov |
| 10 | Pausha | 30 | 22 Dec |
| 11 | Magha | 30 | 21 Jan |
| 12 | Phalguna | 30 | 20 Feb |

Names are the national calendar's spellings (Asvina, Sravana, Bhadra,
Jyaishtha, Agrahayana) — not the lunisolar variants (Ashwin, Shravana,
Bhadrapada, Jyeshtha, Margashirsha). Leap rule: a Saka year is leap when
`Saka year + 78` is a Gregorian leap year; the extra day lengthens
Chaitra to 31.

## Files

| File | Role |
|---|---|
| `Cargo.toml` | crate manifest; `[[bin]] path = "src/main.rs"` so cargo always compiles the real source |
| `src/main.rs` | the converter (~150 lines, zero dependencies) |
| `saka-today.ps1` | wrapper the widget calls; passes today's date, prints long form |
| `target/release/saka.exe` | build output (gitignored, rebuild with `cargo build --release`) |

> History: a stale `saka.rs` at the crate root used to shadow
> `src/main.rs` because `Cargo.toml` pointed at it — source edits never
> reached the bar. The `[[bin]] path` line pins the true source. Later,
> the first month table used lunisolar spellings and a wrong anchor
> (23/22 March) — caught by comparing the bar against the published
> national calendar (2026-09-29 is Asvina **7**, not "Ashwin 4"); the
> current table is verified against every official month start.

## CLI

```sh
saka.exe 2026-09-29          # 7 Asvina 1948           (short form)
saka.exe 2026-09-29 --long   # Asvina 7, Saka 1948     (era-correct form)
saka.exe 2026-09-29 --deva   # 7 आश्विन 1948           (Devanagari)
```

## Algorithm (src/main.rs)

1. **Days library** — `days_from_civil` / `civil_from_days` are Howard
   Hinnant's classic proleptic-Gregorian algorithms. Everything is done in
   absolute day numbers, so month-length math never touches day-of-month
   edge cases.
2. **Anchor** — Chaitra 1 of the Saka year running in Gregorian year `g`:
   21 March when `g` is a Gregorian leap year, else 22 March (day-of-year
   80/81 → later months land on the fixed dates in the table above).
3. **Year selection** — `today − anchor(g)`; if negative (Jan–mid-March),
   retry with the previous Gregorian year. Saka year = `g − 78`.
4. **Month walk** — subtract the Gazette lengths
   `[30,31,31,31,31,31,30,30,30,30,30,30]` (Chaitra 31 in leap years)
   in order until the remainder fits one month; the remainder is the day,
   the index is the month.

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

`cargo test` (in this crate) pins the converter to the **official
month-start table**: all twelve months of Saka 1948, the Jan/Feb
next-year dates, the leap-year Chaitra cases (Saka 1950: 21 March
anchor, 31-day Chaitra), the January rollback to Saka 1947, the
Devanagari mode, and the headline case that caught the original bug
(2026-09-29 = Asvina 7, not "Ashwin 4"). Six tests, all green — run
them after touching the month table.
