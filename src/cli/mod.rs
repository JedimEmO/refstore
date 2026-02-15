pub mod add;
pub mod bundle;
pub mod config;
pub mod info;
pub mod init;
pub mod install_mcp;
pub mod list;
pub mod mcp;
pub mod registry;
pub mod remove;
pub mod search;
pub mod self_ref;
pub mod status;
pub mod store;
pub mod sync;
pub mod versions;

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

        /// Skip the refstore self-reference prompt
        #[arg(long)]
        no_self_ref: bool,

        /// Automatically add the refstore self-reference without prompting
        #[arg(long, conflicts_with = "no_self_ref")]
        self_ref: bool,

        /// Automatically install MCP server without prompting
        #[arg(long, conflicts_with = "no_mcp")]
        install_mcp: bool,

        /// Skip MCP server installation prompt
        #[arg(long, conflicts_with = "install_mcp")]
        no_mcp: bool,
    },

    /// Add a reference or bundle to the project manifest
    Add {
        /// Name of the reference or bundle (must exist in central repository)
        name: String,

        /// Add a bundle instead of a single reference
        #[arg(long)]
        bundle: bool,

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

        /// Sync content immediately after adding
        #[arg(long)]
        sync: bool,
    },

    /// Remove a reference or bundle from the project manifest
    Remove {
        /// Name of the reference or bundle to remove
        name: String,

        /// Remove a bundle instead of a single reference
        #[arg(long)]
        bundle: bool,

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

    /// List all available references across registries
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Filter by kind (file, directory, git_repo)
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// Search content across references
    Search {
        /// Text to search for (case-insensitive)
        query: String,

        /// Limit search to a specific reference
        #[arg(long = "ref")]
        reference: Option<String>,
    },

    /// Show detailed information about a reference or bundle
    Info {
        /// Name of the reference or bundle
        name: String,
    },

    /// Show version history for a reference
    Versions {
        /// Name of the reference
        name: String,
    },

    /// Manage the local reference store
    #[command(subcommand)]
    Store(StoreSubcommand),

    /// Manage bundles (named groups of references)
    #[command(subcommand)]
    Bundle(BundleSubcommand),

    /// Manage registries (local and remote)
    #[command(subcommand)]
    Registry(RegistrySubcommand),

    /// Start the MCP server (stdio transport)
    Mcp,

    /// Register refstore as an MCP server in .mcp.json
    InstallMcp {
        /// Server name in .mcp.json (default: refstore)
        #[arg(long, default_value = "refstore")]
        name: String,

        /// Target directory (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Manage global configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),
}

#[derive(Debug, Subcommand)]
pub enum StoreSubcommand {
    /// Add a reference to the local store
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

    /// Remove a reference from the local store
    Remove {
        /// Name of the reference to remove
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Update cached content for a reference (re-fetch from source)
    Update {
        /// Name of the reference to update (omit for all)
        name: Option<String>,
    },

    /// Tag the current state of the registry for version pinning
    Tag {
        /// Tag name (e.g., "v1.0")
        name: String,

        /// Optional tag message (creates annotated tag)
        #[arg(short, long)]
        message: Option<String>,
    },

    /// List tags on the registry
    Tags,

    /// Push a reference from the local store to another registry
    Push {
        /// Name of the reference to push
        name: String,

        /// Path to the target registry
        #[arg(long)]
        to: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum BundleSubcommand {
    /// Create a new bundle
    Create {
        /// Unique name for the bundle
        name: String,

        /// References to include in this bundle
        #[arg(long = "ref", required = true)]
        refs: Vec<String>,

        /// Human-readable description
        #[arg(short, long)]
        description: Option<String>,

        /// Tags for organization
        #[arg(short, long)]
        tag: Vec<String>,
    },

    /// List all bundles
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Show detailed information about a bundle
    Info {
        /// Name of the bundle
        name: String,
    },

    /// Add or remove references from a bundle
    Update {
        /// Name of the bundle to modify
        name: String,

        /// References to add
        #[arg(long = "add-ref")]
        add_refs: Vec<String>,

        /// References to remove
        #[arg(long = "remove-ref")]
        remove_refs: Vec<String>,

        /// Update description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove a bundle from the central repository
    Remove {
        /// Name of the bundle to remove
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RegistrySubcommand {
    /// List all configured registries
    List,

    /// Add a remote registry
    Add {
        /// Unique name for this registry
        name: String,
        /// Git URL of the registry repository
        url: String,
    },

    /// Remove a remote registry
    Remove {
        /// Name of the registry to remove
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Update registry definitions (git pull)
    Update {
        /// Specific registry to update (omit for all)
        name: Option<String>,
    },

    /// Initialize a new registry at the given path
    Init {
        /// Path to create the registry at
        path: PathBuf,
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
