//! PRBot CLI Implementation
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Args)]
struct PrepareArgs {
    /// Input to prepare messages From
    #[clap(default_value = "/dev/stdin")]
    input: String,
    /// Immeditely commit to review after reading
    #[clap(short, long)]
    commit: bool,
}

/// Available Subcommands for PrBot
#[derive(Debug, Subcommand)]
enum Command {
    /// Prepare messages for final review
    Prepare(PrepareArgs),
    /// Commit parsed messages into final review
    Commit,
}

/// Collect and Submit PR Feedback using Gitea Actions Annotations
#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}
