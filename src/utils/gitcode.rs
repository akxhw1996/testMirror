use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use crate::utils::request;

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
    println!("Getting commit list for PR:");
    println!("  Platform: {}", platform);
    println!("  Base URL: {}", base_url);
    println!("  Namespace: {}", namespace);
    println!("  Repo: {}", repo_name);
    println!("  PR ID: {}", pull_id);

    let token = match platform {
        "github" => {
            let token = std::env::var("GITHUB_TOKEN")
                .map_err(|_| "GITHUB_TOKEN not set")?;
            println!("Using GitHub token: {}...", &token[..10]);
            token
        },
        "gitcode" => {
            let token = std::env::var("GITCODE_TOKEN")
                .map_err(|_| "GITCODE_TOKEN not set")?;
            println!("Using GitCode token: {}...", &token[..10]);
            token
        },
        _ => return Err("Unsupported platform".into()),
    };
    
    let url = format!(
        "{}/{}/{}/pulls/{}/commits",
        base_url, namespace, repo_name, pull_id
    );
    println!("Request URL: {}", url);

    let mut headers = HeaderMap::new();
    let auth_header = format!("Bearer {}", token);
    println!("Setting Authorization header: Bearer {}...", &token[..10]);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)?,
    );

    if platform == "github" {
        println!("Adding GitHub API version header");
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        
        println!("Adding User-Agent header");
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("akxhw1996"),
        );
    }

    println!("Making HTTP request...");
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url)
        .headers(headers)
        .send()?;
    
    let status = response.status();
    println!("Response status: {}", status);
    if !status.is_success() {
        let error_text = response.text()?;
        println!("Error response body: {}", error_text);
        return Err(format!("Request failed with status {}: {}", status, error_text).into());
    }

    println!("Parsing response body...");
    let commits: Vec<GitCommit> = response.json()?;
    println!("Found {} commits", commits.len());
    
    Ok(commits)
}

pub fn post_comment_on_pr(
    base_url: &str,
    namespace: &str,
    repo_name: &str,
    pull_id: u32,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Posting comment on PR:");
    println!("  Base URL: {}", base_url);
    println!("  Namespace: {}", namespace);
    println!("  Repo: {}", repo_name);
    println!("  PR ID: {}", pull_id);
    
    let token = std::env::var("GITCODE_TOKEN")
        .map_err(|_| "GITCODE_TOKEN not set")?;
    println!("Using GitCode token: {}...", &token[..10]);
    
    let url = format!(
        "{}/{}/{}/pulls/{}/comments",
        base_url, namespace, repo_name, pull_id
    );
    println!("Request URL: {}", url);

    let mut headers = HeaderMap::new();
    let auth_header = format!("Bearer {}", token);
    println!("Setting Authorization header: Bearer {}...", &token[..10]);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)?,
    );
    
    println!("Adding User-Agent header");
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("GitBot"),
    );

    let comment = CommentRequest {
        body: message.to_string(),
    };

    println!("Making HTTP request...");
    let client = reqwest::blocking::Client::new();
    let response = client.post(&url)
        .headers(headers)
        .json(&comment)
        .send()?;
    
    let status = response.status();
    println!("Response status: {}", status);
    if !status.is_success() {
        let error_text = response.text()?;
        println!("Error response body: {}", error_text);
        return Err(format!("Request failed with status {}: {}", status, error_text).into());
    }

    println!("Comment posted successfully");
    Ok(())
}
