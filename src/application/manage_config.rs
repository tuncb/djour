//! Config management use case

use crate::domain::JournalMode;
use crate::error::{DjourError, Result};
use crate::infrastructure::{Config, FileSystemRepository, JournalRepository};
use std::str::FromStr;

/// Get a single config value.
pub fn get_config(repository: &FileSystemRepository, key: &str) -> Result<String> {
    let config = repository.load_config()?;

    match key {
        "mode" => Ok(format!("{:?}", config.mode).to_lowercase()),
        "editor" => Ok(config.editor.clone()),
        _ => Err(DjourError::Config(format!(
            "Unknown config key: '{}'. Valid keys are: mode, editor",
            key
        ))),
    }
}

/// Set a config value.
pub fn set_config(repository: &FileSystemRepository, key: &str, value: &str) -> Result<()> {
    let mut config = repository.load_config()?;

    match key {
        "mode" => {
            let mode = JournalMode::from_str(value).map_err(DjourError::Config)?;
            config.mode = mode;
        }
        "editor" => {
            config.editor = value.to_string();
        }
        _ => {
            return Err(DjourError::Config(format!(
                "Unknown config key: '{}'. Valid keys are: mode, editor",
                key
            )));
        }
    }

    repository.save_config(&config)?;
    Ok(())
}

/// List all config values.
pub fn list_config(repository: &FileSystemRepository) -> Result<Config> {
    repository.load_config()
}
