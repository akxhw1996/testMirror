use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub color: Option<String>,
    pub created_at: Option<String>,
    pub description: Option<String>,
    pub expires_at: Option<String>,
    pub group_id: Option<i64>,
    pub id: Option<i64>,
    pub project_id: Option<i64>,
    pub template: Option<bool>,
    pub title: String,
    pub r#type: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectAttributes {
    pub state: Option<String>,
    pub action: Option<String>,
    pub url: Option<String>,
    pub iid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub git_http_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    #[serde(default = "default_event_type")]
    pub event_type: String,
    pub object_attributes: Option<ObjectAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
    pub repository: Repository,
    pub project: Project,
}

pub fn default_event_type() -> String {
    "unknown".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub url: Option<String>,
    pub state: Option<String>,
    pub number: Option<u32>,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    pub html_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub name: String,
    pub clone_url: String,
    pub full_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubWebhookPayload {
    pub action: Option<String>,
    pub pull_request: GitHubPullRequest,
    pub repository: GitHubRepository,
}

#[derive(Debug)]
pub struct ParsedWebhookData {
    pub labels: Vec<Label>,
    pub event_type: String,
    pub action: Option<String>,
    pub state: Option<String>,
    pub url: Option<String>,
    pub repo_name: String,
    pub repo_url: String,
    pub namespace: String,
    pub iid: Option<u32>,
}

impl ToString for ParsedWebhookData {
    fn to_string(&self) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Event Type: {}\n", self.event_type));
        if let Some(action) = &self.action {
            output.push_str(&format!("Action: {}\n", action));
        }
        if let Some(state) = &self.state {
            output.push_str(&format!("State: {}\n", state));
        }
        output.push_str(&format!("Repository Name: {}\n", self.repo_name));
        output.push_str(&format!("Repository URL: {}\n", self.repo_url));
        output.push_str(&format!("Namespace: {}\n", self.namespace));
        if let Some(iid) = self.iid {
            output.push_str(&format!("IID: {}\n", iid));
        }
        if !self.labels.is_empty() {
            output.push_str("Labels:\n");
            for label in &self.labels {
                output.push_str(&format!("  - {}\n", label.title));
            }
        }
        
        output
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCodeAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCodeCommit {
    pub id: String,
    pub message: String,
    pub timestamp: String,
    pub url: String,
    pub author: GitCodeAuthor,
}

impl GitCodeCommit {
    pub fn get_cherry_pick_url(&self) -> Option<String> {
        const CHERRY_PICK_MARKER: &str = "Cherry-picked from: ";
        
        // Find the marker in the message
        self.message
            .find(CHERRY_PICK_MARKER)
            .map(|start_idx| {
                // Get the substring starting after the marker
                let url_start = start_idx + CHERRY_PICK_MARKER.len();
                self.message[url_start..].trim().to_string()
            })
    }

    pub fn get_original_pr_number(&self) -> Option<u32> {
        self.get_cherry_pick_url().and_then(|url| {
            url.split('/')
                .last()
                .and_then(|num_str| num_str.parse::<u32>().ok())
        })
    }
}

#[derive(Debug)]
pub struct CommentInfo {
    pub message: String,
    pub pr_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCodePushProject {
    pub name: String,
    pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCodePushRepository {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCodePushPayload {
    pub user_name: String,
    pub user_email: String,
    pub commits: Vec<GitCodeCommit>,
    pub repository: GitCodePushRepository,
    pub project: GitCodePushProject,
    pub git_branch: String,
}

#[derive(Debug)]
pub struct ParsedPushData {
    pub user_name: String,
    pub user_email: String,
    pub commits: Vec<GitCodeCommit>,
    pub repo_name: String,
    pub project_name: String,
    pub namespace: String,
    pub branch: String,
}

impl ToString for ParsedPushData {
    fn to_string(&self) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("User: {} <{}>\n", self.user_name, self.user_email));
        output.push_str(&format!("Repository: {}\n", self.repo_name));
        output.push_str(&format!("Project: {}\n", self.project_name));
        output.push_str(&format!("Namespace: {}\n", self.namespace));
        output.push_str(&format!("Branch: {}\n", self.branch));
        output.push_str("Commits:\n");
        for commit in &self.commits {
            output.push_str(&format!("  - {} by {} <{}>\n    {}\n", 
                commit.id,
                commit.author.name,
                commit.author.email,
                commit.message.lines().next().unwrap_or("")
            ));
        }
        
        output
    }
}

impl ParsedPushData {
    pub fn get_comment_info(&self) -> Vec<CommentInfo> {
        self.commits
            .iter()
            .filter_map(|commit| {
                commit.get_cherry_pick_url().map(|_| {
                    let commit_id = &commit.id[..8];
                    CommentInfo {
                        message: format!(
                            "**{}** pushed a commit on branch {} that referenced this pull request: [{}]({})",
                            self.user_name, self.branch, commit_id, format!("{}?ref={}", commit.url, self.branch)
                        ),
                        pr_id: commit.get_original_pr_number(),
                    }
                })
            })
            .collect()
    }
}
