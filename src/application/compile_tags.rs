//! Tag compilation use case
//!
//! Orchestrates the full workflow of compiling tagged content from journal entries.

use crate::domain::tags::{
    CompilationDateStyle, CompilationFormat, TagCompiler, TagParser, TagQuery, TaggedContent,
};
use crate::domain::JournalMode;
use crate::error::{DjourError, Result};
use crate::infrastructure::repository::JournalRepository;
use crate::infrastructure::FileSystemRepository;
use chrono::NaiveDate;
use std::path::PathBuf;

/// Options for compilation
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Tag query to filter by
    pub query: String,

    /// Output file path (None = default: compilations/<tag>.md)
    pub output: Option<PathBuf>,

    /// Start date filter (inclusive)
    pub from: Option<NaiveDate>,

    /// End date filter (inclusive)
    pub to: Option<NaiveDate>,

    /// Output format
    pub format: CompilationFormat,

    /// Include parent section headings for context
    pub include_context: bool,
}

/// Service for compiling tags
pub struct CompileTagsService {
    repository: FileSystemRepository,
}

impl CompileTagsService {
    /// Create new compile tags service
    pub fn new(repository: FileSystemRepository) -> Self {
        CompileTagsService { repository }
    }

    /// Execute the compilation
    ///
    /// Returns the path to the generated compilation file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The query is invalid
    /// - No notes are found
    /// - No content matches the query
    /// - File I/O fails
    pub fn execute(&self, options: CompileOptions) -> Result<PathBuf> {
        // 1. Parse query
        let query = TagQuery::parse(&options.query)?;

        // 2. Load config to get mode
        let config = self.repository.load_config()?;

        // 3. List all note files (with date filters)
        let notes = self.repository.list_notes(
            config.get_mode(),
            options.from,
            options.to,
            None, // No limit - get all notes
        )?;

        if notes.is_empty() {
            return Err(DjourError::TagNotFound(format!(
                "No notes found for query: {}",
                options.query
            )));
        }

        // 4. Parse all files and extract tagged content
        let mut all_content: Vec<TaggedContent> = Vec::new();

        for note in notes {
            let content = self.repository.read_note(&note.filename)?;
            if content.is_empty() {
                continue;
            }

            let file_path = self.repository.root().join(&note.filename);
            let tagged = TagParser::extract_from_markdown(&content, &file_path, note.date);

            all_content.extend(tagged);
        }

        // 5. Filter by query
        let filtered = TagCompiler::filter(all_content, &query);

        if filtered.is_empty() {
            return Err(DjourError::TagNotFound(format!(
                "No content found matching query: {}",
                options.query
            )));
        }

        // 6. Generate markdown output
        let date_style = match config.get_mode() {
            JournalMode::Weekly => CompilationDateStyle::WeekRange,
            JournalMode::Monthly => CompilationDateStyle::MonthRange,
            _ => CompilationDateStyle::SingleDate,
        };

        let markdown = TagCompiler::to_markdown(
            filtered,
            &query,
            options.format,
            date_style,
            options.include_context,
        );

        // 7. Determine output path
        let output_path = if let Some(path) = options.output {
            // Use provided path
            if path.is_absolute() {
                path
            } else {
                self.repository.root().join(path)
            }
        } else {
            // Default: compilations/<query>.md (sanitize query string)
            let sanitized = sanitize_filename(&options.query);
            self.repository
                .root()
                .join("compilations")
                .join(format!("{}.md", sanitized))
        };

        // 8. Write output file
        // Convert absolute path to relative for repository.write_note
        let relative_path = output_path
            .strip_prefix(self.repository.root())
            .map_err(|_| {
                DjourError::Config("Output path must be within journal directory".to_string())
            })?;

        let relative_str = relative_path
            .to_str()
            .ok_or_else(|| DjourError::Config("Invalid output path".to_string()))?;

        self.repository.write_note(relative_str, &markdown)?;

        Ok(output_path)
    }
}

/// Sanitize query string for use as filename
///
/// Converts spaces to hyphens, keeps alphanumeric characters and hyphens/underscores,
/// replaces other characters with underscores.
fn sanitize_filename(query: &str) -> String {
    query
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c.to_ascii_lowercase(),
            ' ' => '-',
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("work"), "work");
        assert_eq!(sanitize_filename("work AND urgent"), "work-and-urgent");
        assert_eq!(sanitize_filename("work OR personal"), "work-or-personal");
        assert_eq!(sanitize_filename("work NOT meeting"), "work-not-meeting");
        assert_eq!(sanitize_filename("#project-alpha"), "project-alpha");
        assert_eq!(sanitize_filename("work@email"), "work_email");
    }

    #[test]
    fn test_sanitize_filename_trim() {
        assert_eq!(sanitize_filename("_work_"), "work");
        assert_eq!(sanitize_filename("__work__"), "work");
    }

    #[test]
    fn test_sanitize_filename_case() {
        assert_eq!(sanitize_filename("WORK AND URGENT"), "work-and-urgent");
        assert_eq!(sanitize_filename("Work"), "work");
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        assert_eq!(sanitize_filename("work!urgent"), "work_urgent");
        assert_eq!(sanitize_filename("work(test)"), "work_test");
    }

    // Integration tests would require setting up a FileSystemRepository with temp directories
    // Those will be covered in the integration test file
}
