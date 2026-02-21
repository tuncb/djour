//! List notes use case

use crate::domain::JournalMode;
use crate::error::Result;
use crate::infrastructure::{FileSystemRepository, NoteEntry};
use chrono::NaiveDate;

/// List notes with optional date range and limit.
pub fn list_notes(
    repository: &FileSystemRepository,
    mode: JournalMode,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: Option<usize>,
    recursive: bool,
) -> Result<Vec<NoteEntry>> {
    repository.list_notes(mode, from, to, limit, recursive)
}
