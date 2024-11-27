use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::post;
use crate::utils::{parser, git};

#[post("/test", format = "json", data = "<data>")]
pub async fn test(data: Json<Value>) -> &'static str {
    // Convert the JSON data to a string
    let json_str = serde_json::to_string(&data.into_inner()).unwrap();
    
    // Parse the webhook data using the parser function
    match parser::parse_pr_data(&json_str) {
        Ok(parsed_data) => {
            println!("Parsed Webhook Data:");
            println!("Event Type: {}", parsed_data.event_type);
            println!("Action: {}", parsed_data.action.as_deref().unwrap_or("None"));
            println!("State: {}", parsed_data.state.as_deref().unwrap_or("None"));
            println!("Repository Name: {}", parsed_data.repo_name);
            println!("Repository URL: {}", parsed_data.repo_url);
            println!("Labels:");
            for label in &parsed_data.labels {
                println!("  - {}", label.title);
            }

            // Check if this is a merge request
            if parsed_data.event_type == "merge_request" {
                // Spawn blocking operation in a separate thread
                match tokio::task::spawn_blocking(move || {
                    git::process_pr(&parsed_data)
                }).await {
                    Ok(Ok(_)) => println!("Successfully processed merge request"),
                    Ok(Err(e)) => println!("Error processing merge request: {}", e),
                    Err(e) => println!("Task join error: {}", e),
                }
            }
        },
        Err(e) => println!("Error parsing webhook data: {}", e),
    }
    "Webhook received"
}
