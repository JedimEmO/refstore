use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::ConfigSubcommand;
use crate::store::RepositoryStore;

pub fn run(data_dir: Option<&PathBuf>, cmd: ConfigSubcommand) -> Result<()> {
    match cmd {
        ConfigSubcommand::Show => run_show(data_dir),
        ConfigSubcommand::Set { key, value } => run_set(data_dir, key, value),
        ConfigSubcommand::Get { key } => run_get(data_dir, key),
    }
}

fn run_show(data_dir: Option<&PathBuf>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let config = repo.config();
    println!("Data directory: {}", repo.root().display());
    println!("MCP scope:      {}", config.mcp_scope);
    println!("Git depth:      {}", config.git_depth);
    if let Some(branch) = &config.default_branch {
        println!("Default branch: {branch}");
    }
    Ok(())
}

fn run_set(_data_dir: Option<&PathBuf>, key: String, value: String) -> Result<()> {
    // TODO: implement config set
    println!("TODO: set {key} = {value}");
    Ok(())
}

fn run_get(data_dir: Option<&PathBuf>, key: String) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let config = repo.config();
    match key.as_str() {
        "mcp_scope" => println!("{}", config.mcp_scope),
        "git_depth" => println!("{}", config.git_depth),
        "default_branch" => {
            println!("{}", config.default_branch.as_deref().unwrap_or("(not set)"))
        }
        _ => anyhow::bail!("unknown config key: {key}"),
    }
    Ok(())
}
