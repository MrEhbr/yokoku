use std::{fmt, path::PathBuf};

use yokoku_domain::{Confidence, Episode, EpisodeSpan, FileTarget, Movie, Numbering, Series, SeriesId};

use crate::media::{
    Episodes, RowMatch,
    detect::{Classified, EpisodeHint, ListedFile, ParsedName, Video, titles::normalize},
};

/// What the files are matched against.
#[derive(Debug, Clone, Copy)]
pub enum MatchScope<'a> {
    Series(&'a Series),
    /// A series whose names without a season are in `season`.
    SeriesSeason {
        series: &'a Series,
        season: u16,
    },
    Movie(&'a Movie),
    /// Not linked: match against everything in the library.
    Library {
        series: &'a [Series],
        movies: &'a [Movie],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    /// Ordered by video path.
    pub rows: Vec<PlanRow>,
    pub ignored: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    pub video: Video,
    pub parsed: ParsedName,
    pub target: Option<FileTarget>,
    /// The series chosen for the file, even when its episodes were not found in it.
    pub series: Option<SeriesId>,
    pub confidence: Confidence,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// Another row in the plan has the same episode or movie.
    SharedTarget,
    /// The episode or movie already has a file in the library.
    AlreadyHasFile,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SharedTarget => "same as another row",
            Self::AlreadyHasFile => "already has a file",
        })
    }
}

impl ImportPlan {
    /// Matches every video in `files` to the library.
    pub fn new(files: &[ListedFile], scope: MatchScope<'_>) -> Self {
        let Classified { videos, mut ignored } = Classified::from_files(files);

        let rows = match scope {
            MatchScope::Series(series) => {
                videos.into_iter().map(|video| PlanRow::episodes(video, None, |_| Some((series, true)))).collect()
            },
            MatchScope::SeriesSeason { series, season } => videos
                .into_iter()
                .map(|video| PlanRow::episodes(video, Some(season), |_| Some((series, true))))
                .collect(),
            MatchScope::Movie(movie) => {
                PlanRow::movie(videos, &mut ignored, |_| Some((movie, true))).into_iter().collect()
            },
            MatchScope::Library { series, movies } => {
                let series_rows: Vec<_> = videos
                    .iter()
                    .cloned()
                    .map(|video| PlanRow::episodes(video, None, |parsed| parsed.choose(series)))
                    .collect();
                let mut movie_ignored = Vec::new();
                let movie_row = PlanRow::movie(videos, &mut movie_ignored, |parsed| parsed.choose(movies));

                let movie_matched = movie_row.as_ref().is_some_and(|row| row.target.is_some());
                if movie_matched && series_rows.iter().all(|row| row.target.is_none()) {
                    ignored.extend(movie_ignored);
                    movie_row.into_iter().collect()
                } else {
                    series_rows
                }
            },
        };

        let mut plan = Self { rows, ignored };
        plan.mark_shared_targets();
        plan
    }

    /// Every row is certain and free of conflicts.
    pub fn is_automatic(&self) -> bool {
        !self.rows.is_empty()
            && self.rows.iter().all(|row| row.confidence == Confidence::Certain && row.conflicts.is_empty())
    }

    fn mark_shared_targets(&mut self) {
        let shared: Vec<bool> = (0..self.rows.len())
            .map(|i| {
                self.rows.iter().enumerate().any(|(j, other)| match (self.rows[i].target, other.target) {
                    (Some(target), Some(other)) => i != j && target.overlaps(&other),
                    _ => false,
                })
            })
            .collect();
        for (row, shared) in self.rows.iter_mut().zip(shared) {
            if shared {
                row.conflicts.push(Conflict::SharedTarget);
            }
        }
    }
}

impl PlanRow {
    fn episodes<'a>(
        video: Video,
        season: Option<u16>,
        choose_series: impl Fn(&ParsedName) -> Option<(&'a Series, bool)>,
    ) -> Self {
        let parsed = ParsedName::parse(&video.path);
        let resolved = choose_series(&parsed).and_then(|(series, series_certain)| {
            let (span, episodes_certain) = parsed.episodes_in(series, season)?;
            Some((series, span, series_certain && episodes_certain))
        });

        let Some((series, span, certain)) = resolved else {
            let series = choose_series(&parsed).map(|(series, _)| series.id);
            return Self { series, ..Self::unknown(video, parsed) };
        };
        let has_file =
            span.refs().any(|reference| series.episode(reference).is_some_and(|episode| episode.file.is_some()));
        Self::matched(video, parsed, FileTarget::Episodes { series: series.id, span }, certain, has_file)
    }

    /// What the row is known to hold: its match, or else its series with the season and episode
    /// numbers its name gives, its seasonless numbers placed in `season`.
    pub fn row_match(&self, season: Option<u16>) -> RowMatch {
        if let Some(target) = self.target {
            return target.into();
        }
        let Some(series) = self.series else { return RowMatch::None };
        let (named, episodes) = match &self.parsed.episode_hint {
            EpisodeHint::Episodes { season, episodes } => (Some(*season), Episodes::spanning(episodes)),
            EpisodeHint::Seasonless { episodes } => (season, Episodes::spanning(episodes)),
            EpisodeHint::Date(_) | EpisodeHint::None => (season, None),
        };
        RowMatch::Series { series, season: named, episodes }
    }

    /// Only the largest video is the movie; the others are extras.
    fn movie<'a>(
        mut videos: Vec<Video>,
        ignored: &mut Vec<PathBuf>,
        choose_movie: impl Fn(&ParsedName) -> Option<(&'a Movie, bool)>,
    ) -> Option<Self> {
        let main = videos.iter().enumerate().max_by_key(|(_, video)| video.size)?.0;
        let video = videos.remove(main);
        for extra in videos {
            ignored.push(extra.path);
            ignored.extend(extra.subtitles.into_iter().map(|subtitle| subtitle.path));
        }

        let parsed = ParsedName::parse(&video.path);
        Some(match choose_movie(&parsed) {
            Some((movie, certain)) => {
                Self::matched(video, parsed, FileTarget::Movie(movie.id), certain, movie.file.is_some())
            },
            None => Self::unknown(video, parsed),
        })
    }

    fn matched(video: Video, parsed: ParsedName, target: FileTarget, certain: bool, has_file: bool) -> Self {
        Self {
            video,
            parsed,
            target: Some(target),
            series: match target {
                FileTarget::Episodes { series, .. } => Some(series),
                FileTarget::Movie(_) => None,
            },
            confidence: if certain { Confidence::Certain } else { Confidence::Guess },
            conflicts: has_file.then_some(Conflict::AlreadyHasFile).into_iter().collect(),
        }
    }

    fn unknown(video: Video, parsed: ParsedName) -> Self {
        Self { video, parsed, target: None, series: None, confidence: Confidence::Unknown, conflicts: Vec::new() }
    }
}

impl ParsedName {
    /// The episodes this name refers to, and whether that reading is certain.
    /// A name without a season is in `season` when one is given.
    fn episodes_in(&self, series: &Series, given: Option<u16>) -> Option<(EpisodeSpan, bool)> {
        let only = |matches: &dyn Fn(&Episode) -> bool| {
            let mut matching = series.numbered_episodes().filter(|(_, episode)| matches(episode));
            let (reference, _) = matching.next()?;
            matching.next().is_none().then_some((EpisodeSpan::single(reference), true))
        };

        match (&self.episode_hint, given) {
            (EpisodeHint::Episodes { season, episodes }, _) => Some((series.span(*season, episodes)?, true)),
            (EpisodeHint::Seasonless { episodes }, Some(season)) => Some((series.span(season, episodes)?, true)),
            (EpisodeHint::Seasonless { episodes }, None) => match series.numbering {
                Numbering::Absolute => Some((series.absolute_span(episodes)?, true)),
                Numbering::Standard => {
                    let regular: Vec<u16> =
                        series.seasons.iter().map(|season| season.number).filter(|&number| number != 0).collect();
                    let guess = match regular.as_slice() {
                        [only] => series.span(*only, episodes),
                        _ => series.absolute_span(episodes),
                    };
                    Some((guess?, false))
                },
            },
            (EpisodeHint::Date(date), _) => only(&|episode| episode.air_date == Some(*date)),
            (EpisodeHint::None, _) => {
                let title = normalize(self.episode_title.as_deref().or(self.title.as_deref())?);
                if title.is_empty() {
                    return None;
                }
                only(&|episode| normalize(&episode.title) == title)
            },
        }
    }
}
