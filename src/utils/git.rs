use std::path::PathBuf;
use git2::{Repository, RemoteCallbacks, PushOptions};
use std::env;

use crate::models::webhook::{ParsedWebhookData, Label};
use crate::utils::{file, gitcode};

pub fn clone_repository(repo_url: &str, local_path: &PathBuf) -> Result<Repository, git2::Error> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(git_credentials_callback);

    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    builder.clone(repo_url, local_path)
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
            let local_path = current_dir.join(&webhook_data.repo_name);

            // Create a new folder at local_path, deleting existing one if present
            file::create_empty_folder(&local_path)
                .map_err(|e| git2::Error::from_str(&format!("Failed to prepare directory: {}", e)))?;

            // Clone the repository
            let repo = clone_repository(&webhook_data.repo_url, &local_path)?;
            
            // Set up Git configuration for the repository
            let mut config = repo.config()?;
            let username = env::var("GIT_USERNAME").expect("GIT_USERNAME not set in environment");
            let user_email = env::var("GIT_USER_EMAIL").expect("GIT_USER_EMAIL not set in environment");
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
            ) {
                Ok(commits) => commits,
                Err(e) => return Err(git2::Error::from_str(&e.to_string())),
            };

            let _result = fetch_merge_request(&local_path, "origin", iid);

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

pub fn push_repository(
    repo_path: &PathBuf,
    remote_name: &str,
    branch: &str,
) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote(remote_name)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(git_credentials_callback);

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // Ensure we're pushing to the correct refspec
    let refspec = format!("+refs/heads/{}:refs/heads/{}", branch, branch);
    remote.push(&[&refspec], Some(&mut push_options))?;

    Ok(())
}

pub fn git_credentials_callback(
    _user: &str,
    _user_from_url: Option<&str>,
    _cred: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    let username = env::var("GIT_USERNAME").expect("GIT_USERNAME not set in environment");
    let token = env::var("GIT_TOKEN").expect("GIT_TOKEN not set in environment");
    // For HTTP(S) URLs, we need to provide the username and token as password
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

pub fn fetch_merge_request(repo_path: &PathBuf, remote_name: &str, iid: u32) -> Result<(), git2::Error> {
    println!("Fetching merge request - Path: {:?}, Remote: {}, PR: {}", repo_path, remote_name, iid);
    let repo = Repository::open(repo_path)?;
    println!("Repository opened successfully");
    let mut remote = repo.find_remote(remote_name)?;
    println!("Found remote: {}", remote_name);

    // Set up callbacks for authentication
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(git_credentials_callback);
    println!("Set up authentication callbacks");

    // Configure fetch options
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Create the refspec for the specific PR
    let refspec = format!("+refs/merge-requests/{}/head:refs/remotes/{}/mr/{}", 
        iid, remote_name, iid);
    println!("Created refspec: {}", refspec);

    // Fetch the specific merge request
    println!("Starting fetch operation...");
    remote.fetch(
        &[&refspec],
        Some(&mut fetch_opts),
        None
    )?;
    println!("Fetch completed successfully");

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