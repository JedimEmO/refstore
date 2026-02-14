use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSetBuilder};

use crate::model::ManifestEntry;
use crate::store::{ProjectStore, RepositoryStore};

pub fn run(data_dir: Option<&PathBuf>, name: Option<String>, force: bool) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;
    let project = ProjectStore::open(None).context("failed to open project")?;

    let refs_dir = project.references_dir();
    std::fs::create_dir_all(&refs_dir)
        .with_context(|| format!("failed to create {}", refs_dir.display()))?;

    // Resolve all references (explicit + bundle-expanded)
    let resolved = project
        .resolve_all_references(&repo)
        .context("failed to resolve references")?;

    let entries: Vec<_> = match &name {
        Some(n) => {
            let entry = resolved
                .get(n)
                .ok_or_else(|| anyhow::anyhow!("reference '{n}' not found in project manifest (including bundle-expanded references)"))?;
            vec![(n.as_str(), entry)]
        }
        None => resolved.iter().map(|(k, v)| (k.as_str(), v)).collect(),
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
                eprintln!("warning: '{ref_name}' not found in central repository, skipping");
                failed += 1;
                continue;
            }
        };
        let _ = reference; // used for future metadata; currently we only need the content path

        let target_dir = match &entry.path {
            Some(p) => refs_dir.join(p),
            None => refs_dir.join(ref_name),
        };

        // If version is pinned, extract content from that specific git ref
        let versioned_source = if let Some(version) = &entry.version {
            match repo.content_at_version(ref_name, version) {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!("  {ref_name}: FAILED - {e}");
                    failed += 1;
                    continue;
                }
            }
        } else {
            None
        };

        let source_dir = if let Some(ref versioned) = versioned_source {
            versioned.clone()
        } else {
            match repo.resolve_content_path(ref_name) {
                Some(p) if p.exists() => p,
                _ => {
                    eprintln!("warning: no cached content for '{ref_name}', skipping");
                    failed += 1;
                    continue;
                }
            }
        };

        if target_dir.exists() && !force && versioned_source.is_none() {
            if crate::git::is_git_repo(&source_dir) && crate::git::is_git_repo(&target_dir) {
                let source_hash = crate::git::head_hash(&source_dir).unwrap_or_default();
                let target_hash = crate::git::head_hash(&target_dir).unwrap_or_default();
                if source_hash == target_hash && !source_hash.is_empty() {
                    println!("  {ref_name}: up to date ({:.8})", source_hash);
                    synced += 1;
                    continue;
                }
            }

            let _ = std::fs::remove_dir_all(&target_dir);
        } else if target_dir.exists() {
            let _ = std::fs::remove_dir_all(&target_dir);
        }

        match copy_reference(&source_dir, &target_dir, entry) {
            Ok(count) => {
                let mut suffix_parts = Vec::new();
                if !entry.include.is_empty() || !entry.exclude.is_empty() {
                    suffix_parts.push(format!("{count} files, filtered"));
                }
                if let Some(version) = &entry.version {
                    suffix_parts.push(format!("version: {version}"));
                }
                let suffix = if suffix_parts.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", suffix_parts.join(", "))
                };
                println!("  {ref_name}: synced{suffix}");
                synced += 1;
            }
            Err(e) => {
                eprintln!("  {ref_name}: FAILED - {e}");
                failed += 1;
            }
        }

        // Clean up versioned temp dir if used
        if let Some(versioned) = versioned_source {
            let _ = std::fs::remove_dir_all(&versioned);
        }
    }

    println!("\nSync complete: {synced} synced, {failed} failed");
    Ok(())
}

fn copy_reference(source: &Path, target: &Path, entry: &ManifestEntry) -> Result<usize> {
    if source.is_file() {
        std::fs::create_dir_all(target.parent().unwrap_or(target))?;
        std::fs::copy(source, target)?;
        return Ok(1);
    }

    let include_set = if entry.include.is_empty() {
        None
    } else {
        let mut builder = GlobSetBuilder::new();
        for pattern in &entry.include {
            builder.add(Glob::new(pattern).with_context(|| format!("invalid include glob: {pattern}"))?);
        }
        Some(builder.build().context("failed to build include globset")?)
    };

    let exclude_set = if entry.exclude.is_empty() {
        None
    } else {
        let mut builder = GlobSetBuilder::new();
        for pattern in &entry.exclude {
            builder.add(Glob::new(pattern).with_context(|| format!("invalid exclude glob: {pattern}"))?);
        }
        Some(builder.build().context("failed to build exclude globset")?)
    };

    let has_filters = include_set.is_some() || exclude_set.is_some();
    let mut count = 0;

    std::fs::create_dir_all(target)?;
    for entry in walkdir::WalkDir::new(source).min_depth(1) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let relative_str = relative.to_string_lossy();
        let dest = target.join(relative);

        if entry.file_type().is_dir() {
            // Only create dirs eagerly if no filters; otherwise let file copies create parents
            if !has_filters {
                std::fs::create_dir_all(&dest)?;
            }
            continue;
        }

        // Apply include filter (if any includes, file must match at least one)
        if let Some(ref inc) = include_set {
            if !inc.is_match(relative_str.as_ref()) {
                continue;
            }
        }

        // Apply exclude filter
        if let Some(ref exc) = exclude_set {
            if exc.is_match(relative_str.as_ref()) {
                continue;
            }
        }

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(entry.path(), &dest)?;
        count += 1;
    }
    Ok(count)
}
