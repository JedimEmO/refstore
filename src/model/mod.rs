pub mod bundle;
pub mod config;
pub mod manifest;
pub mod reference;
pub mod registry;
pub mod repository;

pub use bundle::Bundle;
pub use config::{GlobalConfig, McpScope};
pub use manifest::{Manifest, ManifestEntry};
pub use reference::{Reference, ReferenceKind, ReferenceSource};
pub use registry::Registry;
pub use repository::RepositoryIndex;
