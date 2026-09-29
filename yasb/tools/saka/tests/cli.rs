// Integration tests: run the built CLI against the official month-start
// table of the Indian national calendar (Gazette of India / Rashtriya
// Panchang, via en.wikipedia.org/wiki/Indian_national_calendar).
//
// `cargo test` builds the saka binary first and exposes its path through
// CARGO_BIN_EXE_saka, so these tests always exercise fresh source.
use std::process::Command;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_saka"))
        .args(args)
        .output()
        .expect("saka binary runs");
    assert!(out.status.success(), "saka {args:?} exited non-zero");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The user-visible headline case that caught the original bug:
/// 2026-09-29 must be Asvina 7, Saka 1948 (not "Ashwin 4").
#[test]
fn headline_case_asvina_7() {
    assert_eq!(run(&["2026-09-29", "--long"]), "Asvina 7, Saka 1948");
}

/// Every month of Saka 1948 starts on its fixed Gregorian date
/// (non-leap Saka year -> Chaitra 1 = 22 March 2026).
#[test]
fn all_official_month_starts_saka_1948() {
    let cases = [
        ("2026-03-22", "1 Chaitra 1948"),
        ("2026-04-21", "1 Vaisakha 1948"),
        ("2026-05-22", "1 Jyaishtha 1948"),
        ("2026-06-22", "1 Ashadha 1948"),
        ("2026-07-23", "1 Sravana 1948"),
        ("2026-08-23", "1 Bhadra 1948"),
        ("2026-09-23", "1 Asvina 1948"),
        ("2026-10-23", "1 Kartika 1948"),
        ("2026-11-22", "1 Agrahayana 1948"),
        ("2026-12-22", "1 Pausha 1948"),
    ];
    for (greg, expected) in cases {
        assert_eq!(run(&[greg]), expected, "month start {greg}");
    }
}

/// January/February 2027 still belong to Saka 1948 (the new year is in
/// March), on the fixed Magha/Phalguna start dates.
#[test]
fn jan_feb_still_previous_saka_year() {
    assert_eq!(run(&["2027-01-21"]), "1 Magha 1948");
    assert_eq!(run(&["2027-02-20"]), "1 Phalguna 1948");
}

/// Saka leap year 1950 (1950 + 78 = 2028, Gregorian leap): Chaitra 1
/// lands on 21 March and Chaitra has 31 days.
#[test]
fn leap_year_chaitra_saka_1950() {
    assert_eq!(run(&["2028-03-21"]), "1 Chaitra 1950");
    assert_eq!(run(&["2028-04-20"]), "31 Chaitra 1950");
    assert_eq!(run(&["2028-04-21"]), "1 Vaisakha 1950");
}

/// Before the March new year, dates belong to the *previous* Saka year
/// (79-year gap, not 78).
#[test]
fn rollback_before_new_year() {
    assert_eq!(run(&["2026-01-05"]), "15 Pausha 1947");
    assert_eq!(run(&["2026-03-01", "--long"]), "Phalguna 10, Saka 1947");
}

/// Devanagari mode tracks the same month table.
#[test]
fn devanagari_mode() {
    assert_eq!(run(&["2026-09-29", "--deva"]), "7 \u{0906}\u{0936}\u{094D}\u{0935}\u{093F}\u{0928} 1948");
}
