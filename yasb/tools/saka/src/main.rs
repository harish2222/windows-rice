// Rangalipi saka — Indian national (Saka) calendar converter.
// Official month names per the Calendar Reform Committee 1957 / Gazette of India.
// Usage:
//   saka.exe YYYY-MM-DD           -> "4 Ashwin 1948"
//   saka.exe YYYY-MM-DD --long    -> "Ashwin 4, Saka 1948"  (era-correct form)
//   saka.exe YYYY-MM-DD --deva    -> "4 अश्विन 1948"         (Devanagari)
//   saka.exe --today <epoch_days> -> long form for the bar wrapper
use std::io::Write;

/// Official transliterations (Gregorian-of-Saka alignment per Gazette).
const MONTHS: [&str; 12] = [
    "Chaitra", "Vaishakha", "Jyeshtha", "Ashadha", "Shravana", "Bhadrapada",
    "Ashwin", "Kartika", "Margashirsha", "Pausha", "Magha", "Phalguna",
];
/// Devanagari forms for the --deva mode.
const MONTHS_DEVA: [&str; 12] = [
    "चैत्र", "वैशाख", "ज्येष्ठ", "आषाढ", "श्रावण", "भाद्रपद",
    "आश्विन", "कार्तिक", "मार्गशीर्ष", "पौष", "माघ", "फाल्गुन",
];

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
    let anchor = if is_greg_leap(y) { (y, 3u32, 22u32) } else { (y, 3u32, 23u32) };
    let days = days_from_civil(y, m, d) - days_from_civil(anchor.0, anchor.1, anchor.2);

    let (saka_year, doy) = if days >= 0 {
        (y - 78, days as u32)
    } else {
        let py = y - 1;
        let plen = if is_greg_leap(py) { 366u32 } else { 365u32 };
        (py - 78, plen - 82 + days as u32) // Mar 23/22 = doy 82
    };

    let mut lens = [31u32, 31, 33, 31, 31, 31, 31, 30, 30, 30, 30, 30];
    lens[0] = if is_greg_leap(y) { 31 } else { 30 }; // Chaitra
    let mut rem = doy;
    let mut mi = 0usize;
    while mi < 11 && rem >= lens[mi] {
        rem -= lens[mi];
        mi += 1;
    }
    (saka_year, mi, rem + 1)
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
