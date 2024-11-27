use crate::models::webhook::{WebhookPayload, ParsedWebhookData, Label};
use serde_json;

pub fn parse_pr_data(json_str: &str) -> Result<ParsedWebhookData, serde_json::Error> {
    // Parse the JSON string into our struct
    let payload: WebhookPayload = serde_json::from_str(json_str)?;
    
    // Extract labels with titles and descriptions if they exist, otherwise use empty vector
    let labels: Vec<Label> = payload.labels
        .map(|labels| labels.into_iter().map(|label| Label {
            title: label.title,
            description: label.description,
            color: None,
            created_at: None,
            expires_at: None,
            group_id: None,
            id: None,
            project_id: None,
            template: None,
            r#type: None,
            updated_at: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_pr_data() {
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

        let result = parse_pr_data(json_str).unwrap();
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
}