//! PRBot CLI Implementation
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use crate::api::{self, ReviewRequest};
use crate::message::*;

fn commit(pr_url: &str, user: &str, token: &str, messages: Vec<LogMessage>) -> Result<()> {
    log::info!("cleaing old reviews");
    api::clean_old_reviews(pr_url, user, token).context("failed to clean old reviews")?;
    let commit_id = match std::env::var("GITHUB_SHA") {
        Ok(cid) => cid,
        Err(_) => {
            log::info!("retrieving latest commit-id");
            api::latest_commit(pr_url, token).context("failed to find commit-id")?
        }
    };
    log::info!("submitting new review");
    api::new_review(
        pr_url,
        token,
        ReviewRequest {
            body: "Error Report from Gitea Actions".to_owned(),
            commit_id,
            comments: messages.iter().map(|m| m.comment()).collect(),
        },
    )
    .context("pr review submission failed")?;
    Ok(())
}

#[derive(Debug, Args)]
pub struct PrepareArgs {
    /// Input to prepare messages From
    #[clap(default_value = "/dev/stdin")]
    input: String,
    /// Immeditely commit to review after reading
    #[clap(short, long)]
    commit: bool,
}

impl PrepareArgs {
    pub fn new() -> Self {
        Self {
            input: "/dev/stdin".to_owned(),
            commit: false,
        }
    }
}

impl PrepareArgs {
    pub fn prepare(&self, pr_url: &str, user: &str, token: &str, cache: &str) -> Result<()> {
        // read existing message cache (if any)
        let path = PathBuf::from(&cache);
        let mut messages = match path.exists() {
            true => read_cache(&path).context("failed to read message cache")?,
            false => vec![],
        };
        // read new messages from input
        let f = std::fs::File::open(&self.input).context("failed to read input")?;
        let r = BufReader::new(f);
        for line in r
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| l.starts_with("::"))
        {
            let message = LogMessage::parse(&line)?;
            messages.push(message);
        }
        // write to cache if not immeditely commiting
        match self.commit {
            false => {
                log::info!("saving {} messages to cache: {path:?}", messages.len());
                save_cache(messages, &path).context("failed to write cache")?
            }
            true => {
                log::info!("commiting {} messages for final review", messages.len());
                commit(pr_url, user, token, messages).context("review commit failed")?;
                if path.exists() {
                    std::fs::remove_file(&path).context("failed to cleanup message cache")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Args)]
pub struct CommitArgs {}

impl CommitArgs {
    pub fn commit(&self, pr_url: &str, user: &str, token: &str, cache: &str) -> Result<()> {
        let path = PathBuf::from(&cache);
        if !path.exists() {
            return Err(anyhow::anyhow!("cache {path:?} is missing"));
        }
        let messages = read_cache(&path).context("failed to read message cache")?;
        if messages.is_empty() {
            log::warn!("collect messages are empty. skipping review");
            return Ok(());
        }
        commit(pr_url, user, token, messages).context("review commit failed")?;
        if path.exists() {
            std::fs::remove_file(&path).context("failed to cleanup message cache")?;
        }
        Ok(())
    }
}

/// Available Subcommands for PrBot
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Prepare messages for final review
    Prepare(PrepareArgs),
    /// Commit parsed messages into final review
    Commit(CommitArgs),
}

/// Collect and Submit PR Feedback using Gitea Actions Annotations
#[derive(Debug, Parser)]
pub struct Cli {
    /// Gitea Instance URL
    #[clap(short, long, env = "GITHUB_SERVER_URL")]
    pub gitea: String,
    /// Bot username used to post PR review
    #[clap(short, long, env = "GITEA_BOT_USER")]
    pub user: String,
    /// Bot access-token used to authenticate to API
    #[clap(short, long, env = "GITEA_BOT_TOKEN")]
    pub token: String,
    /// Repository name
    #[clap(short, long, env = "GITHUB_REPOSITORY")]
    pub repo: String,
    /// Pull Request number
    #[clap(short, long, env = "GITEA_PULL_REQUEST_NUMBER")]
    pub number: Option<usize>,
    /// Message cache filepath
    #[clap(short, long, default_value = "messages.cache")]
    pub cache: String,
    /// Available commands
    #[clap(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// Generate API URL to Manage Pull-Request
    pub fn pr_url(&self) -> Result<String> {
        log::debug!(
            "gitea={:?} repo={:?}, pr={:?}",
            self.gitea,
            self.repo,
            self.number
        );
        let (owner, repo) = self.repo.split_once('/').context("invalid repo string")?;
        let number = match self.number {
            Some(number) => number,
            None => {
                let gref = std::env::var("GITHUB_REF_NAME").context("cannot find pr number")?;
                let (number, _) = match gref.contains('/') {
                    true => gref
                        .split_once('/')
                        .context(format!("invalid `GITHUB_REF_NAME`: {gref:?}"))?,
                    false => (gref.as_str(), ""),
                };
                number
                    .parse()
                    .context(format!("invalid pr number in `GITHUB_REF_NAME`: {gref:?}"))?
            }
        };
        Ok(format!(
            "{}/api/v1/repos/{owner}/{repo}/pulls/{number}",
            self.gitea
        ))
    }
}
