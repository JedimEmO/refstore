use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::store::ProjectStore;

pub fn run(
    path: Option<PathBuf>,
    commit_references: bool,
    self_ref: bool,
    no_self_ref: bool,
    install_mcp: bool,
    no_mcp: bool,
) -> Result<()> {
    let gitignore = !commit_references;
    let store = ProjectStore::init(path.as_deref(), gitignore)
        .context("failed to initialize project")?;

    println!("Initialized refstore at {}", store.root().display());
    println!("  manifest: {}/refstore.toml", store.root().display());
    println!("  references dir: {}", store.references_dir().display());
    if gitignore {
        println!("  .references/ added to .gitignore");
    } else {
        println!("  .references/ will be committed to git");
    }

    if self_ref {
        super::self_ref::install(store.root())?;
    } else if !no_self_ref {
        super::self_ref::maybe_install(store.root())?;
    }

    if install_mcp {
        super::install_mcp::run("refstore".into(), Some(store.root().to_path_buf()))?;
    } else if !no_mcp {
        eprint!("Install MCP server (.mcp.json)? [Y/n] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let answer = input.trim();
        if answer.is_empty() || answer.eq_ignore_ascii_case("y") {
            super::install_mcp::run("refstore".into(), Some(store.root().to_path_buf()))?;
        }
    }

    Ok(())
}
