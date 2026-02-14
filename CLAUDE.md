<!-- refstore -->
## refstore

This project uses refstore to manage reference documentation. Read files in `.references/` for project-relevant context — each subdirectory maps to an entry in `refstore.toml`.

References can be added individually or via **bundles** (named groups of references defined in the central repository, e.g. a tech stack or project template). Bundles are listed under `bundles = [...]` in `refstore.toml` and expanded at sync time.

Commands: `refstore status`, `refstore sync`, `refstore repo list`, `refstore add <name>`, `refstore add --bundle <name>`, `refstore remove <name> --purge`

MCP tools: `list_references`, `get_reference`, `read_reference_file`, `list_reference_files`, `search_references`, `list_bundles`, `get_bundle`
