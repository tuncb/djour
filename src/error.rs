//! Error types for djour

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for djour application
#[derive(Debug, Error)]
pub enum DjourError {
    #[error("Not a djour directory: {0}")]
    NotDjourDirectory(PathBuf),

    #[error("Invalid time reference: {0}")]
    InvalidTimeReference(String),

    #[error("Tag not found: {0}")]
    TagNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Editor error: {0}")]
    Editor(String),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

impl DjourError {
    /// Get the exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            DjourError::NotDjourDirectory(_) => 2,
            DjourError::InvalidTimeReference(_) => 3,
            DjourError::TagNotFound(_) => 4,
            _ => 1,
        }
    }
}

/// Result type using DjourError
pub type Result<T> = std::result::Result<T, DjourError>;
