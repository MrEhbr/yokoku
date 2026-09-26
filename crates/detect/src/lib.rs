//! Pure detection: downloaded files to an import plan.

mod classify;
mod parse;
mod plan;
mod titles;

pub use classify::{Classified, DownloadFile, Subtitle, Video};
pub use parse::{Numbers, ParsedName};
pub use plan::{Conflict, ImportPlan, PlanRow, Target, plan};
