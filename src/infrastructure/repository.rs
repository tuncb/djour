//! File system repository

use crate::error::{DjourError, Result};
use crate::infrastructure::Config;
use std::fs;
use std::path::{Path, PathBuf};

/// Abstract repository for journal operations
pub trait JournalRepository {
    /// Get the root directory of this repository
    fn root(&self) -> &Path;

    /// Load configuration from .djour/config.toml
    fn load_config(&self) -> Result<Config>;

    /// Save configuration to .djour/config.toml
    fn save_config(&self, config: &Config) -> Result<()>;

    /// Check if .djour directory exists
    fn is_initialized(&self) -> bool;

    /// Create .djour directory structure
    fn initialize(&self) -> Result<()>;
}

/// File system implementation of JournalRepository
#[derive(Debug, Clone)]
pub struct FileSystemRepository {
    pub root: PathBuf,
}

impl FileSystemRepository {
    /// Create a new repository with the given root directory
    pub fn new(root: PathBuf) -> Self {
        FileSystemRepository { root }
    }

    /// Discover journal root by walking up from current directory
    pub fn discover() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::discover_from(&current_dir)
    }

    /// Discover journal root by walking up from a specific starting directory
    pub fn discover_from(start: &Path) -> Result<Self> {
        let mut current = start.to_path_buf();

        loop {
            if Self::has_djour_dir(&current) {
                return Ok(FileSystemRepository::new(current));
            }

            // Try to move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    // Reached filesystem root without finding .djour
                    return Err(DjourError::NotDjourDirectory(start.to_path_buf()));
                }
            }
        }
    }

    /// Check if a path contains a .djour directory
    fn has_djour_dir(path: &Path) -> bool {
        path.join(".djour").is_dir()
    }
}

impl JournalRepository for FileSystemRepository {
    fn root(&self) -> &Path {
        &self.root
    }

    fn load_config(&self) -> Result<Config> {
        Config::load_from_dir(&self.root)
    }

    fn save_config(&self, config: &Config) -> Result<()> {
        config.save_to_dir(&self.root)
    }

    fn is_initialized(&self) -> bool {
        Self::has_djour_dir(&self.root)
    }

    fn initialize(&self) -> Result<()> {
        let djour_dir = self.root.join(".djour");

        if djour_dir.exists() {
            return Err(DjourError::Config(format!(
                "Directory already initialized: {}",
                self.root.display()
            )));
        }

        fs::create_dir(&djour_dir)?;
        Ok(())
    }
}

// Note operations (not part of trait - filesystem-specific)
impl FileSystemRepository {
    /// Check if a note file exists
    pub fn note_exists(&self, filename: &str) -> bool {
        self.root.join(filename).exists()
    }

    /// Read note content (returns empty string if file doesn't exist)
    pub fn read_note(&self, filename: &str) -> Result<String> {
        let path = self.root.join(filename);

        if !path.exists() {
            return Ok(String::new());
        }

        fs::read_to_string(&path).map_err(DjourError::Io)
    }

    /// Write note content (creates if doesn't exist, overwrites if exists)
    pub fn write_note(&self, filename: &str, content: &str) -> Result<()> {
        let path = self.root.join(filename);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&path, content).map_err(DjourError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::JournalMode;
    use tempfile::TempDir;

    #[test]
    fn test_new_repository() {
        let path = PathBuf::from("/tmp/test");
        let repo = FileSystemRepository::new(path.clone());
        assert_eq!(repo.root, path);
    }

    #[test]
    fn test_is_initialized() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Not initialized yet
        assert!(!repo.is_initialized());

        // Create .djour directory
        repo.initialize().unwrap();

        // Now it should be initialized
        assert!(repo.is_initialized());
    }

    #[test]
    fn test_initialize_creates_djour_dir() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();

        assert!(temp.path().join(".djour").exists());
        assert!(temp.path().join(".djour").is_dir());
    }

    #[test]
    fn test_initialize_twice_fails() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // First initialization succeeds
        repo.initialize().unwrap();

        // Second initialization fails
        let result = repo.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_from_subdirectory() {
        let temp = TempDir::new().unwrap();

        // Create .djour in root
        fs::create_dir(temp.path().join(".djour")).unwrap();

        // Create a subdirectory
        let subdir = temp.path().join("sub").join("deep");
        fs::create_dir_all(&subdir).unwrap();

        // Discover from subdirectory should find root
        let repo = FileSystemRepository::discover_from(&subdir).unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_discover_from_root() {
        let temp = TempDir::new().unwrap();

        // Create .djour in root
        fs::create_dir(temp.path().join(".djour")).unwrap();

        // Discover from root should work
        let repo = FileSystemRepository::discover_from(temp.path()).unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_discover_fails_when_no_djour() {
        let temp = TempDir::new().unwrap();

        // No .djour directory
        let result = FileSystemRepository::discover_from(temp.path());
        assert!(result.is_err());

        match result.unwrap_err() {
            DjourError::NotDjourDirectory(_) => {}
            _ => panic!("Expected NotDjourDirectory error"),
        }
    }

    #[test]
    fn test_save_and_load_config() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize
        repo.initialize().unwrap();

        // Create and save config
        let config = Config::new(JournalMode::Weekly);
        repo.save_config(&config).unwrap();

        // Load config
        let loaded = repo.load_config().unwrap();
        assert_eq!(loaded.mode, config.mode);
    }

    #[test]
    fn test_note_exists_true() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create a note file
        let note_path = temp.path().join("2025-01-17.md");
        fs::write(&note_path, "test content").unwrap();

        assert!(repo.note_exists("2025-01-17.md"));
    }

    #[test]
    fn test_note_exists_false() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        assert!(!repo.note_exists("nonexistent.md"));
    }

    #[test]
    fn test_read_note_existing() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create a note file
        let content = "# My Note\n\nSome content here.";
        let note_path = temp.path().join("note.md");
        fs::write(&note_path, content).unwrap();

        let read_content = repo.read_note("note.md").unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_read_note_missing() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Reading nonexistent file returns empty string
        let content = repo.read_note("nonexistent.md").unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_write_note_creates_file() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        let content = "# Test Note\n\nContent";
        repo.write_note("test.md", content).unwrap();

        // Verify file was created
        let note_path = temp.path().join("test.md");
        assert!(note_path.exists());

        // Verify content
        let read_content = fs::read_to_string(note_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_note_overwrites() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Write initial content
        repo.write_note("test.md", "initial").unwrap();

        // Overwrite with new content
        repo.write_note("test.md", "updated").unwrap();

        // Verify overwrite
        let content = repo.read_note("test.md").unwrap();
        assert_eq!(content, "updated");
    }

    #[test]
    fn test_write_note_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Write to nested path that doesn't exist
        repo.write_note("sub/dir/note.md", "content").unwrap();

        // Verify parent dirs were created
        assert!(temp.path().join("sub").join("dir").exists());
        assert!(temp.path().join("sub").join("dir").join("note.md").exists());
    }
}
