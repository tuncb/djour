//! Open note use case

use crate::domain::{load_template, JournalMode, TimeReference};
use crate::error::{DjourError, Result};
use crate::infrastructure::{EditorSession, FileSystemRepository, JournalRepository};
use chrono::Local;

/// Resolve time reference to note filename, creating the note if needed.
/// Opens the file in editor only when `open_in_editor` is true.
pub fn open_note(
    repository: &FileSystemRepository,
    time_ref_str: &str,
    open_in_editor: bool,
) -> Result<String> {
    // 1. Load config to get mode and editor
    let config = repository.load_config()?;

    // 2. Parse time reference
    let time_ref = TimeReference::parse(time_ref_str)?;

    // 3. Validate mode-specific time references
    let mode = config.get_mode();
    if let Some(required_mode) = time_ref.required_mode() {
        if mode != required_mode {
            return Err(DjourError::TimeReferenceModeMismatch {
                time_ref: time_ref_str.to_string(),
                required_mode,
                current_mode: mode,
                period: time_ref.period_name().unwrap_or("time"),
            });
        }
    }

    // 4. Resolve to date
    let date = time_ref.resolve(Local::now().date_naive());

    // 5. Generate filename based on mode
    let filename = mode.filename_for_date(date);

    // 6. Check if file exists
    if !repository.note_exists(&filename) {
        // 7. Create file with template
        let template_name = mode.template_name();
        let template = load_template(repository.root(), template_name)?;
        let content = template.render(date);

        // Special handling for Single mode
        if matches!(mode, JournalMode::Single) {
            // Append to existing file
            let existing = repository.read_note(&filename)?;
            let new_content = if existing.is_empty() {
                content
            } else {
                format!("{}\n{}", existing, content)
            };
            repository.write_note(&filename, &new_content)?;
        } else {
            // Create new file
            repository.write_note(&filename, &content)?;
        }
    }

    // 8. Open in editor when requested
    if open_in_editor {
        let editor_cmd = config.get_editor();
        let editor = EditorSession::new(editor_cmd);

        let file_path = repository.root().join(&filename);
        editor.open(&file_path)?;
    }

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::JournalMode;
    use crate::infrastructure::Config;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarRestore {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarRestore {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                previous: std::env::var_os(key),
            }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

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

    #[test]
    fn test_day_offset_requires_daily_mode() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_MODE");
        std::env::remove_var("DJOUR_MODE");

        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();
        repo.save_config(&Config::new(JournalMode::Weekly)).unwrap();

        let result = open_note(&repo, "day +2", false);
        assert!(matches!(
            result,
            Err(DjourError::TimeReferenceModeMismatch { .. })
        ));
    }

    #[test]
    fn test_week_offset_requires_weekly_mode() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_MODE");
        std::env::remove_var("DJOUR_MODE");

        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();
        repo.save_config(&Config::new(JournalMode::Daily)).unwrap();

        let result = open_note(&repo, "week -2", false);
        assert!(matches!(
            result,
            Err(DjourError::TimeReferenceModeMismatch { .. })
        ));
    }

    #[test]
    fn test_day_offset_creates_daily_note() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_MODE");
        std::env::remove_var("DJOUR_MODE");

        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();
        repo.save_config(&Config::new(JournalMode::Daily)).unwrap();

        let date = Local::now().date_naive() + chrono::Duration::days(2);
        let expected = JournalMode::Daily.filename_for_date(date);
        let filename = open_note(&repo, "day +2", false).unwrap();

        assert_eq!(filename, expected);
        assert!(repo.note_exists(&expected));
    }

    #[test]
    fn test_week_offset_creates_weekly_note() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_MODE");
        std::env::remove_var("DJOUR_MODE");

        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();
        repo.save_config(&Config::new(JournalMode::Weekly)).unwrap();

        let date = Local::now().date_naive() - chrono::Duration::weeks(2);
        let expected = JournalMode::Weekly.filename_for_date(date);
        let filename = open_note(&repo, "week -2", false).unwrap();

        assert_eq!(filename, expected);
        assert!(repo.note_exists(&expected));
    }
}
