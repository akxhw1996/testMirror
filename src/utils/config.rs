use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoConfig {
    pub target_repo: String,
    pub namespace: String,
    pub repo_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "repo1")]
    pub repo1: RepoConfig,
    #[serde(rename = "repo2")]
    pub repo2: RepoConfig,
}

pub fn read_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config() {
        let config = read_config("config.yaml").unwrap();
        
        // Test repo1
        assert_eq!(config.repo1.target_repo, "https://gitcode.com/test-org/test-repo-1.git");
        assert_eq!(config.repo1.namespace, "test-org");
        assert_eq!(config.repo1.repo_name, "test-repo-1");
        
        // Test repo2
        assert_eq!(config.repo2.target_repo, "https://gitcode.com/test-org/test-repo-2.git");
        assert_eq!(config.repo2.namespace, "test-org");
        assert_eq!(config.repo2.repo_name, "test-repo-2");
    }
}
