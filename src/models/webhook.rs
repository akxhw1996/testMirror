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
