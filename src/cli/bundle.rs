use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::cli::BundleSubcommand;
use crate::model::Bundle;
use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, cmd: BundleSubcommand) -> Result<()> {
    match cmd {
        BundleSubcommand::Create {
            name,
            refs,
            description,
            tag,
        } => run_create(data_dir, name, refs, description, tag),
        BundleSubcommand::List { tag } => run_list(data_dir, tag),
        BundleSubcommand::Info { name } => run_info(data_dir, name),
        BundleSubcommand::Update {
            name,
            add_refs,
            remove_refs,
            description,
        } => run_update(data_dir, name, add_refs, remove_refs, description),
        BundleSubcommand::Remove { name, force } => run_remove(data_dir, name, force),
    }
}

fn run_create(
    data_dir: Option<&PathBuf>,
    name: String,
    refs: Vec<String>,
    description: Option<String>,
    tags: Vec<String>,
) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let bundle = Bundle {
        name: name.clone(),
        description,
        tags,
        references: refs,
        created_at: Utc::now(),
    };

    repo.add_bundle(bundle)
        .context("failed to create bundle")?;

    println!("Created bundle '{name}'.");
    Ok(())
}

fn run_list(data_dir: Option<&PathBuf>, tag: Option<String>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let bundles = repo.list_bundles(tag.as_deref());

    if bundles.is_empty() {
        println!("No bundles in repository.");
        return Ok(());
    }

    for b in bundles {
        let tags = if b.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", b.tags.join(", "))
        };
        let desc = b
            .description
            .as_ref()
            .map(|d| format!(" - {d}"))
            .unwrap_or_default();
        println!(
            "  {} ({} refs){}{}",
            b.name,
            b.references.len(),
            desc,
            tags
        );
    }
    Ok(())
}

fn run_info(data_dir: Option<&PathBuf>, name: String) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let bundle = repo
        .get_bundle(&name)
        .ok_or_else(|| anyhow::anyhow!("bundle '{name}' not found"))?;

    println!("Name:        {}", bundle.name);
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
    Ok(())
}

fn run_update(
    data_dir: Option<&PathBuf>,
    name: String,
    add_refs: Vec<String>,
    remove_refs: Vec<String>,
    description: Option<String>,
) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    repo.update_bundle(&name, add_refs, remove_refs, description)
        .context("failed to update bundle")?;

    println!("Updated bundle '{name}'.");
    Ok(())
}

fn run_remove(data_dir: Option<&PathBuf>, name: String, force: bool) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    if !force {
        let bundle = repo
            .get_bundle(&name)
            .ok_or_else(|| anyhow::anyhow!("bundle '{name}' not found"))?;

        eprint!(
            "Remove bundle '{}' ({} refs) from central repository? [y/N] ",
            name,
            bundle.references.len()
        );
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    repo.remove_bundle(&name)
        .context("failed to remove bundle")?;

    println!("Removed bundle '{name}' from central repository.");
    Ok(())
}
