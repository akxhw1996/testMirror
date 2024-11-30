use crate::models::webhook::{
    WebhookPayload, ParsedWebhookData, Label, GitHubWebhookPayload,
    GitCodePushPayload, ParsedPushData
};
use serde_json;

pub fn parse_gitcode_pr_data(json_str: &str) -> Result<ParsedWebhookData, serde_json::Error> {
    // Parse the JSON string into our struct
    let payload: WebhookPayload = serde_json::from_str(json_str)?;
    
    // Extract labels with titles and descriptions if they exist, otherwise use empty vector
    let labels: Vec<Label> = payload.labels
        .map(|labels| labels.into_iter().map(|label| Label {
            title: label.title,
            description: label.description,
            r#type: None,
        }).collect())
        .unwrap_or_default();
    
    // Create the parsed data struct
    Ok(ParsedWebhookData {
        labels,
        event_type: payload.event_type,
        action: payload.object_attributes.as_ref().and_then(|attrs| attrs.action.clone()),
        state: payload.object_attributes.as_ref().and_then(|attrs| attrs.state.clone()),
        url: payload.object_attributes.as_ref().and_then(|attrs| attrs.url.clone()),
        repo_name: payload.repository.name,
        repo_url: payload.repository.git_http_url,
        namespace: payload.project.namespace,
        iid: payload.object_attributes.as_ref().and_then(|attrs| attrs.iid),
    })
}

pub fn parse_github_pr_data(json_str: &str) -> Result<ParsedWebhookData, serde_json::Error> {
    // Parse the JSON string into our GitHub-specific struct
    let payload: GitHubWebhookPayload = serde_json::from_str(json_str)?;
    
    // Extract labels with titles and descriptions
    let labels: Vec<Label> = payload.pull_request.labels
        .into_iter()
        .map(|label| Label {
            title: label.name,
            description: label.description,
            r#type: None,
        })
        .collect();
    
    // Split repository full_name to get namespace
    let namespace = payload.repository.full_name
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();
    
    // Create the parsed data struct
    Ok(ParsedWebhookData {
        labels,
        event_type: if payload.pull_request.url.is_some() { "pull_request".to_string() } else { "unknown".to_string() },
        action: payload.action,
        state: payload.pull_request.state,
        url: payload.pull_request.html_url,
        repo_name: payload.repository.name,
        repo_url: payload.repository.clone_url,
        namespace,
        iid: payload.pull_request.number,
    })
}

pub fn parse_gitcode_push_data(json_str: &str) -> Result<ParsedPushData, serde_json::Error> {
    // Parse the JSON string into our struct
    let payload: GitCodePushPayload = serde_json::from_str(json_str)?;
    
    // Create the parsed data struct
    Ok(ParsedPushData {
        user_name: payload.user_name,
        user_email: payload.user_email,
        commits: payload.commits,
        repo_name: payload.repository.name,
        project_name: payload.project.name,
        namespace: payload.project.namespace,
        branch: payload.git_branch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_gitcode_pr_data() {
        let json_str = r#"{
            "event_type": "merge_request",
            "object_attributes": {
                "state": "opened",
                "action": "open",
                "url": "https://gitcode.com/pr/123",
                "iid": 123
            },
            "repository": {
                "name": "test-repo",
                "git_http_url": "https://gitcode.com/test/test-repo.git"
            },
            "project": {
                "namespace": "test"
            },
            "labels": [
                {
                    "title": "bug",
                    "description": "feature/test-branch"
                }
            ]
        }"#;

        let result = parse_gitcode_pr_data(json_str).unwrap();
        assert_eq!(result.event_type, "merge_request");
        assert_eq!(result.action.unwrap(), "open");
        assert_eq!(result.state.unwrap(), "opened");
        assert_eq!(result.url.unwrap(), "https://gitcode.com/pr/123");
        assert_eq!(result.repo_name, "test-repo");
        assert_eq!(result.repo_url, "https://gitcode.com/test/test-repo.git");
        assert_eq!(result.namespace, "test");
        assert_eq!(result.iid.unwrap(), 123);
        assert_eq!(result.labels.len(), 1);
        assert_eq!(result.labels[0].title, "bug");
        assert_eq!(result.labels[0].description.as_ref().unwrap(), "feature/test-branch");
    }

    #[test]
    fn test_parse_github_pr_data() {
        let json_str = r#"{
            "action": "closed",
            "number": 1,
            "pull_request": {
                "url": "https://api.github.com/repos/test-org/test-repo/pulls/1",
                "id": 123456789,
                "node_id": "PR_test123",
                "html_url": "https://github.com/test-org/test-repo/pull/1",
                "state": "closed",
                "number": 1,
                "title": "Test pull request"
            },
            "repository": {
                "id": 987654321,
                "name": "test-repo",
                "full_name": "test-org/test-repo",
                "clone_url": "https://github.com/test-org/test-repo.git"
            },
            "labels": [
                {
                    "name": "type: feature",
                    "description": ""
                },
                {
                    "name": "version: 1.0",
                    "description": "version-1.0"
                },
                {
                    "name": "branch: main",
                    "description": "main"
                }
            ]
        }"#;

        let result = parse_github_pr_data(json_str).unwrap();
        println!("Parsed GitHub webhook data: {:#?}", result);

        assert_eq!(result.event_type, "pull_request");
        assert_eq!(result.action, Some("closed".to_string()));
        assert_eq!(result.state, Some("closed".to_string()));
        assert_eq!(result.url, Some("https://github.com/test-org/test-repo/pull/1".to_string()));
        assert_eq!(result.repo_name, "test-repo");
        assert_eq!(result.repo_url, "https://github.com/test-org/test-repo.git");
        assert_eq!(result.namespace, "test-org");
        assert_eq!(result.iid, Some(1));
        
        // Verify labels
        assert_eq!(result.labels.len(), 3);
        
        // Check first label
        assert_eq!(result.labels[0].title, "type: feature");
        assert_eq!(result.labels[0].description, Some("".to_string()));
        
        // Check second label
        assert_eq!(result.labels[1].title, "version: 1.0");
        assert_eq!(result.labels[1].description, Some("version-1.0".to_string()));
        
        // Check third label
        assert_eq!(result.labels[2].title, "branch: main");
        assert_eq!(result.labels[2].description, Some("main".to_string()));
    }

    #[test]
    fn test_parse_gitcode_push_data() {
        let json_str = r#"{
            "user_name": "test-user",
            "user_email": "test@example.com",
            "commits": [
                {
                    "id": "abcdef1234567890abcdef1234567890abcdef12",
                    "message": "test commit message",
                    "timestamp": "2024-01-01T00:00:00Z",
                    "url": "https://gitcode.com/test-org/test-repo/commits/detail/abcdef1234567890abcdef1234567890abcdef12",
                    "author": {
                        "name": "Test Author",
                        "email": "author@example.com"
                    }
                }
            ],
            "repository": {
                "name": "test-repo"
            },
            "project": {
                "name": "test-repo",
                "namespace": "test-org"
            },
            "git_branch": "test-branch"
        }"#;

        let result = parse_gitcode_push_data(json_str).unwrap();
        
        assert_eq!(result.user_name, "test-user");
        assert_eq!(result.user_email, "test@example.com");
        assert_eq!(result.repo_name, "test-repo");
        assert_eq!(result.project_name, "test-repo");
        assert_eq!(result.namespace, "test-org");
        assert_eq!(result.branch, "test-branch");
        
        // Verify commits
        assert_eq!(result.commits.len(), 1);
        let commit = &result.commits[0];
        assert_eq!(commit.id, "abcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(commit.message, "test commit message");
        assert_eq!(commit.author.name, "Test Author");
        assert_eq!(commit.author.email, "author@example.com");
    }
}