use std::path::PathBuf;

use yokoku_domain::{Confidence, Episode, EpisodeRef, EpisodeSpan, FileTarget, Movie, Numbering, Series};

use crate::{
    DownloadFile, Numbers, ParsedName, Video, classify,
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

impl ImportPlan {
    /// Every row is certain and free of conflicts, so the plan can be imported without review.
    pub fn is_automatic(&self) -> bool {
        !self.rows.is_empty()
            && self.rows.iter().all(|row| row.confidence == Confidence::Certain && row.conflicts.is_empty())
    }
}

/// Matches every video in a download to the library (FR-4).
pub fn plan(files: &[DownloadFile], target: Target<'_>) -> ImportPlan {
    let classified = classify(files);
    let mut ignored = classified.ignored;
    let videos = classified.videos;

    let mut rows = match target {
        Target::Series(series) => videos.into_iter().map(|video| series_row(video, |_| Some((series, true)))).collect(),
        Target::Movie(movie) => movie_rows(videos, &mut ignored, |_| Some((movie, true))),
        Target::Library { series, movies } => {
            let series_rows: Vec<_> =
                videos.iter().cloned().map(|video| series_row(video, |parsed| choose(parsed, series))).collect();
            let mut movie_ignored = Vec::new();
            let movie_rows = movie_rows(videos, &mut movie_ignored, |parsed| choose(parsed, movies));

            let matched = |rows: &[PlanRow]| rows.iter().any(|row| row.target.is_some());
            if !matched(&series_rows) && matched(&movie_rows) {
                ignored.extend(movie_ignored);
                movie_rows
            } else {
                series_rows
            }
        },
    };

    mark_shared_targets(&mut rows);
    ImportPlan { rows, ignored }
}

fn series_row<'a>(video: Video, choose_series: impl Fn(&ParsedName) -> Option<(&'a Series, bool)>) -> PlanRow {
    let parsed = ParsedName::parse(&video.path);
    let resolved = choose_series(&parsed).and_then(|(series, series_certain)| {
        let (span, episodes_certain) = resolve_episodes(series, &parsed)?;
        Some((series, span, series_certain && episodes_certain))
    });

    let Some((series, span, certain)) = resolved else {
        return unknown(video, parsed);
    };
    let has_file = span.refs().any(|reference| series.episode(reference).is_some_and(|episode| episode.file.is_some()));
    PlanRow {
        video,
        parsed,
        target: Some(FileTarget::Episodes { series: series.id, span }),
        confidence: if certain { Confidence::Certain } else { Confidence::Guess },
        conflicts: has_file.then_some(Conflict::AlreadyHasFile).into_iter().collect(),
    }
}

/// Only the largest video is the movie; the others are extras (FR-4.13).
fn movie_rows<'a>(
    videos: Vec<Video>,
    ignored: &mut Vec<PathBuf>,
    choose_movie: impl Fn(&ParsedName) -> Option<(&'a Movie, bool)>,
) -> Vec<PlanRow> {
    let Some(main) = videos.iter().enumerate().max_by_key(|(_, video)| video.size).map(|(index, _)| index) else {
        return Vec::new();
    };
    let mut main_video = None;
    for (index, video) in videos.into_iter().enumerate() {
        if index == main {
            main_video = Some(video);
        } else {
            ignored.push(video.path);
            ignored.extend(video.subtitles.into_iter().map(|subtitle| subtitle.path));
        }
    }
    let video = main_video.expect("the largest video is in the list");
    let parsed = ParsedName::parse(&video.path);

    let row = match choose_movie(&parsed) {
        Some((movie, certain)) => PlanRow {
            video,
            parsed,
            target: Some(FileTarget::Movie(movie.id)),
            confidence: if certain { Confidence::Certain } else { Confidence::Guess },
            conflicts: movie.file.is_some().then_some(Conflict::AlreadyHasFile).into_iter().collect(),
        },
        None => unknown(video, parsed),
    };
    vec![row]
}

fn unknown(video: Video, parsed: ParsedName) -> PlanRow {
    PlanRow { video, parsed, target: None, confidence: Confidence::Unknown, conflicts: Vec::new() }
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

/// `None` when both years are known and more than one year apart.
fn year_fit(parsed: Option<i16>, item: Option<i16>) -> Option<YearFit> {
    match (parsed, item) {
        (Some(parsed), Some(item)) if parsed == item => Some(YearFit::Exact),
        (Some(parsed), Some(item)) if (i32::from(parsed) - i32::from(item)).abs() == 1 => Some(YearFit::Near),
        (Some(_), Some(_)) => None,
        _ => Some(YearFit::Unknown),
    }
}

/// The single best title match; certain only for an exact title whose year agrees or is unknown (FR-4.6).
fn choose<'a, T: Titled>(parsed: &ParsedName, items: &'a [T]) -> Option<(&'a T, bool)> {
    let title = parsed.title.as_deref()?;
    let scored: Vec<_> = items
        .iter()
        .filter_map(|item| {
            let fit = year_fit(parsed.year, item.year())?;
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

/// The episodes a parsed name refers to, and whether that reading is certain (FR-4.3, 4.5, 4.8, 4.9).
fn resolve_episodes(series: &Series, parsed: &ParsedName) -> Option<(EpisodeSpan, bool)> {
    match &parsed.numbers {
        Numbers::Episodes { season, episodes } => Some((existing_span(series, *season, episodes)?, true)),
        Numbers::Seasonless { episodes } => match series.numbering {
            Numbering::Absolute => Some((absolute_span(series, episodes)?, true)),
            Numbering::Standard => {
                let regular: Vec<u16> =
                    series.seasons.iter().map(|season| season.number).filter(|&number| number != 0).collect();
                let guess = match regular.as_slice() {
                    [only] => existing_span(series, *only, episodes),
                    _ => absolute_span(series, episodes),
                };
                Some((guess?, false))
            },
        },
        Numbers::Date(date) => {
            unique(series.numbered_episodes().filter(|(_, episode)| episode.air_date == Some(*date)))
        },
        Numbers::None => {
            let title = normalize(parsed.episode_title.as_deref().or(parsed.title.as_deref())?);
            if title.is_empty() {
                return None;
            }
            unique(series.numbered_episodes().filter(|(_, episode)| normalize(&episode.title) == title))
        },
    }
}

fn unique<'a>(mut matching: impl Iterator<Item = (EpisodeRef, &'a Episode)>) -> Option<(EpisodeSpan, bool)> {
    let (reference, _) = matching.next()?;
    matching.next().is_none().then_some((EpisodeSpan::single(reference), true))
}

fn existing_span(series: &Series, season: u16, episodes: &[u16]) -> Option<EpisodeSpan> {
    let span = consecutive(season, episodes)?;
    span.refs().all(|reference| series.episode(reference).is_some()).then_some(span)
}

fn absolute_span(series: &Series, episodes: &[u16]) -> Option<EpisodeSpan> {
    let refs: Vec<EpisodeRef> =
        episodes.iter().map(|&number| series.absolute_to_ref(u32::from(number))).collect::<Option<_>>()?;
    let season = refs.first()?.season;
    if refs.iter().any(|reference| reference.season != season) {
        return None;
    }
    consecutive(season, &refs.iter().map(|reference| reference.episode).collect::<Vec<_>>())
}

/// A span only when the sorted episode numbers have no gaps.
fn consecutive(season: u16, episodes: &[u16]) -> Option<EpisodeSpan> {
    let (&first, &last) = (episodes.first()?, episodes.last()?);
    if usize::from(last.checked_sub(first)?) + 1 != episodes.len() {
        return None;
    }
    EpisodeSpan::new(season, first, last)
}

fn mark_shared_targets(rows: &mut [PlanRow]) {
    for i in 0..rows.len() {
        for j in i + 1..rows.len() {
            if overlaps(rows[i].target, rows[j].target) {
                for index in [i, j] {
                    if !rows[index].conflicts.contains(&Conflict::SharedTarget) {
                        rows[index].conflicts.push(Conflict::SharedTarget);
                    }
                }
            }
        }
    }
}

fn overlaps(a: Option<FileTarget>, b: Option<FileTarget>) -> bool {
    match (a, b) {
        (
            Some(FileTarget::Episodes { series: a_series, span: a_span }),
            Some(FileTarget::Episodes { series: b_series, span: b_span }),
        ) => a_series == b_series && a_span.refs().any(|reference| b_span.refs().any(|other| other == reference)),
        (Some(FileTarget::Movie(a)), Some(FileTarget::Movie(b))) => a == b,
        _ => false,
    }
}
