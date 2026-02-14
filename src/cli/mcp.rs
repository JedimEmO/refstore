use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::{ProjectStore, RepositoryStore};

pub async fn run(data_dir: Option<PathBuf>) -> Result<()> {
    let repo = RepositoryStore::open(data_dir.as_deref())
        .context("failed to open central repository")?;
    let scope = repo.config().mcp_scope.clone();
    let project = ProjectStore::open(None).ok();

    crate::mcp::serve(repo, scope, project).await
}
