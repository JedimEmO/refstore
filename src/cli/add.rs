use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::model::ManifestEntry;
use crate::store::{ProjectStore, RepositoryStore};

pub fn run(
    data_dir: Option<&PathBuf>,
    name: String,
    version: Option<String>,
    path: Option<PathBuf>,
    include: Vec<String>,
    exclude: Vec<String>,
) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    if repo.get(&name).is_none() {
        anyhow::bail!(
            "reference '{name}' not found in central repository. Add it first with `refstore repo add`."
        );
    }

    let mut project = ProjectStore::open(None).context("failed to open project")?;

    let entry = ManifestEntry {
        path,
        version,
        include,
        exclude,
    };

    project
        .add_reference(name.clone(), entry)
        .context("failed to add reference to manifest")?;

    println!("Added '{name}' to project manifest.");
    println!("Run `refstore sync` to fetch the content.");
    Ok(())
}
