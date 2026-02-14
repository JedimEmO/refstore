use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, name: String) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let entries = repo
        .versions(&name)
        .with_context(|| format!("failed to get version history for '{name}'"))?;

    if entries.is_empty() {
        println!("No version history for '{name}'.");
        return Ok(());
    }

    println!("Version history for '{name}':");
    println!();
    for entry in &entries {
        println!("  {} {} {}", entry.hash, entry.date, entry.message);
    }
    println!();
    println!("Tip: use --pin <hash> when adding to pin to a specific version.");

    // Show available tags
    if let Ok(tags) = repo.list_tags() {
        if !tags.is_empty() {
            println!("Registry tags: {}", tags.join(", "));
        }
    }

    Ok(())
}
