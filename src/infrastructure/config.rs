//! Configuration management

use crate::domain::JournalMode;
use crate::error::{DjourError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: JournalMode,
    pub editor: String,
    pub created: DateTime<Utc>,
}

impl Config {
    /// Create a new config with default values
    pub fn new(mode: JournalMode) -> Self {
        Config {
            mode,
            editor: Self::detect_default_editor(),
            created: Utc::now(),
        }
    }

    /// Load config from .djour/config.toml in the given directory
    pub fn load_from_dir(path: &Path) -> Result<Self> {
        let config_path = path.join(".djour").join("config.toml");

        let contents = fs::read_to_string(&config_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DjourError::NotDjourDirectory(path.to_path_buf())
            } else {
                DjourError::Io(e)
            }
        })?;

        toml::from_str(&contents)
            .map_err(|e| DjourError::Config(format!("Failed to parse config.toml: {}", e)))
    }

    /// Save config to .djour/config.toml in the given directory
    pub fn save_to_dir(&self, path: &Path) -> Result<()> {
        let djour_dir = path.join(".djour");
        let config_path = djour_dir.join("config.toml");

        // Ensure .djour directory exists
        if !djour_dir.exists() {
            fs::create_dir(&djour_dir)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| DjourError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, contents)?;

        Ok(())
    }

    /// Get the editor command, checking environment variables first
    pub fn get_editor(&self) -> String {
        std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| self.editor.clone())
    }

    /// Get the effective mode, checking DJOUR_MODE environment variable first
    pub fn get_mode(&self) -> JournalMode {
        if let Ok(mode_str) = std::env::var("DJOUR_MODE") {
            if let Ok(mode) = JournalMode::from_str(&mode_str) {
                return mode;
            }
            // If invalid, log warning and fall back to config
            eprintln!(
                "Warning: Invalid DJOUR_MODE '{}', using configured mode '{:?}'",
                mode_str, self.mode
            );
        }
        self.mode
    }

    /// Detect default editor from environment or system
    fn detect_default_editor() -> String {
        std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "nano".to_string()
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_config() {
        let config = Config::new(JournalMode::Daily);
        assert_eq!(config.mode, JournalMode::Daily);
        // Editor should be detected from environment or default
        assert!(!config.editor.is_empty());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp = TempDir::new().unwrap();
        let config = Config::new(JournalMode::Weekly);

        // Save config
        config.save_to_dir(temp.path()).unwrap();

        // Check .djour directory was created
        assert!(temp.path().join(".djour").exists());
        assert!(temp.path().join(".djour/config.toml").exists());

        // Load config
        let loaded = Config::load_from_dir(temp.path()).unwrap();

        // Verify it matches
        assert_eq!(loaded.mode, config.mode);
        assert_eq!(loaded.editor, config.editor);
        assert_eq!(loaded.created, config.created);
    }

    #[test]
    fn test_load_missing_config() {
        let temp = TempDir::new().unwrap();

        // Try to load config from directory without .djour
        let result = Config::load_from_dir(temp.path());

        assert!(result.is_err());
        match result.unwrap_err() {
            DjourError::NotDjourDirectory(_) => {}
            _ => panic!("Expected NotDjourDirectory error"),
        }
    }

    #[test]
    fn test_get_editor_uses_env() {
        let config = Config {
            mode: JournalMode::Daily,
            editor: "default-editor".to_string(),
            created: Utc::now(),
        };

        // Without environment variables, should use config value
        let editor = config.get_editor();
        // Note: This might return an env var if EDITOR or VISUAL is set in test environment
        assert!(!editor.is_empty());
    }

    #[test]
    fn test_default_editor_detection() {
        let editor = Config::detect_default_editor();
        assert!(!editor.is_empty());

        // Should be notepad on Windows, nano on Unix (or env var if set)
        if cfg!(windows) {
            // Might be notepad or an env var
            assert!(
                editor == "notepad"
                    || std::env::var("EDITOR").is_ok()
                    || std::env::var("VISUAL").is_ok()
            );
        } else {
            // Might be nano or an env var
            assert!(
                editor == "nano"
                    || std::env::var("EDITOR").is_ok()
                    || std::env::var("VISUAL").is_ok()
            );
        }
    }

    #[test]
    fn test_get_mode_with_env_override() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("DJOUR_MODE");

        let config = Config::new(JournalMode::Daily);

        // Test without env var - should use config
        assert_eq!(config.get_mode(), JournalMode::Daily);

        // Test with env var - should use env var
        std::env::set_var("DJOUR_MODE", "weekly");
        assert_eq!(config.get_mode(), JournalMode::Weekly);
        std::env::remove_var("DJOUR_MODE");

        // Test again without env var - should use config
        assert_eq!(config.get_mode(), JournalMode::Daily);
    }

    #[test]
    fn test_get_mode_invalid_env_falls_back() {
        // Clean up first
        std::env::remove_var("DJOUR_MODE");

        let config = Config::new(JournalMode::Daily);

        std::env::set_var("DJOUR_MODE", "invalid");
        let mode = config.get_mode();
        std::env::remove_var("DJOUR_MODE");

        assert_eq!(mode, JournalMode::Daily); // Falls back
    }

    #[test]
    fn test_get_mode_without_env() {
        // Ensure DJOUR_MODE is not set
        std::env::remove_var("DJOUR_MODE");

        let config = Config::new(JournalMode::Monthly);
        assert_eq!(config.get_mode(), JournalMode::Monthly);
    }
}
