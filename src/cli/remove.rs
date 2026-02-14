use std::fs;

use anyhow::{Context, Result};

use crate::store::ProjectStore;

pub fn run(name: String, purge: bool) -> Result<()> {
    let mut project = ProjectStore::open(None).context("failed to open project")?;

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
    Ok(())
}
