use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::{ProjectStore, RepositoryStore};

pub fn run(data_dir: Option<&PathBuf>, name: Option<String>, force: bool) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;
    let project = ProjectStore::open(None).context("failed to open project")?;

    let refs_dir = project.references_dir();
    std::fs::create_dir_all(&refs_dir)
        .with_context(|| format!("failed to create {}", refs_dir.display()))?;

    let entries: Vec<_> = match &name {
        Some(n) => {
            let entry = project
                .manifest()
                .references
                .get(n)
                .ok_or_else(|| anyhow::anyhow!("reference '{n}' not in project manifest"))?;
            vec![(n.as_str(), entry)]
        }
        None => project
            .manifest()
            .references
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect(),
    };

    if entries.is_empty() {
        println!("No references in manifest. Add some with `refstore add`.");
        return Ok(());
    }

    let mut synced = 0;
    let mut failed = 0;

    for (ref_name, entry) in &entries {
        let reference = match repo.get(ref_name) {
            Some(r) => r,
            None => {
                eprintln!(
                    "warning: '{ref_name}' not found in central repository, skipping"
                );
                failed += 1;
                continue;
            }
        };

        let target_dir = match &entry.path {
            Some(p) => refs_dir.join(p),
            None => refs_dir.join(ref_name),
        };

        let source_dir = repo.content_path(ref_name);
        if !source_dir.exists() {
            eprintln!("warning: no cached content for '{ref_name}', skipping");
            failed += 1;
            continue;
        }

        if target_dir.exists() && !force {
            // Check if content is a git repo and compare hashes
            if crate::git::is_git_repo(&source_dir) && crate::git::is_git_repo(&target_dir) {
                let source_hash = crate::git::head_hash(&source_dir).unwrap_or_default();
                let target_hash = crate::git::head_hash(&target_dir).unwrap_or_default();
                if source_hash == target_hash && !source_hash.is_empty() {
                    println!("  {ref_name}: up to date ({:.8})", source_hash);
                    synced += 1;
                    continue;
                }
            }

            // Remove and re-copy
            let _ = std::fs::remove_dir_all(&target_dir);
        }

        match copy_reference(&source_dir, &target_dir, reference) {
            Ok(()) => {
                println!("  {ref_name}: synced to {}", target_dir.display());
                synced += 1;
            }
            Err(e) => {
                eprintln!("  {ref_name}: FAILED - {e}");
                failed += 1;
            }
        }
    }

    println!("\nSync complete: {synced} synced, {failed} failed");
    Ok(())
}

fn copy_reference(
    source: &std::path::Path,
    target: &std::path::Path,
    _reference: &crate::model::Reference,
) -> Result<()> {
    if source.is_file() {
        std::fs::create_dir_all(target.parent().unwrap_or(target))?;
        std::fs::copy(source, target)?;
    } else {
        copy_dir_recursive(source, target)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src)?;
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
