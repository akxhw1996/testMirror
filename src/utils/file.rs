use std::fs;
use std::path::{Path, PathBuf};
use std::io;

pub fn create_empty_folder(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)
}

/// Delete a folder and all its contents recursively
pub fn delete_folder(folder_path: &PathBuf) -> Result<(), std::io::Error> {
    println!("Deleting folder at {}", folder_path.display());
    if let Err(e) = std::fs::remove_dir_all(folder_path) {
        println!("Error deleting folder: {}", e);
        return Err(e);
    }
    println!("Folder deletion completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_folder() {
        // Create a temporary directory
        let temp_dir = std::path::PathBuf::from("/tmp/test_folder");
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create some test files
        std::fs::write(temp_dir.join("test.txt"), "test content").unwrap();
        
        // Test deletion
        let result = delete_folder(&temp_dir);
        assert!(result.is_ok());
        assert!(!temp_dir.exists());
    }
}
