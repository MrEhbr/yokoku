//! Values as every page writes them.

use jiff::civil::Date;

/// `2023`, or `—` when unknown.
pub fn year(year: Option<i16>) -> String {
    year.map_or_else(|| "—".to_owned(), |year| year.to_string())
}

/// `Oct 3, 2026`.
pub fn date(date: Date) -> String {
    date.strftime("%b %-d, %Y").to_string()
}

/// `date` from `today`: `today`, `tomorrow`, `in 3 days`, `2 weeks ago`, `in 4 months`, `3 years ago`.
pub fn relative(date: Date, today: Date) -> String {
    let days = date.since(today).map_or(0, |span| span.get_days());
    let (count, unit) = match days.unsigned_abs() {
        0 => return "today".to_owned(),
        1 if days > 0 => return "tomorrow".to_owned(),
        1 => return "yesterday".to_owned(),
        n @ 2..14 => (n, "day"),
        n @ 14..60 => (n / 7, "week"),
        n @ 60..730 => (n / 30, "month"),
        n => (n / 365, "year"),
    };
    let unit = if count == 1 { unit.to_owned() } else { format!("{unit}s") };
    if days > 0 { format!("in {count} {unit}") } else { format!("{count} {unit} ago") }
}

/// `S01E02`.
pub fn episode(season: u16, number: u16) -> String {
    format!("S{season:02}E{number:02}")
}

/// `2h 35m`, or `25 min` under an hour.
pub fn runtime(minutes: u64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, 0) => "under 1 min".to_owned(),
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours}h {minutes:02}m"),
    }
}

/// `1.4 GB`, `700 MB` under a gigabyte, or `350 KB` under a megabyte.
pub fn size(bytes: u64) -> String {
    match bytes {
        0..1_000_000 => format!("{} KB", bytes / 1_000),
        1_000_000..1_000_000_000 => format!("{} MB", bytes / 1_000_000),
        _ => format!("{:.1} GB", bytes as f64 / 1e9),
    }
}

/// `2.4 MB/s`, from bytes per second.
pub fn rate(bytes: u64) -> String {
    format!("{}/s", size(bytes))
}

/// The usual name of a picture size, by width so cropped films count too: `2160p`, `1080p`,
/// `720p`, else the height, like `480p`.
pub fn resolution(width: u32, height: u32) -> String {
    match width {
        3800.. => "2160p".to_owned(),
        1900..3800 => "1080p".to_owned(),
        1260..1900 => "720p".to_owned(),
        _ => format!("{height}p"),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use rstest::rstest;

    use super::{relative, resolution, runtime, size};

    #[rstest]
    #[case(date(2026, 3, 10), "today")]
    #[case(date(2026, 3, 11), "tomorrow")]
    #[case(date(2026, 3, 9), "yesterday")]
    #[case(date(2026, 3, 13), "in 3 days")]
    #[case(date(2026, 2, 24), "2 weeks ago")]
    #[case(date(2026, 3, 24), "in 2 weeks")]
    #[case(date(2026, 3, 17), "in 7 days")]
    #[case(date(2026, 7, 10), "in 4 months")]
    #[case(date(2023, 3, 10), "3 years ago")]
    fn writes_a_date_relative_to_today(#[case] day: jiff::civil::Date, #[case] expected: &str) {
        assert_eq!(relative(day, date(2026, 3, 10)), expected);
    }

    #[rstest]
    #[case(0, "under 1 min")]
    #[case(25, "25 min")]
    #[case(60, "1h 00m")]
    #[case(155, "2h 35m")]
    fn writes_a_runtime(#[case] minutes: u64, #[case] expected: &str) {
        assert_eq!(runtime(minutes), expected);
    }

    #[rstest]
    #[case(350_000, "350 KB")]
    #[case(700_000_000, "700 MB")]
    #[case(1_430_000_000, "1.4 GB")]
    fn writes_a_size(#[case] bytes: u64, #[case] expected: &str) {
        assert_eq!(size(bytes), expected);
    }

    #[rstest]
    #[case(3840, 2160, "2160p")]
    #[case(1920, 800, "1080p")]
    #[case(1280, 720, "720p")]
    #[case(720, 480, "480p")]
    fn names_a_resolution(#[case] width: u32, #[case] height: u32, #[case] expected: &str) {
        assert_eq!(resolution(width, height), expected);
    }
}
