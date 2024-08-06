mod api;
mod cli;
mod message;

use anyhow::Result;
use clap::Parser;

use crate::cli::*;

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let pr_url = cli.pr_url()?;
    match cli.command {
        Command::Prepare(args) => args.prepare(&pr_url, &cli.user, &cli.token, &cli.cache),
        Command::Commit(args) => args.commit(&pr_url, &cli.user, &cli.token, &cli.cache),
    }
}
