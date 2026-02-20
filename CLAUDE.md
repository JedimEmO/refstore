# refstore — Development Guide

CLI tool + MCP server for managing reference documentation for LLM coding agents. Written in Rust (edition 2024, MSRV 1.85).

## Build & Test

```bash
cargo build                     # compile
cargo test                      # 88 integration tests (no unit tests)
cargo test cli_registry         # run a specific test module
cargo run -- --help             # run locally
```

Tests use `assert_cmd` + `tempfile`. Each test creates a `TestEnv` with isolated `data_dir` and `project_dir` temp directories. See `tests/integration/common.rs` for helpers like `add_repo_ref()`, `create_bundle()`, `init_project()`, `create_fake_registry()`.

## Architecture

```
src/
  main.rs              # CLI entry point — parses args, dispatches to cli::* handlers
  cli/
    mod.rs             # clap derive: Cli, Command, StoreSubcommand, BundleSubcommand, etc.
    store.rs           # `refstore store add/remove/update/tag/tags/push`
    bundle.rs          # `refstore bundle create/list/info/update/remove`
    registry.rs        # `refstore registry list/add/remove/update/init`
    add.rs             # `refstore add` (project-level)
    sync.rs            # `refstore sync` — copies content to .references/
    list.rs            # `refstore list` — top-level discovery
    search.rs          # `refstore search` — content search across references
    info.rs            # `refstore info` — shows reference or bundle details
    status.rs          # `refstore status`
    init.rs            # `refstore init`
    versions.rs        # `refstore versions`
    config.rs          # `refstore config show/set/get`
    self_ref.rs        # Auto-injects refstore instructions into CLAUDE.md/AGENTS.md
    mcp.rs             # `refstore mcp` — starts MCP server
    install_mcp.rs     # `refstore install-mcp` — writes .mcp.json
  store/
    repository.rs      # RepositoryStore — wraps local RegistryStore + remote RegistryStores
    registry.rs        # RegistryStore — single registry (index.toml + content/)
    project.rs         # ProjectStore — project manifest (refstore.toml + .references/)
  model/
    reference.rs       # Reference, ReferenceKind, ReferenceSource
    bundle.rs          # Bundle
    manifest.rs        # Manifest, ManifestEntry
    repository.rs      # RepositoryIndex (references + bundles maps)
    config.rs          # GlobalConfig, McpScope, Registry
  git/mod.rs           # All git operations via std::process::Command (not git2)
  mcp/tools.rs         # MCP server: 6 tools via rmcp 0.15.0
  error.rs             # RefstoreError enum (thiserror)
```

### Key data flow

- **RepositoryStore** is the main store entry point. It opens the local registry (`~/.local/share/refstore/`) and loads any remote registries from `registries/` submodules.
- **RegistryStore** represents a single registry — just an `index.toml` + `content/<name>/` directory. Both local and remote registries use the same structure.
- **ProjectStore** manages `refstore.toml` in a project. `resolve_all_references()` expands bundles and merges explicit entries (explicit wins over bundle).
- Multi-registry resolution: `RepositoryStore::resolve()` checks local first, then remotes alphabetically.
- Every write to the local registry (add/remove/update) auto-commits via `git::commit()`.

### CLI ↔ Store relationship

CLI handlers are thin — they parse args, call store methods, and print output. Business logic lives in `src/store/`. When adding new CLI commands:
1. Add the variant to the enum in `cli/mod.rs`
2. Add the match arm in `main.rs`
3. Create or update the handler in `cli/`
4. Add store methods if needed in `store/`

## Conventions

- **Error handling**: Store layer returns `Result<_, RefstoreError>` (thiserror). CLI layer uses `anyhow::Result` with `.context()` for user-facing messages.
- **Git operations**: Always use `src/git/mod.rs` functions, never `git2`/libgit2. Shallow clones with `--single-branch` by default.
- **Name validation**: `validate_name()` in `repository.rs` — alphanumeric, hyphens, underscores, dots only.
- **Interactive prompts**: Use `eprint!` to stderr, read from stdin. Provide `--force` to skip.
- **Output**: `println!` for normal output, `eprintln!` for warnings. Progress uses `stdout().flush()`.
- **Serialization**: All config/index files are TOML. MCP params use serde + schemars::JsonSchema.

## MCP Server (rmcp 0.15.0)

The MCP server in `src/mcp/tools.rs` uses:
- `#[tool_router]` on the impl block with `#[tool]` methods
- `#[tool_handler]` on `impl ServerHandler` block
- `Parameters<T>` wrapper where T derives `schemars::JsonSchema`
- Must `use rmcp::schemars;` for the derive macro to resolve

## Gotchas

- clap: `#[arg(long, name = "ref")]` does NOT set the long flag to `--ref`. Use `#[arg(long = "ref")]`.
- clap: `propagate_version = true` adds `--version` to all subcommands — don't name args `version`.
- git submodule: `file://` URLs need `-c protocol.file.allow=always` on modern git.
- git submodule: After `deinit + rm`, changes are already staged — don't try to re-stage the removed path.
- `RepositoryStore::open()` auto-creates the data dir and runs `git init` if needed. It's safe to call on a fresh machine.

<!-- refstore -->
## refstore

This project uses refstore to manage reference documentation. Synced references live in `.references/` — **read them directly with your filesystem tools** (Read, Grep, Glob). Each subdirectory maps to an entry in `refstore.toml`.

Do NOT use MCP tools to read reference content. MCP tools are for discovery and management only.

Commands: `refstore status`, `refstore sync`, `refstore list`, `refstore search <query>`, `refstore add <name>`, `refstore add --bundle <name>`, `refstore remove <name> --purge`

MCP tools: `list_references`, `get_reference`, `add_to_project`, `list_bundles`, `get_bundle`, `get_tutorial`
