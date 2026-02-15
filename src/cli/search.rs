use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, query: String, reference: Option<String>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let refs: Vec<_> = match &reference {
        Some(name) => match repo.get(name) {
            Some(r) => vec![r],
            None => anyhow::bail!("reference '{name}' not found"),
        },
        None => repo.list(None, None),
    };

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for r in refs {
        let content_dir = match repo.resolve_content_path(&r.name) {
            Some(p) => p,
            None => repo.content_path(&r.name),
        };
        if !content_dir.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        let rel = entry
                            .path()
                            .strip_prefix(&content_dir)
                            .unwrap_or(entry.path());
                        results.push(format!(
                            "{}:{}:{}: {}",
                            r.name,
                            rel.display(),
                            i + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
    }

    if results.is_empty() {
        println!("No matches found for '{query}'.");
    } else {
        let count = results.len();
        for line in &results {
            println!("{line}");
        }
        if count > 50 {
            println!("... showing first 50 of {count} results");
        }
    }
    Ok(())
}
