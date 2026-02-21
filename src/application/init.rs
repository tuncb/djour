//! Initialize journal use case

use crate::domain::JournalMode;
use crate::error::Result;
use crate::infrastructure::{Config, FileSystemRepository, JournalRepository};
use std::fs;
use std::path::Path;

/// Initialize a new journal at the specified path.
pub fn init(path: &Path, mode: JournalMode) -> Result<()> {
    // Create the directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    // Create repository for this path
    let repo = FileSystemRepository::new(path.to_path_buf());

    // Initialize .djour directory
    repo.initialize()?;

    // Create default config
    let config = Config::new(mode);

    // Save config
    repo.save_config(&config)?;

    println!("Initialized djour journal at {}", path.display());
    println!("Mode: {:?}", mode);

    Ok(())
}
