//! Upcoming releases and missing files of monitored items (FR-6.3, 6.4, 7.1–7.4).

use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MovieId, SeriesId};

use super::{FileStatus, detail::Release};
#[cfg(feature = "server")]
use crate::api::{Calendar, Dep};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Period {
    /// From today, a year ahead; `day` is ignored.
    #[default]
    ComingUp,
    /// Monday through Sunday.
    Week,
    Month,
}

/// The monitored releases of one period.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Agenda {
    pub from: Date,
    pub to: Date,
    /// Today in the server's time zone.
    pub today: Date,
    /// Ordered by date.
    pub entries: Vec<AgendaEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgendaEntry {
    pub date: Date,
    pub item: ItemId,
    pub title: String,
    pub release: AgendaRelease,
    pub status: FileStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgendaRelease {
    Episode { season: u16, number: u16, title: String },
    Movie(Release),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Missing {
    /// By title.
    pub series: Vec<MissingSeries>,
    /// By title.
    pub movies: Vec<MissingMovie>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissingSeries {
    pub id: SeriesId,
    pub title: String,
    pub year: Option<i16>,
    pub episodes: Vec<MissingEpisode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissingEpisode {
    pub season: u16,
    pub number: u16,
    pub title: String,
    pub air_date: Date,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissingMovie {
    pub id: MovieId,
    pub title: String,
    pub year: Option<i16>,
}

impl Period {
    pub const ALL: [Self; 3] = [Self::ComingUp, Self::Week, Self::Month];

    pub fn label(self) -> &'static str {
        match self {
            Self::ComingUp => "Coming up",
            Self::Week => "Week",
            Self::Month => "Month",
        }
    }
}

/// The `period` holding `day`, or holding today when `day` is `None`.
#[get("/api/calendar?period&day", calendar: Dep<Calendar>)]
pub async fn agenda(period: Option<Period>, day: Option<Date>) -> Result<Agenda, ServerFnError> {
    server::agenda(&calendar, period.unwrap_or_default(), day).await
}

#[get("/api/missing", calendar: Dep<Calendar>)]
pub async fn missing() -> Result<Missing, ServerFnError> {
    server::missing(&calendar).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::{logger::tracing::error, prelude::*};
    use jiff::{ToSpan, civil::Date};
    use yokoku_library::{CalendarEntry, CalendarRelease, month_of, week_of};

    use super::{
        Agenda, AgendaEntry, AgendaRelease, Calendar, Missing, MissingEpisode, MissingMovie, MissingSeries, Period,
    };

    pub(super) async fn agenda(
        calendar: &Calendar,
        period: Period,
        day: Option<Date>,
    ) -> Result<Agenda, ServerFnError> {
        let today = calendar.today();
        let day = day.unwrap_or(today);
        let (from, to) = match period {
            Period::ComingUp => (today, today.saturating_add(1.year())),
            Period::Week => week_of(day),
            Period::Month => month_of(day),
        };
        let entries = calendar.entries(from, to).await.map_err(|error| {
            error!(%error, %from, %to, "loading the calendar failed");
            ServerFnError::new("The calendar could not be loaded")
        })?;
        Ok(Agenda { from, to, today, entries: entries.into_iter().map(AgendaEntry::from).collect() })
    }

    pub(super) async fn missing(calendar: &Calendar) -> Result<Missing, ServerFnError> {
        let missing = calendar.missing().await.map_err(|error| {
            error!(%error, "listing missing files failed");
            ServerFnError::new("Missing files could not be loaded")
        })?;
        Ok(Missing {
            series: missing
                .series
                .into_iter()
                .map(|series| MissingSeries {
                    id: series.id,
                    title: series.title,
                    year: series.year,
                    episodes: series
                        .episodes
                        .into_iter()
                        .map(|episode| MissingEpisode {
                            season: episode.reference.season,
                            number: episode.reference.episode,
                            title: episode.title,
                            air_date: episode.air_date,
                        })
                        .collect(),
                })
                .collect(),
            movies: missing
                .movies
                .into_iter()
                .map(|movie| MissingMovie { id: movie.id, title: movie.title, year: movie.year })
                .collect(),
        })
    }

    impl From<CalendarEntry> for AgendaEntry {
        fn from(entry: CalendarEntry) -> Self {
            let release = match entry.release {
                CalendarRelease::Episode { reference, title } => {
                    AgendaRelease::Episode { season: reference.season, number: reference.episode, title }
                },
                CalendarRelease::Movie(kind) => AgendaRelease::Movie(kind.into()),
            };
            Self { date: entry.date, item: entry.item, title: entry.title, release, status: entry.status.into() }
        }
    }
}
