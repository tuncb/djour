//! Config management use case

use crate::domain::JournalMode;
use crate::error::{DjourError, Result};
use crate::infrastructure::{Config, FileSystemRepository, JournalRepository};
use std::str::FromStr;

/// Service for managing journal configuration
pub struct ConfigService {
    repository: FileSystemRepository,
}

impl ConfigService {
    /// Create a new config service
    pub fn new(repository: FileSystemRepository) -> Self {
        ConfigService { repository }
    }

    /// Get a single config value
    pub fn get(&self, key: &str) -> Result<String> {
        let config = self.repository.load_config()?;

        match key {
            "mode" => Ok(format!("{:?}", config.mode).to_lowercase()),
            "editor" => Ok(config.editor.clone()),
            "created" => Ok(config.created.to_rfc3339()),
            _ => Err(DjourError::Config(format!(
                "Unknown config key: '{}'. Valid keys are: mode, editor, created",
                key
            ))),
        }
    }

    /// Set a config value
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let mut config = self.repository.load_config()?;

        match key {
            "mode" => {
                let mode = JournalMode::from_str(value).map_err(DjourError::Config)?;
                config.mode = mode;
            }
            "editor" => {
                config.editor = value.to_string();
            }
            "created" => {
                return Err(DjourError::Config(
                    "Cannot modify 'created' field (read-only)".to_string(),
                ));
            }
            _ => {
                return Err(DjourError::Config(format!(
                    "Unknown config key: '{}'. Valid keys are: mode, editor",
                    key
                )));
            }
        }

        self.repository.save_config(&config)?;
        Ok(())
    }

    /// List all config values
    pub fn list(&self) -> Result<Config> {
        self.repository.load_config()
    }
}
