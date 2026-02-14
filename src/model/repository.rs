use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::reference::Reference;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryIndex {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub references: BTreeMap<String, Reference>,
}

fn default_version() -> u32 {
    1
}
