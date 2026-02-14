use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};

use crate::model::McpScope;
use crate::store::{ProjectStore, RepositoryStore};

// Parameter types for each tool

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct ListReferencesParams {
    #[schemars(description = "Optional tag to filter by")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct GetReferenceParams {
    #[schemars(description = "Name of the reference")]
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct ReadReferenceFileParams {
    #[schemars(description = "Name of the reference")]
    pub reference: String,
    #[schemars(description = "File path within the reference")]
    pub path: String,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct ListReferenceFilesParams {
    #[schemars(description = "Name of the reference")]
    pub reference: String,
    #[schemars(description = "Subdirectory path within the reference")]
    pub subpath: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct SearchReferencesParams {
    #[schemars(description = "Text to search for (case-insensitive substring match)")]
    pub query: String,
    #[schemars(description = "Limit search to a specific reference name")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct AddToProjectParams {
    #[schemars(description = "Name of the reference to add")]
    pub name: String,
}

// Server struct

pub struct RefstoreMcpServer {
    repo: RepositoryStore,
    scope: McpScope,
    _project: Option<ProjectStore>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl RefstoreMcpServer {
    pub fn new(repo: RepositoryStore, scope: McpScope, project: Option<ProjectStore>) -> Self {
        Self {
            repo,
            scope,
            _project: project,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all available references in the refstore repository")]
    async fn list_references(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ListReferencesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let refs = self.repo.list(params.tag.as_deref(), None);
        let output: Vec<_> = refs
            .iter()
            .map(|r| {
                let tags = if r.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", r.tags.join(", "))
                };
                let desc = r
                    .description
                    .as_ref()
                    .map(|d| format!(" - {d}"))
                    .unwrap_or_default();
                format!("{} ({}){}{}", r.name, r.kind, desc, tags)
            })
            .collect();

        let text = if output.is_empty() {
            "No references found.".to_string()
        } else {
            output.join("\n")
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get detailed information about a specific reference")]
    async fn get_reference(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<GetReferenceParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let reference = match self.repo.get(&params.name) {
            Some(r) => r,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Reference '{}' not found.",
                    params.name
                ))]));
            }
        };

        let info = format!(
            "Name: {}\nKind: {}\nSource: {}\nDescription: {}\nTags: {}",
            reference.name,
            reference.kind,
            reference.source,
            reference.description.as_deref().unwrap_or("(none)"),
            if reference.tags.is_empty() {
                "(none)".to_string()
            } else {
                reference.tags.join(", ")
            }
        );

        Ok(CallToolResult::success(vec![Content::text(info)]))
    }

    #[tool(description = "Read a file from a reference's cached content")]
    async fn read_reference_file(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ReadReferenceFileParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let content_dir = self.repo.content_path(&params.reference);
        if !content_dir.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Reference '{}' has no cached content.",
                params.reference
            ))]));
        }

        let file_path = content_dir.join(&params.path);
        if !file_path.starts_with(&content_dir) {
            return Ok(CallToolResult::error(vec![Content::text(
                "Path traversal not allowed.".to_string(),
            )]));
        }

        match std::fs::read_to_string(&file_path) {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to read file: {e}"
            ))])),
        }
    }

    #[tool(description = "List files in a reference's cached content directory")]
    async fn list_reference_files(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ListReferenceFilesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let content_dir = self.repo.content_path(&params.reference);
        let target = match &params.subpath {
            Some(p) => content_dir.join(p),
            None => content_dir.clone(),
        };

        if !target.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Path does not exist: {}",
                target.display()
            ))]));
        }

        if !target.starts_with(&content_dir) {
            return Ok(CallToolResult::error(vec![Content::text(
                "Path traversal not allowed.".to_string(),
            )]));
        }

        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&target) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let kind = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    "dir"
                } else {
                    "file"
                };
                files.push(format!("{name} ({kind})"));
            }
        }
        files.sort();

        let text = if files.is_empty() {
            "Directory is empty.".to_string()
        } else {
            files.join("\n")
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for text within reference files (case-insensitive)")]
    async fn search_references(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<SearchReferencesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let refs: Vec<_> = match &params.reference {
            Some(name) => match self.repo.get(name) {
                Some(r) => vec![r],
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Reference '{name}' not found."
                    ))]));
                }
            },
            None => self.repo.list(None, None),
        };

        let query_lower = params.query.to_lowercase();
        let mut results = Vec::new();

        for r in refs {
            let content_dir = self.repo.content_path(&r.name);
            if !content_dir.exists() {
                continue;
            }

            for entry in walkdir::WalkDir::new(&content_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (i, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&query_lower) {
                            let rel = entry
                                .path()
                                .strip_prefix(&content_dir)
                                .unwrap_or(entry.path());
                            results.push(format!(
                                "{}:{}:{}: {}",
                                r.name,
                                rel.display(),
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }

        let text = if results.is_empty() {
            format!("No matches found for '{}'.", params.query)
        } else {
            let count = results.len();
            let truncated = if count > 50 {
                results.truncate(50);
                format!("\n... and {} more results", count - 50)
            } else {
                String::new()
            };
            format!("{}{truncated}", results.join("\n"))
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Add a reference to the current project manifest (requires write permission)")]
    async fn add_to_project(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<AddToProjectParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        if self.scope != McpScope::ReadWrite {
            return Ok(CallToolResult::error(vec![Content::text(
                "MCP server is in read-only mode. Set mcp_scope to 'read_write' in config."
                    .to_string(),
            )]));
        }

        if self.repo.get(&params.name).is_none() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Reference '{}' not found in central repository.",
                params.name
            ))]));
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "TODO: add '{}' to project manifest. Run `refstore add {}` from CLI for now.",
            params.name, params.name
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for RefstoreMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Reference documentation manager for LLM coding agents. \
                 Use list_references to discover available references, \
                 get_reference for details, read_reference_file to read content, \
                 and search_references to find relevant documentation."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
