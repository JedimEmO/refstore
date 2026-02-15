use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, name: String) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    // Try as a reference first
    if let Some(reference) = repo.get(&name) {
        println!("Name:        {}", reference.name);
        println!("Kind:        {}", reference.kind);
        println!("Source:      {}", reference.source);
        if let Some(desc) = &reference.description {
            println!("Description: {desc}");
        }
        if !reference.tags.is_empty() {
            println!("Tags:        {}", reference.tags.join(", "));
        }
        println!(
            "Added:       {}",
            reference.added_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        if let Some(synced) = &reference.last_synced {
            println!("Last synced: {}", synced.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        if let Some(checksum) = &reference.checksum {
            println!("Checksum:    {checksum}");
        }

        let content_path = repo.content_path(&name);
        if content_path.exists() {
            println!("Content:     {}", content_path.display());
        } else {
            println!("Content:     (not cached)");
        }
        return Ok(());
    }

    // Try as a bundle
    if let Some(bundle) = repo.get_bundle(&name) {
        println!("Name:        {}", bundle.name);
        println!("Type:        bundle");
        if let Some(desc) = &bundle.description {
            println!("Description: {desc}");
        }
        if !bundle.tags.is_empty() {
            println!("Tags:        {}", bundle.tags.join(", "));
        }
        println!(
            "Created:     {}",
            bundle.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("References:");
        for ref_name in &bundle.references {
            println!("  - {ref_name}");
        }
        return Ok(());
    }

    anyhow::bail!("'{name}' not found (not a reference or bundle)")
}
