use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use crate::utils::request;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub author: GitAuthor,
    pub committer: GitAuthor,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitUser {
    pub id: String,
    pub login: String,
    pub name: String,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitParents {
    pub sha: String,
    pub shas: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCommit {
    pub sha: String,
    pub html_url: String,
    pub commit: GitCommitInfo,
    pub author: GitUser,
    pub committer: GitUser,
    pub parents: GitParents,
}

pub fn get_commit_list_of_pr(base_url: &str, namespace: &str, repo_name: &str, pull_id: u32) -> Result<Vec<GitCommit>, Box<dyn std::error::Error>> {
    let token = "MGEvR9x8753MAQtQwsJPmXxv";
    let url = format!(
        "{}/{}/{}/pulls/{}/commits",
        base_url, namespace, repo_name, pull_id
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    let response = request::send_request("GET", &url, None, Some(headers.into_iter()
        .filter_map(|(k, v)| {
            k.map(|key| (key.as_str().to_string(), v.to_str().unwrap().to_string()))
        })
        .collect()))?;
    let commits: Vec<GitCommit> = serde_json::from_str(&response)?;
    Ok(commits)
}
