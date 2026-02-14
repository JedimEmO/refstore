use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::{ProjectStore, RepositoryStore};

pub fn run(data_dir: Option<&PathBuf>, name: String, is_bundle: bool, purge: bool) -> Result<()> {
    let mut project = ProjectStore::open(None).context("failed to open project")?;

    if is_bundle {
        if purge {
            // Resolve bundle refs to purge their synced content,
            // but only refs not explicitly listed in the manifest.
            let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
                .context("failed to open central repository")?;
            if let Some(bundle) = repo.get_bundle(&name) {
                let refs_dir = project.references_dir();
                for ref_name in &bundle.references {
                    if project.manifest().references.contains_key(ref_name) {
                        continue; // skip refs explicitly in manifest
                    }
                    let ref_dir = refs_dir.join(ref_name);
                    if ref_dir.exists() {
                        fs::remove_dir_all(&ref_dir).with_context(|| {
                            format!("failed to purge {}", ref_dir.display())
                        })?;
                        println!("Purged content from {}", ref_dir.display());
                    }
                }
            }
        }

        project
            .remove_bundle(&name)
            .context("failed to remove bundle from manifest")?;
        println!("Removed bundle '{name}' from project manifest.");
    } else {
        project
            .remove_reference(&name)
            .context("failed to remove reference from manifest")?;
        println!("Removed '{name}' from project manifest.");

        if purge {
            let ref_dir = project.references_dir().join(&name);
            if ref_dir.exists() {
                fs::remove_dir_all(&ref_dir)
                    .with_context(|| format!("failed to purge {}", ref_dir.display()))?;
                println!("Purged content from {}", ref_dir.display());
            }
        }
    }
    Ok(())
}
