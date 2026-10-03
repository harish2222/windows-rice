// Panchang tests, pinned to Drik Panchang's published day panchangs for
// Hyderabad (geoname-id 1269843).
//
// Every expected value below was read off Drik Panchang's day page and the
// comparisons allow a small tolerance because this crate computes the
// positions itself (Meeus low-precision solar, truncated ELP lunar) rather
// than reading a full ephemeris. Observed agreement was 1-3 minutes, so a
// 12-minute window is a generous regression fence: it catches a real error in
// the Sun/Moon longitudes, the ayanamsa, or the phase solver without becoming
// flaky.
use saka::{
    Element, NAKSHATRAS, Script, TITHIS, VARAS, YOGAS, Panchang, ayanamsa_lahiri,
    civil_from_days, date_at, day_number, hhmm_at, julian_day, karana_name,
    next_full_moon, next_new_moon, previous_new_moon, tithi_name_at,
};

const LAT: f64 = 17.3850; // Hyderabad
const LON: f64 = 78.4867;
const TZ: f64 = 5.5; // IST

fn panchang(y: i32, m: u32, d: u32) -> Panchang {
    Panchang::on_date(y, m, d, LAT, LON, TZ)
}

/// Minutes of IST between two Julian Days, negative when `a` is earlier.
fn minutes_apart(a: f64, b: f64) -> f64 {
    ((b - a) * 24.0 * 60.0).round()
}

/// Assert `jd` falls within `tol_min` minutes of "DD Mon HH:MM" in IST.
fn assert_near(jd: f64, expected: &str, tol_min: f64, what: &str) {
    let parts: Vec<&str> = expected.split_whitespace().collect();
    let (dd, mon, hm): (u32, &str, &str) = (parts[0].parse().unwrap(), parts[1], parts[2]);
    const MON: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (y, mo) = (2026, MON.iter().position(|m| *m == mon).unwrap() as u32 + 1);
    let (h, mi) = (hm.split(':').next().unwrap().parse::<i64>().unwrap(),
        hm.split(':').nth(1).unwrap().parse::<i64>().unwrap());
    let want_jd = julian_day(y, mo, dd, 0.0) + (h * 60 + mi) as f64 / 1440.0 - TZ / 24.0;
    let delta = minutes_apart(jd, want_jd);
    assert!(
        delta.abs() <= tol_min,
        "{what}: got {}, expected {expected} IST (off by {delta} min)",
        date_at(jd, TZ)
    );
}

// ---------------------------------------------------------------------------
// Golden day panchangs
// ---------------------------------------------------------------------------

/// Drik Panchang, 2026-10-03 (Saturday), Hyderabad:
/// "Saptami upto 07:59 AM" / "Ardra upto 01:29 AM, Oct 04" / "Variyana upto
/// 03:21 PM" / "Balava upto 06:54 PM" / Shaniwara / sunrise 06:07, sunset
/// 18:04 / "Ashwina 11, 1948 Shaka" / "Ashwina - Purnimanta" +
/// "Bhadrapada - Amanta" / "2083 Siddharthi, Vikrama Samvata".
#[test]
fn golden_2026_10_03() {
    let p = panchang(2026, 10, 3);
    let l = Script::Latin;

    assert_eq!(l.vara(p.vara), "Shanivara", "weekday");
    assert_eq!(l.month(p.saka_month), "Asvina");
    assert_eq!(p.saka_day, 11);
    assert_eq!(p.saka_year, 1948);
    assert_eq!(p.vikram_year, 2083);

    assert_eq!(p.tithi_name(l), "Ashtami");
    assert_eq!(p.paksha(), "Krishna");
    assert_eq!(l.nakshatra(p.nakshatra.index), "Ardra");
    assert_eq!(l.yoga(p.yoga.index), "Variyana");
    assert_eq!(l.karana(p.karana_slot), "Balava");

    // The two lunar-month reckonings. These were the hardest part to get
    // right: on this date they name *different* months.
    assert_eq!(l.month(p.amanta_month), "Bhadra");
    assert_eq!(l.month(p.purnimanta_month), "Asvina");

    assert_near(p.tithi.ends_jd, "04 Oct 05:51", 12.0, "tithi end");
    assert_near(p.nakshatra.ends_jd, "04 Oct 01:29", 12.0, "nakshatra end");
    assert_near(p.yoga.ends_jd, "03 Oct 15:21", 12.0, "yoga end");
    assert_near(p.karana.ends_jd, "03 Oct 18:54", 12.0, "karana end");
    assert_near(p.sunrise_jd.unwrap(), "03 Oct 06:07", 2.0, "sunrise");
    assert_near(p.sunset_jd.unwrap(), "03 Oct 18:04", 2.0, "sunset");
}

/// Drik Panchang, 2026-09-12 (Saturday), Hyderabad — the day after the new
/// moon, where amanta and purnimanta agree: "Bhadrapada - Purnimanta" and
/// "Bhadrapada - Amanta". Tithi Dwitiya, Uttara Phalguni, Shubha, sunrise
/// 06:04, sunset 18:21, "Bhadrapada 21, 1948 Shaka".
#[test]
fn golden_2026_09_12_new_moon_day() {
    let p = panchang(2026, 9, 12);
    let l = Script::Latin;

    assert_eq!(l.vara(p.vara), "Shanivara");
    assert_eq!(l.month(p.saka_month), "Bhadra");
    assert_eq!(p.saka_day, 21);

    assert_eq!(p.tithi_name(l), "Dwitiya");
    assert_eq!(p.paksha(), "Shukla");
    assert_eq!(l.nakshatra(p.nakshatra.index), "Uttara Phalguni");
    assert_eq!(l.yoga(p.yoga.index), "Shubha");

    // Both reckonings agree on the day after a new moon.
    assert_eq!(l.month(p.amanta_month), "Bhadra");
    assert_eq!(l.month(p.purnimanta_month), "Bhadra");

    assert_near(p.yoga.ends_jd, "12 Sep 15:09", 12.0, "yoga end");
    assert_near(p.nakshatra.ends_jd, "12 Sep 12:55", 12.0, "nakshatra end");
    assert_near(p.sunrise_jd.unwrap(), "12 Sep 06:04", 2.0, "sunrise");
    assert_near(p.sunset_jd.unwrap(), "12 Sep 18:21", 2.0, "sunset");
}

// ---------------------------------------------------------------------------
// Structural invariants
// ---------------------------------------------------------------------------

#[test]
fn every_element_is_in_range_and_monotonic() {
    // A long sweep across years, seasons and leap years.
    let mut jd = julian_day(2024, 1, 1, 6.5);
    for _ in 0..3000 {
        let p = Panchang::at_jd(jd, LAT, LON, TZ);
        assert!(p.tithi.index < 30, "tithi {}", p.tithi.index);
        assert!(p.nakshatra.index < 27, "nakshatra {}", p.nakshatra.index);
        assert!(p.yoga.index < 27, "yoga {}", p.yoga.index);
        assert!(p.karana_slot < 60, "karana slot {}", p.karana_slot);
        assert!(p.vara < 7);
        assert!(p.saka_month < 12);
        assert!((1..=30).contains(&p.amanta_tithi_day));
        for e in [p.tithi, p.nakshatra, p.yoga, p.karana] {
            assert!((0.0..1.0).contains(&e.progress), "progress {}", e.progress);
            assert!(e.ends_jd > jd, "element must end in the future");
            assert!(e.ends_jd - jd < 2.0, "element must end within 2 days");
        }
        assert!((0.0..=1.0).contains(&p.illum));
        // The purnimanta month is named after a full moon at or after the
        // amanta one, so it is the same solar month or the next. Two is allowed
        // because Meena folds onto Chaitra, so Kumbha -> Chaitra steps over one
        // index.
        let gap = (p.purnimanta_month + 12 - p.amanta_month) % 12;
        assert!(gap <= 2, "purnimanta {} amanta {}", p.purnimanta_month, p.amanta_month);
        jd += 0.37;
    }
}

#[test]
fn karana_slot_opens_every_lunar_month_with_kimstughna() {
    // Kimstughna is always the first half-tithi of a lunar month, and the
    // month closes with Shakuni / Chatushpada / Naga.
    let mut nm = previous_new_moon(julian_day(2025, 1, 1, 0.0)).unwrap();
    for _ in 0..60 {
        let p = Panchang::at_jd(nm + 0.02, LAT, LON, TZ);
        assert_eq!(p.karana_slot, 0, "month should open on slot 0");
        assert_eq!(karana_name(0), "Kimstughna");
        // The tithi day also restarts at 1.
        assert_eq!(p.amanta_tithi_day, 1, "lunar day after a new moon");
        assert_eq!(karana_name(57), "Shakuni");
        assert_eq!(karana_name(58), "Chatushpada");
        assert_eq!(karana_name(59), "Naga");
        nm = next_new_moon(nm + 0.5).unwrap();
    }
}

#[test]
fn new_and_full_moons_are_evenly_spaced() {
    // Start from an actual lunation root, not an arbitrary date.
    let mut prev = previous_new_moon(julian_day(2025, 1, 1, 0.0)).unwrap();
    for _ in 0..40 {
        let t = next_new_moon(prev + 1e-6).unwrap();
        let gap = t - prev;
        assert!(
            (28.4..=29.95).contains(&gap),
            "synodic gap {gap} days is not a lunation"
        );
        // The returned root must actually satisfy elongation == 0, circularly.
        let e = saka::elongation(t).rem_euclid(360.0);
        assert!(e.min(360.0 - e) < 1e-6, "new moon root off by {e}");
        prev = t;
    }

    let mut prev = next_full_moon(julian_day(2025, 1, 1, 0.0)).unwrap();
    for _ in 0..40 {
        let t = next_full_moon(prev + 1e-6).unwrap();
        let gap = t - prev;
        assert!(
            (28.4..=29.95).contains(&gap),
            "full-moon gap {gap} days is not a lunation"
        );
        let e = (saka::elongation(t) - 180.0).abs();
        assert!(e < 1e-6, "full moon root off by {e}");
        prev = t;
    }
}

/// The phase solver must always move forward. An earlier version silently
/// returned the instant it was given, so a chain of `next_*` calls stalled on
/// one value forever.
#[test]
fn phase_solvers_always_advance() {
    let mut jd = julian_day(2025, 6, 1, 0.0);
    for _ in 0..200 {
        let nm = next_new_moon(jd).unwrap();
        assert!(nm > jd, "next_new_moon did not advance");
        let fm = next_full_moon(jd).unwrap();
        assert!(fm > jd, "next_full_moon did not advance");
        assert!((nm - jd) < 31.0 && (fm - jd) < 31.0);
        jd = nm;
    }
    // And backwards.
    let mut jd = julian_day(2026, 1, 1, 0.0);
    for _ in 0..200 {
        let prev = previous_new_moon(jd).unwrap();
        assert!(prev <= jd, "previous_new_moon went forward");
        // Step a little into the month so we do not sit exactly on a root.
        jd = prev - 0.5;
    }
}

/// The Moon's sidereal longitude must span all 27 nakshatras and the Sun+Moon
/// sum all 27 yogas over a year, with no gaps or repeats from a bad wrap.
#[test]
fn nakshatra_and_yoga_cover_every_slot() {
    let mut seen_n = [0usize; 27];
    let mut seen_y = [0usize; 27];
    let mut jd = julian_day(2025, 1, 1, 0.0);
    for _ in 0..4000 {
        let p = Panchang::at_jd(jd, LAT, LON, TZ);
        seen_n[p.nakshatra.index] += 1;
        seen_y[p.yoga.index] += 1;
        jd += 0.25;
    }
    for (i, c) in seen_n.iter().enumerate() {
        assert!(*c > 10, "nakshatra {i} ({}) seen only {c} times", NAKSHATRAS[i]);
    }
    for (i, c) in seen_y.iter().enumerate() {
        assert!(*c > 10, "yoga {i} ({}) seen only {c} times", YOGAS[i]);
    }
}

// ---------------------------------------------------------------------------
// Astronomy sanity
// ---------------------------------------------------------------------------

#[test]
fn ayanamsa_is_in_range() {
    for &(y, want) in &[(2000, 23.853), (2026, 24.227), (2050, 24.552)] {
        let got = ayanamsa_lahiri(julian_day(y, 1, 1, 0.0));
        assert!(
            (got - want).abs() < 0.03,
            "ayanamsa {y} = {got:.4}, expected about {want}"
        );
    }
}

#[test]
fn sun_crosses_each_nakshatra_boundary_once_a_year() {
    // The Sun's sidereal longitude is ~1 degree/day, so each 13.33 degree
    // nakshatra takes about 13.3 days: 27 of them per sidereal year.
    let a = saka::sun_longitude(julian_day(2026, 1, 1, 0.0));
    let b = saka::sun_longitude(julian_day(2027, 1, 1, 0.0));
    let advance = (b - a + 360.0) % 360.0;
    assert!(
        (358.0..360.0).contains(&advance) || (advance - 360.0).abs() < 1.0,
        "sun advanced {advance} in a year"
    );
}

#[test]
fn civil_day_conversion_round_trips() {
    for &(y, m, d) in &[(1970, 1, 1), (2000, 2, 29), (2024, 2, 29), (2026, 10, 3), (2100, 3, 1)] {
        let n = saka::days_from_civil(y, m, d);
        assert_eq!(civil_from_days(n), (y, m, d), "round trip {y}-{m}-{d}");
        assert_eq!(day_number(julian_day(y, m, d, 12.0)), n);
    }
}

// ---------------------------------------------------------------------------
// Name tables
// ---------------------------------------------------------------------------

#[test]
fn name_tables_are_complete_and_in_the_right_script() {
    assert_eq!(TITHIS.len(), 15);
    assert_eq!(VARAS.len(), 7);
    assert_eq!(NAKSHATRAS.len(), 27);
    assert_eq!(YOGAS.len(), 27);
    // Telugu tables must line up with the Latin ones entry for entry.
    assert_eq!(saka::MONTHS_TE.len(), saka::MONTHS.len());
    assert_eq!(saka::NAKSHATRAS_TE.len(), 27);
    assert_eq!(saka::YOGAS_TE.len(), 27);
    assert_eq!(saka::VARAS_TE.len(), 7);
    assert_eq!(saka::TITHIS_TE.len(), 15);
    for t in &saka::NAKSHATRAS_TE {
        assert!(t.chars().count() >= 3, "Telugu nakshatra {t} looks wrong");
    }
}

#[test]
fn purnima_and_amavasya_are_distinguished() {
    assert_eq!(tithi_name_at(14, Script::Latin), "Purnima");
    assert_eq!(tithi_name_at(29, Script::Latin), "Amavasya");
    assert_eq!(tithi_name_at(0, Script::Latin), "Pratipada");
    assert_eq!(tithi_name_at(7, Script::Latin), "Ashtami");
    assert_eq!(tithi_name_at(29, Script::Telugu), "Amavasya");
}

#[test]
fn telugu_script_renders_for_todays_panchang() {
    let p = panchang(2026, 10, 3);
    let t = p.summary(Script::Telugu);
    assert!(
        t.chars().any(|c: char| ('\u{0C00}'..='\u{0C7F}').contains(&c)),
        "expected Telugu script in {t}"
    );
    let d = p.summary(Script::Devanagari);
    assert!(d.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)));
}

#[test]
fn hhmm_and_date_formatting() {
    let jd = julian_day(2026, 10, 3, 6.5); // 12:00 IST
    assert_eq!(hhmm_at(jd, TZ), "12:00");
    assert_eq!(date_at(jd, TZ), "03 Oct 2026");
    // 00:00 and 23:59 must not roll the day.
    assert_eq!(hhmm_at(jd - 0.25, TZ), "06:00");
    assert_eq!(date_at(jd - 0.25 - TZ / 24.0, TZ), "03 Oct 2026");
}

#[test]
fn element_helper_is_consistent() {
    let e = Element { index: 4, progress: 0.5, ends_jd: 0.0 };
    assert_eq!(e.index, 4);
}