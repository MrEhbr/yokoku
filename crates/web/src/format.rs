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
