//! Retag use case

use crate::domain::tags::retag_markdown;
use crate::error::{DjourError, Result};
use crate::infrastructure::repository::JournalRepository;
use crate::infrastructure::FileSystemRepository;
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct RetagOptions {
    pub from_tag: String,
    pub to_tag: String,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub recursive: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetagFileChange {
    pub filename: String,
    pub replacements: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetagReport {
    pub scanned_files: usize,
    pub changed_files: usize,
    pub total_replacements: usize,
    pub dry_run: bool,
    pub changes: Vec<RetagFileChange>,
}

pub fn retag_notes(
    repository: &FileSystemRepository,
    options: RetagOptions,
) -> Result<RetagReport> {
    let from_tag = normalize_tag_argument(&options.from_tag)?;
    let to_tag = normalize_tag_argument(&options.to_tag)?;

    let config = repository.load_config()?;
    let notes = repository.list_notes(
        config.get_mode(),
        options.from,
        options.to,
        None,
        options.recursive,
    )?;

    let mut changes = Vec::new();
    let mut total_replacements = 0usize;

    for note in &notes {
        let content = repository.read_note(&note.filename)?;
        if content.is_empty() {
            continue;
        }

        let result = retag_markdown(&content, &from_tag, &to_tag);
        if result.replacements == 0 {
            continue;
        }

        if !options.dry_run {
            repository.write_note_atomic(&note.filename, &result.content)?;
        }

        total_replacements += result.replacements;
        changes.push(RetagFileChange {
            filename: note.filename.clone(),
            replacements: result.replacements,
        });
    }

    Ok(RetagReport {
        scanned_files: notes.len(),
        changed_files: changes.len(),
        total_replacements,
        dry_run: options.dry_run,
        changes,
    })
}

fn normalize_tag_argument(input: &str) -> Result<String> {
    let tag = input.strip_prefix('#').unwrap_or(input);
    if tag.is_empty() {
        return Err(DjourError::Config(format!("Invalid tag: {}", input)));
    }

    if !tag
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(DjourError::Config(format!(
            "Invalid tag: {}. Allowed characters: letters, numbers, '-', '_'",
            input
        )));
    }

    Ok(tag.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tag_argument_accepts_hash_prefix() {
        assert_eq!(normalize_tag_argument("#Work").unwrap(), "work");
        assert_eq!(normalize_tag_argument("Work").unwrap(), "work");
    }

    #[test]
    fn normalize_tag_argument_rejects_invalid() {
        assert!(normalize_tag_argument("work@email").is_err());
        assert!(normalize_tag_argument("#").is_err());
        assert!(normalize_tag_argument("##work").is_err());
    }
}
