use git2::{Repository, RemoteCallbacks};
use std::{env, path::PathBuf};
use crate::utils::git::{github_credentials_callback, gitcode_credentials_callback};

pub fn clone_bare_repository(repo_url: &str, local_path: &PathBuf, platform: &str) -> Result<Repository, git2::Error> {
    println!("Starting bare repository clone:");
    println!("URL: {}", repo_url);
    println!("Local path: {:?}", local_path);
    
    // Set up fetch options with authentication
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(match platform {
        "github" => github_credentials_callback,
        "gitcode" => gitcode_credentials_callback,
        _ => return Err(git2::Error::from_str("Unsupported platform")),
    });
    
    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    
    // Set up builder for bare clone
    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(fetch_options);
    
    // Perform the clone operation
    match builder.clone(repo_url, local_path) {
        Ok(repo) => {
            println!("Bare repository cloned successfully");
            Ok(repo)
        },
        Err(e) => {
            println!("Failed to clone repository: {}", e);
            Err(e)
        }
    }
}

pub fn push_mirror(repo_path: &PathBuf, remote_url: &str, platform: &str) -> Result<(), git2::Error> {
    println!("Starting mirror push to {}", remote_url);
    
    // Construct the authenticated URL
    let remote_url_with_auth = match platform {
        "github" => {
            if let (Ok(token), Ok(username)) = (env::var("GITHUB_TOKEN"), env::var("GITHUB_USERNAME")) {
                println!("GitHub token is available and starts with: {}...", &token[..10]);
                println!("GitHub username is set to: {}", username);
                remote_url.replace("https://", &format!("https://{}:{}@", username, token))
            } else {
                println!("WARNING: GitHub credentials not found in environment");
                remote_url.to_string()
            }
        },
        "gitcode" => {
            if let (Ok(token), Ok(username)) = (env::var("GITCODE_TOKEN"), env::var("GITCODE_USERNAME")) {
                println!("GitCode token is available and starts with: {}...", &token[..10]);
                println!("GitCode username is set to: {}", username);
                remote_url.replace("https://", &format!("https://{}:{}@", username, token))
            } else {
                println!("WARNING: GitCode credentials not found in environment");
                remote_url.to_string()
            }
        },
        _ => {
            println!("WARNING: Unknown platform {}", platform);
            remote_url.to_string()
        }
    };

    // Set up git configuration
    if let (Ok(username), Ok(email)) = (
        env::var(format!("{}_USERNAME", platform.to_uppercase())),
        env::var(format!("{}_USER_EMAIL", platform.to_uppercase()))
    ) {
        println!("Setting git config user.name={} user.email={}", username, email);
        let _ = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", &username])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", &email])
            .output();
    }

    // Push mirror directly to URL with verbose output
    println!("Pushing with mirror...");
    let push_result = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["-c", "http.sslVerify=false", "push", "-v", "--mirror", &remote_url_with_auth])
        .output()
        .map_err(|e| git2::Error::from_str(&format!("Failed to push: {}", e)))?;

    println!("Push stdout: {}", String::from_utf8_lossy(&push_result.stdout));
    println!("Push stderr: {}", String::from_utf8_lossy(&push_result.stderr));

    if !push_result.status.success() {
        return Err(git2::Error::from_str(&format!(
            "Failed to push: {}",
            String::from_utf8_lossy(&push_result.stderr)
        )));
    }

    println!("Push completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use dotenv::dotenv;

    #[test]
    fn test_mirror_operations() {
        dotenv().ok();
        
        // Set up test environment variables
        std::env::set_var("GITHUB_USER_EMAIL", "akxhw1996@gmail.com");
        std::env::set_var("GITCODE_USER_EMAIL", "tmtmtt@gitcode.com");
        std::env::set_var("GITHUB_USERNAME", "akxhw1996");
        std::env::set_var("GITCODE_USERNAME", "tmtmtt");
        // Decrypt and print tokens for debugging
        if let Ok(token) = env::var("GITCODE_TOKEN") {
            println!("Decrypted token for GITCODE_TOKEN: {}...", &token[..10]);
        }
        if let Ok(token) = env::var("GITHUB_TOKEN") {
            println!("Decrypted token for GITHUB_TOKEN: {}...", &token[..10]);
        }

        // Test bare clone from GitCode
        println!("Testing bare clone from GitCode...");
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let source_url = "https://gitcode.com/openHiTLS/openhitls-auto-cherry-test.git";
        let dest_url = "https://github.com/akxhw1996/testMirror.git";
        
        // Clone the source repository
        let repo = match clone_bare_repository(source_url, &temp_path, "gitcode") {
            Ok(repo) => repo,
            Err(e) => {
                println!("Failed to clone repository: {:?}", e);
                panic!("Failed to clone repository");
            }
        };
    
        // Set up git configuration for the test repository
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        
        // Test mirror push to GitHub
        println!("Testing mirror push to GitHub...");
        match push_mirror(&temp_path, dest_url, "github") {
            Ok(_) => (),
            Err(e) => {
                println!("Push failed with error: {:?}", e);
                println!("Error class: {:?}, code: {:?}", e.class(), e.code());
                println!("Error message: {}", e.message());
                panic!("Failed to push mirror");
            }
        }
    }
}