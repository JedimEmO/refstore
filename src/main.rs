use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;
mod error;
mod git;
mod mcp;
mod model;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let is_mcp = matches!(cli.command, cli::Command::Mcp);
    if !is_mcp {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    if cli.verbose {
                        "refstore=debug"
                    } else {
                        "refstore=info"
                    }
                    .parse()
                    .unwrap()
                }),
            )
            .init();
    }

    match cli.command {
        cli::Command::Init {
            commit_references,
            path,
            self_ref,
            no_self_ref,
            install_mcp,
            no_mcp,
        } => cli::init::run(path, commit_references, self_ref, no_self_ref, install_mcp, no_mcp),
        cli::Command::Add {
            name,
            bundle,
            pin,
            path,
            include,
            exclude,
            sync,
        } => cli::add::run(cli.data_dir.as_ref(), name, bundle, pin, path, include, exclude, sync),
        cli::Command::Remove {
            name,
            bundle,
            purge,
        } => cli::remove::run(cli.data_dir.as_ref(), name, bundle, purge),
        cli::Command::Sync { name, force } => {
            cli::sync::run(cli.data_dir.as_ref(), name, force)
        }
        cli::Command::Status => cli::status::run(cli.data_dir.as_ref()),
        cli::Command::List { tag, kind } => cli::list::run(cli.data_dir.as_ref(), tag, kind),
        cli::Command::Search { query, reference } => {
            cli::search::run(cli.data_dir.as_ref(), query, reference)
        }
        cli::Command::Info { name } => cli::info::run(cli.data_dir.as_ref(), name),
        cli::Command::Versions { name } => cli::versions::run(cli.data_dir.as_ref(), name),
        cli::Command::Store(cmd) => cli::store::run(cli.data_dir.as_ref(), cmd),
        cli::Command::Bundle(cmd) => cli::bundle::run(cli.data_dir.as_ref(), cmd),
        cli::Command::Registry(cmd) => cli::registry::run(cli.data_dir.as_ref(), cmd),
        cli::Command::Mcp => cli::mcp::run(cli.data_dir).await,
        cli::Command::InstallMcp { name, path } => cli::install_mcp::run(name, path),
        cli::Command::Config(cmd) => cli::config::run(cli.data_dir.as_ref(), cmd),
    }
}
