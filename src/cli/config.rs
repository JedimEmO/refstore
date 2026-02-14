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

fn run_set(data_dir: Option<&PathBuf>, key: String, value: String) -> Result<()> {
    let mut repo = RepositoryStore::open(data_dir.map(|p| p.as_path()))
        .context("failed to open central repository")?;

    let config = repo.config_mut();
    match key.as_str() {
        "mcp_scope" => {
            config.mcp_scope = match value.as_str() {
                "read_only" => crate::model::McpScope::ReadOnly,
                "read_write" => crate::model::McpScope::ReadWrite,
                _ => anyhow::bail!("invalid mcp_scope value: {value} (expected read_only or read_write)"),
            };
        }
        "git_depth" => {
            config.git_depth = value
                .parse::<u32>()
                .with_context(|| format!("invalid git_depth value: {value} (expected a number)"))?;
        }
        "default_branch" => {
            config.default_branch = if value == "" || value == "none" {
                None
            } else {
                Some(value.clone())
            };
        }
        _ => anyhow::bail!("unknown config key: {key}\nValid keys: mcp_scope, git_depth, default_branch"),
    }

    repo.save_config().context("failed to save config")?;
    println!("Set {key} = {value}");
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
