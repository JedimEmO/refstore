use std::path::PathBuf;

use anyhow::{Context, Result};

pub fn run(name: String, path: Option<PathBuf>) -> Result<()> {
    let dir = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let mcp_path = dir.join(".mcp.json");

    let mut root: serde_json::Value = if mcp_path.exists() {
        let content = std::fs::read_to_string(&mcp_path)
            .with_context(|| format!("failed to read {}", mcp_path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", mcp_path.display()))?
    } else {
        serde_json::json!({ "mcpServers": {} })
    };

    let servers = root
        .as_object_mut()
        .context("invalid .mcp.json: expected an object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("invalid .mcp.json: mcpServers is not an object")?;

    if servers.contains_key(&name) {
        println!(
            "MCP server '{name}' already configured in {}",
            mcp_path.display()
        );
        return Ok(());
    }

    let bin = std::env::current_exe()
        .context("failed to determine refstore binary path")?
        .canonicalize()
        .context("failed to canonicalize binary path")?;

    servers.insert(
        name.clone(),
        serde_json::json!({
            "command": bin.to_string_lossy(),
            "args": ["mcp"]
        }),
    );

    let content = serde_json::to_string_pretty(&root).context("failed to serialize .mcp.json")?;
    std::fs::write(&mcp_path, content + "\n")
        .with_context(|| format!("failed to write {}", mcp_path.display()))?;

    println!("Added '{name}' to {}", mcp_path.display());
    Ok(())
}
