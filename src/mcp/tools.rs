use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};

use tokio::sync::Mutex;

use crate::model::{ManifestEntry, McpScope};
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

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct ListBundlesParams {
    #[schemars(description = "Optional tag to filter by")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct GetBundleParams {
    #[schemars(description = "Name of the bundle")]
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub struct GetTutorialParams {}

// Server struct

pub struct RefstoreMcpServer {
    repo: RepositoryStore,
    scope: McpScope,
    project: Mutex<Option<ProjectStore>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl RefstoreMcpServer {
    pub fn new(
        repo: RepositoryStore,
        scope: McpScope,
        project: Mutex<Option<ProjectStore>>,
    ) -> Self {
        Self {
            repo,
            scope,
            project,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get a tutorial explaining common refstore usage patterns and workflows")]
    async fn get_tutorial(
        &self,
        rmcp::handler::server::wrapper::Parameters(_params): rmcp::handler::server::wrapper::Parameters<GetTutorialParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let tutorial = "\
# refstore Tutorial — Common Usage Patterns

## What is refstore?
refstore manages reference documentation for LLM coding agents. References are
curated docs (API guides, style guides, framework docs) stored in a central
repository and synced into projects via `.references/` directories.

## Quick Start Workflow

### 1. Discover available references
Use `list_references` to see all references in the repository.
Use `list_references` with a `tag` to filter (e.g. tag=\"rust\").

### 2. Explore a reference
Use `get_reference` with the name to see its description, kind, and tags.
Use `list_reference_files` to browse its file tree.
Use `read_reference_file` to read specific files.

### 3. Search across references
Use `search_references` with a query string to find content across all
references. Optionally pass `reference` to limit to one reference.

### 4. Add a reference to the current project
Use `add_to_project` with the reference name. This updates the project's
`refstore.toml` manifest. The user then runs `refstore sync` in their
terminal to download the content into `.references/`.

Note: `add_to_project` requires write permissions. If it fails with a
scope error, the MCP server was started in read-only mode.

## Bundles
Bundles are named groups of references (e.g. a tech stack).
Use `list_bundles` to see available bundles.
Use `get_bundle` to see which references a bundle contains.
Bundles cannot be added via MCP — the user adds them with:
  `refstore add --bundle <name>` then `refstore sync`

## Key Concepts
- **Central repository**: ~/.local/share/refstore/ — stores all reference content
- **Project manifest**: refstore.toml in the project root — lists which refs to sync
- **Synced content**: .references/<name>/ — read these files for project context
- **Registries**: local registry is searched first, then remote registries alphabetically

## Useful CLI Commands
- `refstore list` — list all available references
- `refstore search <query>` — search content across references
- `refstore info <name>` — show details about a reference or bundle
- `refstore add <name> --sync` — add to project and sync immediately
- `refstore store add <name> <source>` — add content to the local store
- `refstore bundle create <name> --ref ...` — create a bundle
- `refstore registry init <path>` — create a new shareable registry

## Tips
- Always `list_references` first to discover what's available
- Read files in `.references/` for already-synced project context
- After `add_to_project`, remind the user to run `refstore sync`
- Use `search_references` to find specific APIs or patterns across all docs
";
        Ok(CallToolResult::success(vec![Content::text(tutorial)]))
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
        let content_dir = match self.repo.resolve_content_path(&params.reference) {
            Some(p) if p.exists() => p,
            _ => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Reference '{}' has no cached content.",
                    params.reference
                ))]));
            }
        };

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
        let content_dir = self.repo.resolve_content_path(&params.reference)
            .unwrap_or_else(|| self.repo.content_path(&params.reference));
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
            let content_dir = match self.repo.resolve_content_path(&r.name) {
                Some(p) => p,
                None => self.repo.content_path(&r.name),
            };
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

    #[tool(description = "List all bundles (named groups of references)")]
    async fn list_bundles(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ListBundlesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let bundles = self.repo.list_bundles(params.tag.as_deref());
        let output: Vec<_> = bundles
            .iter()
            .map(|b| {
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
                format!("{} ({} refs){}{}", b.name, b.references.len(), desc, tags)
            })
            .collect();

        let text = if output.is_empty() {
            "No bundles found.".to_string()
        } else {
            output.join("\n")
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get detailed information about a bundle (named group of references)")]
    async fn get_bundle(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<GetBundleParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let bundle = match self.repo.get_bundle(&params.name) {
            Some(b) => b,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Bundle '{}' not found.",
                    params.name
                ))]));
            }
        };

        let refs_list = bundle.references.join(", ");
        let info = format!(
            "Name: {}\nDescription: {}\nTags: {}\nReferences: {}",
            bundle.name,
            bundle.description.as_deref().unwrap_or("(none)"),
            if bundle.tags.is_empty() {
                "(none)".to_string()
            } else {
                bundle.tags.join(", ")
            },
            refs_list
        );

        Ok(CallToolResult::success(vec![Content::text(info)]))
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

        let mut project_guard = self.project.lock().await;
        let project = match project_guard.as_mut() {
            Some(p) => p,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "No project manifest found. Run `refstore init` first.".to_string(),
                )]));
            }
        };

        let entry = ManifestEntry::default();
        match project.add_reference(params.name.clone(), entry) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Added '{}' to project manifest. Run `refstore sync` to fetch content.",
                params.name
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to add '{}': {e}",
                params.name
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for RefstoreMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Reference documentation manager for LLM coding agents. \
                 Call get_tutorial for usage patterns and workflows. \
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
