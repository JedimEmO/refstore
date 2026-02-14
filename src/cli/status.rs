use anyhow::{Context, Result};

use crate::store::ProjectStore;

pub fn run() -> Result<()> {
    let project = ProjectStore::open(None).context("failed to open project")?;

    let manifest = project.manifest();
    let refs_dir = project.references_dir();

    if manifest.references.is_empty() {
        println!("No references in manifest.");
        return Ok(());
    }

    println!("Project: {}", project.root().display());
    println!("References directory: {}", refs_dir.display());
    println!();

    for (name, entry) in &manifest.references {
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

        println!("  {name}{version_info}: {status}");
    }
    Ok(())
}
