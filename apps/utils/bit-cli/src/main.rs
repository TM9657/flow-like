mod catalog;
mod commands;
mod context;
mod hub_admin;
mod spec;
mod store;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::context::Context;

#[derive(Parser)]
#[command(
    name = "flow-bits",
    version,
    about = "Inspect and manage the local Flow-Like bit store"
)]
struct Cli {
    /// Bit store directory, defaults to the one the desktop app is configured with
    #[arg(long, global = true, value_name = "DIR")]
    store: Option<PathBuf>,

    /// Hub domain for catalog lookups, defaults to the active profile's hub
    #[arg(long, global = true, value_name = "DOMAIN")]
    hub: Option<String>,

    /// Personal access token for catalog writes, defaults to FLOW_LIKE_PAT
    #[arg(long, global = true, value_name = "PAT")]
    token: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Summarize the store: size, artifacts and problems
    Status {
        #[arg(long)]
        json: bool,
    },
    /// List the artifacts held in the store
    List {
        /// Name the directories by querying the hub catalog
        #[arg(long)]
        resolve: bool,
        #[arg(long)]
        json: bool,
    },
    /// Search the hub catalog
    Search {
        query: Option<String>,
        /// Restrict to a bit type, repeatable (Llm, Embedding, Tts, …)
        #[arg(long = "type", value_name = "TYPE")]
        bit_type: Vec<String>,
        #[arg(long, default_value_t = 25)]
        limit: u64,
        #[arg(long)]
        json: bool,
    },
    /// Show a bit with its dependencies and install state
    Info {
        bit: String,
        #[arg(long)]
        json: bool,
    },
    /// Download a bit and everything it depends on
    Install { bits: Vec<String> },
    /// Delete store directories by bit id or by hash
    Remove {
        targets: Vec<String>,
        /// Include the artifacts the bit depends on, which other bits may share
        #[arg(long)]
        with_dependencies: bool,
        /// Delete instead of printing the plan
        #[arg(long)]
        yes: bool,
    },
    /// Author bits in the hub catalog
    #[command(subcommand)]
    Hub(HubCommand),
    /// Report broken store entries, optionally cleaning them up
    Doctor {
        /// Delete empty directories, superseded partials and zero byte files
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum HubCommand {
    /// Check which user the token acts as and whether it may write bits
    Whoami,
    /// Write a spec file to the catalog: dependencies first, then metadata
    Push {
        spec: PathBuf,
        /// Resolve and order the spec without contacting the hub
        #[arg(long)]
        dry_run: bool,
    },
    /// Turn a catalog bit into a spec file to edit and push back
    Pull {
        bit: String,
        /// Include the bits it depends on
        #[arg(long)]
        with_dependencies: bool,
        /// Write to this file instead of standard output
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Remove bits from the catalog
    Delete {
        bits: Vec<String>,
        /// Delete instead of printing the plan
        #[arg(long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run(Cli::parse()).await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let ctx = Context::new(cli.store, cli.hub, cli.token)?;

    match cli.command {
        Command::Status { json } => commands::status(&ctx, json),
        Command::List { resolve, json } => commands::list(&ctx, resolve, json).await,
        Command::Search {
            query,
            bit_type,
            limit,
            json,
        } => commands::search_catalog(&ctx, query, bit_type, limit, json).await,
        Command::Info { bit, json } => commands::info(&ctx, &bit, json).await,
        Command::Install { bits } => commands::install(&ctx, &bits).await,
        Command::Remove {
            targets,
            with_dependencies,
            yes,
        } => commands::remove(&ctx, &targets, with_dependencies, yes).await,
        Command::Hub(command) => match command {
            HubCommand::Whoami => commands::hub_whoami(&ctx).await,
            HubCommand::Push { spec, dry_run } => commands::hub_push(&ctx, &spec, dry_run).await,
            HubCommand::Pull {
                bit,
                with_dependencies,
                out,
            } => commands::hub_pull(&ctx, &bit, with_dependencies, out).await,
            HubCommand::Delete { bits, yes } => commands::hub_delete(&ctx, &bits, yes).await,
        },
        Command::Doctor { fix, json } => commands::doctor(&ctx, fix, json),
    }
}
