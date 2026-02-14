# refstore

Reference documentation manager for LLM coding agents.

Manage a central repository of reference material — documentation, example code, config templates, style guides — and sync relevant subsets into each project's `.references/` directory. Agents (or humans) can then read from `.references/` for grounded, project-specific context.

## Why

LLM coding agents work better with reference material: API docs, coding conventions, example implementations. But managing these across many projects is tedious. refstore solves this:

- **Central repository** — add references once (local files, directories, or git repos), reuse everywhere.
- **Per-project manifests** — each project declares which references it needs in `refstore.toml`.
- **Selective sync** — include/exclude globs to pull only what's relevant.
- **MCP server** — expose references to agents via Model Context Protocol, with configurable read/write scope.

## Install

```
cargo install --path .
```

Requires Rust 1.85+ and git.

## Quick start

```bash
# Add a reference to the central repository
refstore repo add rust-guidelines ~/docs/rust-guidelines --tag rust --description "Team Rust conventions"
refstore repo add api-examples https://github.com/org/api-examples.git --ref main --tag api

# Initialize refstore in a project
cd my-project
refstore init

# Add references to the project
refstore add rust-guidelines
refstore add api-examples --include "**/*.rs" --exclude "**/tests/*"

# Sync content into .references/
refstore sync

# Check status
refstore status
```

After syncing, `my-project/.references/` contains:

```
.references/
  rust-guidelines/
    ...
  api-examples/
    src/
      lib.rs
      ...
```

## CLI reference

### Central repository

```bash
refstore repo add <name> <source>    # Add a reference (file, dir, or git URL)
  --description "..."                #   Human-readable description
  --tag <tag>                        #   Tags for filtering (repeatable)
  --ref <branch|tag|commit>          #   Git ref to checkout
  --subpath <path>                   #   Subdirectory within a git repo

refstore repo list                   # List all references
  --tag <tag>                        #   Filter by tag
  --kind <file|directory|git_repo>   #   Filter by kind

refstore repo info <name>            # Show reference details
refstore repo update [name]          # Re-fetch content from source (all if omitted)
refstore repo remove <name>          # Remove a reference
  --force                            #   Skip confirmation prompt
```

### Project workflow

```bash
refstore init                        # Initialize refstore.toml in current directory
  --path <dir>                       #   Target directory
  --commit-references                #   Don't gitignore .references/

refstore add <name>                  # Add a reference to the project manifest
  --include <glob>                   #   Only sync matching files (repeatable)
  --exclude <glob>                   #   Skip matching files (repeatable)
  --pin <rev>                        #   Pin to a version/commit
  --path <override>                  #   Custom path within .references/

refstore remove <name>               # Remove from manifest
  --purge                            #   Also delete synced content

refstore sync [name]                 # Sync .references/ from manifest
  --force                            #   Re-sync even if up to date

refstore status                      # Show sync status of all references
```

### Configuration

```bash
refstore config show                 # Show all settings
refstore config get <key>            # Get a value
refstore config set <key> <value>    # Set a value
```

| Key | Values | Default |
|-----|--------|---------|
| `mcp_scope` | `read_only`, `read_write` | `read_only` |
| `git_depth` | any positive integer | `1` |
| `default_branch` | branch name or `none` | (not set) |

### MCP server

```bash
refstore install-mcp                 # Register in .mcp.json (auto-detects binary path)
  --name <name>                      #   Override server name (default: refstore)
  --path <dir>                       #   Target directory (default: cwd)

refstore mcp                         # Start MCP server (stdio transport)
```

To set up the MCP server in a project, just run `refstore install-mcp` — it creates or updates `.mcp.json` with the correct binary path and args. Works with Claude Code, Cursor, and other MCP clients.

Exposes tools: `list_references`, `get_reference`, `read_reference_file`, `list_reference_files`, `search_references`, and `add_to_project` (requires `read_write` scope).

## Project manifest

`refstore.toml` in the project root:

```toml
version = 1
gitignore_references = true

[references.rust-guidelines]

[references.api-examples]
include = ["**/*.rs"]
exclude = ["**/tests/*"]
```

## Data layout

Central repository lives at `~/.local/share/refstore/` (or `$XDG_DATA_HOME/refstore/`, overridable with `--data-dir`):

```
~/.local/share/refstore/
  index.toml      # Reference metadata
  config.toml     # Global settings
  content/        # Cached reference content
    rust-guidelines/
    api-examples/
```

## Development

```bash
cargo build
cargo test          # 42 integration tests
```

## License

MIT
