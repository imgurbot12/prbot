//! Gitea API Components and Functions

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chttp::{http::StatusCode, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_position: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub body: String,
    pub commit_id: String,
    pub comments: Vec<Comment>,
}

fn check_status(res: &mut Response<Body>) -> Result<()>
where
{
    let status = res.status();
    match status {
        StatusCode::OK => Ok(()),
        StatusCode::NO_CONTENT => Ok(()),
        _ => {
            let body = res.body_mut();
            let text = body.text().context("invalid response body")?;
            Err(anyhow!("unexpected http response: {status} {text}"))
        }
    }
}

/// Clean Old PR Reviews Attached to User
pub fn clean_old_reviews(pr_url: String, user: String, token: String) -> Result<()> {
    // retrieve list of PR reviews
    log::debug!("GET {pr_url}/reviews");
    let req = Request::get(format!("{pr_url}/reviews"))
        .header("Authorization", format!("token {token}"))
        .timeout(Duration::from_secs(5))
        .body(())
        .context("failed to build request")?;
    let mut res = chttp::send(req).context("review request failed")?;
    check_status(&mut res).context("failed to list pr reviews")?;
    // iterate reviews
    let body = res.body_mut();
    let reviews: Vec<serde_json::Value> = body.json().context("failed to parse review list")?;
    for review in reviews {
        let id = review
            .get("id")
            .context("failed to find review id")?
            .as_u64()
            .context("invalid review id")?;
        let username = review
            .get("user")
            .context("failed to find review user object")?
            .get("username")
            .context("failed to find review username")?
            .as_str()
            .context("invalid review username")?;
        if username != user {
            continue;
        }
        log::debug!("DELETE {pr_url}/reviews/{id}");
        let req = Request::delete(format!("{pr_url}/reviews/{id}"))
            .header("Authorization", format!("token {token}"))
            .timeout(Duration::from_secs(5))
            .body(())
            .context("failed to build delete request")?;
        let mut res = chttp::send(req).context("delete request failed")?;
        check_status(&mut res).context("failed to delete pr review")?;
    }
    Ok(())
}

/// Retrieve Latest Commit for PR
pub fn latest_commit(pr_url: String, token: String) -> Result<String> {
    log::debug!("GET {pr_url}/commits");
    let req = Request::get(format!("{pr_url}/commits"))
        .header("Authorization", format!("token {token}"))
        .timeout(Duration::from_secs(5))
        .body(())
        .context("failed to build commit lookup request")?;
    let mut res = chttp::send(req).context("commits request failed")?;
    check_status(&mut res).context("failed to list pr reviews")?;
    // parse commit-id from response
    let commits: Vec<serde_json::Value> = res.json().context("invalid commits json")?;
    Ok(commits[0]
        .get("sha")
        .context("commit sha missing")?
        .as_str()
        .context("invalid commit sha")?
        .to_string())
}

/// Submit new PR Review
pub fn new_review(pr_url: String, token: String, review: ReviewRequest) -> Result<()> {
    log::debug!("POST {pr_url}/reviews");
    let req = Request::post(format!("{pr_url}/reviews"))
        .header("Authorization", format!("token {token}"))
        .timeout(Duration::from_secs(5))
        .body(serde_json::to_string(&review).context("failed to serialize request")?)
        .context("failed to build review submission request")?;
    let mut res = chttp::send(req).context("review submission request failed")?;
    check_status(&mut res).context("failed to submit pr review")?;
    Ok(())
}
