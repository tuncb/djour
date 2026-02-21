//! File system repository

use crate::domain::JournalMode;
use crate::error::{DjourError, Result};
use crate::infrastructure::Config;
use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Represents a note file with its metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEntry {
    pub filename: String,
    pub date: Option<NaiveDate>,
}

impl NoteEntry {
    pub fn new(filename: String, date: Option<NaiveDate>) -> Self {
        NoteEntry { filename, date }
    }
}

/// Abstract repository for journal operations
pub trait JournalRepository {
    /// Get the root directory of this repository
    fn root(&self) -> &Path;

    /// Load configuration from .djour/config.toml
    fn load_config(&self) -> Result<Config>;

    /// Save configuration to .djour/config.toml
    fn save_config(&self, config: &Config) -> Result<()>;

    /// Check if .djour directory exists
    fn is_initialized(&self) -> bool;

    /// Create .djour directory structure
    fn initialize(&self) -> Result<()>;
}

/// File system implementation of JournalRepository
#[derive(Debug, Clone)]
pub struct FileSystemRepository {
    pub root: PathBuf,
}

impl FileSystemRepository {
    /// Create a new repository with the given root directory
    pub fn new(root: PathBuf) -> Self {
        FileSystemRepository { root }
    }

    /// Discover journal root by walking up from current directory
    /// First checks DJOUR_ROOT environment variable, then falls back to discovery
    pub fn discover() -> Result<Self> {
        // 1. Check DJOUR_ROOT environment variable first
        if let Ok(root_path) = std::env::var("DJOUR_ROOT") {
            let path = PathBuf::from(root_path);
            if Self::has_djour_dir(&path) {
                return Ok(FileSystemRepository::new(path));
            } else {
                return Err(DjourError::Config(format!(
                    "DJOUR_ROOT is set to '{}' but no .djour directory found. \
                    Run 'djour init' in that directory or unset DJOUR_ROOT.",
                    path.display()
                )));
            }
        }

        // 2. Fall back to walking up from current directory
        let current_dir = std::env::current_dir()?;
        Self::discover_from(&current_dir)
    }

    /// Discover journal root by walking up from a specific starting directory
    pub fn discover_from(start: &Path) -> Result<Self> {
        let mut current = start.to_path_buf();

        loop {
            if Self::has_djour_dir(&current) {
                return Ok(FileSystemRepository::new(current));
            }

            // Try to move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    // Reached filesystem root without finding .djour
                    return Err(DjourError::NotDjourDirectory(start.to_path_buf()));
                }
            }
        }
    }

    /// Check if a path contains a .djour directory
    fn has_djour_dir(path: &Path) -> bool {
        path.join(".djour").is_dir()
    }
}

impl JournalRepository for FileSystemRepository {
    fn root(&self) -> &Path {
        &self.root
    }

    fn load_config(&self) -> Result<Config> {
        Config::load_from_dir(&self.root)
    }

    fn save_config(&self, config: &Config) -> Result<()> {
        config.save_to_dir(&self.root)
    }

    fn is_initialized(&self) -> bool {
        Self::has_djour_dir(&self.root)
    }

    fn initialize(&self) -> Result<()> {
        let djour_dir = self.root.join(".djour");

        if djour_dir.exists() {
            return Err(DjourError::Config(format!(
                "Directory already initialized: {}",
                self.root.display()
            )));
        }

        fs::create_dir(&djour_dir)?;
        Ok(())
    }
}

// Note operations (not part of trait - filesystem-specific)
impl FileSystemRepository {
    /// Check if a note file exists
    pub fn note_exists(&self, filename: &str) -> bool {
        self.root.join(filename).exists()
    }

    /// Read note content (returns empty string if file doesn't exist)
    pub fn read_note(&self, filename: &str) -> Result<String> {
        let path = self.root.join(filename);

        if !path.exists() {
            return Ok(String::new());
        }

        fs::read_to_string(&path).map_err(DjourError::Io)
    }

    /// Write note content (creates if doesn't exist, overwrites if exists)
    pub fn write_note(&self, filename: &str, content: &str) -> Result<()> {
        let path = self.root.join(filename);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&path, content).map_err(DjourError::Io)
    }

    /// Create a directory (and parents) relative to the repository root.
    pub fn create_dir_all(&self, dir: &str) -> Result<()> {
        let path = self.root.join(dir);
        fs::create_dir_all(path).map_err(DjourError::Io)
    }

    /// Copy a note file (relative paths) within the repository.
    pub fn copy_note(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.root.join(from);
        let to_path = self.root.join(to);

        if !from_path.exists() {
            return Err(DjourError::Config(format!(
                "Cannot copy missing file: {}",
                from_path.display()
            )));
        }

        if let Some(parent) = to_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::copy(from_path, to_path)?;
        Ok(())
    }

    /// Move (rename) a note file (relative paths) within the repository.
    pub fn move_note(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.root.join(from);
        let to_path = self.root.join(to);

        if !from_path.exists() {
            return Err(DjourError::Config(format!(
                "Cannot move missing file: {}",
                from_path.display()
            )));
        }

        if to_path.exists() {
            return Err(DjourError::Config(format!(
                "Destination already exists: {}",
                to_path.display()
            )));
        }

        if let Some(parent) = to_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::rename(from_path, to_path)?;
        Ok(())
    }

    /// Write note content using a best-effort atomic replace:
    /// write to a temp file in the same directory, then rename into place.
    ///
    /// On Windows, `rename` does not overwrite existing files, so we remove the destination first.
    pub fn write_note_atomic(&self, filename: &str, content: &str) -> Result<()> {
        let path = self.root.join(filename);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let tmp_name = format!(
            "{}.djour-tmp-{}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("note.md"),
            std::process::id()
        );
        let tmp_path = path.with_file_name(tmp_name);

        fs::write(&tmp_path, content)?;

        if path.exists() {
            // Best-effort atomic-ish replacement; we rely on archive backups for rollback.
            fs::remove_file(&path)?;
        }

        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    fn normalize_relative_path(path: &Path) -> Option<String> {
        let parts: Vec<&str> = path
            .iter()
            .map(|part| part.to_str())
            .collect::<Option<_>>()?;
        Some(parts.join("/"))
    }

    fn note_entry_from_relative_path(mode: JournalMode, rel: &Path) -> Option<NoteEntry> {
        let filename = Self::normalize_relative_path(rel)?;
        let leaf = rel.file_name()?.to_str()?;

        // Only consider markdown files.
        if !leaf.ends_with(".md") {
            return None;
        }

        match mode {
            JournalMode::Single => {
                if leaf == "journal.md" {
                    Some(NoteEntry::new(filename, None))
                } else {
                    None
                }
            }
            _ => mode
                .date_from_filename(leaf)
                .map(|d| NoteEntry::new(filename, Some(d))),
        }
    }

    fn collect_root_note_entries(&self, mode: JournalMode) -> Result<Vec<NoteEntry>> {
        let entries = fs::read_dir(&self.root)?;
        let mut notes = Vec::new();

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&self.root) else {
                continue;
            };
            if let Some(note) = Self::note_entry_from_relative_path(mode, rel) {
                notes.push(note);
            }
        }

        Ok(notes)
    }

    fn collect_recursive_note_entries(&self, mode: JournalMode) -> Vec<NoteEntry> {
        let mut notes = Vec::new();

        let walker = WalkDir::new(&self.root).into_iter().filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if !entry.file_type().is_dir() {
                return true;
            }
            entry
                .file_name()
                .to_str()
                .is_none_or(|name| !name.starts_with('.'))
        });

        for entry in walker {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(rel) = entry.path().strip_prefix(&self.root) else {
                continue;
            };
            if let Some(note) = Self::note_entry_from_relative_path(mode, rel) {
                notes.push(note);
            }
        }

        notes
    }

    /// List all note files for the given mode
    /// Filters and sorts by date, applying optional date range and limit
    pub fn list_notes(
        &self,
        mode: JournalMode,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: Option<usize>,
        recursive: bool,
    ) -> Result<Vec<NoteEntry>> {
        let mut notes = if recursive {
            self.collect_recursive_note_entries(mode)
        } else {
            self.collect_root_note_entries(mode)?
        };

        // Apply date range filters
        if let Some(from_date) = from {
            notes.retain(|e| e.date.is_none_or(|d| d >= from_date));
        }
        if let Some(to_date) = to {
            notes.retain(|e| e.date.is_none_or(|d| d <= to_date));
        }

        // Sort by date descending (newest first)
        notes.sort_by(|a, b| match (a.date, b.date) {
            (Some(da), Some(db)) => db.cmp(&da), // Reverse order for descending
            (Some(_), None) => std::cmp::Ordering::Less, // Dated before undated
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.filename.cmp(&b.filename),
        });

        // Apply limit
        if let Some(n) = limit {
            notes.truncate(n);
        }

        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::JournalMode;
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
    fn test_new_repository() {
        let path = PathBuf::from("/tmp/test");
        let repo = FileSystemRepository::new(path.clone());
        assert_eq!(repo.root, path);
    }

    #[test]
    fn test_is_initialized() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Not initialized yet
        assert!(!repo.is_initialized());

        // Create .djour directory
        repo.initialize().unwrap();

        // Now it should be initialized
        assert!(repo.is_initialized());
    }

    #[test]
    fn test_initialize_creates_djour_dir() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.initialize().unwrap();

        assert!(temp.path().join(".djour").exists());
        assert!(temp.path().join(".djour").is_dir());
    }

    #[test]
    fn test_initialize_twice_fails() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // First initialization succeeds
        repo.initialize().unwrap();

        // Second initialization fails
        let result = repo.initialize();
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_from_subdirectory() {
        let temp = TempDir::new().unwrap();

        // Create .djour in root
        fs::create_dir(temp.path().join(".djour")).unwrap();

        // Create a subdirectory
        let subdir = temp.path().join("sub").join("deep");
        fs::create_dir_all(&subdir).unwrap();

        // Discover from subdirectory should find root
        let repo = FileSystemRepository::discover_from(&subdir).unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_discover_from_root() {
        let temp = TempDir::new().unwrap();

        // Create .djour in root
        fs::create_dir(temp.path().join(".djour")).unwrap();

        // Discover from root should work
        let repo = FileSystemRepository::discover_from(temp.path()).unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_discover_fails_when_no_djour() {
        let temp = TempDir::new().unwrap();

        // No .djour directory
        let result = FileSystemRepository::discover_from(temp.path());
        assert!(result.is_err());

        match result.unwrap_err() {
            DjourError::NotDjourDirectory(_) => {}
            _ => panic!("Expected NotDjourDirectory error"),
        }
    }

    #[test]
    fn test_save_and_load_config() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Initialize
        repo.initialize().unwrap();

        // Create and save config
        let config = Config::new(JournalMode::Weekly);
        repo.save_config(&config).unwrap();

        // Load config
        let loaded = repo.load_config().unwrap();
        assert_eq!(loaded.mode, config.mode);
    }

    #[test]
    fn test_note_exists_true() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create a note file
        let note_path = temp.path().join("2025-01-17.md");
        fs::write(&note_path, "test content").unwrap();

        assert!(repo.note_exists("2025-01-17.md"));
    }

    #[test]
    fn test_note_exists_false() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        assert!(!repo.note_exists("nonexistent.md"));
    }

    #[test]
    fn test_read_note_existing() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create a note file
        let content = "# My Note\n\nSome content here.";
        let note_path = temp.path().join("note.md");
        fs::write(&note_path, content).unwrap();

        let read_content = repo.read_note("note.md").unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_read_note_missing() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Reading nonexistent file returns empty string
        let content = repo.read_note("nonexistent.md").unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_write_note_creates_file() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        let content = "# Test Note\n\nContent";
        repo.write_note("test.md", content).unwrap();

        // Verify file was created
        let note_path = temp.path().join("test.md");
        assert!(note_path.exists());

        // Verify content
        let read_content = fs::read_to_string(note_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_copy_note_copies_file() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.write_note("a.md", "hello").unwrap();
        repo.copy_note("a.md", ".djour/archive/a.md").unwrap();

        assert!(temp.path().join(".djour/archive/a.md").exists());
        let copied = fs::read_to_string(temp.path().join(".djour/archive/a.md")).unwrap();
        assert_eq!(copied, "hello");
    }

    #[test]
    fn test_move_note_moves_file() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.write_note("a.md", "hello").unwrap();
        repo.move_note("a.md", ".djour/archive/a.md").unwrap();

        assert!(!temp.path().join("a.md").exists());
        assert!(temp.path().join(".djour/archive/a.md").exists());
    }

    #[test]
    fn test_write_note_atomic_overwrites() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        repo.write_note("a.md", "one").unwrap();
        repo.write_note_atomic("a.md", "two").unwrap();

        let final_content = fs::read_to_string(temp.path().join("a.md")).unwrap();
        assert_eq!(final_content, "two");
    }

    #[test]
    fn test_write_note_overwrites() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Write initial content
        repo.write_note("test.md", "initial").unwrap();

        // Overwrite with new content
        repo.write_note("test.md", "updated").unwrap();

        // Verify overwrite
        let content = repo.read_note("test.md").unwrap();
        assert_eq!(content, "updated");
    }

    #[test]
    fn test_write_note_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Write to nested path that doesn't exist
        repo.write_note("sub/dir/note.md", "content").unwrap();

        // Verify parent dirs were created
        assert!(temp.path().join("sub").join("dir").exists());
        assert!(temp.path().join("sub").join("dir").join("note.md").exists());
    }

    #[test]
    fn test_list_notes_empty() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, None, false)
            .unwrap();
        assert_eq!(notes.len(), 0);
    }

    #[test]
    fn test_list_notes_daily() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create some note files
        fs::write(temp.path().join("2025-01-17.md"), "note 1").unwrap();
        fs::write(temp.path().join("2025-01-16.md"), "note 2").unwrap();
        fs::write(temp.path().join("2025-01-15.md"), "note 3").unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, None, false)
            .unwrap();

        assert_eq!(notes.len(), 3);
        // Should be sorted newest first
        assert_eq!(notes[0].filename, "2025-01-17.md");
        assert_eq!(notes[1].filename, "2025-01-16.md");
        assert_eq!(notes[2].filename, "2025-01-15.md");
    }

    #[test]
    fn test_list_notes_ignores_other_files() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        // Create note files and other files
        fs::write(temp.path().join("2025-01-17.md"), "note").unwrap();
        fs::write(temp.path().join("readme.txt"), "text").unwrap();
        fs::write(temp.path().join("invalid.md"), "bad").unwrap();
        fs::create_dir(temp.path().join(".djour")).unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, None, false)
            .unwrap();

        // Should only include valid daily note
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].filename, "2025-01-17.md");
    }

    #[test]
    fn test_list_notes_non_recursive_skips_nested_files() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-01-17.md"), "root").unwrap();
        fs::create_dir_all(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested").join("2025-01-18.md"), "nested").unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, None, false)
            .unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].filename, "2025-01-17.md");
    }

    #[test]
    fn test_list_notes_recursive_includes_nested_and_skips_dot_dirs() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-01-15.md"), "root").unwrap();
        fs::create_dir_all(temp.path().join("nested").join("project")).unwrap();
        fs::write(
            temp.path()
                .join("nested")
                .join("project")
                .join("2025-01-16.md"),
            "nested",
        )
        .unwrap();
        fs::create_dir_all(temp.path().join(".hidden")).unwrap();
        fs::write(temp.path().join(".hidden").join("2025-01-17.md"), "hidden").unwrap();
        fs::create_dir_all(temp.path().join("nested").join(".cache")).unwrap();
        fs::write(
            temp.path()
                .join("nested")
                .join(".cache")
                .join("2025-01-18.md"),
            "hidden nested",
        )
        .unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, None, true)
            .unwrap();

        let filenames = notes
            .iter()
            .map(|entry| entry.filename.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            filenames,
            vec!["nested/project/2025-01-16.md", "2025-01-15.md"]
        );
    }

    #[test]
    fn test_list_notes_with_date_range() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-01-10.md"), "note").unwrap();
        fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();
        fs::write(temp.path().join("2025-01-20.md"), "note").unwrap();

        let from = NaiveDate::from_ymd_opt(2025, 1, 12).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 18).unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, Some(from), Some(to), None, false)
            .unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].filename, "2025-01-15.md");
    }

    #[test]
    fn test_list_notes_with_limit() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-01-17.md"), "note").unwrap();
        fs::write(temp.path().join("2025-01-16.md"), "note").unwrap();
        fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();

        let notes = repo
            .list_notes(JournalMode::Daily, None, None, Some(2), false)
            .unwrap();

        assert_eq!(notes.len(), 2);
        // Should get newest 2
        assert_eq!(notes[0].filename, "2025-01-17.md");
        assert_eq!(notes[1].filename, "2025-01-16.md");
    }

    #[test]
    fn test_list_notes_single_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("journal.md"), "note").unwrap();
        fs::write(temp.path().join("2025-01-17.md"), "other").unwrap();

        let notes = repo
            .list_notes(JournalMode::Single, None, None, None, false)
            .unwrap();

        // Should only include journal.md
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].filename, "journal.md");
        assert_eq!(notes[0].date, None);
    }

    #[test]
    fn test_list_notes_weekly_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-W03-2025-01-13.md"), "week 3").unwrap();
        fs::write(temp.path().join("2025-W02-2025-01-06.md"), "week 2").unwrap();
        fs::write(temp.path().join("2025-01-17.md"), "daily").unwrap(); // Should be ignored

        let notes = repo
            .list_notes(JournalMode::Weekly, None, None, None, false)
            .unwrap();

        // Should only include weekly notes
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].filename, "2025-W03-2025-01-13.md");
        assert_eq!(notes[1].filename, "2025-W02-2025-01-06.md");
    }

    #[test]
    fn test_list_notes_monthly_mode() {
        let temp = TempDir::new().unwrap();
        let repo = FileSystemRepository::new(temp.path().to_path_buf());

        fs::write(temp.path().join("2025-01.md"), "jan").unwrap();
        fs::write(temp.path().join("2024-12.md"), "dec").unwrap();
        fs::write(temp.path().join("2025-01-17.md"), "daily").unwrap(); // Should be ignored

        let notes = repo
            .list_notes(JournalMode::Monthly, None, None, None, false)
            .unwrap();

        // Should only include monthly notes
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].filename, "2025-01.md");
        assert_eq!(notes[1].filename, "2024-12.md");
    }

    #[test]
    fn test_discover_with_djour_root_env() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_ROOT");

        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".djour")).unwrap();

        // Set DJOUR_ROOT
        std::env::set_var("DJOUR_ROOT", temp.path());

        let repo = FileSystemRepository::discover().unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_discover_djour_root_not_initialized() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_ROOT");

        let temp = TempDir::new().unwrap();
        // No .djour directory

        std::env::set_var("DJOUR_ROOT", temp.path());

        let result = FileSystemRepository::discover();
        assert!(result.is_err());

        match result.unwrap_err() {
            DjourError::Config(msg) => {
                assert!(msg.contains("no .djour directory"));
            }
            _ => panic!("Expected Config error"),
        }
    }

    #[test]
    fn test_discover_without_djour_root_env() {
        let _env_lock = env_test_lock().lock().unwrap();
        let _restore = EnvVarRestore::capture("DJOUR_ROOT");

        // Ensure DJOUR_ROOT is not set
        std::env::remove_var("DJOUR_ROOT");

        // This test will fail if run outside a djour directory
        // but it tests that the code path works when env var is not set
        let result = FileSystemRepository::discover();

        // Either discovers a journal or fails with NotDjourDirectory
        match result {
            Ok(_) => {}                                 // Found a journal
            Err(DjourError::NotDjourDirectory(_)) => {} // Expected
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}
