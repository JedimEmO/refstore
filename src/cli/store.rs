use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::cli::StoreSubcommand;
use crate::model::{Reference, ReferenceKind, ReferenceSource};
use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, cmd: StoreSubcommand) -> Result<()> {
    match cmd {
        StoreSubcommand::Add {
            name,
            source,
            description,
            tag,
            git_ref,
            subpath,
        } => run_add(data_dir, name, source, description, tag, git_ref, subpath),
        StoreSubcommand::Remove { name, force } => run_remove(data_dir, name, force),
        StoreSubcommand::Update { name } => run_update(data_dir, name),
        StoreSubcommand::Tag { name, message } => run_tag(data_dir, name, message),
        StoreSubcommand::Tags => run_tags(data_dir),
        StoreSubcommand::Push { name, to } => run_push(data_dir, name, to),
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

fn run_remove(data_dir: Option<&PathBuf>, name: String, force: bool) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    if !force {
        let reference = repo
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("reference '{name}' not found"))?;

        eprint!(
            "Remove '{}' ({}) from central repository? [y/N] ",
            name, reference.source
        );
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    repo.remove(&name)
        .context("failed to remove reference from repository")?;

    println!("Removed '{name}' from central repository.");
    Ok(())
}

fn run_update(data_dir: Option<&PathBuf>, name: Option<String>) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let names: Vec<String> = match name {
        Some(n) => vec![n],
        None => repo.list(None, None).iter().map(|r| r.name.clone()).collect(),
    };

    if names.is_empty() {
        println!("No references in repository.");
        return Ok(());
    }

    let mut updated = 0;
    let mut failed = 0;

    for ref_name in &names {
        print!("  {ref_name}: updating... ");
        std::io::stdout().flush()?;

        match repo.update(ref_name) {
            Ok(()) => {
                println!("done");
                updated += 1;
            }
            Err(e) => {
                println!("FAILED - {e}");
                failed += 1;
            }
        }
    }

    println!("\nUpdate complete: {updated} updated, {failed} failed");
    Ok(())
}

fn run_tag(data_dir: Option<&PathBuf>, name: String, message: Option<String>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    repo.create_tag(&name, message.as_deref())
        .with_context(|| format!("failed to create tag '{name}'"))?;

    println!("Created tag '{name}' on the local registry.");
    println!("Use --pin {name} when adding references to pin to this version.");
    Ok(())
}

fn run_tags(data_dir: Option<&PathBuf>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let tags = repo.list_tags().context("failed to list tags")?;

    if tags.is_empty() {
        println!("No tags. Create one with `refstore store tag <name>`.");
        return Ok(());
    }

    println!("Registry tags:");
    for tag in &tags {
        println!("  {tag}");
    }
    Ok(())
}

fn run_push(data_dir: Option<&PathBuf>, name: String, to: PathBuf) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    repo.push_to(&name, &to)
        .with_context(|| format!("failed to push '{name}' to {}", to.display()))?;

    println!("Pushed '{name}' to {}", to.display());
    Ok(())
}

fn parse_source(
    source: &str,
    git_ref: Option<String>,
    subpath: Option<PathBuf>,
) -> Result<(ReferenceKind, ReferenceSource)> {
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

    let path = PathBuf::from(source);
    let path = if path.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&path)
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
