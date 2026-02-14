use serde::{Deserialize, Serialize};

/// Metadata for a remote registry, stored in config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub name: String,
    pub url: String,
}
