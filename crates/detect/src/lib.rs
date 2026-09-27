//! Pure detection: downloaded files to an import plan.

mod classify;
mod parse;
mod plan;
mod titles;

pub use classify::{Classified, ListedFile, Subtitle, Video};
pub use parse::{EpisodeHint, ParsedName};
pub use plan::{Conflict, ImportPlan, MatchScope, PlanRow};
