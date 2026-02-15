# refstore

Reference documentation manager for LLM coding agents.

Manage a central repository of reference material — documentation, example code, config templates, style guides — and sync relevant subsets into each project's `.references/` directory. Agents (or humans) can then read from `.references/` for grounded, project-specific context.

## Why

LLM coding agents work better with reference material: API docs, coding conventions, example implementations. But managing these across many projects is tedious. refstore solves this:

- **Central store** — add references once (local files, directories, or git repos), reuse everywhere.
- **Per-project manifests** — each project declares which references it needs in `refstore.toml`.
- **Selective sync** — include/exclude globs to pull only what's relevant.
- **Bundles** — group related references together (e.g., "rust-stack") and add them to projects in one step.
- **Remote registries** — share references across machines and teams via git. Pull community or company registries as submodules.
- **Registry authoring** — create and publish your own registries for others to consume.
- **Version pinning** — pin references to specific registry tags or commits for reproducible builds.
- **MCP server** — expose references to agents via Model Context Protocol, with configurable read/write scope.

## Quickstart

```bash
cargo install --git https://github.com/JedimEmO/refstore
cd my-project
refstore init
refstore install-mcp
```

That's it. Once the MCP server is registered, Claude Code (and other MCP clients) will use it automatically — you don't need to learn the full CLI.

## Install

```bash
cargo install --git https://github.com/JedimEmO/refstore
```

Or from a local checkout:

```bash
cargo install --path .
```

Requires Rust 1.85+ and git.

## Usage

```bash
# Add a reference to the local store
refstore store add rust-guidelines ~/docs/rust-guidelines --tag rust --description "Team Rust conventions"
refstore store add api-examples https://github.com/org/api-examples.git --ref main --tag api

# Initialize refstore in a project
cd my-project
refstore init

# Add references to the project and sync
refstore add rust-guidelines --sync
refstore add api-examples --include "**/*.rs" --exclude "**/tests/*"
refstore sync

# Discover and search
refstore list
refstore search "authentication"
refstore info rust-guidelines
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

### Discovery

```bash
refstore list                        # List all references across registries
  --tag <tag>                        #   Filter by tag
  --kind <file|directory|git_repo>   #   Filter by kind

refstore search <query>              # Search content across references
  --ref <name>                       #   Limit to a specific reference

refstore info <name>                 # Show details (works for references and bundles)
refstore versions <name>             # Show version history for a reference
```

### Local store

```bash
refstore store add <name> <source>   # Add a reference (file, dir, or git URL)
  --description "..."                #   Human-readable description
  --tag <tag>                        #   Tags for filtering (repeatable)
  --ref <branch|tag|commit>          #   Git ref to checkout
  --subpath <path>                   #   Subdirectory within a git repo

refstore store update [name]         # Re-fetch content from source (all if omitted)
refstore store remove <name>         # Remove a reference
  --force                            #   Skip confirmation prompt

refstore store push <name>           # Copy a reference to another registry
  --to <path>                        #   Path to the target registry
```

### Bundles

Group references together for easy reuse across projects:

```bash
refstore bundle create <name> --ref <ref1> --ref <ref2>
  --description "..."                #   Human-readable description
  --tag <tag>                        #   Tags for filtering (repeatable)

refstore bundle list                 # List all bundles
  --tag <tag>                        #   Filter by tag

refstore bundle info <name>          # Show bundle details
refstore bundle update <name>        # Modify a bundle
  --add-ref <ref>                    #   Add references (repeatable)
  --remove-ref <ref>                 #   Remove references (repeatable)
  --description "..."                #   Update description

refstore bundle remove <name>        # Remove a bundle
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
refstore registry list               # List all registries (local + remote)
refstore registry add <name> <url>   # Add a remote registry
refstore registry update [name]      # Pull latest from remote (all if omitted)
refstore registry remove <name>      # Remove a remote registry
  --force                            #   Skip confirmation prompt
```

References from remote registries appear in `refstore list` and can be added to projects just like local references. When names conflict, local references take precedence.

### Registry authoring

Create and publish your own registries for others to consume:

```bash
# Create a new registry
refstore registry init /path/to/my-registry

# Push references from your local store into it
refstore store push rust-guidelines --to /path/to/my-registry
refstore store push api-examples --to /path/to/my-registry

# Or add content directly using --data-dir
refstore --data-dir /path/to/my-registry store add react-docs ./react-docs/

# Create bundles in it
refstore --data-dir /path/to/my-registry bundle create frontend-stack --ref react-docs

# Publish: push the registry to a git remote
cd /path/to/my-registry
git remote add origin git@github.com:team/registry.git
git push -u origin main
```

Others can then consume the registry:

```bash
refstore registry add team git@github.com:team/registry.git
refstore list                        # team's references are now visible
refstore add react-docs --sync       # sync from the team registry
```

### Versioning

The local registry is a git repo. Every `store add`, `store update`, and `store remove` creates a commit. You can tag states and pin projects to specific versions:

```bash
refstore store tag <name>            # Tag the current registry state
  -m "..."                           #   Optional tag message (annotated tag)

refstore store tags                  # List all tags

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
  --sync                             #   Sync content immediately after adding

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
cargo test          # 88 integration tests
```

## License

MIT
