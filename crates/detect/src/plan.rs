use std::{fmt, path::PathBuf};

use yokoku_domain::{Confidence, Episode, EpisodeSpan, FileTarget, Movie, Numbering, Series};

use crate::{
    Classified, DownloadFile, Numbers, ParsedName, Video,
    titles::{TitleMatch, best_match, normalize},
};

/// What the download was added for (FR-4.7).
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    Series(&'a Series),
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
    /// Matches every video in a download to the library (FR-4).
    pub fn new(files: &[DownloadFile], target: Target<'_>) -> Self {
        let Classified { videos, mut ignored } = Classified::from_files(files);

        let rows = match target {
            Target::Series(series) => {
                videos.into_iter().map(|video| PlanRow::episodes(video, |_| Some((series, true)))).collect()
            },
            Target::Movie(movie) => PlanRow::movie(videos, &mut ignored, |_| Some((movie, true))).into_iter().collect(),
            Target::Library { series, movies } => {
                let series_rows: Vec<_> = videos
                    .iter()
                    .cloned()
                    .map(|video| PlanRow::episodes(video, |parsed| parsed.choose(series)))
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

    /// Every row is certain and free of conflicts, so the plan can be imported without review.
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
    fn episodes<'a>(video: Video, choose_series: impl Fn(&ParsedName) -> Option<(&'a Series, bool)>) -> Self {
        let parsed = ParsedName::parse(&video.path);
        let resolved = choose_series(&parsed).and_then(|(series, series_certain)| {
            let (span, episodes_certain) = parsed.episodes_in(series)?;
            Some((series, span, series_certain && episodes_certain))
        });

        let Some((series, span, certain)) = resolved else {
            return Self::unknown(video, parsed);
        };
        let has_file =
            span.refs().any(|reference| series.episode(reference).is_some_and(|episode| episode.file.is_some()));
        Self::matched(video, parsed, FileTarget::Episodes { series: series.id, span }, certain, has_file)
    }

    /// Only the largest video is the movie; the others are extras (FR-4.13).
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
            confidence: if certain { Confidence::Certain } else { Confidence::Guess },
            conflicts: has_file.then_some(Conflict::AlreadyHasFile).into_iter().collect(),
        }
    }

    fn unknown(video: Video, parsed: ParsedName) -> Self {
        Self { video, parsed, target: None, confidence: Confidence::Unknown, conflicts: Vec::new() }
    }
}

trait Titled {
    fn titles(&self) -> impl Iterator<Item = &str>;
    fn year(&self) -> Option<i16>;
}

impl Titled for Series {
    fn titles(&self) -> impl Iterator<Item = &str> {
        [&self.title, &self.original_title].into_iter().chain(&self.alternate_titles).map(String::as_str)
    }

    fn year(&self) -> Option<i16> {
        self.year
    }
}

impl Titled for Movie {
    fn titles(&self) -> impl Iterator<Item = &str> {
        [&self.title, &self.original_title].into_iter().chain(&self.alternate_titles).map(String::as_str)
    }

    fn year(&self) -> Option<i16> {
        self.year
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum YearFit {
    /// One year apart, e.g. a festival premiere against the release year.
    Near,
    Unknown,
    Exact,
}

impl YearFit {
    /// `None` when both years are known and more than one year apart.
    fn of(parsed: Option<i16>, item: Option<i16>) -> Option<Self> {
        match (parsed, item) {
            (Some(parsed), Some(item)) if parsed == item => Some(Self::Exact),
            (Some(parsed), Some(item)) if (i32::from(parsed) - i32::from(item)).abs() == 1 => Some(Self::Near),
            (Some(_), Some(_)) => None,
            _ => Some(Self::Unknown),
        }
    }
}

impl ParsedName {
    /// The single best title match; certain only for an exact title whose year agrees or is unknown (FR-4.6).
    fn choose<'a, T: Titled>(&self, items: &'a [T]) -> Option<(&'a T, bool)> {
        let title = self.title.as_deref()?;
        let scored: Vec<_> = items
            .iter()
            .filter_map(|item| {
                let fit = YearFit::of(self.year, item.year())?;
                Some((item, (best_match(title, item.titles())?, fit)))
            })
            .collect();

        let best = scored.iter().map(|(_, score)| *score).max()?;
        let mut top = scored.into_iter().filter(|(_, score)| *score == best);
        let (item, (title_match, fit)) = top.next()?;
        if top.next().is_some() {
            return None;
        }
        Some((item, title_match == TitleMatch::Exact && fit != YearFit::Near))
    }

    /// The episodes this name refers to, and whether that reading is certain (FR-4.3, 4.5, 4.8, 4.9).
    fn episodes_in(&self, series: &Series) -> Option<(EpisodeSpan, bool)> {
        let only = |matches: &dyn Fn(&Episode) -> bool| {
            let mut matching = series.numbered_episodes().filter(|(_, episode)| matches(episode));
            let (reference, _) = matching.next()?;
            matching.next().is_none().then_some((EpisodeSpan::single(reference), true))
        };

        match &self.numbers {
            Numbers::Episodes { season, episodes } => Some((series.span(*season, episodes)?, true)),
            Numbers::Seasonless { episodes } => match series.numbering {
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
            Numbers::Date(date) => only(&|episode| episode.air_date == Some(*date)),
            Numbers::None => {
                let title = normalize(self.episode_title.as_deref().or(self.title.as_deref())?);
                if title.is_empty() {
                    return None;
                }
                only(&|episode| normalize(&episode.title) == title)
            },
        }
    }
}
