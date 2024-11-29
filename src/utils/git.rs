use std::path::PathBuf;
use git2::{Repository, RemoteCallbacks, PushOptions};
use std::env;

use crate::models::webhook::{ParsedWebhookData, Label, ParsedPushData};
use crate::utils::{file, gitcode};

pub fn clone_repository(repo_url: &str, local_path: &PathBuf, platform: &str) -> Result<Repository, git2::Error> {
    println!("Starting repository clone:");
    println!("  URL: {}", repo_url);
    println!("  Local path: {:?}", local_path);
    println!("  Platform: {}", platform);

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(match platform {
        "github" => {
            println!("Using GitHub credentials");
            github_credentials_callback
        },
        "gitcode" => {
            println!("Using GitCode credentials");
            gitcode_credentials_callback
        },
        _ => return Err(git2::Error::from_str("Unsupported platform")),
    });

    println!("Setting up fetch options");
    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    println!("Configuring repository builder");
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    println!("Starting clone operation");
    let repo = builder.clone(repo_url, local_path)?;
    println!("Repository cloned successfully");

    Ok(repo)
}

pub fn process_pr(webhook_data: &ParsedWebhookData) -> Result<String, git2::Error> {
    // Check if action is "merge" and state is "merged"
    match (&webhook_data.action, &webhook_data.state) {
        (Some(action), Some(state)) if action == "close" && state == "closed" => {
            // Check if the label in webhook_data contains a label with title "approval: ready"
            if !webhook_data.labels.iter().any(|label| label.title == "approval: ready") {
                return Ok("PR is closed but doesn't have approval: ready label".to_string());
            }

            let br_labels: Vec<&Label> = webhook_data.labels.iter()
                .filter(|label| label.title.starts_with("br:"))
                .collect();

            if br_labels.is_empty() {
                return Ok("No branch labels found".to_string());
            }

            // Get current directory and append repo name
            let current_dir = std::env::current_dir()
                .map_err(|e| git2::Error::from_str(&e.to_string()))?;
            let local_path = current_dir.join("gitcode").join(&webhook_data.repo_name);

            // Create a new folder at local_path, deleting existing one if present
            file::create_empty_folder(&local_path)
                .map_err(|e| git2::Error::from_str(&format!("Failed to prepare directory: {}", e)))?;

            // Clone the repository
            let repo = clone_repository(&webhook_data.repo_url, &local_path, "gitcode")?;
            
            // Set up Git configuration for the repository
            let mut config = repo.config()?;
            let username = env::var("GITCODE_USERNAME").expect("GITCODE_USERNAME not set in environment");
            let user_email = env::var("GITCODE_USER_EMAIL").expect("GITCODE_USER_EMAIL not set in environment");
            config.set_str("user.name", &username)?;
            config.set_str("user.email", &user_email)?;
            println!("Repository Git configuration set up successfully");
            
            let iid: u32 = webhook_data.iid.unwrap();
            // Get the commit list for the PR
            let commits = match gitcode::get_commit_list_of_pr(
                "https://api.gitcode.com/api/v5/repos",
                &webhook_data.namespace,
                &webhook_data.repo_name,
                iid,
                "gitcode"
            ) {
                Ok(commits) => commits,
                Err(e) => return Err(git2::Error::from_str(&e.to_string())),
            };

            let _result = fetch_merge_request(&local_path, "origin", iid, "gitcode");

            for br_label in br_labels {
                let branch_name = br_label.description.as_ref().unwrap();
                switch_branch(&local_path, &branch_name)?;
                println!("Switching to branch {}", &branch_name);
                
                for commit in commits.iter().rev() {
                    let url = webhook_data.url.as_deref().unwrap_or("unknown");
                    cherry_pick_commit(&local_path, &commit.sha, &branch_name, url)?;
                }
                // Push the changes back to origin
                push_repository(&local_path, "origin", &branch_name)?;
            }

            // Clean up the local repository
            if let Err(e) = file::delete_folder(&local_path) {
                return Err(git2::Error::from_str(&format!("Failed to cleanup repository: {}", e)));
            }

            Ok("Successfully processed PR".to_string())
        }
        _ => Ok("PR is not closed or merged".to_string()),
    }
}

pub fn process_github_pr(webhook_data: &ParsedWebhookData) -> Result<String, git2::Error> {
    println!("Starting GitHub PR processing");
    println!("Webhook data: {:?}", webhook_data);
    
    // Check if action is "merge" and state is "merged"
    match (&webhook_data.action, &webhook_data.state) {
        (Some(action), Some(state)) if action == "closed" && state == "closed" => {
            println!("PR is closed, checking labels");
            
            // Check if the label in webhook_data contains a label with title "approval: ready"
            if !webhook_data.labels.iter().any(|label| label.title == "approval: ready") {
                println!("PR doesn't have approval: ready label");
                return Ok("PR is closed but doesn't have approval: ready label".to_string());
            }
            println!("Found approval: ready label");

            let br_labels: Vec<&Label> = webhook_data.labels.iter()
                .filter(|label| label.title.starts_with("br:"))
                .collect();
            println!("Found {} branch labels: {:?}", br_labels.len(), br_labels);

            if br_labels.is_empty() {
                println!("No branch labels found");
                return Ok("No branch labels found".to_string());
            }

            // Get current directory and append repo name
            let current_dir = std::env::current_dir()
                .map_err(|e| git2::Error::from_str(&e.to_string()))?;
            let local_path = current_dir.join("github").join(&webhook_data.repo_name);

            // Create a new folder at local_path, deleting existing one if present
            file::create_empty_folder(&local_path)
                .map_err(|e| git2::Error::from_str(&format!("Failed to prepare directory: {}", e)))?;

            // Clone the repository
            println!("Cloning repository from URL: {}", webhook_data.repo_url);
            let repo = clone_repository(&webhook_data.repo_url, &local_path, "github")?;
            println!("Repository cloned successfully");
            
            // Set up Git configuration for the repository
            println!("Setting up Git configuration");
            let mut config = repo.config()?;
            let username = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME not set in environment");
            let user_email = env::var("GITHUB_USER_EMAIL").expect("GITHUB_USER_EMAIL not set in environment");
            config.set_str("user.name", &username)?;
            config.set_str("user.email", &user_email)?;
            println!("Repository Git configuration set up successfully");
            
            let iid: u32 = webhook_data.iid.unwrap();
            println!("Processing PR #{}", iid);
            
            // Get the commit list for the PR
            println!("Fetching commit list from GitHub API");
            let commits = match gitcode::get_commit_list_of_pr(
                "https://api.github.com/repos",
                &webhook_data.namespace,
                &webhook_data.repo_name,
                iid,
                "github"
            ) {
                Ok(commits) => commits,
                Err(e) => return Err(git2::Error::from_str(&e.to_string())),
            };

            println!("Fetching merge request");
            let result = fetch_merge_request(&local_path, "origin", iid, "github");
            if let Err(e) = result {
                println!("Failed to fetch merge request: {}", e);
                return Err(git2::Error::from_str(&format!("Failed to fetch merge request: {}", e)));
            }
            println!("Merge request fetched successfully");
            
            println!("Adding target remote repository");
            match add_remote_repository(&local_path, "target", "https://gitcode.com/openHiTLS/openhitls-auto-cherry-test.git") {
                Ok(_) => println!("Target remote added successfully"),
                Err(e) => {
                    println!("Failed to add remote repository: {}", e);
                    return Err(git2::Error::from_str(&format!("Failed to add remote repository: {}", e)));
                }
            }
            
            for br_label in br_labels {
                let branch_name = br_label.description.as_ref().unwrap();
                println!("Processing branch: {}", branch_name);
                
                switch_branch(&local_path, &branch_name)?;
                println!("Switched to branch {}", &branch_name);
                
                println!("Cherry-picking commits");
                for commit in commits.iter().rev() {
                    println!("Cherry-picking commit: {}", commit.sha);
                    let url = webhook_data.url.as_deref().unwrap_or("unknown");
                    cherry_pick_commit(&local_path, &commit.sha, &branch_name, url)?;
                }
                
                println!("Pushing changes to target remote");
                push_repository(&local_path, "target", &branch_name)?;
                println!("Successfully pushed to branch {}", branch_name);
            }

            println!("Cleaning up repository");
            if let Err(e) = file::delete_folder(&local_path) {
                println!("Failed to cleanup repository: {}", e);
                return Err(git2::Error::from_str(&format!("Failed to cleanup repository: {}", e)));
            }
            println!("Repository cleanup successful");

            Ok("Successfully processed PR".to_string())
        }
        _ => {
            println!("PR is not closed or merged. Action: {:?}, State: {:?}", 
                    webhook_data.action, webhook_data.state);
            Ok("PR is not closed or merged".to_string())
        }
    }
}

pub fn process_push_event(push_data: &ParsedPushData) -> Result<String, git2::Error> {
    println!("=== Process Push Event Debug ===");
    println!("Processing push event for repository: {}/{}", push_data.namespace, push_data.repo_name);

    // Check if the user_name matches GITCODE_BOT_USERNAME
    let bot_username = match env::var("GITCODE_BOT_USERNAME") {
        Ok(username) => {
            println!("Bot username from env: {}", username);
            username
        },
        Err(e) => {
            println!("Failed to get bot username: {}", e);
            return Err(git2::Error::from_str(&e.to_string()));
        }
    };

    if push_data.user_name != bot_username {
        println!("Skipping: User {} is not bot {}", push_data.user_name, bot_username);
        return Ok("User is not bot, skipping".to_string());
    }
    println!("Verified: Push is from bot user");

    // Get comment info from the push data
    let comments = push_data.get_comment_info();
    println!("Found {} comments to process", comments.len());

    // Post each comment on the corresponding PR
    for (index, comment) in comments.iter().enumerate() {
        println!("Processing comment {}/{}", index + 1, comments.len());
        if let Some(pr_id) = comment.pr_id {
            println!("Posting comment to PR #{}", pr_id);
            match gitcode::post_comment_on_pr(
                "https://api.gitcode.com/api/v5/repos",
                &push_data.namespace,
                &push_data.repo_name,
                pr_id,
                &comment.message,
            ) {
                Ok(_) => println!("Successfully posted comment to PR #{}", pr_id),
                Err(e) => {
                    println!("Failed to post comment to PR #{}: {}", pr_id, e);
                    return Err(git2::Error::from_str(&e.to_string()));
                }
            }
        }
    }

    println!("=== Push Event Processing Complete ===");
    Ok("Successfully processed push event".to_string())
}

pub fn push_repository(
    repo_path: &PathBuf,
    remote_name: &str,
    branch: &str,
) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote(remote_name)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(gitcode_credentials_callback);

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // Ensure we're pushing to the correct refspec
    let refspec = format!("+refs/heads/{}:refs/heads/{}", branch, branch);
    remote.push(&[&refspec], Some(&mut push_options))?;

    Ok(())
}

pub fn gitcode_credentials_callback(
    _user: &str,
    _user_from_url: Option<&str>,
    _cred: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    let username = env::var("GITCODE_USERNAME").expect("GITCODE_USERNAME not set in environment");
    let token = env::var("GITCODE_TOKEN").expect("GITCODE_TOKEN not set in environment");
    // For HTTP(S) URLs, we need to provide the username and token as password
    git2::Cred::userpass_plaintext(&username, &token)
}

pub fn github_credentials_callback(
    _user: &str,
    _user_from_url: Option<&str>,
    _cred: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    println!("GitHub credentials callback triggered");
    let username = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME not set in environment");
    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment");
    println!("Using GitHub credentials for user: {}", username);
    // For GitHub, we use the token as the password
    git2::Cred::userpass_plaintext(&username, &token)
}

pub fn switch_branch(repo_path: &PathBuf, branch_name: &str) -> Result<(), git2::Error> {
    // Open the repository at the given path
    let repo = Repository::open(repo_path)?;

    // Try to find local branch first
    let local_branch = repo.find_branch(branch_name, git2::BranchType::Local);

    match local_branch {
        Ok(branch) => {
            // Local branch exists, just check it out
            let obj = branch.get().peel_to_commit()?.into_object();
            repo.checkout_tree(&obj, None)?;
            repo.set_head(branch.get().name().unwrap())?;
        },
        Err(_) => {
            // Local branch doesn't exist, try to find it in remote
            let _remote = repo.find_remote("origin")?;
            let remote_branch_name = format!("origin/{}", branch_name);

            // Find the remote branch reference
            let remote_ref = repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))?;
            let remote_commit = remote_ref.peel_to_commit()?;

            // Create a new local branch that tracks the remote branch
            repo.branch(branch_name, &remote_commit, false)?;

            // Set up tracking relationship
            let mut local_branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
            local_branch.set_upstream(Some(&remote_branch_name))?;

            // Checkout the new branch
            let obj = remote_commit.into_object();
            repo.checkout_tree(&obj, None)?;
            repo.set_head(&format!("refs/heads/{}", branch_name))?;
        }
    }

    Ok(())
}

pub fn cherry_pick_commit(repo_path: &PathBuf, commit_id: &str, _branch_name: &str, pr_url: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    println!("Repository opened successfully");

    // Find the commit to cherry-pick
    let commit = repo.find_commit(repo.revparse_single(commit_id)?.id())?;
    println!("Found commit to cherry-pick: {}", commit_id);

    // Get the tree of the commit
    let tree = commit.tree()?;

    // Get the current HEAD as parent
    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    // Create the new commit with original author and committer information
    let author = commit.author();
    let committer = repo.signature()?;
    let message = commit.message().unwrap_or("").to_owned() + "\n\nCherry-picked from: " + pr_url;

    // Create the cherry-picked commit
    repo.commit(
        Some("HEAD"),
        &author,
        &committer,
        &message,
        &tree,
        &[&parent_commit]
    )?;

    println!("Cherry-pick completed successfully");
    Ok(())
}

pub fn fetch_merge_request(repo_path: &PathBuf, remote_name: &str, iid: u32, platform: &str) -> Result<(), git2::Error> {
    println!("Fetching merge request - Path: {:?}, Remote: {}, PR: {}", repo_path, remote_name, iid);
    let repo = Repository::open(repo_path)?;
    println!("Repository opened successfully");
    let mut remote = repo.find_remote(remote_name)?;
    println!("Found remote: {}", remote_name);

    // Set up callbacks for authentication
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(match platform {
        "github" => github_credentials_callback,
        "gitcode" => gitcode_credentials_callback,
        _ => return Err(git2::Error::from_str("Unsupported platform")),
    });
    println!("Set up authentication callbacks");

    // Configure fetch options
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Create the refspec based on platform
    let refspec = match platform {
        "github" => format!("pull/{}/head:refs/remotes/{}/pr/{}", iid, remote_name, iid),
        "gitcode" => format!("+refs/merge-requests/{}/head:refs/remotes/{}/mr/{}", iid, remote_name, iid),
        _ => return Err(git2::Error::from_str("Unsupported platform")),
    };
    println!("Created refspec: {}", refspec);

    // Fetch the specific merge request/pull request
    println!("Starting fetch operation...");
    remote.fetch(
        &[&refspec],
        Some(&mut fetch_opts),
        None
    )?;
    println!("Fetch completed successfully");

    Ok(())
}


pub fn add_remote_repository(
    repo_path: &PathBuf,
    remote_name: &str,
    remote_url: &str,
) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    
    // Check if remote already exists
    if let Ok(_) = repo.find_remote(remote_name) {
        // If it exists, remove it first
        repo.remote_delete(remote_name)?;
    }
    
    // Add the new remote
    repo.remote(remote_name, remote_url)?;
    println!("Added remote '{}' with URL: {}", remote_name, remote_url);
    
    Ok(())
}

pub fn parse_branch_label(labels: &[Label]) -> Vec<Label> {
    labels.iter()
        .filter(|label| label.title.starts_with("br: "))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branch_label() {
        let label = Label {
            title: "br: main".to_string(),
            description: Some("main".to_string()),
            color: None,
            created_at: None,
            expires_at: None,
            group_id: None,
            id: None,
            project_id: None,
            template: None,
            r#type: None,
            updated_at: None,
        };
        let labels = vec![label];
        let branch_labels = parse_branch_label(&labels);
        assert_eq!(branch_labels.len(), 1);
        assert_eq!(branch_labels[0].title, "br: main");
        assert_eq!(branch_labels[0].description.as_ref().unwrap(), "main");
    }
}