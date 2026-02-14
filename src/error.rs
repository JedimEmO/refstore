use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RefstoreError {
    #[error("failed to read file: {path}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file: {path}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create directory: {path}")]
    DirCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("manifest not found; run `refstore init` first")]
    ManifestNotFound,

    #[error("failed to parse manifest: {0}")]
    ManifestParse(#[from] toml::de::Error),

    #[error("manifest already exists at {0}")]
    ManifestExists(PathBuf),

    #[error("reference '{name}' not found in repository")]
    ReferenceNotFound { name: String },

    #[error("reference '{name}' already exists in repository")]
    ReferenceExists { name: String },

    #[error("invalid reference name '{name}': {reason}")]
    InvalidName { name: String, reason: String },

    #[error("git command failed: {0}")]
    GitCommand(String),

    #[error("git is not installed or not in PATH")]
    GitNotFound,

    #[error("failed to determine data directory; set XDG_DATA_HOME or --data-dir")]
    DataDirNotFound,

    #[error("sync failed for '{name}': {reason}")]
    SyncFailed { name: String, reason: String },

    #[error("failed to serialize TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("bundle '{name}' not found in repository")]
    BundleNotFound { name: String },

    #[error("bundle '{name}' already exists in repository")]
    BundleExists { name: String },

    #[error("bundle '{bundle}' references unknown reference '{reference}'")]
    BundleInvalidReference { bundle: String, reference: String },
}
