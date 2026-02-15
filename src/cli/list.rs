use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, tag: Option<String>, kind: Option<String>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let refs = repo.list(tag.as_deref(), kind.as_deref());

    if refs.is_empty() {
        println!("No references in repository.");
        return Ok(());
    }

    for r in refs {
        let tags = if r.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", r.tags.join(", "))
        };
        let desc = r
            .description
            .as_ref()
            .map(|d| format!(" - {d}"))
            .unwrap_or_default();

        println!("  {} ({}){}{}", r.name, r.kind, desc, tags);
    }
    Ok(())
}
