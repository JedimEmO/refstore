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

## Reading Reference Content

Synced references live in `.references/` in the project root. Each subdirectory
corresponds to a reference listed in `refstore.toml`.

**Use your filesystem tools (Read, Grep, Glob) to access reference content directly.**
Do NOT use MCP tools to read reference content — the MCP tools are for discovery
and management only.

## Discovering and Adding References

### 1. Discover available references
Use `list_references` to see all references in the repository.
Use `list_references` with a `tag` to filter (e.g. tag=\"rust\").

### 2. Get reference details
Use `get_reference` with the name to see its description, kind, and tags.

### 3. Add a reference to the current project
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
- **Synced content**: .references/<name>/ — read these with your filesystem tools
- **Registries**: local registry is searched first, then remote registries alphabetically

## Tips
- Read `.references/` directly with your filesystem tools (Read, Grep, Glob)
- Use `list_references` to discover what's available to add
- After `add_to_project`, remind the user to run `refstore sync`
";
        Ok(CallToolResult::success(vec![Content::text(tutorial)]))
    }

    #[tool(description = "List all available references in the refstore repository")]
    async fn list_references(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ListReferencesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let refs = self.repo.list(params.tag.as_deref(), None);
        let has_remotes = self.repo.has_remotes();
        let output: Vec<_> = refs
            .iter()
            .map(|resolved| {
                let r = resolved.reference;
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
                let registry = if has_remotes && resolved.registry_name != "local" {
                    format!("{}: ", resolved.registry_name)
                } else {
                    String::new()
                };
                format!("{}{} ({}){}{}", registry, r.name, r.kind, desc, tags)
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
        let resolved = match self.repo.resolve(&params.name) {
            Some(r) => r,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Reference '{}' not found.",
                    params.name
                ))]));
            }
        };
        let reference = resolved.reference;

        let info = format!(
            "Name: {}\nKind: {}\nSource: {}\nDescription: {}\nTags: {}\nRegistry: {}",
            reference.name,
            reference.kind,
            reference.source,
            reference.description.as_deref().unwrap_or("(none)"),
            if reference.tags.is_empty() {
                "(none)".to_string()
            } else {
                reference.tags.join(", ")
            },
            resolved.registry_name
        );

        Ok(CallToolResult::success(vec![Content::text(info)]))
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
                 Synced references live in .references/ — use your filesystem tools \
                 (Read, Grep, Glob) to access them directly. \
                 Use list_references to discover available references, \
                 get_reference for details, and add_to_project to add new references."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
