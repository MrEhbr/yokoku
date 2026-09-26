//! Pure detection: downloaded files to an import plan.

mod classify;
mod parse;
mod plan;
mod titles;

pub use classify::{Classified, DownloadFile, Subtitle, Video, classify};
pub use parse::{Numbers, ParsedName, parse};
pub use plan::{Conflict, ImportPlan, MatchTarget, PlanRow, Target, plan};
