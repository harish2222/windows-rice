//! Rangalipi panchangam — Saka national calendar + full Telugu panchangam.
//!
//! Two calendars live here, and they are genuinely different things:
//!
//! * **Saka (Indian national calendar)** — a *tropical solar* calendar pinned
//!   to the Gregorian cycle: Chaitra 1 falls on 22 March (21 in leap years),
//!   so every later month starts on a fixed Gregorian date. Zero astronomy.
//! * **Panchangam (lunisolar)** — driven by real Sun/Moon geometry: tithi,
//!   nakshatra, yoga, karana, the amanta/purnimanta lunar month, and the lunar
//!   phase. This is the Telugu/Andhra calendar proper.
//!
//! Not implemented: **adhika masa** (the intercalary lunar month). Detecting it
//! needs sequential month numbering — each lunar month takes the next solar
//! name in order, and a lapse in the Sun's progress inserts a repeat and a
//! skip. A naive "does this month repeat its predecessor's name" test looks
//! convincing but puts an intercalary month in roughly every year instead of
//! once in 33, so it is left out rather than shipped wrong.
//!
//! Both are reported side by side because a Telugu panchang always frames the
//! date in *two* eras at once: the Saka national year (for the solar month) and
//! the Vikram Samvat year (for the lunisolar year, which is what Telugu
//! almanacs count).
//!
//! Astronomy is self-contained, zero-dependency, and deliberately modest:
//!
//! * Sun — Meeus ch. 25 low-precision apparent longitude (≈0.01°).
//! * Moon — Meeus ch. 47 truncated ELP-2000/82 longitude (≈0.02°).
//! * Ayanamsa — Lahiri/Chitrapaksha linear approximation (≈0.01°).
//! * Nutation in longitude — leading two terms, applied to *both* bodies so
//!   it cancels exactly in tithi and yoga.
//!
//! Those errors are all far below one twelfth of a tithi (12°) or one
//! twenty-seventh of a nakshatra (13.33°), so tithi/nakshatra/yoga/karana
//! land within a few minutes of a published panchang.

use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Civil date helpers (Howard Hinnant's proleptic Gregorian algorithms)
// ---------------------------------------------------------------------------

pub fn is_greg_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = ((m + 9) % 12) as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------------------------------------------------------------------------
// Saka (Indian national) calendar — tropical solar, Gazette of India
// ---------------------------------------------------------------------------

/// Official month names per the Gazette of India / Rashtriya Panchang
/// (Calendar Reform Committee, 1957) — the *national solar* calendar's
/// spellings, not the lunisolar panchang variants (Ashwin, Shravana, ...).
pub const MONTHS: [&str; 12] = [
    "Chaitra", "Vaisakha", "Jyaishtha", "Ashadha", "Sravana", "Bhadra",
    "Asvina", "Kartika", "Agrahayana", "Pausha", "Magha", "Phalguna",
];

/// Devanagari forms.
pub const MONTHS_DEVA: [&str; 12] = [
    "चैत्र", "वैशाख", "ज्येष्ठ", "आषाढ", "श्रावण", "भाद्र",
    "आश्विन", "कार्तिक", "अग्रहायण", "पौष", "माघ", "फाल्गुन",
];

/// Telugu forms — the same amanta panchang month names a Telugu almanac
/// (Telugu panchangam) prints.
pub const MONTHS_TE: [&str; 12] = [
    "చైత్ర", "వైశాఖ", "జ్యేష్ఠ", "ఆషాఢ", "శ్రావణ", "భాద్ర",
    "ఆశ్విన", "కార్తిక", "అగహాయణ", "పౌష", "మాఘ", "ఫాల్గుణ",
];

/// Gazette month lengths: Chaitra 30 (31 in Saka leap years),
/// Vaisakha..Bhadra 31, Asvina..Phalguna 30. Sum = 365 (366 in leap).
const MONTH_LENS: [u32; 12] = [30, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30];

/// Convert a Gregorian date to (Saka year, month index, day of month).
pub fn saka_from_greg(y: i32, m: u32, d: u32) -> (i32, usize, u32) {
    // Chaitra 1 of the Saka year running in Gregorian year g falls on
    // 22 March (21 March when g is a leap year) — day-of-year 81 either
    // way, so every later month starts on a fixed Gregorian date:
    // Apr 21, May 22, Jun 22, Jul 23, Aug 23, Sep 23, Oct 23, Nov 22,
    // Dec 22, Jan 21, Feb 20.
    let chaitra1 = |g: i32| days_from_civil(g, 3, if is_greg_leap(g) { 21 } else { 22 });
    let today = days_from_civil(y, m, d);

    let mut g = y;
    let mut days = today - chaitra1(g);
    if days < 0 {
        // Jan 1 - Mar 20/21: still the Saka year that began last March.
        g -= 1;
        days = today - chaitra1(g);
    }

    // Saka leap rule: Saka year + 78 (== g here) being a Gregorian leap
    // year makes the Saka year leap; the extra day lengthens Chaitra to 31.
    let mut lens = MONTH_LENS;
    if is_greg_leap(g) {
        lens[0] = 31;
    }

    let mut rem = days as u32;
    let mut mi = 0usize;
    while mi < 11 && rem >= lens[mi] {
        rem -= lens[mi];
        mi += 1;
    }
    (g - 78, mi, rem + 1)
}

// ---------------------------------------------------------------------------
// Astronomy
// ---------------------------------------------------------------------------

const DEG: f64 = std::f64::consts::PI / 180.0;
const J2000: f64 = 2451545.0;
/// Julian Day of 1970-01-01T00:00, i.e. day number 0.
const EPOCH_JD: f64 = 2440587.5;
/// Mean synodic month, days.
const SYNODIC: f64 = 29.530588853;
/// Reference new moon: 2000 January 6, 18:14 UT.
/// (kept for reference: the bracket solver needs no mean-phase seed)
#[allow(dead_code)] const NEW_MOON_J2000: f64 = J2000 + 5.2597;

/// Convert a Julian Day to a day number since 1970-01-01, the unit that
/// `days_from_civil` / `civil_from_days` speak. Mixing the two silently
/// produced nonsense dates, so everything goes through here.
#[inline]
pub fn day_number(jd: f64) -> i64 {
    (jd + 0.5).floor() as i64 - 2440588
}

/// Civil date of the UT day containing `jd`.
#[inline]
fn civil_of(jd: f64) -> (i32, u32, u32) {
    civil_from_days(day_number(jd))
}

/// Gregorian month (1-12) of the UT day containing `jd`.
#[inline]
fn greg_month_of(jd: f64) -> u32 {
    civil_of(jd).1
}

#[inline]
fn norm360(x: f64) -> f64 {
    let r = x % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[inline]
fn sind(x: f64) -> f64 {
    (x * DEG).sin()
}

#[inline]
fn cosd(x: f64) -> f64 {
    (x * DEG).cos()
}

/// Julian Day for a proleptic Gregorian date plus a fraction-of-day.
///
/// `hour` is UT hours from midnight (0.0 .. 24.0).
pub fn julian_day(y: i32, m: u32, d: u32, hour: f64) -> f64 {
    2440587.5 + days_from_civil(y, m, d) as f64 + hour / 24.0
}

/// Julian centuries since J2000.
fn julian_centuries(jd: f64) -> f64 {
    (jd - J2000) / 36525.0
}

/// Obliquity of the ecliptic, degrees (Laskar).
fn obliquity(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    23.439291111
        - 0.0130041667 * t
        - 1.63889e-7 * t * t
        + 5.036e-7 * t * t * t
}

/// Nutation in longitude, degrees (leading two terms).
///
/// Applied to *both* the Sun and the Moon so it cancels in their difference
/// (tithi) and their sum (yoga); only nakshatra sees it, and 0.005° is a
/// third of a percent of one nakshatra.
pub fn nutation_in_lon(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    let om = 125.04452 - 1934.136261 * t;
    -0.00264096 * sind(om) - 0.00016352 * sind(2.0 * om)
}

/// Apparent geocentric ecliptic longitude of the Sun, degrees.
pub fn sun_longitude(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    let l0 = 280.46646 + 36000.76983 * t + 0.0003032 * t * t;
    let m = 357.52911 + 35999.05029 * t - 0.0001537 * t * t;
    let c = (1.914602 - 0.004817 * t - 0.000014 * t * t) * sind(m)
        + (0.019993 - 0.000101 * t) * sind(2.0 * m)
        + 0.000289 * sind(3.0 * m);
    let true_long = l0 + c;
    let omega = 125.04 - 1934.136 * t;
    let apparent = true_long - 0.00569 - 0.00478 * sind(omega);
    norm360(apparent + nutation_in_lon(jd))
}

/// Apparent geocentric ecliptic longitude of the Moon, degrees.
///
/// Meeus ch. 47, truncated to the terms that matter for tithi / nakshatra /
/// yoga (which are longitude-only; latitude is irrelevant to all three).
pub fn moon_longitude(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;

    let lp = 218.3164477 + 481267.88123421 * t - 0.0015786 * t2 + t3 / 538841.0
        - t4 / 65194000.0;
    let d = 297.8501921 + 445267.1114034 * t - 0.0018819 * t2 + t3 / 545868.0
        - t4 / 113065000.0;
    let m = 357.5291092 + 35999.0502909 * t - 0.0001536 * t2 + t3 / 24490000.0;
    let mp = 134.9633964 + 477198.8675055 * t + 0.0087414 * t2 + t3 / 69699.0
        - t4 / 14712000.0;
    let f = 93.2720950 + 483202.0175233 * t - 0.0036539 * t2 - t3 / 3526000.0
        + t4 / 863310000.0;

    // Terms with a solar-anomaly (M) argument get scaled by the eccentricity
    // factor E (or E^2 for the second-order terms).
    let e = 1.0 - 0.002516 * t - 0.0000074 * t2;
    let e2 = e * e;

    // (D, M, M', F, sigma_l in 1e-6 deg, sigma_r in 1e-3 arcsec, M exponent)
    const TERMS: [(i32, i32, i32, i32, f64, f64, i32); 60] = [
        (0, 0, 1, 0, 6288774.0, -20905355.0, 0),
        (2, 0, -1, 0, 1274027.0, -3699111.0, 0),
        (2, 0, 0, 0, 658314.0, -2955968.0, 0),
        (0, 0, 2, 0, 213618.0, -569925.0, 0),
        (0, 1, 0, 0, -185116.0, 48888.0, 1),
        (0, 0, 0, 2, -114332.0, -3149.0, 0),
        (2, 0, -2, 0, 58793.0, 246158.0, 0),
        (2, -1, -1, 0, 57066.0, -152138.0, 0),
        (2, 0, 1, 0, 53322.0, -170733.0, 0),
        (2, -1, 0, 0, 45758.0, -204586.0, 0),
        (0, 1, -1, 0, -40923.0, -129620.0, 1),
        (1, 0, 0, 0, -34720.0, 108743.0, 0),
        (0, 1, 1, 0, -30383.0, 104755.0, 1),
        (2, 0, 0, -2, 15327.0, 10321.0, 0),
        (0, 0, 1, 2, -12528.0, 0.0, 0),
        (0, 0, 1, -2, 10980.0, 79661.0, 0),
        (4, 0, -1, 0, 10675.0, -34782.0, 0),
        (0, 0, 3, 0, 10034.0, -23210.0, 0),
        (4, 0, -2, 0, 8548.0, -21636.0, 0),
        (2, 1, -1, 0, -7888.0, 24208.0, 1),
        (2, 1, 0, 0, -6766.0, 30824.0, 1),
        (1, 0, -1, 0, -5163.0, -8379.0, 0),
        (1, 1, 0, 0, 4987.0, -16675.0, 1),
        (2, -1, 1, 0, 4036.0, -12831.0, 0),
        (2, 0, 2, 0, 3994.0, -10445.0, 0),
        (4, 0, 0, 0, 3861.0, -11650.0, 0),
        (2, 0, -3, 0, 3665.0, 14403.0, 0),
        (0, 1, -2, 0, -2689.0, -7003.0, 1),
        (2, 0, -1, 2, -2602.0, 0.0, 0),
        (2, -1, -2, 0, 2390.0, 10056.0, 0),
        (1, 0, 1, 0, -2348.0, 6322.0, 0),
        (2, -2, 0, 0, 2236.0, -9884.0, 0),
        (0, 1, 2, 0, -2120.0, 5751.0, 1),
        (0, 2, 0, 0, -2069.0, 0.0, 2),
        (2, -2, -1, 0, 2048.0, -4950.0, 0),
        (2, 0, 1, -2, -1773.0, 4130.0, 0),
        (2, 0, 0, 2, -1595.0, 0.0, 0),
        (4, -1, -1, 0, 1215.0, -3958.0, 0),
        (0, 0, 2, 2, -1110.0, 0.0, 0),
        (3, 0, -1, 0, -892.0, 3258.0, 0),
        (2, 1, 1, 0, -810.0, 2616.0, 1),
        (4, -1, -2, 0, 759.0, -1897.0, 0),
        (0, 2, -1, 0, -713.0, -2117.0, 2),
        (2, 2, -1, 0, -700.0, 2354.0, 2),
        (2, 1, -2, 0, 691.0, 0.0, 1),
        (2, -1, 0, -2, 596.0, 0.0, 0),
        (4, 0, 1, 0, 549.0, -1423.0, 0),
        (0, 0, 4, 0, 537.0, -1117.0, 0),
        (4, -1, 0, 0, 520.0, -1571.0, 0),
        (1, 0, -2, 0, -487.0, -1739.0, 0),
        (2, 1, 0, -2, -399.0, 0.0, 1),
        (0, 0, 2, -2, -381.0, -4421.0, 0),
        (1, 1, 1, 0, 351.0, 0.0, 1),
        (3, 0, -2, 0, -340.0, 0.0, 0),
        (4, 0, -3, 0, 330.0, 0.0, 0),
        (2, -1, 2, 0, 327.0, 0.0, 0),
        (0, 2, 1, 0, -323.0, 1165.0, 2),
        (1, 1, -1, 0, 299.0, 0.0, 1),
        (2, 0, 3, 0, 294.0, 0.0, 0),
        (2, 0, -1, -2, 0.0, 8752.0, 0),
    ];

    let mut sigma_l = 0.0;
    for &(cd, cm, cmp, cf, sl, _sr, me) in TERMS.iter() {
        let arg = cd as f64 * d + cm as f64 * m + cmp as f64 * mp + cf as f64 * f;
        let ecc = match me {
            1 => e,
            2 => e2,
            _ => 1.0,
        };
        sigma_l += sl * ecc * sind(arg);
    }

    // Additive terms from Venus (A1), Jupiter (A2) and the flattening of the
    // Earth (A3).
    let a1 = 119.75 + 131.849 * t;
    let a2 = 53.09 + 479264.290 * t;
    // A3 (313.45 + 481266.484 T) only feeds the latitude terms in Meeus
    // 47.B, and latitude is not used by tithi, nakshatra or yoga.
    sigma_l += 3958.0 * e * sind(a1)
        + 1962.0 * sind(lp - f)
        + 318.0 * e2 * sind(a2);

    norm360(lp + sigma_l / 1_000_000.0 + nutation_in_lon(jd))
}

/// Lahiri (Chitrapaksha) ayanamsa, degrees.
///
/// 23°51'11.5" at the J2000 epoch, precessing at 50.29"/year — i.e.
/// 1.39694° per Julian century. The rate is *per century*, not per year: using
/// the per-year figure against a century variable is a factor-of-100 error that
/// leaves tithi and karana untouched (the ayanamsa cancels in the elongation)
/// while sliding nakshatra and yoga by half a degree, i.e. roughly half an hour.
pub fn ayanamsa_lahiri(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    23.853194 + 1.39694 * t
}

/// Sidereal (nirayana) ecliptic longitude, degrees.
pub fn sidereal_long(jd: f64, tropical: f64) -> f64 {
    norm360(tropical - ayanamsa_lahiri(jd))
}

/// Elongation Moon − Sun in [0, 360). Drives tithi, and the new/full moon
/// search.
pub fn elongation(jd: f64) -> f64 {
    norm360(moon_longitude(jd) - sun_longitude(jd))
}

/// Signed shortest distance from `target`, in degrees.
#[inline]
fn off_target(x: f64, target: f64) -> f64 {
    let d = norm360(x - target);
    if d > 180.0 { d - 360.0 } else { d }
}

/// First instant strictly after `after` at which the angle `f` reaches
/// `target` (mod 360°), or `None` if it never does within a sane window.
///
/// The angle is unwrapped while scanning: whenever the raw value drops by more
/// than 180° a whole turn is added back, so the running value increases
/// smoothly and monotonically. That is what makes the crossing reliable. Every
/// earlier attempt compared a *wrapped* signed distance instead, which is
/// discontinuous at the 360° -> 0° seam (about +179 jumping to about -179);
/// those phantom sign changes are what made `next_full_moon` answer with the
/// new moon and left the tithi end a fortnight out.
pub fn solve_angle_after(
    after: f64,
    f: &dyn Fn(f64) -> f64,
    target: f64,
    step: f64,
) -> Option<f64> {
    // The longest plausible cycle is the synodic month; 34 days is ample.
    let max_days = 34.0;

    let raw0 = f(after);
    let d = off_target(raw0, target);
    // Lift the target into the continuous frame anchored at `after`. The
    // epsilon matters: `after` is often a root returned by a previous call and
    // sits within rounding of the target, and without the guard the scan would
    // immediately "find" that same root again and never advance.
    const ON_ROOT: f64 = 1e-7; // ~8.6 ms
    let target_cont = if d > ON_ROOT {
        raw0 - d + 360.0
    } else if d < -ON_ROOT {
        raw0 - d
    } else {
        raw0 + 360.0
    };

    let mut turns = 0.0f64;
    let mut prev_raw = raw0;
    let mut prev_cont = raw0;
    let mut prev_t = after;
    let mut t = after;

    while t < after + max_days {
        t += step;
        let raw = f(t);
        if raw < prev_raw - 180.0 {
            turns += 1.0;
        }
        let cont = turns * 360.0 + raw;
        if prev_cont < target_cont && cont >= target_cont {
            // Bisect inside the bracket. The bracket straddles the 360 -> 0
            // seam whenever the crossing *is* the wrap, and `f` jumps by a full
            // turn there, so the difference has to be taken back into the
            // circular frame: comparing raw values directly made the search
            // collapse onto the wrong side of the seam and report a new moon
            // roughly 0.8 degrees (about an hour) away from the real one.
            let circular = |x: f64| {
                let r = (f(x) - target_cont).rem_euclid(360.0);
                if r > 180.0 { r - 360.0 } else { r }
            };
            let (mut a, mut b) = (prev_t, t);
            for _ in 0..64 {
                let m = 0.5 * (a + b);
                let off = circular(m);
                if off.abs() < 1e-10 {
                    return Some(m);
                }
                if off < 0.0 { a = m; } else { b = m; }
            }
            return Some(0.5 * (a + b));
        }
        prev_t = t;
        prev_cont = cont;
        prev_raw = raw;
    }
    None
}

/// Last instant at or before `t` at which `f` reaches `target`.
fn last_angle_at_or_before(
    t: f64,
    f: &dyn Fn(f64) -> f64,
    target: f64,
    step: f64,
) -> Option<f64> {
    // Walk forward from well before `t` until the next crossing is past it.
    let mut cur = solve_angle_after(t - 34.0, f, target, step)?;
    for _ in 0..4 {
        let nxt = solve_angle_after(cur + 1e-6, f, target, step)?;
        if nxt > t {
            return Some(cur);
        }
        cur = nxt;
    }
    Some(cur)
}

/// Sidereal longitude of the Moon, degrees. Nakshatras are fixed stars, so this
/// — not the elongation — is what determines them.
#[inline]
pub fn moon_sidereal(jd: f64) -> f64 {
    sidereal_long(jd, moon_longitude(jd))
}

/// Sun + Moon sidereal longitudes, summed mod 360. This is the yoga axis.
#[inline]
fn yoga_sum(jd: f64) -> f64 {
    norm360(sidereal_long(jd, sun_longitude(jd)) + moon_sidereal(jd))
}

/// Step sizes for the three curves. The tightest spacing is a half-tithi, near
/// 0.45 days, so 0.1 days is comfortably fine for the elongation.
const STEP_ELONG: f64 = 0.1;
const STEP_MOON: f64 = 0.2;
const STEP_YOGA: f64 = 0.15;

/// The new moon strictly after `after`.
pub fn next_new_moon(after: f64) -> Option<f64> {
    solve_angle_after(after, &elongation, 0.0, STEP_ELONG)
}

/// The new moon at or before `jd`.
pub fn previous_new_moon(jd: f64) -> Option<f64> {
    last_angle_at_or_before(jd, &elongation, 0.0, STEP_ELONG)
}

/// The full moon strictly after `after`.
pub fn next_full_moon(after: f64) -> Option<f64> {
    solve_angle_after(after, &elongation, 180.0, STEP_ELONG)
}

/// The full moon at or before `jd`.
pub fn previous_full_moon(jd: f64) -> Option<f64> {
    last_angle_at_or_before(jd, &elongation, 180.0, STEP_ELONG)
}

/// The new moon at or after `after`.
pub fn next_new_moon_after(after: f64) -> Option<f64> {
    solve_angle_after(after, &elongation, 0.0, STEP_ELONG)
}

// ---------------------------------------------------------------------------
// Solar / sidereal time and sunrise
// ---------------------------------------------------------------------------

/// Greenwich mean sidereal time, degrees.
fn gmst_deg(jd: f64) -> f64 {
    let t = julian_centuries(jd);
    norm360(
        280.46061837 + 360.98564736629 * (jd - J2000) + 0.000387933 * t * t
            - t * t * t / 38710000.0,
    )
}

/// Altitude of the Sun, degrees, above the horizon.
fn sun_altitude(jd: f64, lat: f64, lon: f64) -> f64 {
    let lam = sun_longitude(jd);
    let eps = obliquity(jd);
    let (ra, dec) = ecliptic_to_equatorial(lam, eps);
    let ha = (gmst_deg(jd) + lon - ra) * DEG;
    (sind(dec) * sind(lat) + cosd(dec) * cosd(lat) * ha.cos()).asin() / DEG
}

/// Right ascension and declination, degrees, for ecliptic longitude `lon`
/// at ecliptic latitude 0 (we never need lunar latitude for the panchang
/// elements: tithi, nakshatra and yoga are all pure longitude quantities).
fn ecliptic_to_equatorial(lon: f64, eps: f64) -> (f64, f64) {
    let ra = (cosd(eps) * sind(lon)).atan2(cosd(lon)) / DEG;
    let dec = sind(eps) * sind(lon).clamp(-1.0, 1.0).asin() / DEG;
    (norm360(ra), dec)
}

/// Standard refraction-corrected altitude of the Sun's upper limb at the
/// moment it touches the horizon.
const HORIZON: f64 = -0.833;

/// Sunrise or sunset within the local civil day that contains `jd`.
///
/// Scans the local day in two-minute steps for the first crossing of
/// `HORIZON` going in the wanted direction, then bisects. Returns `None`
/// during polar day/night when no crossing happens.
pub fn sun_event(jd: f64, lat: f64, lon: f64, utc_offset: f64, rising: bool) -> Option<f64> {
    // Local civil midnight of this day, converted back to UT.
    let local = jd + utc_offset / 24.0;
    let local_midnight = (local + 0.5).floor() - 0.5;
    let start = local_midnight - utc_offset / 24.0;

    let step = 1.0 / 720.0; // two minutes
    let n = 1440; // a whole local day of two-minute samples
    let mut prev_t = start;
    let mut prev = sun_altitude(prev_t, lat, lon) - HORIZON;
    for i in 1..=n {
        let t = start + i as f64 * step;
        let cur = sun_altitude(t, lat, lon) - HORIZON;
        // A rising event is below -> above; a setting event is above -> below.
        let crossed = if rising {
            prev < 0.0 && cur >= 0.0
        } else {
            prev > 0.0 && cur <= 0.0
        };
        if crossed {
            let (mut a, mut b, mut fa) = (prev_t, t, prev);
            for _ in 0..50 {
                let mid = 0.5 * (a + b);
                let fm = sun_altitude(mid, lat, lon) - HORIZON;
                if fm.abs() < 1e-9 {
                    return Some(mid);
                }
                if fa.signum() == fm.signum() {
                    a = mid;
                    fa = fm;
                } else {
                    b = mid;
                }
            }
            return Some(0.5 * (a + b));
        }
        prev_t = t;
        prev = cur;
    }
    None
}

// ---------------------------------------------------------------------------
// Panchang element name tables
// ---------------------------------------------------------------------------

pub const NAKSHATRAS: [&str; 27] = [
    "Asvini", "Bharani", "Krittika", "Rohini", "Mrigashira", "Ardra", "Punarvasu",
    "Pushya", "Ashlesha", "Magha", "Purva Phalguni", "Uttara Phalguni", "Hasta",
    "Chitra", "Svati", "Vishakha", "Anuradha", "Jyeshtha", "Mula",
    "Purva Ashadha", "Uttara Ashadha", "Sravana", "Dhanishta", "Shatabhisha",
    "Purva Bhadrapada", "Uttara Bhadrapada", "Revati",
];

pub const NAKSHATRAS_TE: [&str; 27] = [
    "అశ్విని", "భరణి", "కృత్తిక", "రోహిణి", "మృగశిర", "ఆర్ద్ర", "పునర్వసు",
    "పుష్య", "ఆశ్వథ", "మాఘ", "పూर्व ఫాల్గుణి", "ఉత్తర ఫాల్గుణి", "హస్త",
    "చిత్ర", "స్వాతి", "విశాఖ", "అనూరాధ", "జ్యేష్ఠ", "మూల",
    "పూर्व ఆషాఢ", "ఉత్తర ఆషాఢ", "శ్రావణ", "ధనిష్ఠ", "శతభిష",
    "పూर्व భాద్ర", "ఉత్తర భాద్ర", "రేవతి",
];

pub const YOGAS: [&str; 27] = [
    "Vishkambha", "Priti", "Ayushman", "Saubhagya", "Shobhana", "Atiganda",
    "Sukarman", "Dhriti", "Shula", "Ganda", "Vriddhi", "Dhruva", "Vyaghata",
    "Harshana", "Vajra", "Siddhi", "Vyatipata", "Variyana", "Parigha", "Shiva",
    "Siddha", "Sadhya", "Shubha", "Shukla", "Brahma", "Indra", "Vaidhriti",
];

pub const YOGAS_TE: [&str; 27] = [
    "విశ్కంభ", "ప్రీతి", "ఆయుష్మాన్", "సౌభాగ్య", "శోభన", "అతిగండ",
    "సుకర్మణ", "ధృతి", "శూల", "గండ", "వృద్ధి", "ధ్రువ", "వ్యాఘాత",
    "హర్షణ", "వజ్ర", "సిద్ధి", "వ్యతిపాత", "వరియాణ", "పరిఘ", "శివ",
    "సిద్ధ", "సాధ్య", "శుభ", "శుక్ల", "బ్రహ్మ", "ఇంద్ర", "వైధৃతి",
];

/// The 15 tithi names, index 0..14. Index 14 is Purnima (full) or
/// Amavasya (new) depending on paksha.
pub const TITHIS: [&str; 15] = [
    "Pratipada", "Dwitiya", "Tritiya", "Chaturthi", "Panchami", "Shashthi",
    "Saptami", "Ashtami", "Navami", "Dashami", "Ekadashi", "Dwadashi",
    "Trayodashi", "Chaturdashi", "Purnima",
];

pub const TITHIS_TE: [&str; 15] = [
    "ప్రతిపద", "ద్వితీయ", "తృతీయ", "చతుర్థి", "పంచమి", "షష్ఠి",
    "సప్తమి", "అష్టమి", "నవమి", "దశమి", "ఏకాదశి", "ద్వాదశి",
    "త్రయోదశి", "చతుర్దశి", "పూర్ణిమ",
];

/// Weekday names, index 0 = Sunday.
pub const VARAS: [&str; 7] = [
    "Ravivara", "Somavara", "Mangalavara", "Budhavara", "Guruvara",
    "Shukravara", "Shanivara",
];

pub const VARAS_TE: [&str; 7] = [
    "రవివార", "సోమవార", "మంగళవార", "బుధవార", "గురువార",
    "శుక్రవార", "శనివార",
];

/// Karanas: seven repeating plus four fixed. Index 0 of a lunar month is
/// always Kimstughna, then the seven repeat, and the month closes with
/// Shakuni, Chatushpada and Naga.
const KARANA_REPEAT: [&str; 7] = [
    "Bava", "Balava", "Kaulava", "Taitila", "Gara", "Vanija", "Vishti",
];
const KARANA_TAIL: [&str; 3] = ["Shakuni", "Chatushpada", "Naga"];

pub fn karana_name(slot_in_month: usize) -> &'static str {
    if slot_in_month == 0 {
        "Kimstughna"
    } else if slot_in_month >= 57 {
        KARANA_TAIL[slot_in_month - 57]
    } else {
        KARANA_REPEAT[(slot_in_month - 1) % 7]
    }
}

pub const KARANA_REPEAT_TE: [&str; 7] = [
    "బవ", "బాలవ", "కౌలవ", "తైతిల", "గర", "వణిజ", "విష్టి",
];
pub const KARANA_TAIL_TE: [&str; 3] = ["శకుని", "చతుర్పద", "నాగ"];

pub fn karana_name_te(slot_in_month: usize) -> &'static str {
    if slot_in_month == 0 {
        "కిమ్స్తుగ్న"
    } else if slot_in_month >= 57 {
        KARANA_TAIL_TE[slot_in_month - 57]
    } else {
        KARANA_REPEAT_TE[(slot_in_month - 1) % 7]
    }
}

// ---------------------------------------------------------------------------
// Panchang
// ---------------------------------------------------------------------------

/// Which script to print names in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Script {
    Latin,
    Telugu,
    Devanagari,
}

impl Script {
    pub fn month(self, i: usize) -> &'static str {
        match self {
            Script::Latin => MONTHS[i],
            Script::Telugu => MONTHS_TE[i],
            Script::Devanagari => MONTHS_DEVA[i],
        }
    }
    pub fn nakshatra(self, i: usize) -> &'static str {
        match self {
            Script::Latin => NAKSHATRAS[i],
            Script::Telugu => NAKSHATRAS_TE[i],
            Script::Devanagari => NAKSHATRAS[i],
        }
    }
    pub fn yoga(self, i: usize) -> &'static str {
        match self {
            Script::Latin => YOGAS[i],
            Script::Telugu => YOGAS_TE[i],
            Script::Devanagari => YOGAS[i],
        }
    }
    pub fn vara(self, i: usize) -> &'static str {
        match self {
            Script::Latin => VARAS[i],
            Script::Telugu => VARAS_TE[i],
            Script::Devanagari => VARAS[i],
        }
    }
    pub fn tithi(self, i: usize) -> &'static str {
        match self {
            Script::Latin => TITHIS[i],
            Script::Telugu => TITHIS_TE[i],
            Script::Devanagari => TITHIS[i],
        }
    }
    pub fn karana(self, slot: usize) -> &'static str {
        match self {
            Script::Latin => karana_name(slot),
            Script::Telugu => karana_name_te(slot),
            Script::Devanagari => karana_name(slot),
        }
    }
}

/// A panchang element: which one, its index, how far through it we are, and
/// when it ends.
#[derive(Clone, Debug)]
pub struct Element {
    pub index: usize,
    /// Fraction of the element already elapsed, 0.0 .. 1.0.
    pub progress: f64,
    /// JD at which the current element ends.
    pub ends_jd: f64,
}

/// The full panchang for one instant at one place.
#[derive(Clone, Debug)]
pub struct Panchang {
    pub jd: f64,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour_ut: f64,
    /// Offset from UT in hours, used for every "HH:MM" this struct reports.
    pub utc_offset: f64,
    pub lat: f64,
    pub lon: f64,

    // Saka national (tropical solar) calendar.
    pub saka_year: i32,
    pub saka_month: usize,
    pub saka_day: u32,

    // Lunisolar.
    pub tithi: Element,
    pub nakshatra: Element,
    pub yoga: Element,
    pub karana: Element,
    /// True when the karana is the first half of a tithi.
    pub karana_first_half: bool,
    /// Slot of the karana within the current lunar month, 0..60.
    pub karana_slot: usize,

    /// Weekday, 0 = Sunday.
    pub vara: usize,

    // Lunar month framing.
    /// Solar-month index (0 = Chaitra) that names the current **amanta**
    /// month — the month running new moon to new moon.
    pub amanta_month: usize,
    /// Solar-month index (0 = Chaitra) that names the current **purnimanta**
    /// month — the month running full moon to full moon.
    pub purnimanta_month: usize,
    /// Tithi within the amanta month, 1..30.
    pub amanta_tithi_day: u32,
    /// JD of the new moon that started the amanta month.
    pub amanta_start_jd: f64,

    // Moon phase.
    /// Illuminated fraction, 0.0 .. 1.0.
    pub illum: f64,
    pub waxing: bool,
    pub phase_index: usize,

    // Eras.
    /// Vikram Samvat (Telugu) year; rolls at Ugadi, not 1 January.
    pub vikram_year: i32,

    // Events.
    pub sunrise_jd: Option<f64>,
    pub sunset_jd: Option<f64>,
    /// Tithi in force at sunrise (the *praayana* tithi a panchang quotes).
    pub tithi_at_sunrise: Option<Element>,
    pub next_new_moon_jd: f64,
    pub next_full_moon_jd: f64,
}

/// "HH:MM" of a Julian Day in the given UTC offset (hours east of UT).
pub fn hhmm_at(jd: f64, utc_offset: f64) -> String {
    let mins = ((jd + utc_offset / 24.0 + 0.5) * 24.0 * 60.0).round() as i64;
    format!(
        "{:02}:{:02}",
        mins.div_euclid(60).rem_euclid(24),
        mins.rem_euclid(60)
    )
}

/// "DD Mon YYYY" of a Julian Day in the given UTC offset.
pub fn date_at(jd: f64, utc_offset: f64) -> String {
    let (y, m, d) = civil_from_days(day_number(jd + utc_offset / 24.0));
    const MON: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
        "Nov", "Dec",
    ];
    format!("{d:02} {} {y}", MON[(m - 1) as usize])
}

/// The *solar* month containing an instant, from the Sun's sidereal sign.
///
/// Mesha -> Chaitra, Vrishabha -> Vaisakha, ... Kumbha -> Phalguna, and Meena
/// folds back to Chaitra because it is the intercalary slot.
///
/// This is the classical anchor for lunar month names. It is deliberately *not*
/// the Gregorian month: two consecutive full moons 29.5 days apart can share a
/// 31-day Gregorian month, so a Gregorian shortcut labels the amanta months
/// wrongly about every 30 months and interleaves the intercalary months at the
/// wrong point. Verified against Drik Panchang for Hyderabad on 2026-09-12,
/// 2026-10-03 and 2030-06-15 — all five of those month labels match.
pub fn solar_month_index(jd: f64) -> usize {
    let l = sidereal_long(jd, sun_longitude(jd));
    match (l / 30.0).floor() as usize {
        // Meena is the thirteenth sign and carries the intercalary Chaitra.
        11 => 0,
        other => other,
    }
}

/// Name of tithi number `index` (0..30) in the given script, resolving the
/// fifteenth tithi to Purnima or Amavasya by paksha.
pub fn tithi_name_at(index: usize, s: Script) -> &'static str {
    let n = index % 15;
    if n == 14 {
        if index == 29 { "Amavasya" } else { "Purnima" }
    } else {
        s.tithi(n)
    }
}

/// Paksha name for a tithi index, in the given script.
pub fn paksha_at(index: usize, s: Script) -> &'static str {
    let waxing = index < 15;
    match s {
        Script::Telugu => {
            if waxing { "శుక్ల" } else { "కృష్ణ" }
        }
        _ => {
            if waxing { "Shukla" } else { "Krishna" }
        }
    }
}

fn tithi_at(jd: f64) -> Element {
    let deg = elongation(jd);
    let idx = (deg / 12.0).floor() as usize % 30;
    Element {
        index: idx,
        progress: (deg - idx as f64 * 12.0) / 12.0,
        // Tithi `idx` *ends* at (idx + 1) * 12 degrees of elongation. Solving
        // for idx * 12 finds where the tithi began, which put the reported end
        // a whole synodic month into the future.
        ends_jd: solve_angle_after(jd, &elongation, (idx + 1) as f64 * 12.0, STEP_ELONG)
            .unwrap_or(jd + 1.0),
    }
}

impl Panchang {
    /// Build the panchang for a Julian Day at a place.
    pub fn at_jd(jd: f64, lat: f64, lon: f64, utc_offset: f64) -> Panchang {
        let day_index = day_number(jd);
        let (year, month, day) = civil_from_days(day_index);
        let hour_ut = jd - (day_index as f64 + EPOCH_JD);
        // day_index 0 is 1970-01-01, a Thursday. VARAS starts at Ravivara
        // (Sunday), so shift by 4 to land on the right weekday.
        let vara = (day_index + 4).rem_euclid(7) as usize;

        let sun = sun_longitude(jd);
        let moon_sid = moon_sidereal(jd);

        let tithi = tithi_at(jd);
        let nak_deg = moon_sid;
        let nak_span = 360.0 / 27.0;
        let nak_idx = (nak_deg / nak_span).floor() as usize % 27;
        let nakshatra = Element {
            index: nak_idx,
            progress: (nak_deg - nak_idx as f64 * nak_span) / nak_span,
            ends_jd: solve_angle_after(
                jd,
                &moon_sidereal,
                (nak_idx + 1) as f64 * nak_span,
                STEP_MOON,
            )
            .unwrap_or(jd + 1.0),
        };

        let yoga_deg = norm360(sun - ayanamsa_lahiri(jd) + moon_sid);
        let yoga_idx = (yoga_deg / nak_span).floor() as usize % 27;
        let yoga = Element {
            index: yoga_idx,
            progress: (yoga_deg - yoga_idx as f64 * nak_span) / nak_span,
            ends_jd: solve_angle_after(
                jd,
                &yoga_sum,
                (yoga_idx + 1) as f64 * nak_span,
                STEP_YOGA,
            )
            .unwrap_or(jd + 1.0),
        };

        // Karana is a half-tithi, so the slot advances twice per tithi.
        let half = if tithi.progress < 0.5 { 0 } else { 1 };
        let karana_slot_global = tithi.index * 2 + half;
        let karana = Element {
            index: karana_slot_global % 60,
            progress: (tithi.progress - half as f64 * 0.5) * 2.0,
            ends_jd: solve_angle_after(
                jd,
                &elongation,
                tithi.index as f64 * 12.0 + if half == 0 { 6.0 } else { 12.0 },
                STEP_ELONG,
            )
            .unwrap_or(jd + 1.0),
        };

        // Lunar month framing.
        //
        // A lunar month is named after the *solar* month in which its full
        // moon falls, so both reckonings reduce to "solar month at the relevant
        // full moon" — they differ only in which full moon that is.
        //
        // *Amanta* runs new moon to new moon, so its full moon is the first one
        // inside that span: the first full moon at or after the new moon that
        // started it.
        //
        // *Purnimanta* runs full moon to full moon and ends on the full moon, so
        // the month in progress is named after the full moon that will *end*
        // it: the next full moon from now.
        //
        // This is why the two can name different months on the same day, and
        // why they agree on the day after a new moon.
        let nm = previous_new_moon(jd).unwrap_or(jd);
        let next_nm = next_new_moon(jd).unwrap_or(jd + SYNODIC);

        // Full moon inside the amanta month, and the full moon ending the
        // purnimanta month in progress.
        let amanta_full_moon = next_full_moon(nm + 1e-7).unwrap_or(nm + 14.77);
        let purnimanta_full_moon = next_full_moon(jd).unwrap_or(jd + 14.77);

        let amanta_month = solar_month_index(amanta_full_moon);
        let purnimanta_month = solar_month_index(purnimanta_full_moon);

        let fm = purnimanta_full_moon;

        // Tithi day within the amanta month (1..30).
        let days_in = (next_nm - nm).max(1.0);
        let amanta_tithi_day =
            (((jd - nm) / days_in * 30.0).floor() as u32).clamp(1, 30);

        // Karana slot *within the month*: the global 0..60 counter resets at
        // each new moon, and Kimstughna always opens a month. saturating_sub
        // guards the wrap so a bad day count can never underflow.
        let karana_slot = karana_slot_global.saturating_sub((amanta_tithi_day as usize - 1) * 2);

        // Moon phase.
        let illum = (1.0 - cosd(elongation(jd))) / 2.0;
        let waxing = elongation(jd) < 180.0;
        let phase_index = ((elongation(jd) + 22.5) / 45.0).floor() as usize % 8;

        // Vikram Samvat rolls at Ugadi = amanta Chaitra 1, i.e. the new moon
        // that falls in March. Search forward from 1 February and stop as soon
        // as the month is March; this terminates in at most two steps because
        // consecutive new moons are 29.5 days apart.
        let ugadi_this_year = {
            let mut t = next_new_moon(julian_day(year, 2, 1, 0.0)).unwrap_or(jd);
            for _ in 0..4 {
                if greg_month_of(t) >= 3 {
                    break;
                }
                t = next_new_moon(t + 0.5).unwrap_or(t + SYNODIC);
            }
            t
        };
        let vikram_year = if jd >= ugadi_this_year {
            year + 57
        } else {
            year + 56
        };

        let sunrise = sun_event(jd, lat, lon, utc_offset, true);
        let sunset = sun_event(jd, lat, lon, utc_offset, false);
        let tithi_at_sunrise = sunrise.map(tithi_at);

        let (saka_year, saka_month, saka_day) = saka_from_greg(year, month, day);

        Panchang {
            jd,
            year,
            month,
            day,
            hour_ut,
            utc_offset,
            lat,
            lon,
            saka_year,
            saka_month,
            saka_day,
            tithi,
            nakshatra,
            yoga,
            karana,
            karana_first_half: half == 0,
            karana_slot,
            vara,
            amanta_month,
            purnimanta_month,
            amanta_tithi_day,
            amanta_start_jd: nm,
            illum,
            waxing,
            phase_index,
            vikram_year,
            sunrise_jd: sunrise,
            sunset_jd: sunset,
            tithi_at_sunrise,
            next_new_moon_jd: next_nm,
            next_full_moon_jd: fm,
        }
    }

    /// Panchang for the current wall-clock instant.
    ///
    /// `utc_offset` is hours east of UT (India is +5.5).
    pub fn now(lat: f64, lon: f64, utc_offset: f64) -> Panchang {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
       _self_place(secs / 86400.0 + 2440587.5, lat, lon, utc_offset)
    }

    /// Panchang for a whole civil date at local noon.
    pub fn on_date(
        y: i32,
        m: u32,
        d: u32,
        lat: f64,
        lon: f64,
        utc_offset: f64,
    ) -> Panchang {
        let jd = julian_day(y, m, d, 12.0 - utc_offset);
       _self_place(jd, lat, lon, utc_offset)
    }

        pub fn phase_name(&self) -> &'static str {
        const PHASES: [&str; 8] = [
            "New Moon", "Waxing Crescent", "First Quarter", "Waxing Gibbous",
            "Full Moon", "Waning Gibbous", "Last Quarter", "Waning Crescent",
        ];
        PHASES[self.phase_index]
    }

    pub fn paksha(&self) -> &'static str {
        if self.tithi.index < 15 {
            "Shukla"
        } else {
            "Krishna"
        }
    }

    /// Tithi name including the Purnima/Amavasya distinction.
    pub fn tithi_name(&self, s: Script) -> &'static str {
        tithi_name_at(self.tithi.index, s)
    }

    /// "HH:MM" of a Julian Day in this panchang's local time zone.
    pub fn hhmm(&self, jd: f64) -> String {
        hhmm_at(jd, self.utc_offset)
    }

    /// "DD Mon YYYY" of a Julian Day in local time.
    pub fn day_label(&self, jd: f64) -> String {
        date_at(jd, self.utc_offset)
    }

    /// Human-readable one-line summary, used by the CLI and by tests.
    pub fn summary(&self, s: Script) -> String {
        format!(
            "{} {} {} - {} - {} - {} - {}",
            s.vara(self.vara),
            s.month(self.saka_month),
            self.saka_day,
            tithi_name_at(self.tithi.index, s),
            s.nakshatra(self.nakshatra.index),
            s.yoga(self.yoga.index),
            s.karana(self.karana_slot),
        )
    }
}

fn _self_place(jd: f64, lat: f64, lon: f64, utc_offset: f64) -> Panchang {
    Panchang::at_jd(jd, lat, lon, utc_offset)
}
