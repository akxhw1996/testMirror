use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use log::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCommit {
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct CommentRequest {
    body: String,
}

pub fn get_commit_list_of_pr(base_url: &str, namespace: &str, repo_name: &str, pull_id: u32, platform: &str) -> Result<Vec<GitCommit>, Box<dyn std::error::Error>> {
    info!("Getting commit list for PR:");
    info!("  Platform: {}", platform);
    info!("  Base URL: {}", base_url);
    info!("  Namespace: {}", namespace);
    info!("  Repo: {}", repo_name);
    info!("  PR ID: {}", pull_id);

    let token = match platform {
        "github" => {
            let token = std::env::var("GITHUB_TOKEN")
                .map_err(|_| "GITHUB_TOKEN not set")?;
            info!("Using GitHub token: {}...", &token[..10]);
            token
        },
        "gitcode" => {
            let token = std::env::var("GITCODE_TOKEN")
                .map_err(|_| "GITCODE_TOKEN not set")?;
            info!("Using GitCode token: {}...", &token[..10]);
            token
        },
        _ => return Err("Unsupported platform".into()),
    };
    
    let url = format!(
        "{}/{}/{}/pulls/{}/commits",
        base_url, namespace, repo_name, pull_id
    );
    info!("Request URL: {}", url);

    let mut headers = HeaderMap::new();
    let auth_header = format!("Bearer {}", token);
    info!("Setting Authorization header: Bearer {}...", &token[..10]);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)?,
    );

    if platform == "github" {
        info!("Adding GitHub API version header");
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        
        info!("Adding User-Agent header");
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("HiTLS_GIT_BOT"),
        );
    }

    info!("Making HTTP request...");
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url)
        .headers(headers)
        .send()?;
    
    let status = response.status();
    info!("Response status: {}", status);
    if !status.is_success() {
        let error_text = response.text()?;
        error!("Error response body: {}", error_text);
        return Err(format!("Request failed with status {}: {}", status, error_text).into());
    }

    info!("Parsing response body...");
    let commits: Vec<GitCommit> = response.json()?;
    info!("Found {} commits", commits.len());
    
    Ok(commits)
}

pub fn post_comment_on_pr(
    base_url: &str,
    namespace: &str,
    repo_name: &str,
    pull_id: u32,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Posting comment on PR:");
    info!("  Base URL: {}", base_url);
    info!("  Namespace: {}", namespace);
    info!("  Repo: {}", repo_name);
    info!("  PR ID: {}", pull_id);

    let token = std::env::var("GITCODE_TOKEN")
        .map_err(|_| "GITCODE_TOKEN not set")?;
    info!("Using GitCode token: {}...", &token[..10]);

    let url = format!(
        "{}/{}/{}/pulls/{}/comments",
        base_url, namespace, repo_name, pull_id
    );
    info!("Request URL: {}", url);

    let mut headers = HeaderMap::new();
    let auth_header = format!("Bearer {}", token);
    info!("Setting Authorization header: Bearer {}...", &token[..10]);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)?,
    );

    info!("Adding User-Agent header");
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("GitBot"),
    );

    let comment = CommentRequest {
        body: message.to_string(),
    };

    info!("Making HTTP request...");
    let client = reqwest::blocking::Client::new();
    let response = client.post(&url)
        .headers(headers)
        .json(&comment)
        .send()?;

    let status = response.status();
    info!("Response status: {}", status);
    if !status.is_success() {
        let error_text = response.text()?;
        error!("Error response body: {}", error_text);
        return Err(format!("Request failed with status {}: {}", status, error_text).into());
    }

    info!("Comment posted successfully");
    Ok(())
}
