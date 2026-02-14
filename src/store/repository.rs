use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::RefstoreError;
use crate::git;
use crate::model::{
    Bundle, GlobalConfig, Reference, ReferenceSource, Registry,
};

use super::registry::RegistryStore;

/// Information about where a reference was resolved from.
pub struct ResolvedReference<'a> {
    pub reference: &'a Reference,
    pub content_path: PathBuf,
    pub registry_name: &'a str,
}

pub struct RepositoryStore {
    root: PathBuf,
    local: RegistryStore,
    remotes: Vec<(String, RegistryStore)>,
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

        let config = load_config(&root)?;
        let local = RegistryStore::open(&root)?;

        // Ensure the data dir is a git repo
        git::init(&root)?;
        git::ensure_gitignore(&root, &["config.toml"])?;

        // If this is a fresh init (no commits yet), do an initial commit
        if git::head_hash(&root).is_err() {
            git::commit(&root, &["."], "Initialize refstore repository")?;
        }

        // Load remote registries from submodules
        let remotes = load_remote_registries(&root);

        Ok(Self {
            root,
            local,
            remotes,
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
        self.local.content_path(name)
    }

    // --- Multi-registry resolution ---

    /// Resolve a reference by name across all registries.
    /// Local registry is searched first, then remote registries.
    pub fn resolve(&self, name: &str) -> Option<ResolvedReference<'_>> {
        if let Some(r) = self.local.get(name) {
            return Some(ResolvedReference {
                reference: r,
                content_path: self.local.content_path(name),
                registry_name: "local",
            });
        }
        for (reg_name, store) in &self.remotes {
            if let Some(r) = store.get(name) {
                return Some(ResolvedReference {
                    reference: r,
                    content_path: store.content_path(name),
                    registry_name: reg_name,
                });
            }
        }
        None
    }

    /// Get a reference by name (searches all registries, local first).
    pub fn get(&self, name: &str) -> Option<&Reference> {
        self.resolve(name).map(|r| r.reference)
    }

    /// Get the content path for a reference, resolving across registries.
    pub fn resolve_content_path(&self, name: &str) -> Option<PathBuf> {
        self.resolve(name).map(|r| r.content_path)
    }

    /// List all references across all registries.
    /// Local references take precedence (dedup by name).
    pub fn list(&self, tag: Option<&str>, kind: Option<&str>) -> Vec<&Reference> {
        let mut seen = std::collections::BTreeSet::new();
        let mut result = Vec::new();

        // Local first
        for r in self.local.list(tag, kind) {
            seen.insert(r.name.clone());
            result.push(r);
        }

        // Then remotes
        for (_, store) in &self.remotes {
            for r in store.list(tag, kind) {
                if seen.insert(r.name.clone()) {
                    result.push(r);
                }
            }
        }

        result
    }

    // --- Local registry write operations ---

    pub fn add(&mut self, reference: Reference) -> Result<(), RefstoreError> {
        if self.local.get(&reference.name).is_some() {
            return Err(RefstoreError::ReferenceExists {
                name: reference.name,
            });
        }

        validate_name(&reference.name)?;

        let content_dir = self.local.content_path(&reference.name);
        self.fetch_content(&reference, &content_dir)?;

        let name = reference.name.clone();
        self.local.index_mut().references.insert(reference.name.clone(), reference);
        self.local.save_index()?;

        let content_rel = format!("content/{name}");
        git::commit(&self.root, &[&content_rel, "index.toml"], &format!("Add reference: {name}"))?;

        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<Reference, RefstoreError> {
        let reference = self
            .local
            .index_mut()
            .references
            .remove(name)
            .ok_or_else(|| RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            })?;

        let content_dir = self.local.content_path(name);
        if content_dir.exists() {
            let _ = fs::remove_dir_all(&content_dir);
        }

        self.local.save_index()?;

        let content_rel = format!("content/{name}");
        git::commit_removals(&self.root, &[&content_rel, "index.toml"], &format!("Remove reference: {name}"))?;

        Ok(reference)
    }

    pub fn update(&mut self, name: &str) -> Result<(), RefstoreError> {
        let reference = self
            .local
            .get(name)
            .ok_or_else(|| RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            })?
            .clone();

        let content_dir = self.local.content_path(name);
        if content_dir.exists() {
            let _ = fs::remove_dir_all(&content_dir);
        }

        self.fetch_content(&reference, &content_dir)?;

        if let Some(r) = self.local.index_mut().references.get_mut(name) {
            r.last_synced = Some(Utc::now());
            if let ReferenceSource::Git { .. } = &r.source {
                if let Ok(hash) = git::head_hash(&content_dir) {
                    r.checksum = Some(hash);
                }
            }
        }
        self.local.save_index()?;

        let content_rel = format!("content/{name}");
        git::commit_removals(&self.root, &[&content_rel, "index.toml"], &format!("Update reference: {name}"))?;

        Ok(())
    }

    // --- Bundle operations ---

    pub fn get_bundle(&self, name: &str) -> Option<&Bundle> {
        // Search local first, then remotes
        if let Some(b) = self.local.get_bundle(name) {
            return Some(b);
        }
        for (_, store) in &self.remotes {
            if let Some(b) = store.get_bundle(name) {
                return Some(b);
            }
        }
        None
    }

    pub fn list_bundles(&self, tag: Option<&str>) -> Vec<&Bundle> {
        let mut seen = std::collections::BTreeSet::new();
        let mut result = Vec::new();

        for b in self.local.list_bundles(tag) {
            seen.insert(b.name.clone());
            result.push(b);
        }
        for (_, store) in &self.remotes {
            for b in store.list_bundles(tag) {
                if seen.insert(b.name.clone()) {
                    result.push(b);
                }
            }
        }

        result
    }

    pub fn add_bundle(&mut self, bundle: Bundle) -> Result<(), RefstoreError> {
        if self.local.get_bundle(&bundle.name).is_some() {
            return Err(RefstoreError::BundleExists {
                name: bundle.name,
            });
        }
        validate_name(&bundle.name)?;

        for ref_name in &bundle.references {
            if self.get(ref_name).is_none() {
                return Err(RefstoreError::BundleInvalidReference {
                    bundle: bundle.name.clone(),
                    reference: ref_name.clone(),
                });
            }
        }

        let name = bundle.name.clone();
        self.local.index_mut().bundles.insert(bundle.name.clone(), bundle);
        self.local.save_index()?;

        git::commit(&self.root, &["index.toml"], &format!("Add bundle: {name}"))?;

        Ok(())
    }

    pub fn remove_bundle(&mut self, name: &str) -> Result<Bundle, RefstoreError> {
        let bundle = self
            .local
            .index_mut()
            .bundles
            .remove(name)
            .ok_or_else(|| RefstoreError::BundleNotFound {
                name: name.to_string(),
            })?;
        self.local.save_index()?;

        git::commit(&self.root, &["index.toml"], &format!("Remove bundle: {name}"))?;

        Ok(bundle)
    }

    pub fn update_bundle(
        &mut self,
        name: &str,
        add_refs: Vec<String>,
        remove_refs: Vec<String>,
        description: Option<String>,
    ) -> Result<(), RefstoreError> {
        for ref_name in &add_refs {
            if self.get(ref_name).is_none() {
                return Err(RefstoreError::BundleInvalidReference {
                    bundle: name.to_string(),
                    reference: ref_name.clone(),
                });
            }
        }

        let bundle = self
            .local
            .index_mut()
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

        self.local.save_index()?;

        git::commit(&self.root, &["index.toml"], &format!("Update bundle: {name}"))?;

        Ok(())
    }

    // --- Registry management ---

    /// Add a remote registry as a git submodule.
    pub fn add_registry(&mut self, name: &str, url: &str) -> Result<(), RefstoreError> {
        validate_name(name)?;

        if name == "local" {
            return Err(RefstoreError::InvalidName {
                name: name.to_string(),
                reason: "'local' is reserved for the local registry".to_string(),
            });
        }

        let submodule_path = format!("registries/{name}");
        let full_path = self.root.join("registries").join(name);

        if full_path.exists() {
            return Err(RefstoreError::RegistryExists {
                name: name.to_string(),
            });
        }

        // Create registries/ dir if needed
        fs::create_dir_all(self.root.join("registries")).map_err(|source| RefstoreError::DirCreate {
            path: self.root.join("registries"),
            source,
        })?;

        git::submodule_add(&self.root, url, &submodule_path)?;
        git::commit(&self.root, &[".gitmodules", &submodule_path], &format!("Add registry: {name}"))?;

        // Load the new registry
        let store = RegistryStore::open(&full_path)?;
        self.remotes.push((name.to_string(), store));

        // Track in config
        self.config.registries.push(Registry {
            name: name.to_string(),
            url: url.to_string(),
        });
        self.save_config()?;

        Ok(())
    }

    /// Remove a remote registry.
    pub fn remove_registry(&mut self, name: &str) -> Result<(), RefstoreError> {
        if !self.root.join("registries").join(name).exists() {
            return Err(RefstoreError::RegistryNotFound {
                name: name.to_string(),
            });
        }

        let submodule_path = format!("registries/{name}");
        git::submodule_remove(&self.root, &submodule_path)?;

        // git submodule deinit + git rm already stages changes,
        // so just commit directly (also stage .gitmodules which may have changed)
        git::commit(&self.root, &[".gitmodules"], &format!("Remove registry: {name}"))?;

        self.remotes.retain(|(n, _)| n != name);
        self.config.registries.retain(|r| r.name != name);
        self.save_config()?;

        Ok(())
    }

    /// Update remote registry/registries (git submodule update --remote).
    pub fn update_registry(&mut self, name: Option<&str>) -> Result<(), RefstoreError> {
        match name {
            Some(n) => {
                let submodule_path = format!("registries/{n}");
                git::submodule_update(&self.root, Some(&submodule_path))?;
                git::commit(&self.root, &[&submodule_path], &format!("Update registry: {n}"))?;

                // Reload the registry
                let full_path = self.root.join("registries").join(n);
                if let Some((_, store)) = self.remotes.iter_mut().find(|(rn, _)| rn == n) {
                    *store = RegistryStore::open(&full_path)?;
                }
            }
            None => {
                git::submodule_update(&self.root, None)?;
                git::commit(&self.root, &["registries"], "Update all registries")?;

                // Reload all remotes
                self.remotes = load_remote_registries(&self.root);
            }
        }
        Ok(())
    }

    /// List remote registries.
    pub fn list_registries(&self) -> Vec<(&str, &RegistryStore)> {
        self.remotes.iter().map(|(n, s)| (n.as_str(), s)).collect()
    }

    pub fn local_registry(&self) -> &RegistryStore {
        &self.local
    }

    // --- Versioning ---

    /// Get the version history for a reference (git log of content/<name>/).
    /// Returns entries from newest to oldest.
    pub fn versions(&self, name: &str) -> Result<Vec<git::LogEntry>, RefstoreError> {
        // Check the reference exists somewhere
        if self.get(name).is_none() {
            return Err(RefstoreError::ReferenceNotFound {
                name: name.to_string(),
            });
        }

        let content_rel = format!("content/{name}");
        git::log_path(&self.root, &content_rel)
    }

    /// Extract content for a reference at a specific git ref (tag or commit hash).
    /// Returns the path to a temporary directory containing the extracted content.
    /// The caller is responsible for using and cleaning up the returned path.
    pub fn content_at_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, RefstoreError> {
        // Verify the ref exists in the local registry
        if !git::ref_exists(&self.root, version) {
            return Err(RefstoreError::SyncFailed {
                name: name.to_string(),
                reason: format!("version '{version}' not found in registry (not a valid tag or commit)"),
            });
        }

        // Create a temp dir for extraction
        let temp_dir = self.root.join(".tmp-version-extract");
        if temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }

        let content_rel = format!("content/{name}");
        git::archive_path_at_ref(&self.root, version, &content_rel, &temp_dir)?;

        Ok(temp_dir)
    }

    /// List tags on the local registry.
    pub fn list_tags(&self) -> Result<Vec<String>, RefstoreError> {
        git::list_tags(&self.root)
    }

    /// Create a tag on the local registry.
    pub fn create_tag(&self, tag: &str, message: Option<&str>) -> Result<(), RefstoreError> {
        git::create_tag(&self.root, tag, message)
    }

    // --- Content fetching ---

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
                // Strip .git/ so we don't have nested git repos in the registry
                git::strip_git_dir(content_dir)?;
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
}

fn default_data_dir() -> Result<PathBuf, RefstoreError> {
    dirs::data_dir()
        .map(|d| d.join("refstore"))
        .ok_or(RefstoreError::DataDirNotFound)
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

/// Scan the registries/ directory for submodule registries.
fn load_remote_registries(root: &Path) -> Vec<(String, RegistryStore)> {
    let registries_dir = root.join("registries");
    if !registries_dir.exists() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let entries = match fs::read_dir(&registries_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            // Only load if it has an index.toml (i.e., is a valid registry)
            if path.join("index.toml").exists() {
                if let Ok(store) = RegistryStore::open(&path) {
                    result.push((name, store));
                }
            }
        }
    }

    // Sort by name for consistent resolution order
    result.sort_by(|(a, _), (b, _)| a.cmp(b));
    result
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
