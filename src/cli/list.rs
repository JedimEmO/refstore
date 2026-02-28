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

    let has_remotes = repo.has_remotes();

    for resolved in refs {
        let r = resolved.reference;
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
        let registry = if has_remotes && resolved.registry_name != "local" {
            format!("{}: ", resolved.registry_name)
        } else {
            String::new()
        };

        println!("  {}{} ({}){}{}", registry, r.name, r.kind, desc, tags);
    }
    Ok(())
}
