//! Journal aggregate root

use std::path::PathBuf;

/// Represents a journal root directory
#[derive(Debug, Clone)]
pub struct Journal {
    pub root: PathBuf,
}

impl Journal {
    pub fn new(root: PathBuf) -> Self {
        Journal { root }
    }
}
