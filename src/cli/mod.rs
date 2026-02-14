pub mod add;
pub mod config;
pub mod init;
pub mod mcp;
pub mod remove;
pub mod repo;
pub mod status;
pub mod sync;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "refstore",
    about = "Reference documentation manager for LLM coding agents",
    version,
    propagate_version = true
)]
pub struct Cli {
    /// Override the data directory (default: ~/.local/share/refstore)
    #[arg(long, env = "REFSTORE_DATA_DIR", global = true)]
    pub data_dir: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize refstore in the current project
    Init {
        /// Commit .references/ to git instead of gitignoring it
        #[arg(long)]
        commit_references: bool,

        /// Path to initialize (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Add a reference to the project manifest
    Add {
        /// Name of the reference (must exist in central repository)
        name: String,

        /// Pin to a specific version/commit
        #[arg(long, alias = "rev")]
        pin: Option<String>,

        /// Override target path within .references/
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Include only files matching these globs
        #[arg(long)]
        include: Vec<String>,

        /// Exclude files matching these globs
        #[arg(long)]
        exclude: Vec<String>,
    },

    /// Remove a reference from the project manifest
    Remove {
        /// Name of the reference to remove
        name: String,

        /// Also delete the synced content from .references/
        #[arg(long)]
        purge: bool,
    },

    /// Sync .references/ directory from manifest
    Sync {
        /// Only sync a specific reference
        name: Option<String>,

        /// Force re-download even if content appears up to date
        #[arg(short, long)]
        force: bool,
    },

    /// Show sync status of project references
    Status,

    /// Manage the central reference repository
    #[command(subcommand)]
    Repo(RepoSubcommand),

    /// Start the MCP server (stdio transport)
    Mcp,

    /// Manage global configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),
}

#[derive(Debug, Subcommand)]
pub enum RepoSubcommand {
    /// Add a reference to the central repository
    Add {
        /// Unique name for this reference
        name: String,

        /// Source: file path, directory path, or git URL
        source: String,

        /// Human-readable description
        #[arg(short, long)]
        description: Option<String>,

        /// Tags for organization
        #[arg(short, long)]
        tag: Vec<String>,

        /// Git ref (branch/tag/commit) to checkout
        #[arg(long = "ref")]
        git_ref: Option<String>,

        /// Subdirectory within a git repo to use as root
        #[arg(long)]
        subpath: Option<PathBuf>,
    },

    /// List all references in the central repository
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Filter by kind (file, directory, git_repo)
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// Remove a reference from the central repository
    Remove {
        /// Name of the reference to remove
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Show detailed information about a reference
    Info {
        /// Name of the reference
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., mcp_scope, git_depth)
        key: String,
        /// Configuration value
        value: String,
    },

    /// Get a specific configuration value
    Get {
        /// Configuration key
        key: String,
    },
}
