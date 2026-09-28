// Rangalipi saka — Indian national (Saka) calendar converter.
// Usage: saka.exe YYYY-MM-DD          -> "17 Ashwin 1948"
//        saka.exe --doy <year> <doy>  -> raw saka month/day for day-of-year
// Saka era: year = gregorian_year - 78 (after Chaitra 1 boundary).
// Month lengths: Chaitra 30 (31 in Gregorian leap years), then 31,31,33,31,
// 31,31,31,30,30,30,30. New year (Chaitra 1) = March 22 (leap) / March 23.
use std::io::Write;

const MONTHS: [&str; 12] = [
    "Chaitra", "Vaisakha", "Jyeshtha", "Ashadha", "Sravana", "Bhadra",
    "Ashwin", "Kartika", "Agrahayana", "Pausha", "Magha", "Phalguna",
];

fn is_greg_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// days since 1970-01-01 (UTC-independent civil algorithm)
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
    // anchor: Chaitra 1 of gregorian year y
    let anchor = if is_greg_leap(y) { (y, 3u32, 22u32) } else { (y, 3u32, 23u32) };
    let days = days_from_civil(y, m, d) - days_from_civil(anchor.0, anchor.1, anchor.2);

    let (saka_year, doy) = if days >= 0 {
        (y - 78, days as u32)
    } else {
        // before this year's Chaitra 1 -> previous Saka year
        let py = y - 1;
        let panchor_doy = if is_greg_leap(py) { 82u32 } else { 82u32 }; // Mar 23 = doy 82 non-leap; Mar 22 = doy 82 leap
        let plen = if is_greg_leap(py) { 366u32 } else { 365u32 };
        (py - 78, plen - panchor_doy + days as u32)
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

    // --today: read epoch days from env (set by the PowerShell wrapper)
    let (y, m, d) = if args.len() > 1 && args[1] == "--today" {
        let eps: i64 = args
            .get(2)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        civil_from_days(eps)
    } else if args.len() > 1 {
        let p: Vec<&str> = args[1].split('-').collect();
        (
            p[0].parse().unwrap_or(1970),
            p.get(1).and_then(|v| v.parse().ok()).unwrap_or(1),
            p.get(2).and_then(|v| v.parse().ok()).unwrap_or(1),
        )
    } else {
        (1970, 1, 1)
    };

    let (sy, smi, sd) = saka_from_greg(y, m, d);
    let _ = writeln!(w, "{} {} {}", sd, MONTHS[smi], sy);
}
