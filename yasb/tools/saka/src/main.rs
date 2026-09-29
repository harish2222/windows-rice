// Rangalipi saka — Indian national (Saka) calendar converter.
// Official month names per the Calendar Reform Committee 1957 / Gazette of India.
// Usage:
//   saka.exe YYYY-MM-DD           -> "7 Asvina 1948"
//   saka.exe YYYY-MM-DD --long    -> "Asvina 7, Saka 1948"  (era-correct form)
//   saka.exe YYYY-MM-DD --deva    -> "7 आश्विन 1948"         (Devanagari)
//   saka.exe --today <epoch_days> -> long form for the bar wrapper
use std::io::Write;

/// Official month names per the Gazette of India / Rashtriya Panchang
/// (Calendar Reform Committee, 1957) — the *national solar* calendar's
/// spellings, not the lunisolar panchang variants (Ashwin, Shravana, ...).
const MONTHS: [&str; 12] = [
    "Chaitra", "Vaisakha", "Jyaishtha", "Ashadha", "Sravana", "Bhadra",
    "Asvina", "Kartika", "Agrahayana", "Pausha", "Magha", "Phalguna",
];
/// Devanagari forms for the --deva mode.
const MONTHS_DEVA: [&str; 12] = [
    "चैत्र", "वैशाख", "ज्येष्ठ", "आषाढ", "श्रावण", "भाद्र",
    "आश्विन", "कार्तिक", "अग्रहायण", "पौष", "माघ", "फाल्गुन",
];
/// Gazette month lengths: Chaitra 30 (31 in Saka leap years),
/// Vaisakha..Bhadra 31, Asvina..Phalguna 30. Sum = 365 (366 in leap).
const MONTH_LENS: [u32; 12] = [30, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30];

fn is_greg_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = ((m + 9) % 12) as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
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

fn saka_from_greg(y: i32, m: u32, d: u32) -> (i32, usize, u32) {
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stdout = std::io::stdout();
    let mut w = stdout.lock();

    let mut epoch_days: Option<i64> = None;
    let mut date: Option<(i32, u32, u32)> = None;
    let mut mode = "short";

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--today" => {
                epoch_days = args.get(i + 1).and_then(|v| v.parse().ok());
                i += 1;
            }
            "--long" => mode = "long",
            "--deva" => mode = "deva",
            _ => {
                let p: Vec<&str> = args[i].split('-').collect();
                if p.len() == 3 {
                    date = Some((
                        p[0].parse().unwrap_or(1970),
                        p.get(1).and_then(|v| v.parse().ok()).unwrap_or(1),
                        p.get(2).and_then(|v| v.parse().ok()).unwrap_or(1),
                    ));
                }
            }
        }
        i += 1;
    }

    let (y, m, d) = date.or_else(|| epoch_days.map(civil_from_days)).unwrap_or((1970, 1, 1));
    let (sy, smi, sd) = saka_from_greg(y, m, d);

    match mode {
        "long" => {
            let _ = writeln!(w, "{} {}, Saka {}", MONTHS[smi], sd, sy);
        }
        "deva" => {
            let _ = writeln!(w, "{} {} {}", sd, MONTHS_DEVA[smi], sy);
        }
        _ => {
            let _ = writeln!(w, "{} {} {}", sd, MONTHS[smi], sy);
        }
    }
}
