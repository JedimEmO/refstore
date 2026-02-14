use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::RefstoreError;
use crate::git;
use crate::model::{
    Bundle, GlobalConfig, Reference, ReferenceSource, RepositoryIndex,
};

pub struct RepositoryStore {
    root: PathBuf,
    index: RepositoryIndex,
    config: GlobalConfig,
}

impl RepositoryStore {
    pub fn open(data_dir: Option<&Path>) -> Result<Self, RefstoreError> {
        let root = match data_dir {
            Some(dir) => dir.to_path_buf(),
            None => default_data_dir()?,
        };

        fs::create_dir_all(&root).map_err(|source| RefstoreError::DirCreate {
            path: root.clone(),
            source,
        })?;
        fs::create_dir_all(root.join("content")).map_err(|source| RefstoreError::DirCreate {
            path: root.join("content"),
            source,
        })?;

        let index = load_index(&root)?;
        let config = load_config(&root)?;

        Ok(Self {
            root,
            index,
            config,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> &GlobalConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut GlobalConfig {
        &mut self.config
    }

    pub fn save_config(&self) -> Result<(), RefstoreError> {
        let path = self.root.join("config.toml");
        let content = toml::to_string_pretty(&self.config)?;
        fs::write(&path, content).map_err(|source| RefstoreError::FileWrite { path, source })?;
        Ok(())
    }

    pub fn content_path(&self, name: &str) -> PathBuf {
        self.root.join("content").join(name)
    }

    pub fn add(&mut self, reference: Reference) -> Result<(), RefstoreError> {
        if self.index.references.contains_key(&reference.name) {
            return Err(RefstoreError::ReferenceExists {
                name: reference.name,
            });
        }

        validate_name(&reference.name)?;

        let content_dir = self.content_path(&reference.name);
        self.fetch_content(&reference, &content_dir)?;

        self.index
            .references
            .insert(reference.name.clone(), reference);
        self.save_index()?;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<Reference, RefstoreError> {
        let reference = self
            .index
            .references
            .remove(name)
            .ok_or_else(|| RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            })?;

        let content_dir = self.content_path(name);
        if content_dir.exists() {
            let _ = fs::remove_dir_all(&content_dir);
        }

        self.save_index()?;
        Ok(reference)
    }

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

    pub fn update(&mut self, name: &str) -> Result<(), RefstoreError> {
        let reference = self
            .index
            .references
            .get(name)
            .ok_or_else(|| RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            })?
            .clone();

        let content_dir = self.content_path(name);
        if content_dir.exists() {
            let _ = fs::remove_dir_all(&content_dir);
        }

        self.fetch_content(&reference, &content_dir)?;

        if let Some(r) = self.index.references.get_mut(name) {
            r.last_synced = Some(Utc::now());
            if let ReferenceSource::Git { .. } = &r.source {
                if let Ok(hash) = git::head_hash(&content_dir) {
                    r.checksum = Some(hash);
                }
            }
        }
        self.save_index()?;
        Ok(())
    }

    // --- Bundle operations ---

    pub fn add_bundle(&mut self, bundle: Bundle) -> Result<(), RefstoreError> {
        if self.index.bundles.contains_key(&bundle.name) {
            return Err(RefstoreError::BundleExists {
                name: bundle.name,
            });
        }
        validate_name(&bundle.name)?;

        for ref_name in &bundle.references {
            if !self.index.references.contains_key(ref_name) {
                return Err(RefstoreError::BundleInvalidReference {
                    bundle: bundle.name.clone(),
                    reference: ref_name.clone(),
                });
            }
        }

        self.index.bundles.insert(bundle.name.clone(), bundle);
        self.save_index()?;
        Ok(())
    }

    pub fn remove_bundle(&mut self, name: &str) -> Result<Bundle, RefstoreError> {
        let bundle = self
            .index
            .bundles
            .remove(name)
            .ok_or_else(|| RefstoreError::BundleNotFound {
                name: name.to_string(),
            })?;
        self.save_index()?;
        Ok(bundle)
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

    pub fn update_bundle(
        &mut self,
        name: &str,
        add_refs: Vec<String>,
        remove_refs: Vec<String>,
        description: Option<String>,
    ) -> Result<(), RefstoreError> {
        // Validate new refs exist before mutating
        for ref_name in &add_refs {
            if !self.index.references.contains_key(ref_name) {
                return Err(RefstoreError::BundleInvalidReference {
                    bundle: name.to_string(),
                    reference: ref_name.clone(),
                });
            }
        }

        let bundle = self
            .index
            .bundles
            .get_mut(name)
            .ok_or_else(|| RefstoreError::BundleNotFound {
                name: name.to_string(),
            })?;

        for ref_name in add_refs {
            if !bundle.references.contains(&ref_name) {
                bundle.references.push(ref_name);
            }
        }
        bundle.references.retain(|r| !remove_refs.contains(r));

        if let Some(desc) = description {
            bundle.description = Some(desc);
        }

        self.save_index()?;
        Ok(())
    }

    fn fetch_content(
        &self,
        reference: &Reference,
        content_dir: &Path,
    ) -> Result<(), RefstoreError> {
        match &reference.source {
            ReferenceSource::Local { path } => {
                if path.is_file() {
                    fs::create_dir_all(content_dir).map_err(|source| {
                        RefstoreError::DirCreate {
                            path: content_dir.to_path_buf(),
                            source,
                        }
                    })?;
                    let dest = content_dir.join(path.file_name().unwrap_or("file".as_ref()));
                    fs::copy(path, &dest).map_err(|source| RefstoreError::FileRead {
                        path: path.clone(),
                        source,
                    })?;
                } else if path.is_dir() {
                    copy_dir_recursive(path, content_dir)?;
                } else {
                    return Err(RefstoreError::FileRead {
                        path: path.clone(),
                        source: std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "source path does not exist",
                        ),
                    });
                }
            }
            ReferenceSource::Git { url, r#ref, .. } => {
                git::ensure_git()?;
                git::clone_shallow(
                    url,
                    content_dir,
                    r#ref.as_deref(),
                    self.config.git_depth,
                )?;
            }
            ReferenceSource::Remote { url } => {
                return Err(RefstoreError::SyncFailed {
                    name: reference.name.clone(),
                    reason: format!("remote sources not yet supported: {url}"),
                });
            }
        }
        Ok(())
    }

    fn save_index(&self) -> Result<(), RefstoreError> {
        let path = self.root.join("index.toml");
        let content = toml::to_string_pretty(&self.index)?;
        fs::write(&path, content).map_err(|source| RefstoreError::FileWrite { path, source })?;
        Ok(())
    }
}

fn default_data_dir() -> Result<PathBuf, RefstoreError> {
    dirs::data_dir()
        .map(|d| d.join("refstore"))
        .ok_or(RefstoreError::DataDirNotFound)
}

fn load_index(root: &Path) -> Result<RepositoryIndex, RefstoreError> {
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

fn load_config(root: &Path) -> Result<GlobalConfig, RefstoreError> {
    let path = root.join("config.toml");
    if !path.exists() {
        return Ok(GlobalConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|source| RefstoreError::FileRead {
        path: path.clone(),
        source,
    })?;
    let config: GlobalConfig = toml::from_str(&content)?;
    Ok(config)
}

fn validate_name(name: &str) -> Result<(), RefstoreError> {
    if name.is_empty() {
        return Err(RefstoreError::InvalidName {
            name: name.to_string(),
            reason: "name cannot be empty".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(RefstoreError::InvalidName {
            name: name.to_string(),
            reason: "name must contain only alphanumeric characters, hyphens, underscores, or dots"
                .to_string(),
        });
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), RefstoreError> {
    fs::create_dir_all(dst).map_err(|source| RefstoreError::DirCreate {
        path: dst.to_path_buf(),
        source,
    })?;

    for entry in walkdir::WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| RefstoreError::FileRead {
            path: src.to_path_buf(),
            source: e.into(),
        })?;

        let relative = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RefstoreError::DirCreate {
                path: target.clone(),
                source,
            })?;
        } else {
            fs::copy(entry.path(), &target).map_err(|source| RefstoreError::FileRead {
                path: entry.path().to_path_buf(),
                source,
            })?;
        }
    }
    Ok(())
}
