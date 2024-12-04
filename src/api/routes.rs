use rocket::post;
use rocket::http::Status;   
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use rocket::data::{Data, ByteUnit};
use crate::utils::{hmac, parser, git};
use std::env;

const GITHUB_SIGNATURE_HEADER: &str = "X-Hub-Signature-256";
const GITCODE_SIGNATURE_HEADER: &str = "X-GitCode-Signature-256";
const GITHUB_EVENT_HEADER: &str = "X-GitHub-Event";
const GITCODE_EVENT_HEADER: &str = "X-GitCode-Event";

#[derive(Debug)]
pub struct HmacVerified {
    pub signature: String,
    pub event: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HmacVerified {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try both signature headers. abc
        let signature = request.headers().get_one(GITHUB_SIGNATURE_HEADER)
            .or_else(|| request.headers().get_one(GITCODE_SIGNATURE_HEADER));
            
        // Try both event headers
        let event = request.headers().get_one(GITHUB_EVENT_HEADER)
            .or_else(|| request.headers().get_one(GITCODE_EVENT_HEADER));

        match (signature, event) {
            (Some(sig), Some(evt)) => {
                if let Some(signature) = sig.strip_prefix("sha256=") {
                    Outcome::Success(HmacVerified {
                        signature: signature.to_string(),
                        event: evt.to_string(),
                    })
                } else {
                    println!("❌ Invalid signature format (missing sha256= prefix)");
                    Outcome::Forward(Status::BadRequest)
                }
            },
            (None, _) => {
                println!("❌ No signature header found (tried {} and {})", 
                    GITHUB_SIGNATURE_HEADER, GITCODE_SIGNATURE_HEADER);
                Outcome::Forward(Status::BadRequest)
            },
            (_, None) => {
                println!("❌ No event header found (tried {} and {})",
                    GITHUB_EVENT_HEADER, GITCODE_EVENT_HEADER);
                Outcome::Forward(Status::BadRequest)
            }
        }
    }
}

/// Verify the HMAC signature of a webhook request
fn verify_signature(body: &str, key: &str, expected_signature: &str) -> Result<(), &'static str> {
    let computed_signature = hmac::compute_hmac_sha256(body.as_bytes(), key);
    println!("Computed signature: {}", computed_signature);
    println!("Expected signature: {}", expected_signature);

    if computed_signature != expected_signature {
        println!("❌ Signature mismatch");
        return Err("Unauthorized");
    }

    println!("✅ Signature verification successful");
    Ok(())
}

/// Common webhook handling logic for pull/merge requests
async fn handle_pr_webhook(
    body: Data<'_>, 
    hmac_verified: &HmacVerified, 
    env_key: &str,
    platform: &str
) -> Result<String, &'static str> {
    // Read the request body
    let body_str = match body.open(ByteUnit::Mebibyte(1)).into_string().await {
        Ok(s) => s.into_inner(),
        Err(e) => {
            println!("Failed to read request body: {}", e);
            return Err("Internal Server Error");
        }
    };

    // Get the key from environment variable
    let key = match env::var(env_key) {
        Ok(k) => k,
        Err(e) => {
            println!("Failed to get webhook key: {}", e);
            return Err("Internal Server Error");
        }
    };

    // Verify HMAC signature
    verify_signature(&body_str, &key, &hmac_verified.signature)?;

    // Parse the webhook data using the parser function
    match if platform == "github" {
        parser::parse_github_pr_data(&body_str)
    } else if platform == "gitcode" {
        parser::parse_gitcode_pr_data(&body_str)
    } else {
        return Err("Unsupported platform");
    } {
        Ok(parsed_data) => {
            println!("Parsed Webhook Data:\n{}", parsed_data.to_string());

            // Check if this is a merge request
            let event_type = match platform {
                "github" => "pull_request",
                "gitcode" => "merge_request",
                _ => return Err("Unsupported platform"),
            };
            
            if parsed_data.event_type == event_type {
                // Spawn blocking operation in a separate thread
                match platform {
                    "github" => {
                        match tokio::task::spawn_blocking(move || {
                            git::process_github_pr(&parsed_data)
                        }).await {
                            Ok(Ok(_)) => println!("Successfully processed GitHub pull request"),
                            Ok(Err(e)) => {
                                println!("Error processing GitHub pull request: {}", e);
                                return Err("Internal Server Error");
                            },
                            Err(e) => {
                                println!("Task join error: {}", e);
                                return Err("Internal Server Error");
                            },
                        }
                    },
                    "gitcode" => {
                        match tokio::task::spawn_blocking(move || {
                            git::process_pr(&parsed_data)
                        }).await {
                            Ok(Ok(_)) => println!("Successfully processed GitCode merge request"),
                            Ok(Err(e)) => {
                                println!("Error processing GitCode merge request: {}", e);
                                return Err("Internal Server Error");
                            },
                            Err(e) => {
                                println!("Task join error: {}", e);
                                return Err("Internal Server Error");
                            },
                        }
                    },
                    _ => return Err("Unsupported platform"),
                }
            }
            Ok(body_str)
        },
        Err(e) => {
            println!("Error parsing webhook data: {}", e);
            Err("Internal Server Error")
        },
    }
}

/// Handle push event webhook
async fn handle_push_webhook(
    body: Data<'_>,
    hmac_verified: &HmacVerified,
    env_key: &str,
) -> Result<String, &'static str> {
    // Read the request body
    let body_str = match body.open(ByteUnit::Mebibyte(1)).into_string().await {
        Ok(s) => s.into_inner(),
        Err(e) => {
            println!("Failed to read request body: {}", e);
            return Err("Internal Server Error");
        }
    };

    // Get the key from environment variable
    let key = match env::var(env_key) {
        Ok(k) => k,
        Err(e) => {
            println!("Failed to get webhook key: {}", e);
            return Err("Internal Server Error");
        }
    };

    // Verify HMAC signature
    verify_signature(&body_str, &key, &hmac_verified.signature)?;

    // Parse the push event data
    match parser::parse_gitcode_push_data(&body_str) {
        Ok(push_data) => {
            println!("=== Handle Push Webhook Debug ===");
            println!("Webhook Event Type: {}", hmac_verified.event);
            println!("Push Data Details:");
            println!("- Repository: {}/{}", push_data.namespace, push_data.repo_name);
            println!("- User: {}", push_data.user_name);
            println!("- Commit Count: {}", push_data.commits.len());
            println!("================================");

            // Spawn blocking operation in a separate thread
            match tokio::task::spawn_blocking(move || {
                println!("Starting push event processing in spawned thread");
                let result = git::process_push_event(&push_data);
                println!("Push event processing result: {:?}", result);
                result
            }).await {
                Ok(Ok(_)) => {
                    println!("Successfully processed push event");
                    Ok(body_str)
                },
                Ok(Err(e)) => {
                    println!("Error processing push event: {}", e);
                    Err("Internal Server Error")
                },
                Err(e) => {
                    println!("Task join error: {}", e);
                    Err("Internal Server Error")
                },
            }
        },
        Err(e) => {
            println!("Error parsing push data: {}", e);
            Err("Internal Server Error")
        },
    }
}

#[post("/github", data = "<body>")]
pub async fn github_handle(body: Data<'_>, hmac_verified: HmacVerified) -> &'static str {
    match handle_pr_webhook(body, &hmac_verified, "GITHUB_WEBHOOK_VERIFYING_KEY", "github").await {
        Ok(_) => "Webhook received",
        Err(e) => e,
    }
}

#[post("/gitcode", data = "<body>")]
pub async fn gitcode_handle(body: Data<'_>, hmac_verified: HmacVerified) -> &'static str {
    println!("=== GitCode Webhook Handler ===");
    println!("Received event type: {}", hmac_verified.event);

    let result = match hmac_verified.event.as_str() {
        "Push Hook" => {
            println!("Processing push event");
            handle_push_webhook(body, &hmac_verified, "GITCODE_WEBHOOK_VERIFYING_KEY").await
        },
        "Merge Request Hook" => {
            println!("Processing merge request event");
            handle_pr_webhook(body, &hmac_verified, "GITCODE_WEBHOOK_VERIFYING_KEY", "gitcode").await
        },
        _ => {
            println!("Unsupported GitCode event type: {}", hmac_verified.event);
            Err("Unsupported event type")
        }
    };

    match result {
        Ok(_) => {
            println!("Successfully processed GitCode webhook");
            "Webhook received"
        },
        Err(e) => {
            println!("Error processing GitCode webhook: {}", e);
            e
        }
    }
}
