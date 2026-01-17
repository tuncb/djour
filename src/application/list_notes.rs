//! List notes use case

use crate::domain::JournalMode;
use crate::error::Result;
use crate::infrastructure::{FileSystemRepository, NoteEntry};
use chrono::NaiveDate;

/// Service for listing notes
pub struct ListNotesService {
    repository: FileSystemRepository,
}

impl ListNotesService {
    /// Create a new list notes service
    pub fn new(repository: FileSystemRepository) -> Self {
        ListNotesService { repository }
    }

    /// Execute the list notes operation
    pub fn execute(
        &self,
        mode: JournalMode,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteEntry>> {
        self.repository.list_notes(mode, from, to, limit)
    }
}
