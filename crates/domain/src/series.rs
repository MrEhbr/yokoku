use std::collections::{BTreeMap, HashMap};

use jiff::{SignedDuration, Timestamp, ToSpan, civil::Date};
use serde::{Deserialize, Serialize};

use crate::{
    Artwork, Description, EpisodeId, EpisodeRef, EpisodeSpan, ExternalId, FileStatus, ItemFolder, MediaFileId,
    SeriesId, events::RenumberedFile,
};

const SPECIALS: u16 = 0;

/// `None` unless the ordered `episodes` are one gapless run in one season.
fn span_of(episodes: &[EpisodeRef]) -> Option<EpisodeSpan> {
    let season = episodes.first()?.season;
    if episodes.iter().any(|episode| episode.season != season) {
        return None;
    }
    let numbers: Vec<u16> = episodes.iter().map(|episode| episode.episode).collect();
    EpisodeSpan::consecutive(season, &numbers)
}

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

crate::string_enum!(SourceStatus, "source status" {
    Returning => "returning",
    Planned => "planned",
    InProduction => "in_production",
    Pilot => "pilot",
    Ended => "ended",
    Canceled => "canceled",
    Unknown => "unknown",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStatus {
    Continuing,
    OnBreak,
    Ended,
}

crate::string_enum!(SeriesStatus, "series status" {
    Continuing => "continuing",
    OnBreak => "on_break",
    Ended => "ended",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Numbering {
    #[default]
    Standard,
    Absolute,
}

crate::string_enum!(Numbering, "numbering" {
    Standard => "standard",
    Absolute => "absolute",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MonitorPreset {
    #[default]
    All,
    Future,
    LatestSeason,
    None,
}

/// A series as its metadata source describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesMetadata {
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    /// Other names the item is known by, such as romanisations.
    pub alternate_titles: Vec<String>,
    pub year: Option<i16>,
    pub artwork: Artwork,
    pub description: Description,
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
    /// Empty when the source has none.
    pub overview: String,
    pub air_date: Option<Date>,
}

/// Seasons and episodes are ordered by number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    pub id: SeriesId,
    pub source: ExternalId,
    pub title: String,
    pub original_title: String,
    /// Other names the item is known by, such as romanisations.
    pub alternate_titles: Vec<String>,
    pub year: Option<i16>,
    pub artwork: Artwork,
    pub description: Description,
    pub source_status: SourceStatus,
    pub numbering: Numbering,
    /// Set when the series is added; never changes.
    pub folder: ItemFolder,
    pub monitored: bool,
    pub seasons: Vec<Season>,
    pub added_at: Timestamp,
    pub refreshed_at: Timestamp,
    /// Saves so far; storage refuses a save made from an older revision.
    pub revision: u64,
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
    /// Empty when the source has none.
    pub overview: String,
    pub air_date: Option<Date>,
    pub monitored: bool,
    pub file: Option<MediaFileId>,
}

impl Series {
    /// Specials are never monitored by a preset.
    pub fn new(
        metadata: SeriesMetadata,
        folder: ItemFolder,
        preset: MonitorPreset,
        today: Date,
        now: Timestamp,
    ) -> Self {
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
            alternate_titles: metadata.alternate_titles,
            year: metadata.year,
            artwork: metadata.artwork,
            description: metadata.description,
            source_status: metadata.status,
            numbering: Numbering::default(),
            folder,
            monitored: preset != MonitorPreset::None,
            seasons,
            added_at: now,
            refreshed_at: now,
            revision: 0,
        };
        series.sort();
        series
    }

    /// Episodes are matched by source id, so renumbered episodes keep their id, flags and file.
    /// New seasons follow the series flag (specials excepted); new episodes follow their season.
    /// Returns the files whose episodes changed numbers.
    pub fn refresh(&mut self, metadata: SeriesMetadata, now: Timestamp) -> Vec<RenumberedFile> {
        let files_before = self.file_episodes();
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
                            overview: episode.overview,
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
        self.alternate_titles = metadata.alternate_titles;
        self.year = metadata.year;
        self.artwork = metadata.artwork;
        self.description = metadata.description;
        self.source_status = metadata.status;
        self.refreshed_at = now;
        self.sort();

        let renumbered: Vec<RenumberedFile> = self
            .file_episodes()
            .into_iter()
            .filter(|(file, episodes)| files_before.get(file) != Some(episodes))
            .map(|(file, episodes)| RenumberedFile { file, span: span_of(&episodes) })
            .collect();
        for split in renumbered.iter().filter(|renumbered| renumbered.span.is_none()) {
            for episode in self.seasons.iter_mut().flat_map(|season| &mut season.episodes) {
                if episode.file == Some(split.file) {
                    episode.file = None;
                }
            }
        }
        renumbered
    }

    /// The episodes holding each file, in order.
    fn file_episodes(&self) -> BTreeMap<MediaFileId, Vec<EpisodeRef>> {
        let mut files: BTreeMap<MediaFileId, Vec<EpisodeRef>> = BTreeMap::new();
        for season in &self.seasons {
            for episode in &season.episodes {
                if let Some(file) = episode.file {
                    files.entry(file).or_default().push(EpisodeRef { season: season.number, episode: episode.number });
                }
            }
        }
        files
    }

    pub fn status(&self, today: Date) -> SeriesStatus {
        match self.source_status {
            SourceStatus::Ended | SourceStatus::Canceled => SeriesStatus::Ended,
            _ if self.next_episode(today).is_some() => SeriesStatus::Continuing,
            _ => SeriesStatus::OnBreak,
        }
    }

    /// Sonarr's rules: refreshed over 30 days ago, or an aired regular episode still untitled; else
    /// not within 6 hours of the last refresh, and still running or with an episode in the last 30 days.
    pub fn needs_refresh(&self, now: Timestamp, today: Date) -> bool {
        let age = now.duration_since(self.refreshed_at);
        let untitled = self.seasons.iter().filter(|season| season.number != SPECIALS).any(|season| {
            season.episodes.iter().any(|episode| {
                episode.air_date.is_some_and(|date| date < today) && matches!(episode.title.as_str(), "" | "TBA")
            })
        });
        if age > SignedDuration::from_hours(30 * 24) || untitled {
            return true;
        }
        if age < SignedDuration::from_hours(6) {
            return false;
        }
        let recent = today.saturating_sub(30.days());
        !matches!(self.source_status, SourceStatus::Ended | SourceStatus::Canceled)
            || self.episodes().filter_map(|episode| episode.air_date).any(|date| date > recent)
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

    /// The consecutive `episodes` of a season, when every one of them exists.
    pub fn span(&self, season: u16, episodes: &[u16]) -> Option<EpisodeSpan> {
        let span = EpisodeSpan::consecutive(season, episodes)?;
        span.refs().all(|reference| self.episode(reference).is_some()).then_some(span)
    }

    /// The consecutive absolute `episodes`, when they fall in one season.
    pub fn absolute_span(&self, episodes: &[u16]) -> Option<EpisodeSpan> {
        let refs: Vec<EpisodeRef> =
            episodes.iter().map(|&number| self.absolute_to_ref(u32::from(number))).collect::<Option<_>>()?;
        let season = refs.first()?.season;
        if refs.iter().any(|reference| reference.season != season) {
            return None;
        }
        EpisodeSpan::consecutive(season, &refs.iter().map(|reference| reference.episode).collect::<Vec<_>>())
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
            overview: metadata.overview,
            air_date: metadata.air_date,
            monitored,
            file: None,
        }
    }

    /// An episode counts as aired from the day after its air date.
    pub fn file_status(&self, today: Date) -> FileStatus {
        match self.file {
            Some(_) => FileStatus::Downloaded,
            None if has_aired(self.air_date, today) => FileStatus::Missing,
            None => FileStatus::Upcoming,
        }
    }
}

fn has_aired(air_date: Option<Date>, today: Date) -> bool {
    air_date.is_some_and(|date| date < today)
}
