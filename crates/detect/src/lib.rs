//! Pure detection: downloaded files to an import plan.

mod classify;
mod parse;

pub use classify::{Classified, DownloadFile, Subtitle, Video, classify};
pub use parse::{Numbers, ParsedName, parse};
