use std::path::PathBuf;

use anyhow::{Context, Result};

use super::RegistrySubcommand;
use crate::store::{RegistryStore, RepositoryStore};

pub fn run(data_dir: Option<&PathBuf>, cmd: RegistrySubcommand) -> Result<()> {
    match cmd {
        RegistrySubcommand::Add { name, url } => {
            let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
                .context("failed to open central repository")?;
            repo.add_registry(&name, &url)?;
            println!("Added registry '{name}' from {url}");
            Ok(())
        }
        RegistrySubcommand::Remove { name, force: _ } => {
            let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
                .context("failed to open central repository")?;
            repo.remove_registry(&name)?;
            println!("Removed registry '{name}'");
            Ok(())
        }
        RegistrySubcommand::List => {
            let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
                .context("failed to open central repository")?;

            let local = repo.local_registry();
            let local_refs = local.list(None, None);
            let local_bundles = local.list_bundles(None);
            println!("local: {} references, {} bundles", local_refs.len(), local_bundles.len());

            let remotes = repo.list_registries();
            if remotes.is_empty() {
                println!("\nNo remote registries configured.");
                println!("Add one with: refstore registry add <name> <git-url>");
            } else {
                for (name, store) in &remotes {
                    let refs = store.list(None, None);
                    let bundles = store.list_bundles(None);
                    println!("{name}: {} references, {} bundles", refs.len(), bundles.len());
                }
            }
            Ok(())
        }
        RegistrySubcommand::Update { name } => {
            let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
                .context("failed to open central repository")?;
            match &name {
                Some(n) => {
                    repo.update_registry(Some(n))?;
                    println!("Updated registry '{n}'");
                }
                None => {
                    repo.update_registry(None)?;
                    println!("Updated all registries");
                }
            }
            Ok(())
        }
        RegistrySubcommand::Init { path } => {
            RegistryStore::init_new(&path)
                .with_context(|| format!("failed to initialize registry at {}", path.display()))?;
            println!("Initialized registry at {}", path.display());
            println!("Add references with: refstore --data-dir {} store add <name> <source>", path.display());
            Ok(())
        }
    }
}
