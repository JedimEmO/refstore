use std::fs;
use std::path::{Path, PathBuf};

use crate::error::RefstoreError;
use crate::model::{Manifest, ManifestEntry};

const MANIFEST_FILE: &str = "refstore.toml";

pub struct ProjectStore {
    root: PathBuf,
    manifest: Manifest,
}

impl ProjectStore {
    pub fn open(start_dir: Option<&Path>) -> Result<Self, RefstoreError> {
        let start = start_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let root = find_manifest_root(&start).ok_or(RefstoreError::ManifestNotFound)?;
        let manifest = load_manifest(&root)?;

        Ok(Self { root, manifest })
    }

    pub fn init(path: Option<&Path>, gitignore: bool) -> Result<Self, RefstoreError> {
        let root = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let manifest_path = root.join(MANIFEST_FILE);
        if manifest_path.exists() {
            return Err(RefstoreError::ManifestExists(manifest_path));
        }

        let manifest = Manifest::new(gitignore);

        let refs_dir = root.join(".references");
        fs::create_dir_all(&refs_dir).map_err(|source| RefstoreError::DirCreate {
            path: refs_dir,
            source,
        })?;

        if gitignore {
            append_gitignore(&root)?;
        }

        let store = Self { root, manifest };
        store.save_manifest()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn references_dir(&self) -> PathBuf {
        self.root.join(".references")
    }

    pub fn add_reference(
        &mut self,
        name: String,
        entry: ManifestEntry,
    ) -> Result<(), RefstoreError> {
        if self.manifest.references.contains_key(&name) {
            return Err(RefstoreError::ReferenceExists { name });
        }
        self.manifest.references.insert(name, entry);
        self.save_manifest()?;
        Ok(())
    }

    pub fn remove_reference(&mut self, name: &str) -> Result<ManifestEntry, RefstoreError> {
        let entry = self
            .manifest
            .references
            .remove(name)
            .ok_or_else(|| RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            })?;
        self.save_manifest()?;
        Ok(entry)
    }

    fn save_manifest(&self) -> Result<(), RefstoreError> {
        let path = self.root.join(MANIFEST_FILE);
        let content = toml::to_string_pretty(&self.manifest)?;
        fs::write(&path, content).map_err(|source| RefstoreError::FileWrite { path, source })?;
        Ok(())
    }
}

fn find_manifest_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(MANIFEST_FILE).exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn load_manifest(root: &Path) -> Result<Manifest, RefstoreError> {
    let path = root.join(MANIFEST_FILE);
    let content = fs::read_to_string(&path).map_err(|source| RefstoreError::FileRead {
        path: path.clone(),
        source,
    })?;
    let manifest: Manifest = toml::from_str(&content)?;
    Ok(manifest)
}

fn append_gitignore(root: &Path) -> Result<(), RefstoreError> {
    let gitignore_path = root.join(".gitignore");
    let marker = ".references/";

    if gitignore_path.exists() {
        let content =
            fs::read_to_string(&gitignore_path).map_err(|source| RefstoreError::FileRead {
                path: gitignore_path.clone(),
                source,
            })?;
        if content.lines().any(|l| l.trim() == marker) {
            return Ok(());
        }
        let append = if content.ends_with('\n') {
            format!("{marker}\n")
        } else {
            format!("\n{marker}\n")
        };
        fs::write(&gitignore_path, content + &append).map_err(|source| {
            RefstoreError::FileWrite {
                path: gitignore_path,
                source,
            }
        })?;
    } else {
        fs::write(&gitignore_path, format!("{marker}\n")).map_err(|source| {
            RefstoreError::FileWrite {
                path: gitignore_path,
                source,
            }
        })?;
    }
    Ok(())
}
