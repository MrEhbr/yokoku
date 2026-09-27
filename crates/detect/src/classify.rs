use std::path::{Path, PathBuf};

use isolang::Language;
use yokoku_domain::SubtitleTags;

const VIDEO_EXTENSIONS: &[&str] =
    &["mkv", "mp4", "m4v", "avi", "mov", "wmv", "ts", "m2ts", "webm", "mpg", "mpeg", "flv", "ogm"];
const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "sub", "idx", "vtt", "sup", "smi"];
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
    pub subtitles: Vec<Subtitle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtitle {
    pub path: PathBuf,
    pub tags: SubtitleTags,
}

enum Kind {
    Video,
    Subtitle,
    Junk,
}

impl Classified {
    /// Keeps videos, attaches subtitles to them and ignores samples, extras and everything else.
    pub fn from_files(files: &[ListedFile]) -> Self {
        let mut classified = Self::default();
        let mut subtitles = Vec::new();
        for file in files {
            match Kind::of(&file.path) {
                Kind::Video => {
                    classified.videos.push(Video { path: file.path.clone(), size: file.size, subtitles: Vec::new() });
                },
                Kind::Subtitle => subtitles.push(file.path.clone()),
                Kind::Junk => classified.ignored.push(file.path.clone()),
            }
        }
        classified.videos.sort_by(|a, b| a.path.cmp(&b.path));

        for path in subtitles {
            match classified.owner(&path) {
                Some(index) => classified.videos[index].subtitles.push(Subtitle::new(path)),
                None => classified.ignored.push(path),
            }
        }
        classified
    }

    /// The video named as the subtitle's prefix, the video its folder is named after, or the only video.
    fn owner(&self, subtitle: &Path) -> Option<usize> {
        let subtitle_stem = stem(subtitle);
        let by_prefix = self
            .videos
            .iter()
            .enumerate()
            .filter(|(_, video)| {
                let video_stem = stem(&video.path);
                subtitle_stem.strip_prefix(&video_stem).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
            })
            .max_by_key(|(_, video)| stem(&video.path).len())
            .map(|(index, _)| index);

        let folder = subtitle.parent().and_then(Path::file_name).map(|folder| folder.to_string_lossy().into_owned());
        let by_folder = || self.videos.iter().position(|video| Some(stem(&video.path)) == folder);
        let only_video = || (self.videos.len() == 1).then_some(0);

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
            extension if SUBTITLE_EXTENSIONS.contains(&extension) => Self::Subtitle,
            _ => Self::Junk,
        }
    }
}

impl Subtitle {
    /// Reads language and flags from the end of the name, stopping at the first unrecognised word.
    fn new(path: PathBuf) -> Self {
        let stem = stem(&path).to_lowercase();
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
        Self { path, tags }
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
            _ => Language::from_name_lowercase(code),
        };
        language?.to_639_1().map(str::to_owned)
    }
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}
