pub mod tools;

use anyhow::Result;
use tokio::sync::Mutex;

use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::model::McpScope;
use crate::store::{ProjectStore, RepositoryStore};

use tools::RefstoreMcpServer;

pub async fn serve(
    repo: RepositoryStore,
    scope: McpScope,
    project: Option<ProjectStore>,
) -> Result<()> {
    let server = RefstoreMcpServer::new(repo, scope, Mutex::new(project));
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
