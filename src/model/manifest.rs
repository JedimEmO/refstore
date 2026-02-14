use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_true")]
    pub gitignore_references: bool,
    #[serde(default)]
    pub references: BTreeMap<String, ManifestEntry>,
}

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

impl Manifest {
    pub fn new(gitignore_references: bool) -> Self {
        Self {
            version: 1,
            gitignore_references,
            references: BTreeMap::new(),
        }
    }
}
