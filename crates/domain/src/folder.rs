use std::path::{Component, Path, PathBuf};

/// Where an item's files live: a folder named `name` directly in the root folder `root` (FR-8.1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemFolder {
    pub root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a folder name")]
pub struct InvalidFolderName(pub String);

impl ItemFolder {
    /// `name` must be a single path component.
    pub fn new(root: PathBuf, name: String) -> Result<Self, InvalidFolderName> {
        let mut components = Path::new(&name).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(part)), None) if part == name.as_str() => Ok(Self { root, name }),
            _ => Err(InvalidFolderName(name)),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }
}
