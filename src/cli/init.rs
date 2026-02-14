use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::ProjectStore;

pub fn run(
    path: Option<PathBuf>,
    commit_references: bool,
    self_ref: bool,
    no_self_ref: bool,
) -> Result<()> {
    let gitignore = !commit_references;
    let store = ProjectStore::init(path.as_deref(), gitignore)
        .context("failed to initialize project")?;

    println!("Initialized refstore at {}", store.root().display());
    println!("  manifest: {}/refstore.toml", store.root().display());
    println!("  references dir: {}", store.references_dir().display());
    if gitignore {
        println!("  .references/ added to .gitignore");
    } else {
        println!("  .references/ will be committed to git");
    }

    if self_ref {
        super::self_ref::install(store.root())?;
    } else if !no_self_ref {
        super::self_ref::maybe_install(store.root())?;
    }

    Ok(())
}
