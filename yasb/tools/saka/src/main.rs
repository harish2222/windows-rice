// Rangalipi saka — Indian national (Saka) calendar converter and full
// Telugu panchangam.
// Usage:
//   saka.exe YYYY-MM-DD           -> "7 Asvina 1948"
//   saka.exe YYYY-MM-DD --long    -> "Asvina 7, Saka 1948"  (era-correct form)
//   saka.exe YYYY-MM-DD --deva    -> "7 आश्विन 1948"         (Devanagari)
//   saka.exe --today [<epoch_days>] -> long form for the bar wrapper; with no
//                               argument it uses the current date
//   saka.exe YYYY-MM-DD --panchangam -> the full panchangam panel
//   saka.exe --panchangam         -> the same panel for the current instant
//
// Location and time zone default to Hyderabad / IST and can be overridden
// with --lat / --lon / --tz. See saka.md for why.
use std::io::Write;

use saka::{
    MONTHS, MONTHS_DEVA, Script, civil_at, civil_from_days, paksha_at,
    saka_from_greg, tithi_name_at,
};

const DEFAULT_LAT: f64 = 17.3850; // Hyderabad
const DEFAULT_LON: f64 = 78.4867;
const DEFAULT_TZ: f64 = 5.5; // IST

fn bar(width: usize) -> String {
    "\u{2500}".repeat(width)
}

fn render_panel(p: &saka::Panchang, s: Script) -> String {
    let mut o = String::new();
    let rule = bar(52);

    let tithi = p.tithi_name(s);
    let paksha = paksha_at(p.tithi.index, s);
    let nak = s.nakshatra(p.nakshatra.index);
    let yoga = s.yoga(p.yoga.index);
    let kar = s.karana(p.karana_slot);
    let vara = s.vara(p.vara);

    o.push_str(&format!(
        "{vara} \u{00b7} {:02} {} {}\n",
        p.day,
        [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
            "Nov", "Dec"
        ][(p.month - 1) as usize],
        p.year
    ));
    o.push_str(&format!(
        "{} {} \u{00b7} Saka {} \u{00b7} Vikram Samvat {}\n",
        s.month(p.saka_month),
        p.saka_day,
        p.saka_year,
        p.vikram_year
    ));
    o.push_str(&rule);
    o.push('\n');
    o.push_str(&format!(
        "Tithi      {paksha} {tithi:<22} {:>3.0}% \u{00b7} ends {}\n",
        p.tithi.progress * 100.0,
        p.hhmm(p.tithi.ends_jd)
    ));
    o.push_str(&format!(
        "Nakshatra  {nak:<22} {:>3.0}% \u{00b7} ends {}\n",
        p.nakshatra.progress * 100.0,
        p.hhmm(p.nakshatra.ends_jd)
    ));
    o.push_str(&format!(
        "Yoga       {yoga:<22} {:>3.0}% \u{00b7} ends {}\n",
        p.yoga.progress * 100.0,
        p.hhmm(p.yoga.ends_jd)
    ));
    o.push_str(&format!(
        "Karana     {kar:<22} {}\n",
        if p.karana_first_half {
            "1st half of tithi"
        } else {
            "2nd half of tithi"
        }
    ));
    o.push_str(&format!("Vara       {vara:<22}\n"));
    o.push_str(&format!(
        "Moon       {:<22} {:>3.0}% illuminated ({})\n",
        p.phase_name(),
        p.illum * 100.0,
        p.phase_name_western()
    ));
    o.push_str(&rule);
    o.push('\n');
    o.push_str(&format!(
        "Amanta     {} (lunar day {} of the month, new moon to new moon)\n",
        s.month(p.amanta_month),
        p.amanta_tithi_day
    ));
    o.push_str(&format!(
        "Purnimanta {} (month running full moon to full moon)\n",
        s.month(p.purnimanta_month)
    ));
    o.push_str(&format!(
        "Era        Saka {} \u{00b7} Vikram Samvat {}\n",
        p.saka_year, p.vikram_year
    ));
    o.push_str(&format!(
        "Sun        sunrise {} \u{00b7} sunset {}\n",
        p.sunrise_jd.map(|j| p.hhmm(j)).unwrap_or("--:--".into()),
        p.sunset_jd.map(|j| p.hhmm(j)).unwrap_or("--:--".into())
    ));
    if let Some(ts) = p.tithi_at_sunrise.as_ref() {
        o.push_str(&format!(
            "Praayana   tithi at sunrise: {} {} {} ({:.0}%)\n",
            ts.index + 1,
            paksha_at(ts.index, s),
            tithi_name_at(ts.index, s),
            ts.progress * 100.0
        ));
    }
    o.push_str(&format!(
        "Next       new moon {} \u{00b7} full moon {}\n",
        p.day_label(p.next_new_moon_jd),
        p.day_label(p.next_full_moon_jd)
    ));
    o
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stdout = std::io::stdout();
    let mut w = stdout.lock();

    let mut epoch_days: Option<i64> = None;
    let mut today = false;
    let mut date: Option<(i32, u32, u32)> = None;
    let mut mode = "short";
    let mut panchangam = false;
    let mut script = Script::Latin;
    let (mut lat, mut lon, mut tz) = (DEFAULT_LAT, DEFAULT_LON, DEFAULT_TZ);

    let mut i = 1;
    while i < args.len() {
        let take = |i: &mut usize| -> Option<String> {
            *i += 1;
            args.get(*i).cloned()
        };
        match args[i].as_str() {
            "--today" => {
                // Optional argument: `saka.exe --today` means "now", and the
                // bar passes no argument at all.
                let maybe = args.get(i + 1).cloned();
                if let Some(v) = maybe
                    && let Ok(n) = v.parse::<i64>()
                {
                    epoch_days = Some(n);
                    i += 1;
                }
                today = true;
            }
            "--long" => mode = "long",
            "--deva" => mode = "deva",
            "--te" => script = Script::Telugu,
            "--panchangam" | "--pan" => panchangam = true,
            "--lat" => lat = take(&mut i).and_then(|v| v.parse().ok()).unwrap_or(lat),
            "--lon" => lon = take(&mut i).and_then(|v| v.parse().ok()).unwrap_or(lon),
            "--tz" => tz = take(&mut i).and_then(|v| v.parse().ok()).unwrap_or(tz),
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

    if panchangam {
        let p = match date {
            Some((y, m, d)) => saka::Panchang::on_date(y, m, d, lat, lon, tz),
            None => saka::Panchang::now(lat, lon, tz),
        };
        let _ = writeln!(w, "{}", render_panel(&p, script));
        return;
    }

    let (y, m, d) = date
        .or_else(|| epoch_days.map(civil_from_days))
        .or_else(|| {
            // `--today` with no argument, or no arguments at all: use the
            // current date rather than silently reporting 1970.
            if today || (args.len() == 1) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                // Reckon the day in the configured zone, not UT: dividing the
                // raw epoch by 86400 printed yesterday's date from midnight to
                // 05:30 IST.
                Some(civil_at(now, tz))
            } else {
                None
            }
        })
        .unwrap_or((1970, 1, 1));
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
