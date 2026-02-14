# refstore

Reference documentation manager for LLM coding agents.

Manage a central repository of reference material — documentation, example code, config templates, style guides — and sync relevant subsets into each project's `.references/` directory. Agents (or humans) can then read from `.references/` for grounded, project-specific context.

## Why

LLM coding agents work better with reference material: API docs, coding conventions, example implementations. But managing these across many projects is tedious. refstore solves this:

- **Central repository** — add references once (local files, directories, or git repos), reuse everywhere.
- **Per-project manifests** — each project declares which references it needs in `refstore.toml`.
- **Selective sync** — include/exclude globs to pull only what's relevant.
- **Bundles** — group related references together (e.g., "rust-stack") and add them to projects in one step.
- **Remote registries** — share references across machines and teams via git. Pull community or company registries as submodules.
- **Version pinning** — pin references to specific registry tags or commits for reproducible builds.
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

### Bundles

Group references together for easy reuse across projects:

```bash
refstore repo bundle create <name> --ref <ref1> --ref <ref2>
  --description "..."                #   Human-readable description
  --tag <tag>                        #   Tags for filtering (repeatable)

refstore repo bundle list            # List all bundles
  --tag <tag>                        #   Filter by tag

refstore repo bundle info <name>     # Show bundle details
refstore repo bundle update <name>   # Modify a bundle
  --add-ref <ref>                    #   Add references (repeatable)
  --remove-ref <ref>                 #   Remove references (repeatable)
  --description "..."                #   Update description

refstore repo bundle remove <name>   # Remove a bundle
  --force                            #   Skip confirmation prompt
```

Add a bundle to a project and all its references are synced:

```bash
refstore add --bundle rust-stack
refstore sync
```

### Remote registries

Share references across machines and teams. Remote registries are git repos added as submodules to your local registry:

```bash
refstore registry add <name> <git-url>   # Add a remote registry
refstore registry list                   # List all registries (local + remote)
refstore registry update [name]          # Pull latest from remote (all if omitted)
refstore registry remove <name>          # Remove a remote registry
  --force                                #   Skip confirmation prompt
```

References from remote registries appear in `repo list` and can be added to projects just like local references. When names conflict, local references take precedence.

### Versioning

The local registry is a git repo. Every `repo add`, `repo update`, and `repo remove` creates a commit. You can tag states and pin projects to specific versions:

```bash
refstore repo tag <name>             # Tag the current registry state
  -m "..."                           #   Optional tag message (annotated tag)

refstore repo tags                   # List all tags

refstore versions <name>             # Show version history for a reference
```

Pin a reference to a specific version when adding it to a project:

```bash
refstore add api-examples --pin v1.0
refstore sync                        # Syncs content from the v1.0 tag, not HEAD
```

### Project workflow

```bash
refstore init                        # Initialize refstore.toml in current directory
  --path <dir>                       #   Target directory
  --commit-references                #   Don't gitignore .references/

refstore add <name>                  # Add a reference to the project manifest
  --bundle                           #   Add a bundle instead of a single reference
  --include <glob>                   #   Only sync matching files (repeatable)
  --exclude <glob>                   #   Skip matching files (repeatable)
  --pin <rev>                        #   Pin to a registry tag or commit
  --path <override>                  #   Custom path within .references/

refstore remove <name>               # Remove from manifest
  --bundle                           #   Remove a bundle
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

Exposes tools: `list_references`, `get_reference`, `read_reference_file`, `list_reference_files`, `search_references`, `list_bundles`, `get_bundle`, and `add_to_project` (requires `read_write` scope).

## Project manifest

`refstore.toml` in the project root:

```toml
version = 1
gitignore_references = true

[references.rust-guidelines]

[references.api-examples]
include = ["**/*.rs"]
exclude = ["**/tests/*"]
version = "v1.0"

bundles = ["rust-stack"]
```

## Data layout

The central repository lives at `~/.local/share/refstore/` (or `$XDG_DATA_HOME/refstore/`, overridable with `--data-dir`). It is a git repo — all mutations are committed automatically:

```
~/.local/share/refstore/          # git repo
  .git/
  .gitmodules                     # tracks remote registries
  .gitignore                      # excludes config.toml
  index.toml                      # reference & bundle definitions
  config.toml                     # local settings (gitignored)
  content/                        # cached reference content
    rust-guidelines/
    api-examples/
  registries/                     # remote registries (git submodules)
    community/                    # submodule → https://github.com/.../refs.git
      index.toml
      content/
        some-ref/
```

## Development

```bash
cargo build
cargo test          # 80 integration tests
```

## License

MIT
