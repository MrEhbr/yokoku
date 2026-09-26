use std::{collections::HashMap, fmt};

use jiff::{Timestamp, civil::Date};
use serde::{Deserialize, Serialize};

use crate::{EpisodeId, ExternalId, FileStatus, MediaFileId, SeriesId};

const SPECIALS: u16 = 0;

/// Series status as the metadata source reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatus {
    Returning,
    Planned,
    InProduction,
    Pilot,
    Ended,
    Canceled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStatus {
    Continuing,
    OnBreak,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Numbering {
    #[default]
    Standard,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorPreset {
    All,
    Future,
    LatestSeason,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EpisodeRef {
    pub season: u16,
    pub episode: u16,
}

impl fmt::Display for EpisodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S{:02}E{:02}", self.season, self.episode)
    }
}

/// Consecutive episodes of one season, as held by a multi-episode file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "SpanFields")]
pub struct EpisodeSpan {
    season: u16,
    first: u16,
    last: u16,
}

impl EpisodeSpan {
    /// `None` when `first` comes after `last`.
    pub fn new(season: u16, first: u16, last: u16) -> Option<Self> {
        (first <= last).then_some(Self { season, first, last })
    }

    pub fn single(reference: EpisodeRef) -> Self {
        Self { season: reference.season, first: reference.episode, last: reference.episode }
    }

    pub fn season(&self) -> u16 {
        self.season
    }

    pub fn first(&self) -> u16 {
        self.first
    }

    pub fn last(&self) -> u16 {
        self.last
    }

    pub fn refs(&self) -> impl Iterator<Item = EpisodeRef> {
        let season = self.season;
        (self.first..=self.last).map(move |episode| EpisodeRef { season, episode })
    }
}

#[derive(Deserialize)]
struct SpanFields {
    season: u16,
    first: u16,
    last: u16,
}

impl TryFrom<SpanFields> for EpisodeSpan {
    type Error = &'static str;

    fn try_from(fields: SpanFields) -> Result<Self, Self::Error> {
        Self::new(fields.season, fields.first, fields.last).ok_or("the first episode comes after the last")
    }
}

impl fmt::Display for EpisodeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", EpisodeRef { season: self.season, episode: self.first })?;
        if self.last != self.first {
            write!(f, "-E{:02}", self.last)?;
        }
        Ok(())
    }
}

/// A series as its metadata source describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesMetadata {
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub status: SourceStatus,
    pub seasons: Vec<SeasonMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonMetadata {
    pub number: u16,
    pub episodes: Vec<EpisodeMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeMetadata {
    pub source_id: u64,
    pub number: u16,
    pub title: String,
    pub air_date: Option<Date>,
}

/// Seasons and episodes are ordered by number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    pub id: SeriesId,
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    pub poster_path: Option<String>,
    pub source_status: SourceStatus,
    pub numbering: Numbering,
    pub monitored: bool,
    pub seasons: Vec<Season>,
    pub added_at: Timestamp,
    pub refreshed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Season {
    pub number: u16,
    pub monitored: bool,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub id: EpisodeId,
    pub source_id: u64,
    pub number: u16,
    pub title: String,
    pub air_date: Option<Date>,
    pub monitored: bool,
    pub file: Option<MediaFileId>,
}

impl Series {
    /// Specials are never monitored by a preset.
    pub fn add(metadata: SeriesMetadata, preset: MonitorPreset, today: Date, now: Timestamp) -> Self {
        let latest_season = metadata.seasons.iter().map(|season| season.number).filter(|&n| n != SPECIALS).max();

        let seasons = metadata
            .seasons
            .into_iter()
            .map(|season| {
                let monitored = season.number != SPECIALS
                    && match preset {
                        MonitorPreset::All | MonitorPreset::Future => true,
                        MonitorPreset::LatestSeason => Some(season.number) == latest_season,
                        MonitorPreset::None => false,
                    };
                let episodes = season
                    .episodes
                    .into_iter()
                    .map(|episode| {
                        let wanted = preset != MonitorPreset::Future || !has_aired(episode.air_date, today);
                        Episode::new(episode, monitored && wanted)
                    })
                    .collect();
                Season { number: season.number, monitored, episodes }
            })
            .collect();

        let mut series = Self {
            id: SeriesId::generate(),
            source: metadata.source,
            title: metadata.title,
            original_title: metadata.original_title,
            year: metadata.year,
            poster_path: metadata.poster_path,
            source_status: metadata.status,
            numbering: Numbering::default(),
            monitored: preset != MonitorPreset::None,
            seasons,
            added_at: now,
            refreshed_at: now,
        };
        series.sort();
        series
    }

    /// Episodes are matched by source id, so renumbered episodes keep their id, flags and file.
    /// New seasons follow the series flag (specials excepted); new episodes follow their season.
    pub fn refresh(&mut self, metadata: SeriesMetadata, now: Timestamp) {
        let season_flags: HashMap<u16, bool> =
            self.seasons.iter().map(|season| (season.number, season.monitored)).collect();
        let mut known: HashMap<u64, Episode> = self
            .seasons
            .drain(..)
            .flat_map(|season| season.episodes)
            .map(|episode| (episode.source_id, episode))
            .collect();
        let series_monitored = self.monitored;

        self.seasons = metadata
            .seasons
            .into_iter()
            .map(|season| {
                let monitored =
                    season_flags.get(&season.number).copied().unwrap_or(series_monitored && season.number != SPECIALS);
                let episodes = season
                    .episodes
                    .into_iter()
                    .map(|episode| match known.remove(&episode.source_id) {
                        Some(existing) => Episode {
                            number: episode.number,
                            title: episode.title,
                            air_date: episode.air_date,
                            ..existing
                        },
                        None => Episode::new(episode, monitored),
                    })
                    .collect();
                Season { number: season.number, monitored, episodes }
            })
            .collect();

        self.title = metadata.title;
        self.original_title = metadata.original_title;
        self.year = metadata.year;
        self.poster_path = metadata.poster_path;
        self.source_status = metadata.status;
        self.refreshed_at = now;
        self.sort();
    }

    pub fn status(&self, today: Date) -> SeriesStatus {
        match self.source_status {
            SourceStatus::Ended | SourceStatus::Canceled => SeriesStatus::Ended,
            _ if self.next_episode(today).is_some() => SeriesStatus::Continuing,
            _ => SeriesStatus::OnBreak,
        }
    }

    pub fn episodes(&self) -> impl Iterator<Item = &Episode> {
        self.seasons.iter().flat_map(|season| &season.episodes)
    }

    pub fn numbered_episodes(&self) -> impl Iterator<Item = (EpisodeRef, &Episode)> {
        self.seasons.iter().flat_map(|season| {
            season
                .episodes
                .iter()
                .map(|episode| (EpisodeRef { season: season.number, episode: episode.number }, episode))
        })
    }

    /// The earliest followed episode airing today or later.
    pub fn next_episode(&self, today: Date) -> Option<(EpisodeRef, &Episode)> {
        self.followed_episodes()
            .filter_map(|(reference, episode)| Some((episode.air_date?, reference, episode)))
            .filter(|&(date, ..)| date >= today)
            .min_by_key(|&(date, reference, _)| (date, reference))
            .map(|(_, reference, episode)| (reference, episode))
    }

    /// The latest followed episode that aired before today.
    pub fn last_aired(&self, today: Date) -> Option<(EpisodeRef, &Episode)> {
        self.followed_episodes()
            .filter_map(|(reference, episode)| Some((episode.air_date?, reference, episode)))
            .filter(|&(date, ..)| date < today)
            .max_by_key(|&(date, reference, _)| (date, reference))
            .map(|(_, reference, episode)| (reference, episode))
    }

    /// Regular episodes plus specials monitored at season and episode level.
    fn followed_episodes(&self) -> impl Iterator<Item = (EpisodeRef, &Episode)> {
        self.seasons.iter().flat_map(|season| {
            season
                .episodes
                .iter()
                .filter(move |episode| season.number != SPECIALS || (season.monitored && episode.monitored))
                .map(|episode| (EpisodeRef { season: season.number, episode: episode.number }, episode))
        })
    }

    /// Episodes monitored at series, season and episode level.
    pub fn monitored_episodes(&self) -> impl Iterator<Item = (EpisodeRef, &Episode)> {
        self.seasons.iter().filter(|season| self.monitored && season.monitored).flat_map(|season| {
            season
                .episodes
                .iter()
                .filter(|episode| episode.monitored)
                .map(|episode| (EpisodeRef { season: season.number, episode: episode.number }, episode))
        })
    }

    pub fn season_mut(&mut self, number: u16) -> Option<&mut Season> {
        self.seasons.iter_mut().find(|season| season.number == number)
    }

    pub fn episode(&self, reference: EpisodeRef) -> Option<&Episode> {
        self.numbered_episodes().find(|&(candidate, _)| candidate == reference).map(|(_, episode)| episode)
    }

    pub fn episode_mut(&mut self, reference: EpisodeRef) -> Option<&mut Episode> {
        self.season_mut(reference.season)?.episodes.iter_mut().find(|episode| episode.number == reference.episode)
    }

    /// Absolute numbers are 1-based and count episodes in order, excluding specials.
    pub fn absolute_to_ref(&self, absolute: u32) -> Option<EpisodeRef> {
        let index = usize::try_from(absolute.checked_sub(1)?).ok()?;
        self.seasons
            .iter()
            .filter(|season| season.number != SPECIALS)
            .flat_map(|season| {
                season.episodes.iter().map(|episode| EpisodeRef { season: season.number, episode: episode.number })
            })
            .nth(index)
    }

    fn sort(&mut self) {
        self.seasons.sort_by_key(|season| season.number);
        for season in &mut self.seasons {
            season.episodes.sort_by_key(|episode| episode.number);
        }
    }
}

impl Episode {
    fn new(metadata: EpisodeMetadata, monitored: bool) -> Self {
        Self {
            id: EpisodeId::generate(),
            source_id: metadata.source_id,
            number: metadata.number,
            title: metadata.title,
            air_date: metadata.air_date,
            monitored,
            file: None,
        }
    }

    /// An episode counts as aired from the day after its air date.
    pub fn file_status(&self, today: Date) -> FileStatus {
        if self.file.is_some() {
            FileStatus::Downloaded
        } else if has_aired(self.air_date, today) {
            FileStatus::Missing
        } else {
            FileStatus::Upcoming
        }
    }
}

fn has_aired(air_date: Option<Date>, today: Date) -> bool {
    air_date.is_some_and(|date| date < today)
}
