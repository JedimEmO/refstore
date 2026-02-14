use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    File,
    Directory,
    GitRepo,
}

impl std::fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Directory => write!(f, "directory"),
            Self::GitRepo => write!(f, "git_repo"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ReferenceSource {
    Local {
        path: PathBuf,
    },
    Git {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#ref: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subpath: Option<PathBuf>,
    },
    Remote {
        url: String,
    },
}

impl std::fmt::Display for ReferenceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local { path } => write!(f, "{}", path.display()),
            Self::Git { url, r#ref, .. } => {
                write!(f, "{url}")?;
                if let Some(r) = r#ref {
                    write!(f, " (ref: {r})")?;
                }
                Ok(())
            }
            Self::Remote { url } => write!(f, "{url}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub name: String,
    pub kind: ReferenceKind,
    pub source: ReferenceSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub added_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}
