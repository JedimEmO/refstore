use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::{ProjectStore, RepositoryStore};

pub fn run(data_dir: Option<&PathBuf>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;
    let project = ProjectStore::open(None).context("failed to open project")?;

    let manifest = project.manifest();
    let refs_dir = project.references_dir();

    println!("Project: {}", project.root().display());
    println!("References directory: {}", refs_dir.display());
    println!();

    // Show bundles
    if !manifest.bundles.is_empty() {
        println!("Bundles:");
        for bundle_name in &manifest.bundles {
            match repo.get_bundle(bundle_name) {
                Some(b) => {
                    println!("  @{}: {} references", bundle_name, b.references.len());
                }
                None => {
                    println!("  @{}: (not found in repository!)", bundle_name);
                }
            }
        }
        println!();
    }

    // Resolve all references (explicit + bundle-expanded)
    let resolved = project
        .resolve_all_references(&repo)
        .context("failed to resolve references")?;

    if resolved.is_empty() {
        println!("No references in manifest.");
        return Ok(());
    }

    // Build reverse lookup: ref_name -> bundle_name
    let mut ref_to_bundle: BTreeMap<String, String> = BTreeMap::new();
    for bundle_name in &manifest.bundles {
        if let Some(b) = repo.get_bundle(bundle_name) {
            for ref_name in &b.references {
                ref_to_bundle
                    .entry(ref_name.clone())
                    .or_insert_with(|| bundle_name.clone());
            }
        }
    }

    println!("References:");
    for (name, entry) in &resolved {
        let target_dir = match &entry.path {
            Some(p) => refs_dir.join(p),
            None => refs_dir.join(name),
        };

        let status = if target_dir.exists() {
            if crate::git::is_git_repo(&target_dir) {
                match crate::git::head_hash(&target_dir) {
                    Ok(hash) => format!("synced ({})", &hash[..8.min(hash.len())]),
                    Err(_) => "synced".to_string(),
                }
            } else {
                "synced".to_string()
            }
        } else {
            "not synced".to_string()
        };

        let version_info = entry
            .version
            .as_ref()
            .map(|v| format!(" @ {v}"))
            .unwrap_or_default();

        let source = if manifest.references.contains_key(name) {
            String::new()
        } else if let Some(bundle_name) = ref_to_bundle.get(name) {
            format!(" (via bundle: {bundle_name})")
        } else {
            String::new()
        };

        println!("  {name}{version_info}{source}: {status}");
    }
    Ok(())
}
