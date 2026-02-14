pub mod config;
pub mod manifest;
pub mod reference;
pub mod repository;

pub use config::{GlobalConfig, McpScope};
pub use manifest::{Manifest, ManifestEntry};
pub use reference::{Reference, ReferenceKind, ReferenceSource};
pub use repository::RepositoryIndex;
