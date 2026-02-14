use std::fs;
use std::path::{Path, PathBuf};

use crate::error::RefstoreError;
use crate::model::{Bundle, Reference, RepositoryIndex};

/// A registry is a directory containing an index.toml and a content/ subdirectory.
/// It can be the local registry (writable) or a remote submodule registry (read-only).
pub struct RegistryStore {
    root: PathBuf,
    index: RepositoryIndex,
}

impl RegistryStore {
    /// Open a registry from a directory.
    pub fn open(root: &Path) -> Result<Self, RefstoreError> {
        let index = load_registry_index(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            index,
        })
    }

    pub fn content_path(&self, name: &str) -> PathBuf {
        self.root.join("content").join(name)
    }

    // --- Read operations ---

    pub fn get(&self, name: &str) -> Option<&Reference> {
        self.index.references.get(name)
    }

    pub fn list(&self, tag: Option<&str>, kind: Option<&str>) -> Vec<&Reference> {
        self.index
            .references
            .values()
            .filter(|r| {
                if let Some(t) = tag {
                    if !r.tags.iter().any(|rt| rt == t) {
                        return false;
                    }
                }
                if let Some(k) = kind {
                    if r.kind.to_string() != k {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn get_bundle(&self, name: &str) -> Option<&Bundle> {
        self.index.bundles.get(name)
    }

    pub fn list_bundles(&self, tag: Option<&str>) -> Vec<&Bundle> {
        self.index
            .bundles
            .values()
            .filter(|b| {
                if let Some(t) = tag {
                    b.tags.iter().any(|bt| bt == t)
                } else {
                    true
                }
            })
            .collect()
    }

    // --- Write operations ---

    pub fn index_mut(&mut self) -> &mut RepositoryIndex {
        &mut self.index
    }

    pub fn save_index(&self) -> Result<(), RefstoreError> {
        let path = self.root.join("index.toml");
        let content = toml::to_string_pretty(&self.index)?;
        fs::write(&path, content).map_err(|source| RefstoreError::FileWrite { path, source })?;
        Ok(())
    }
}

/// Load a registry index from a directory.
fn load_registry_index(root: &Path) -> Result<RepositoryIndex, RefstoreError> {
    let path = root.join("index.toml");
    if !path.exists() {
        return Ok(RepositoryIndex::default());
    }
    let content = fs::read_to_string(&path).map_err(|source| RefstoreError::FileRead {
        path: path.clone(),
        source,
    })?;

    let index: RepositoryIndex = toml::from_str(&content)?;
    Ok(index)
}
