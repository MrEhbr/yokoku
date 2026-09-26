//! Pure naming: Jellyfin-compatible paths from naming templates.

mod naming;
mod sanitize;
mod subtitle;
mod template;

pub use naming::{Naming, NamingError, NamingTemplates, PatternKind, TemplateError};
pub use sanitize::sanitize;
pub use subtitle::subtitle_path;
