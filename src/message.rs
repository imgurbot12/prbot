//! Gitea Actions Log Message Parser
use std::{collections::HashMap, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::api::Comment;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum LogLevel {
    #[default]
    Debug,
    Notice,
    Warning,
    Error,
}

impl LogLevel {
    pub fn markdown_level(&self) -> &str {
        match self {
            Self::Debug => "NOTE",
            Self::Notice => "IMPORTANT",
            Self::Warning => "WARNING",
            Self::Error => "CAUTION",
        }
    }
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Self::Debug),
            "notice" => Ok(Self::Notice),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(anyhow::anyhow!("invalid loglevel: {s:?}")),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: LogLevel,
    pub message: String,
    pub title: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub end_line: Option<usize>,
    pub col: Option<usize>,
    pub end_col: Option<usize>,
}

#[inline]
fn get_int(map: &HashMap<&str, &str>, key: &str) -> Result<Option<usize>> {
    Ok(match map.get(key) {
        None => None,
        Some(value) => Some(usize::from_str(value).context(format!("invalid {key}: {value:?}"))?),
    })
}

impl LogMessage {
    pub fn parse(line: &str) -> Result<Self> {
        let (_, line) = line.split_once("::").context("missing intitial `::`")?;
        let (flags_raw, message) = line.split_once("::").context("missing secondary `::`")?;
        let (level, flags_raw) = flags_raw.split_once(' ').unwrap_or((flags_raw, ""));
        let flags: HashMap<&str, &str> = flags_raw
            .split(',')
            .into_iter()
            .filter_map(|kv| kv.split_once('='))
            .collect();
        Ok(LogMessage {
            level: LogLevel::from_str(level)?,
            message: message.to_owned(),
            title: flags.get("title").map(|s| s.to_string()),
            file: flags.get("file").map(|s| s.to_string()),
            line: get_int(&flags, "line")?,
            end_line: get_int(&flags, "endLine")?,
            col: get_int(&flags, "col")?,
            end_col: get_int(&flags, "endColumn")?,
        })
    }
    pub fn comment(&self) -> Comment {
        Comment {
            body: format!("> [!{}]\n> {}", self.level.markdown_level(), self.message),
            path: self.file.clone(),
            new_position: self.line,
        }
    }
}

/*

Error Report from Gitea Actions
| <!-- -->    |  <!-- --> |
| ------- | ------ |
| <span style="color:#1e7ac8">**debug**</span>   |  1 |
| <span style="color:#5a30b5">**notice**</span> |  2 |
| <span style="color:#573a08">**warning**</span> |  3 |
| <span style="color:#c82121">**error**</span>    |  4 |

*/

#[inline]
fn count(messages: &Vec<LogMessage>, level: LogLevel) -> usize {
    messages.iter().filter(|m| m.level == level).count()
}

/// Format PR Review Main Body
pub fn body(messages: &Vec<LogMessage>) -> String {
    // determine actions job-url
    let server_url = std::env::var("GITHUB_SERVER_URL").unwrap_or_default();
    let repo = std::env::var("GITHUB_REPOSITORY").unwrap_or_default();
    let job_id = std::env::var("GITHUB_RUN_ID").unwrap_or_default();
    let url = if !server_url.is_empty() && !repo.is_empty() && !job_id.is_empty() {
        format!(" [*]({server_url}/{repo}/actions/runs/{job_id})")
    } else {
        String::new()
    };
    format!(
        r#"
**Error Report from Gitea Actions{url}**
|                    <!-- -->                    | <!-- --> |
| ---------------------------------------------- | -------- |
| <span style="color:#1e7ac8">**debug**</span>   |  {} |
| <span style="color:#5a30b5">**notice**</span>  |  {} |
| <span style="color:#fbbd08">**warning**</span> |  {} |
| <span style="color:#c82121">**error**</span>   |  {} |
    "#,
        count(messages, LogLevel::Debug),
        count(messages, LogLevel::Notice),
        count(messages, LogLevel::Warning),
        count(messages, LogLevel::Error),
    )
    .trim()
    .to_owned()
}

/// Save messages into file cache
pub fn save_cache(messages: Vec<LogMessage>, cache: &PathBuf) -> Result<()> {
    let f = std::fs::File::create(cache).context("failed to create cache file")?;
    serde_json::to_writer(f, &messages).context("failed to write message cache")
}

/// Read messages in file cache
pub fn read_cache(cache: &PathBuf) -> Result<Vec<LogMessage>> {
    let body = std::fs::read_to_string(cache).context("failed to read cache file")?;
    serde_json::from_str(&body).context("failed to deserialize cache")
}
