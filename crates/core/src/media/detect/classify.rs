use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use isolang::Language;
use yokoku_domain::SubtitleTags;

const VIDEO_EXTENSIONS: &[&str] =
    &["mkv", "mp4", "m4v", "avi", "mov", "wmv", "ts", "m2ts", "webm", "mpg", "mpeg", "flv", "ogm"];
const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "sub", "idx", "vtt", "sup", "smi", "mks"];
const AUDIO_EXTENSIONS: &[&str] = &["mka", "ac3", "eac3", "dts", "aac", "flac", "mp3", "m4a", "opus"];
const EXTRAS_FOLDERS: &[&str] = &[
    "sample",
    "samples",
    "extras",
    "featurettes",
    "behind the scenes",
    "deleted scenes",
    "trailers",
    "interviews",
    "shorts",
];
/// Lowercase English names, each to the first language with that name.
static LANGUAGE_NAMES: LazyLock<HashMap<String, Language>> = LazyLock::new(|| {
    let mut names = HashMap::new();
    for language in isolang::languages() {
        names.entry(language.to_name().to_ascii_lowercase()).or_insert(language);
    }
    names
});

/// ISO 639-2/B codes and their ISO 639-3 equivalents.
const BIBLIOGRAPHIC_CODES: [(&str, &str); 20] = [
    ("alb", "sqi"),
    ("arm", "hye"),
    ("baq", "eus"),
    ("bur", "mya"),
    ("chi", "zho"),
    ("cze", "ces"),
    ("dut", "nld"),
    ("fre", "fra"),
    ("geo", "kat"),
    ("ger", "deu"),
    ("gre", "ell"),
    ("ice", "isl"),
    ("mac", "mkd"),
    ("mao", "mri"),
    ("may", "msa"),
    ("per", "fas"),
    ("rum", "ron"),
    ("slo", "slk"),
    ("tib", "bod"),
    ("wel", "cym"),
];

/// A file from a folder listing, with its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedFile {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Classified {
    /// Ordered by path.
    pub videos: Vec<Video>,
    pub ignored: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub path: PathBuf,
    pub size: u64,
    /// Ordered by path.
    pub sidecars: Vec<Sidecar>,
}

/// An external subtitle or audio track of a video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    pub path: PathBuf,
    pub kind: SidecarKind,
    /// What its name in the library carries after the video's name: the words of its folders
    /// below the video's, then the rest of its own name, as dot-separated parts.
    pub suffix: String,
    pub tags: SubtitleTags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarKind {
    Subtitle,
    Audio,
}

enum Kind {
    Video,
    Sidecar(SidecarKind),
    Junk,
}

impl Classified {
    /// Keeps videos, attaches subtitles and audio tracks to them and ignores samples, extras and
    /// everything else.
    pub fn from_files(files: &[ListedFile]) -> Self {
        let mut classified = Self::default();
        let mut sidecars = Vec::new();
        for file in files {
            match Kind::of(&file.path) {
                Kind::Video => {
                    classified.videos.push(Video { path: file.path.clone(), size: file.size, sidecars: Vec::new() });
                },
                Kind::Sidecar(kind) => sidecars.push((file.path.clone(), kind)),
                Kind::Junk => classified.ignored.push(file.path.clone()),
            }
        }
        classified.videos.sort_by(|a, b| a.path.cmp(&b.path));

        for (path, kind) in sidecars {
            match classified.owner(&path, kind) {
                Some(index) => {
                    let video = &mut classified.videos[index];
                    let sidecar = Sidecar::new(path, kind, &video.path);
                    video.sidecars.push(sidecar);
                },
                None => classified.ignored.push(path),
            }
        }
        for video in &mut classified.videos {
            video.sidecars.sort_by(|a, b| a.path.cmp(&b.path));
        }
        classified
    }

    /// The video named as the sidecar's prefix, the video its folder is named after, or, for a
    /// subtitle, the only video.
    fn owner(&self, sidecar: &Path, kind: SidecarKind) -> Option<usize> {
        let sidecar_stem = stem(sidecar);
        let by_prefix = self
            .videos
            .iter()
            .enumerate()
            .filter(|(_, video)| rest_of(&sidecar_stem, &stem(&video.path)).is_some())
            .max_by_key(|(_, video)| stem(&video.path).len())
            .map(|(index, _)| index);

        let folder = sidecar.parent().and_then(Path::file_name).map(|folder| folder.to_string_lossy().into_owned());
        let by_folder = || self.videos.iter().position(|video| Some(stem(&video.path)) == folder);
        let only_video = || (kind == SidecarKind::Subtitle && self.videos.len() == 1).then_some(0);

        by_prefix.or_else(by_folder).or_else(only_video)
    }
}

impl Kind {
    fn of(path: &Path) -> Self {
        let extension = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let in_extras = path
            .parent()
            .into_iter()
            .flat_map(Path::iter)
            .any(|folder| EXTRAS_FOLDERS.contains(&folder.to_string_lossy().to_lowercase().as_str()));
        let is_sample =
            stem(path).split(|c: char| !c.is_alphanumeric()).any(|word| word.eq_ignore_ascii_case("sample"));

        match extension.as_str() {
            _ if in_extras => Self::Junk,
            extension if VIDEO_EXTENSIONS.contains(&extension) && !is_sample => Self::Video,
            extension if SUBTITLE_EXTENSIONS.contains(&extension) => Self::Sidecar(SidecarKind::Subtitle),
            extension if AUDIO_EXTENSIONS.contains(&extension) => Self::Sidecar(SidecarKind::Audio),
            _ => Self::Junk,
        }
    }
}

impl Sidecar {
    fn new(path: PathBuf, kind: SidecarKind, video: &Path) -> Self {
        let video_stem = stem(video);
        let own_stem = stem(&path);
        let folder_words =
            folders_below(path.parent().unwrap_or(Path::new("")), video.parent().unwrap_or(Path::new("")))
                .into_iter()
                .filter(|folder| *folder != video_stem)
                .flat_map(|folder| words(&folder).map(str::to_owned).collect::<Vec<_>>())
                .collect::<Vec<_>>();
        let rest = rest_of(&own_stem, &video_stem).unwrap_or(&own_stem);
        let suffix = folder_words.iter().map(String::as_str).chain(words(rest)).collect::<Vec<_>>().join(".");

        let mut tags = Self::tags(&own_stem);
        if tags.language.is_none() {
            tags.language = folder_words.iter().find_map(|word| Self::language(&word.to_lowercase()));
        }
        Self { path, kind, suffix, tags }
    }

    /// Language and flags from the end of `stem`, stopping at the first unrecognised word.
    fn tags(stem: &str) -> SubtitleTags {
        let stem = stem.to_lowercase();
        let mut tags = SubtitleTags::default();
        for word in stem.split(['.', '_', ' ']).rev() {
            match word {
                "forced" => tags.forced = true,
                "sdh" | "hi" | "cc" => tags.sdh = true,
                word => match Self::language(word) {
                    Some(language) if tags.language.is_none() => tags.language = Some(language),
                    Some(_) => {},
                    None => break,
                },
            }
        }
        tags
    }

    /// An ISO 639-1 code, with the region kept: `en`, `pt-br`.
    fn language(word: &str) -> Option<String> {
        if let Some((code, region)) = word.split_once('-') {
            let valid_region = (2..=3).contains(&region.len()) && region.chars().all(|c| c.is_ascii_alphanumeric());
            return (valid_region && Language::from_639_1(code).is_some()).then(|| word.to_owned());
        }
        let code =
            BIBLIOGRAPHIC_CODES.iter().find(|(bibliographic, _)| *bibliographic == word).map_or(word, |(_, code)| code);
        let language = match code.len() {
            2 => Language::from_639_1(code),
            3 => Language::from_639_3(code),
            _ if code.chars().all(char::is_alphabetic) => LANGUAGE_NAMES.get(code).copied(),
            _ => None,
        };
        language?.to_639_1().map(str::to_owned)
    }
}

/// What follows `prefix` in `stem`, when `stem` is `prefix` or starts with `prefix.`.
fn rest_of<'a>(stem: &'a str, prefix: &str) -> Option<&'a str> {
    stem.strip_prefix(prefix).filter(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// The names of the folders of `folder` below where it parts from `base`.
fn folders_below(folder: &Path, base: &Path) -> Vec<String> {
    let shared = folder.components().zip(base.components()).take_while(|(a, b)| a == b).count();
    folder.components().skip(shared).map(|component| component.as_os_str().to_string_lossy().into_owned()).collect()
}

/// The words of a name, split at dots, underscores and spaces.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(['.', '_', ' ']).filter(|word| !word.is_empty())
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}
