use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::model::ManifestEntry;
use crate::store::{ProjectStore, RepositoryStore};

pub fn run(
    data_dir: Option<&PathBuf>,
    name: String,
    is_bundle: bool,
    version: Option<String>,
    path: Option<PathBuf>,
    include: Vec<String>,
    exclude: Vec<String>,
    sync: bool,
) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let mut project = ProjectStore::open(None).context("failed to open project")?;

    if is_bundle {
        if repo.get_bundle(&name).is_none() {
            anyhow::bail!(
                "bundle '{name}' not found in central repository. \
                Create it first with `refstore bundle create`."
            );
        }
        project
            .add_bundle(name.clone())
            .context("failed to add bundle to manifest")?;
        println!("Added bundle '{name}' to project manifest.");
    } else {
        if repo.get(&name).is_none() {
            anyhow::bail!(
                "reference '{name}' not found in central repository. Add it first with `refstore store add`."
            );
        }

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
    }

    if sync {
        println!("Syncing...");
        drop(project);
        crate::cli::sync::run(data_dir, Some(name), false)?;
    } else {
        println!("Run `refstore sync` to fetch the content.");
    }
    Ok(())
}
