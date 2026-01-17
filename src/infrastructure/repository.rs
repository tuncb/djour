//! File system repository (to be implemented in Phase 2)

use std::path::PathBuf;

/// Abstract repository for journal operations
pub trait JournalRepository {
    // Methods will be added as needed
}

/// File system implementation of JournalRepository
#[derive(Debug)]
pub struct FileSystemRepository {
    pub root: PathBuf,
}

impl FileSystemRepository {
    pub fn new(root: PathBuf) -> Self {
        FileSystemRepository { root }
    }
}
