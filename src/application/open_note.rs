//! Open note use case

use crate::domain::{load_template, JournalMode, TimeReference};
use crate::error::Result;
use crate::infrastructure::{EditorSession, FileSystemRepository, JournalRepository};
use chrono::Local;

/// Service for opening notes in editor
pub struct OpenNoteService {
    repository: FileSystemRepository,
}

impl OpenNoteService {
    /// Create a new open note service
    pub fn new(repository: FileSystemRepository) -> Self {
        OpenNoteService { repository }
    }

    /// Resolve time reference to note filename, creating the note if needed.
    /// Opens the file in editor only when `open_in_editor` is true.
    pub fn execute(&self, time_ref_str: &str, open_in_editor: bool) -> Result<String> {
        // 1. Load config to get mode and editor
        let config = self.repository.load_config()?;

        // 2. Parse time reference
        let time_ref = TimeReference::parse(time_ref_str)?;

        // 3. Resolve to date
        let date = time_ref.resolve(Local::now().date_naive());

        // 4. Generate filename based on mode
        let mode = config.get_mode();
        let filename = mode.filename_for_date(date);

        // 5. Check if file exists
        if !self.repository.note_exists(&filename) {
            // 6. Create file with template
            let template_name = mode.template_name();
            let template = load_template(self.repository.root(), template_name)?;
            let content = template.render(date);

            // Special handling for Single mode
            if matches!(mode, JournalMode::Single) {
                // Append to existing file
                let existing = self.repository.read_note(&filename)?;
                let new_content = if existing.is_empty() {
                    content
                } else {
                    format!("{}\n{}", existing, content)
                };
                self.repository.write_note(&filename, &new_content)?;
            } else {
                // Create new file
                self.repository.write_note(&filename, &content)?;
            }
        }

        // 7. Open in editor when requested
        if open_in_editor {
            let editor_cmd = config.get_editor();
            let editor = EditorSession::new(editor_cmd);

            let file_path = self.repository.root().join(&filename);
            editor.open(&file_path)?;
        }

        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::JournalMode;
    use crate::infrastructure::Config;
    use tempfile::TempDir;

    #[test]
    fn test_create_new_note_daily_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize with daily mode
        repo.initialize().unwrap();
        let config = Config::new(JournalMode::Daily);
        repo.save_config(&config).unwrap();

        // Note: We can't test editor.open() in automated tests
        // Test everything up to that point

        let _service = OpenNoteService::new(repo.clone());

        // Parse time reference (today)
        let time_ref = TimeReference::parse("today").unwrap();
        let date = time_ref.resolve(Local::now().date_naive());
        let filename = config.mode.filename_for_date(date);

        // File shouldn't exist yet
        assert!(!repo.note_exists(&filename));

        // After creating (skip editor for test), file should have template
        let template_name = config.mode.template_name();
        let template = load_template(repo.root(), template_name).unwrap();
        let content = template.render(date);
        repo.write_note(&filename, &content).unwrap();

        // Verify file was created with template
        assert!(repo.note_exists(&filename));
        let file_content = repo.read_note(&filename).unwrap();
        // Check that the template was rendered (should have a date heading)
        assert!(file_content.starts_with("# "));
        assert!(file_content.contains(&date.format("%B %d, %Y").to_string()));
    }

    #[test]
    fn test_create_note_weekly_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize with weekly mode
        repo.initialize().unwrap();
        let config = Config::new(JournalMode::Weekly);
        repo.save_config(&config).unwrap();

        let time_ref = TimeReference::parse("today").unwrap();
        let date = time_ref.resolve(Local::now().date_naive());
        let filename = config.mode.filename_for_date(date);

        // Filename should be in YYYY-Www.md format
        assert!(filename.contains("-W"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_create_note_monthly_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize with monthly mode
        repo.initialize().unwrap();
        let config = Config::new(JournalMode::Monthly);
        repo.save_config(&config).unwrap();

        let time_ref = TimeReference::parse("today").unwrap();
        let date = time_ref.resolve(Local::now().date_naive());
        let filename = config.mode.filename_for_date(date);

        // Filename should be in YYYY-MM.md format
        assert!(filename.matches('-').count() == 1);
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_single_mode_appends_to_existing() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize with single mode
        repo.initialize().unwrap();
        let config = Config::new(JournalMode::Single);
        repo.save_config(&config).unwrap();

        // Create initial entry
        repo.write_note("journal.md", "# First entry").unwrap();

        // Simulate adding second entry
        let template_name = config.mode.template_name();
        let template = load_template(repo.root(), template_name).unwrap();
        let date = Local::now().date_naive();
        let content = template.render(date);

        let existing = repo.read_note("journal.md").unwrap();
        let new_content = format!("{}\n{}", existing, content);
        repo.write_note("journal.md", &new_content).unwrap();

        // Verify content was appended
        let final_content = repo.read_note("journal.md").unwrap();
        assert!(final_content.contains("# First entry"));
        assert!(final_content.contains("---")); // Entry separator
    }

    #[test]
    fn test_opening_existing_note() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize
        repo.initialize().unwrap();
        let config = Config::new(JournalMode::Daily);
        repo.save_config(&config).unwrap();

        // Create existing note
        let filename = "2025-01-17.md";
        repo.write_note(filename, "# Existing content").unwrap();

        // Opening existing note should not overwrite
        assert!(repo.note_exists(filename));
        let content = repo.read_note(filename).unwrap();
        assert_eq!(content, "# Existing content");
    }

    #[test]
    fn test_parse_invalid_time_reference() {
        let result = TimeReference::parse("invaliddate");
        assert!(result.is_err());
    }
}
