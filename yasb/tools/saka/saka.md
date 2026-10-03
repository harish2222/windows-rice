# saka — Saka national calendar + full Telugu panchangam

Two calendars in one crate, because a Telugu panchang needs both:

* **Saka (Indian national calendar)** — a *tropical solar* calendar pinned to
  the Gregorian cycle. No astronomy: Chaitra 1 is 22 March (21 in leap years),
  so every later month starts on a fixed Gregorian date. This is what the bar
  chip shows.
* **Panchangam (lunisolar)** — driven by real Sun/Moon geometry: tithi,
  nakshatra, yoga, karana, vara, moon phase, and the amanta/purnimanta lunar
  month. This is the Telugu/Andhra calendar proper, and what the popup shows.

## Why "1948" when it's 2026?

That is correct, not a bug. The Saka era epoch is 78 years behind Gregorian
(79 from January until the March new year):

```
Saka year = Gregorian year - 78        (March-December)
2026 - 78 = 1948
```

Chaitra 1 = **22 March** in ordinary years, **21 March** in leap years. A Saka
year is leap when `Saka year + 78` is a Gregorian leap year, and the extra day
lengthens Chaitra to 31.

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

These are the national calendar's spellings (Asvina, Sravana, Bhadra,
Jyaishtha, Agrahayana) — not the lunisolar variants (Ashwin, Shravana,
Bhadrapada, Jyeshtha, Margashirsha).

## CLI

```sh
saka.exe 2026-09-29                  # 7 Asvina 1948            (short)
saka.exe 2026-09-29 --long           # Asvina 7, Saka 1948      (era-correct)
saka.exe 2026-09-29 --deva           # 7 आश्विन 1948            (Devanagari)
saka.exe --today                     # today's date, no argument needed
saka.exe --today 20729               # date from an epoch-day count
saka.exe --panchangam                # the full panel for right now
saka.exe 2026-10-03 --panchangam --te  # same, Telugu script
```

`--lat` / `--lon` / `--tz` override the location; they default to Hyderabad
(17.385 N, 78.4867 E) and IST (+5.5), which is what the bar and popup use.
Sunrise, sunset and the sunrise tithi (*praayana*) are the only things that
depend on the location — every other element is position-only.

## The panchangam elements

| Element | Definition |
|---|---|
| **Tithi** | Moon − Sun elongation, 12° each; index 0..29. 0–14 Shukla (waxing), 15–29 Krishna. The 15th resolves to Purnima or Amavasya. |
| **Nakshatra** | Moon's *sidereal* longitude in 27 equal 13.33° spans (Asvini…Revati). Needs the ayanamsa; tithi and yoga do not. |
| **Yoga** | (Sun + Moon) sidereal longitudes mod 360, in 27 spans (Vishkambha…Vaidhriti). The ayanamsa cancels. |
| **Karana** | Half-tithi. Seven repeat (Bava, Balava, Kaulava, Taitila, Gara, Vanija, Vishti), every month opens with **Kimstughna** and closes with **Shakuni, Chatushpada, Naga**. |
| **Vara** | Weekday, Ravivara…Shanivara. |
| **Amanta month** | New moon to new moon; named after the solar month of the full moon inside it. |
| **Purnimanta month** | Full moon to full moon; named after the solar month of the full moon that *ends* it. |
| **Era** | Saka year (solar) and Vikram Samvat year (lunisolar, rolls at Ugadi). Telugu almanacs count in both. |

### Amanta vs purnimanta — the part that is easy to get backwards

They are the same month names offset by half a lunar month, so on most days
they name *different* months. The rule that reproduces published panchangs:

* the **purnimanta** month in progress is named after the solar month of the
  next full moon (the one that will end it);
* the **amanta** month is named after the solar month of the full moon inside
  it — which is the *same* full moon the purnimanta month will end on.

Consequence: on the day after a new moon the two agree, and half a month later
the purnimanta month has already rolled on while the amanta month has not.
Verified against Drik Panchang for Hyderabad on 2026-09-12, 2026-10-03 and
2030-06-15 — all five month labels match.

### Not implemented: adhika masa

The intercalary month is deliberately left out. Detecting it needs *sequential*
month numbering — each lunar month takes the next solar name in order, and a
lapse in the Sun's progress inserts a repeat and a skip. The tempting shortcut
("does this month repeat its predecessor's name?") looks right but puts an
intercalary month in roughly **every year** instead of once in 33, because two
consecutive full moons 29.5 days apart usually share a 30° solar sign. Shipping
a confidently wrong "adhika" flag would be worse than omitting it.

## Astronomy

Zero dependencies, deliberately modest:

* **Sun** — Meeus ch. 25 low-precision apparent longitude (~0.01°).
* **Moon** — Meeus ch. 47 truncated ELP-2000/82 longitude (~0.02°). Latitude
  is not computed because tithi, nakshatra and yoga are pure longitude
  quantities.
* **Ayanamsa** — Lahiri/Chitrapaksha, `23.853194 + 1.39694 × centuries`. The
  rate is **per century**; using the per-year figure here is a factor-of-100
  error that leaves tithi and karana untouched (ayanamsa cancels in the
  elongation) while sliding nakshatra and yoga by half a degree, i.e. about
  half an hour. That bug happened here and is pinned by the golden tests.
* **Nutation** — leading two terms, applied to *both* bodies so it cancels
  exactly in tithi and yoga.

All errors are far below one twelfth of a tithi or one twenty-seventh of a
nakshatra. Measured against Drik Panchang the boundary times land within
**1–3 minutes**.

### Solving for an element boundary

Three different curves need solving — the elongation (new/full moon, tithi,
karana), the Moon's own sidereal longitude (nakshatra), and the Sun+Moon sum
(yoga) — so `solve_angle_after` takes the curve as a parameter and scans for a
crossing, then bisects.

Two things that are easy to get wrong and were both wrong here first:

1. **Wrap-around.** A raw angle jumps 360 → 0, and comparing a *wrapped*
   signed distance treats that jump as a sign change. The scan instead tracks
   a continuously unwrapped angle (adding a whole turn whenever the raw value
   drops by more than 180°), so it is strictly increasing and cannot see a
   phantom crossing.
2. **Bisecting across the seam.** Even with a correct bracket, when the
   crossing *is* the wrap the bracket straddles the jump and a naive
   `f(m) - target` comparison collapses onto the wrong side, landing about an
   hour off. The bisection takes the difference back into the circular frame.

Seeding Newton from a mean synodic phase is also wrong for anything but the
elongation — it silently solved the nakshatra end a fortnight late.

## Files

| File | Role |
|---|---|
| `Cargo.toml` | crate manifest; declares both the `[lib]` (the engine) and the `[[bin]]` (`saka.exe`) |
| `src/lib.rs` | the engine — Saka converter, astronomy, panchangam struct |
| `src/main.rs` | the CLI |
| `tests/cli.rs` | pins the Saka converter to the official month-start table |
| `tests/panchang.rs` | pins the panchangam to Drik Panchang plus structural invariants |
| `target/release/saka.exe` | build output (gitignored) |

> History: a stale `saka.rs` at the crate root used to shadow `src/main.rs`
> because `Cargo.toml` pointed at it — source edits never reached the bar. The
> `[[bin]] path` line pins the true source. Later, the first month table used
> lunisolar spellings and a wrong anchor (23/22 March) — caught by comparing
> the bar against the published national calendar (2026-09-29 is Asvina **7**,
> not "Ashwin 4").

## Bar wiring

`config.yaml` (centre section):

```yaml
saka:
  type: "yasb.custom.CustomWidget"
  options:
    class_name: "saka-widget"
    label: "<span>\u0950</span> {data}"
    exec_options:
      run_cmd: ".../tools/saka/target/release/saka.exe --today"
      run_interval: 600000
      return_format: "string"
    callbacks:
      on_left: "exec silent-run .../tools/saka-popup/target/release/saka-popup.exe"
      on_middle: "toggle_label"
```

The widget calls the exe directly (no PowerShell spawn) and re-checks every ten
minutes; the date rolls over on its own. Left click opens the panchangam panel
— see `../saka-popup/saka-popup.md`.

## Verification

`cargo test` runs two suites:

* **`tests/cli.rs`** — every official month start of Saka 1948, the Jan/Feb
  next-year dates, the Saka 1950 leap-year cases, the January rollback to
  Saka 1947, Devanagari mode, and the headline case 2026-09-29 = Asvina 7.
* **`tests/panchang.rs`** — two golden day panchangs read off Drik Panchang
  (2026-09-12 and 2026-10-03) covering every element, weekday, both eras, both
  lunar-month reckonings, sunrise and sunset; plus invariants: element indices
  in range, every element ends within two days, the phase solvers always
  advance, new/full moons are evenly spaced and their roots satisfy the
  elongation exactly, karana opens every month on Kimstughna, all 27 nakshatras
  and all 27 yogas get exercised, and the civil-day conversions round-trip.

21 tests. Run them after touching the astronomy, the month table, or the solver.