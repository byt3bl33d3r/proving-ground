//! Command-line arguments.

use clap::{ArgAction, Parser, Subcommand};

use crate::output::Format;

/// Command-line client for the {{project-name}} service.
#[derive(Debug, Parser)]
#[command(
    name = "{{project-name}}",
    version,
    about = "Command-line client for the {{project-name}} service"
)]
pub struct Cli {
    /// Server base URL (`--server-url`).
    #[arg(
        long,
        global = true,
        value_name = "URL",
        help = "Server base URL [default: .harness/app.json, then $APP_URL]"
    )]
    pub server_url: Option<String>,
    /// Output format [default: text on a terminal, json otherwise]
    #[arg(long, global = true, value_enum)]
    pub output: Option<Format>,
    /// Verbosity count (`-v`).
    #[arg(short, long, global = true, action = ArgAction::Count,
        help = "More logging: -v debug, -vv trace (overrides RUST_LOG)")]
    pub verbose: u8,
    /// Quietness count (`-q`).
    #[arg(short, long, global = true, action = ArgAction::Count, conflicts_with = "verbose",
        help = "Less logging: -q errors only, -qq nothing (overrides RUST_LOG)")]
    pub quiet: u8,
    /// Export this run's spans and logs over OTLP (off by default)
    #[arg(long, global = true)]
    pub otel: bool,
    /// What to do
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// The `RUST_LOG` filter implied by `-v`/`-q`, if either was given.
    pub const fn log_filter(&self) -> Option<&'static str> {
        match (self.verbose, self.quiet) {
            (0, 0) => None,
            (1, _) => Some("debug"),
            (2.., _) => Some("trace"),
            (0, 1) => Some("error"),
            (0, 2..) => Some("off"),
        }
    }
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create, read, list and delete items
    #[command(subcommand)]
    Items(ItemsCommand),
    /// Check server reachability and print this worktree's harness state as JSON
    Doctor,
}

/// `items` subcommands, one per API route.
#[derive(Debug, Subcommand)]
pub enum ItemsCommand {
    /// Create an item
    Create {
        /// Item name (1 to 64 characters)
        name: String,
    },
    /// Show one item
    Get {
        /// Item id (itm_ followed by 16 hex digits)
        id: String,
    },
    /// List items in id order
    List {
        /// Index of the first item
        #[arg(long)]
        offset: Option<u32>,
        /// Page size (1 to 100)
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Delete an item
    Delete {
        /// Item id (itm_ followed by 16 hex digits)
        id: String,
    },
}
