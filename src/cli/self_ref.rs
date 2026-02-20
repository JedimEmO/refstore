use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

const MARKER: &str = "<!-- refstore -->";
const AGENT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md"];

const AGENT_INSTRUCTIONS: &str = "\
<!-- refstore -->
## refstore

This project uses refstore to manage reference documentation. Synced references live in \
`.references/` — **read them directly with your filesystem tools** (Read, Grep, Glob). Each \
subdirectory maps to an entry in `refstore.toml`.

Do NOT use MCP tools to read reference content. MCP tools are for discovery and management only.

Commands: `refstore status`, `refstore sync`, `refstore list`, `refstore search <query>`, \
`refstore add <name>`, `refstore add --bundle <name>`, `refstore remove <name> --purge`

MCP tools: `list_references`, `get_reference`, `add_to_project`, `list_bundles`, `get_bundle`, \
`get_tutorial`
";

/// Prompt the user and optionally append refstore instructions.
pub fn maybe_install(project_root: &Path) -> Result<()> {
    let existing = find_existing(project_root);
    let targets = find_targets(project_root);

    // Already installed everywhere
    if !existing.is_empty() && targets.is_empty() {
        for file in &existing {
            println!("  {} already contains refstore instructions", file);
        }
        return Ok(());
    }

    if !targets.is_empty() {
        // Existing file(s) to append to
        let names: Vec<&str> = targets.iter().map(|s| s.as_str()).collect();
        eprint!(
            "Add refstore instructions to {}? [Y/n] ",
            names.join(" and ")
        );
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let answer = input.trim();
        if !answer.is_empty() && !answer.eq_ignore_ascii_case("y") {
            return Ok(());
        }

        for file in &targets {
            append_to(project_root, file)?;
        }
    } else {
        // No agent file exists — ask which to create
        eprintln!("Add refstore instructions for LLM agents?");
        eprintln!("  1) CLAUDE.md (Claude Code)");
        eprintln!("  2) AGENTS.md (generic)");
        eprintln!("  3) Skip");
        eprint!("Choice [1]: ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim() {
            "" | "1" => append_to(project_root, "CLAUDE.md")?,
            "2" => append_to(project_root, "AGENTS.md")?,
            _ => return Ok(()),
        }
    }

    Ok(())
}

/// Append refstore instructions without prompting.
/// Appends to existing agent files, or creates AGENTS.md if none exist.
pub fn install(project_root: &Path) -> Result<()> {
    let targets = find_targets(project_root);

    if targets.is_empty() {
        let existing = find_existing(project_root);
        if !existing.is_empty() {
            for file in &existing {
                println!("  {} already contains refstore instructions", file);
            }
        } else {
            append_to(project_root, "AGENTS.md")?;
        }
    } else {
        for file in &targets {
            append_to(project_root, file)?;
        }
    }

    Ok(())
}

/// Files that exist but don't have the marker yet.
fn find_targets(project_root: &Path) -> Vec<String> {
    AGENT_FILES
        .iter()
        .filter(|f| {
            let path = project_root.join(f);
            path.exists() && !file_has_marker(&path)
        })
        .map(|f| f.to_string())
        .collect()
}

/// Files that exist and already have the marker.
fn find_existing(project_root: &Path) -> Vec<String> {
    AGENT_FILES
        .iter()
        .filter(|f| file_has_marker(&project_root.join(f)))
        .map(|f| f.to_string())
        .collect()
}

fn file_has_marker(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.contains(MARKER))
        .unwrap_or(false)
}

fn append_to(project_root: &Path, filename: &str) -> Result<()> {
    let path = project_root.join(filename);

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        if content.contains(MARKER) {
            println!("  {} already contains refstore instructions", filename);
            return Ok(());
        }

        let separator = if content.ends_with('\n') { "\n" } else { "\n\n" };
        std::fs::write(&path, format!("{content}{separator}{AGENT_INSTRUCTIONS}\n"))
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        std::fs::write(&path, format!("{AGENT_INSTRUCTIONS}\n"))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    println!("  added refstore instructions to {}", filename);
    Ok(())
}
