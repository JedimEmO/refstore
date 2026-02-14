use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::cli::RepoSubcommand;
use crate::model::{Reference, ReferenceKind, ReferenceSource};
use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, cmd: RepoSubcommand) -> Result<()> {
    match cmd {
        RepoSubcommand::Add {
            name,
            source,
            description,
            tag,
            git_ref,
            subpath,
        } => run_add(data_dir, name, source, description, tag, git_ref, subpath),
        RepoSubcommand::List { tag, kind } => run_list(data_dir, tag, kind),
        RepoSubcommand::Remove { name, force } => run_remove(data_dir, name, force),
        RepoSubcommand::Info { name } => run_info(data_dir, name),
    }
}

fn run_add(
    data_dir: Option<&PathBuf>,
    name: String,
    source: String,
    description: Option<String>,
    tags: Vec<String>,
    git_ref: Option<String>,
    subpath: Option<PathBuf>,
) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let (kind, ref_source) = parse_source(&source, git_ref, subpath)?;

    let reference = Reference {
        name: name.clone(),
        kind,
        source: ref_source,
        description,
        tags,
        added_at: Utc::now(),
        last_synced: Some(Utc::now()),
        checksum: None,
    };

    repo.add(reference)
        .context("failed to add reference to repository")?;

    println!("Added '{name}' to central repository.");
    println!("Content cached at: {}", repo.content_path(&name).display());
    Ok(())
}

fn run_list(data_dir: Option<&PathBuf>, tag: Option<String>, kind: Option<String>) -> Result<()> {
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

fn run_remove(data_dir: Option<&PathBuf>, name: String, _force: bool) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    repo.remove(&name)
        .context("failed to remove reference from repository")?;

    println!("Removed '{name}' from central repository.");
    Ok(())
}

fn run_info(data_dir: Option<&PathBuf>, name: String) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let reference = repo
        .get(&name)
        .ok_or_else(|| anyhow::anyhow!("reference '{name}' not found"))?;

    println!("Name:        {}", reference.name);
    println!("Kind:        {}", reference.kind);
    println!("Source:      {}", reference.source);
    if let Some(desc) = &reference.description {
        println!("Description: {desc}");
    }
    if !reference.tags.is_empty() {
        println!("Tags:        {}", reference.tags.join(", "));
    }
    println!("Added:       {}", reference.added_at.format("%Y-%m-%d %H:%M:%S UTC"));
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
    Ok(())
}

fn parse_source(
    source: &str,
    git_ref: Option<String>,
    subpath: Option<PathBuf>,
) -> Result<(ReferenceKind, ReferenceSource)> {
    // Check if it looks like a git URL
    if source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("git@")
        || source.starts_with("ssh://")
        || source.ends_with(".git")
    {
        return Ok((
            ReferenceKind::GitRepo,
            ReferenceSource::Git {
                url: source.to_string(),
                r#ref: git_ref,
                subpath,
            },
        ));
    }

    // Otherwise treat as local path
    let path = PathBuf::from(source);
    let path = if path.is_relative() {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&path)
    } else {
        path
    };

    let kind = if path.is_file() {
        ReferenceKind::File
    } else if path.is_dir() {
        ReferenceKind::Directory
    } else {
        anyhow::bail!("source path does not exist: {}", path.display());
    };

    Ok((kind, ReferenceSource::Local { path }))
}
