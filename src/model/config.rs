use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::registry::Registry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpScope {
    ReadOnly,
    ReadWrite,
}

impl Default for McpScope {
    fn default() -> Self {
        Self::ReadOnly
    }
}

impl std::fmt::Display for McpScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadOnly => write!(f, "read_only"),
            Self::ReadWrite => write!(f, "read_write"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
    #[serde(default)]
    pub mcp_scope: McpScope,
    #[serde(default = "default_depth")]
    pub git_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<Registry>,
}

fn default_depth() -> u32 {
    1
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            mcp_scope: McpScope::default(),
            git_depth: 1,
            default_branch: None,
            registries: Vec::new(),
        }
    }
}
