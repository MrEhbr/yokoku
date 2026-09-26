use std::{fmt, path::PathBuf};

use yokoku_domain::{EpisodeRef, EpisodeSpan, Movie, Series};

use crate::{
    sanitize::{file_name, sanitize},
    template::{Template, Token},
};

/// User-editable patterns, one per path component. Tokens: `{title}`, `{year}`, `{season}` (two digits),
/// `{episodes}` (`S01E01` or `S01E01-E02`), `{episode_title}`; `[...]` is dropped when a token inside has no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingTemplates {
    pub movie_folder: String,
    pub movie_file: String,
    pub series_folder: String,
    pub season_folder: String,
    pub episode_file: String,
}

impl Default for NamingTemplates {
    /// Jellyfin's recommended layout.
    fn default() -> Self {
        Self {
            movie_folder: "{title}[ ({year})]".into(),
            movie_file: "{title}[ ({year})]".into(),
            series_folder: "{title}[ ({year})]".into(),
            season_folder: "Season {season}".into(),
            episode_file: "{title}[ ({year})] - {episodes}[ - {episode_title}]".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    MovieFolder,
    MovieFile,
    SeriesFolder,
    SeasonFolder,
    EpisodeFile,
}

impl fmt::Display for PatternKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MovieFolder => "movie folder",
            Self::MovieFile => "movie file",
            Self::SeriesFolder => "series folder",
            Self::SeasonFolder => "season folder",
            Self::EpisodeFile => "episode file",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{pattern} pattern: {message}")]
pub struct TemplateError {
    pub pattern: PatternKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NamingError {
    #[error("episode {0} is not part of the series")]
    UnknownEpisode(EpisodeRef),
}

/// Validated patterns that turn library items into paths relative to their root folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    movie_folder: Template,
    movie_file: Template,
    series_folder: Template,
    season_folder: Template,
    episode_file: Template,
}

impl Naming {
    pub fn new(templates: &NamingTemplates) -> Result<Self, TemplateError> {
        use Token::*;

        let parse = |pattern: PatternKind, source: &str, allowed: &[Token], required: &[Token]| {
            Template::parse(source, allowed, required).map_err(|message| TemplateError { pattern, message })
        };
        Ok(Self {
            movie_folder: parse(PatternKind::MovieFolder, &templates.movie_folder, &[Title, Year], &[])?,
            movie_file: parse(PatternKind::MovieFile, &templates.movie_file, &[Title, Year], &[])?,
            series_folder: parse(PatternKind::SeriesFolder, &templates.series_folder, &[Title, Year], &[])?,
            season_folder: parse(PatternKind::SeasonFolder, &templates.season_folder, &[Season], &[Season])?,
            episode_file: parse(
                PatternKind::EpisodeFile,
                &templates.episode_file,
                &[Title, Year, Season, Episodes, EpisodeTitle],
                &[Episodes],
            )?,
        })
    }

    /// `Movie Folder/Movie File.ext`
    pub fn movie_path(&self, movie: &Movie, extension: &str) -> PathBuf {
        let value = |token| match token {
            Token::Title => Some(movie.title.clone()),
            Token::Year => movie.year.map(|year| year.to_string()),
            _ => None,
        };
        [sanitize(&self.movie_folder.render(&value)), file_name(&self.movie_file.render(&value), extension)]
            .iter()
            .collect()
    }

    /// `Series Folder/Season Folder/Episode File.ext`
    pub fn episode_path(&self, series: &Series, span: EpisodeSpan, extension: &str) -> Result<PathBuf, NamingError> {
        let mut titles = Vec::new();
        for reference in span.refs() {
            let episode = series.episode(reference).ok_or(NamingError::UnknownEpisode(reference))?;
            if !episode.title.is_empty() {
                titles.push(episode.title.as_str());
            }
        }
        let episode_title = (!titles.is_empty()).then(|| titles.join(" + "));

        let value = |token| match token {
            Token::Title => Some(series.title.clone()),
            Token::Year => series.year.map(|year| year.to_string()),
            Token::Season => Some(format!("{:02}", span.season())),
            Token::Episodes => Some(span.to_string()),
            Token::EpisodeTitle => episode_title.clone(),
        };
        Ok([
            sanitize(&self.series_folder.render(&value)),
            sanitize(&self.season_folder.render(&value)),
            file_name(&self.episode_file.render(&value), extension),
        ]
        .iter()
        .collect())
    }
}

impl Default for Naming {
    fn default() -> Self {
        Self::new(&NamingTemplates::default()).expect("default templates are valid")
    }
}
