//! Tag compilation use case
//!
//! Orchestrates the full workflow of compiling tagged content from journal entries.

use crate::domain::tags::{TagCompiler, TagParser, TagQuery, TaggedContent};
use crate::domain::{load_template, JournalMode, Template};
use crate::error::{DjourError, Result};
use crate::infrastructure::repository::JournalRepository;
use crate::infrastructure::FileSystemRepository;
use chrono::NaiveDate;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Options for compilation
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Tag query to filter by
    pub query: String,

    /// Output file path (None = default: .compilations/<tag>.md)
    pub output: Option<PathBuf>,

    /// Start date filter (inclusive)
    pub from: Option<NaiveDate>,

    /// End date filter (inclusive)
    pub to: Option<NaiveDate>,

    /// Search notes recursively (excluding directories that start with '.')
    pub recursive: bool,
}

/// Compile tagged content into an output markdown file.
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
pub fn compile_tags(repository: &FileSystemRepository, options: CompileOptions) -> Result<PathBuf> {
    // 1. Parse query
    let query = TagQuery::parse(&options.query)?;

    // 2. Load config to get mode
    let config = repository.load_config()?;
    let mode = config.get_mode();
    let section_one_template = load_section_one_template(repository.root(), mode)?;

    // 3. Determine output path
    let output_path = if let Some(path) = options.output.clone() {
        // Use provided path
        if path.is_absolute() {
            path
        } else {
            repository.root().join(path)
        }
    } else {
        // Default: .compilations/<query>.md (sanitize query string)
        let sanitized = sanitize_filename(&options.query);
        repository
            .root()
            .join(".compilations")
            .join(format!("{}.md", sanitized))
    };

    // 4. List all note files (with date filters)
    let notes = repository.list_notes(
        mode,
        options.from,
        options.to,
        None, // No limit - get all notes
        options.recursive,
    )?;

    if notes.is_empty() {
        return Err(DjourError::TagNotFound(format!(
            "No notes found for query: {}",
            options.query
        )));
    }

    // 5. Parse all files and extract tagged content
    let mut all_content: Vec<TaggedContent> = Vec::new();

    // Use repository-relative source paths so grouped output can include subdirectories.
    let output_context = output_path.strip_prefix(repository.root()).ok();

    for note in notes {
        let content = repository.read_note(&note.filename)?;
        if content.is_empty() {
            continue;
        }

        let file_path = PathBuf::from(&note.filename);
        let tagged = extract_tagged_content_safely(
            &content,
            &file_path,
            note.date,
            output_context,
            &options.query,
        );
        let tagged = tagged?;

        let section_one =
            section_one_label_for_note(section_one_template.as_ref(), mode, note.date, &file_path);

        all_content.extend(tagged.into_iter().map(|mut item| {
            item.section_one = section_one.clone();
            item
        }));
    }

    // 6. Filter by query
    let filtered = TagCompiler::filter(all_content, &query);

    if filtered.is_empty() {
        return Err(DjourError::TagNotFound(format!(
            "No content found matching query: {}",
            options.query
        )));
    }

    // 7. Generate markdown output
    let markdown = TagCompiler::to_markdown_for_output(filtered, &query, output_context);

    // 8. Write output file
    // Convert absolute path to relative for repository.write_note
    let relative_path = output_path.strip_prefix(repository.root()).map_err(|_| {
        DjourError::Config("Output path must be within journal directory".to_string())
    })?;

    let relative_str = relative_path
        .to_str()
        .ok_or_else(|| DjourError::Config("Invalid output path".to_string()))?;

    repository.write_note(relative_str, &markdown)?;

    Ok(output_path)
}

fn load_section_one_template(
    repo_root: &std::path::Path,
    mode: JournalMode,
) -> Result<Option<Template>> {
    if matches!(mode, JournalMode::Single) {
        return Ok(None);
    }

    load_template(repo_root, mode.template_name()).map(Some)
}

fn extract_tagged_content_safely(
    content: &str,
    file_path: &std::path::Path,
    date: Option<NaiveDate>,
    output_context: Option<&std::path::Path>,
    query: &str,
) -> Result<Vec<TaggedContent>> {
    with_parse_panic_context(file_path, content, query, || {
        TagParser::extract_from_markdown_for_output(content, file_path, date, output_context)
    })
}

fn with_parse_panic_context<T, F>(
    file_path: &std::path::Path,
    content: &str,
    query: &str,
    parse: F,
) -> Result<T>
where
    F: FnOnce() -> T,
{
    let _panic_hook_guard = parse_panic_hook_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let captured_panic = Arc::new(Mutex::new(None::<String>));
    let hook_capture = Arc::clone(&captured_panic);
    let previous_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        let mut message = panic_payload_message(info.payload());
        if let Some(location) = info.location() {
            message.push_str(&format!(" at {}:{}", location.file(), location.line()));
        }
        let mut capture = hook_capture
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *capture = Some(message);
    }));

    let parse_result = panic::catch_unwind(AssertUnwindSafe(parse));

    panic::set_hook(previous_hook);

    match parse_result {
        Ok(tagged) => Ok(tagged),
        Err(payload) => {
            let captured = captured_panic
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let message = captured.unwrap_or_else(|| panic_payload_message(payload.as_ref()));
            Err(parse_error_for_file(
                file_path,
                content,
                format!(
                    "parser panicked while compiling query {:?}: {}",
                    query, message
                ),
            ))
        }
    }
}

fn parse_panic_hook_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn parse_error_for_file(file_path: &std::path::Path, content: &str, message: String) -> DjourError {
    let excerpt = source_excerpt(content);
    DjourError::Parse {
        file: file_path.to_path_buf(),
        line: excerpt.focus_line,
        message,
        context: excerpt.context,
    }
}

struct SourceExcerpt {
    focus_line: Option<usize>,
    context: String,
}

fn source_excerpt(content: &str) -> SourceExcerpt {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return SourceExcerpt {
            focus_line: Some(1),
            context: "   1 | ".to_string(),
        };
    }

    let focus_idx = lines
        .iter()
        .position(|line| line.contains('#'))
        .or_else(|| lines.iter().position(|line| !line.trim().is_empty()))
        .unwrap_or(0);
    let start = focus_idx.saturating_sub(2);
    let end = (focus_idx + 3).min(lines.len());
    let context = (start..end)
        .map(|idx| format!("{:>4} | {}", idx + 1, lines[idx]))
        .collect::<Vec<_>>()
        .join("\n");

    SourceExcerpt {
        focus_line: Some(focus_idx + 1),
        context,
    }
}

fn section_one_label_for_note(
    template: Option<&Template>,
    mode: JournalMode,
    date: Option<NaiveDate>,
    file_path: &std::path::Path,
) -> Option<String> {
    if !matches!(mode, JournalMode::Single) {
        if let (Some(template), Some(date)) = (template, date) {
            if let Some(label) = template.rendered_first_h1(date) {
                return Some(label);
            }

            return Some(date.format("%d-%m-%Y").to_string());
        }
    }

    let fallback = file_path
        .to_string_lossy()
        .replace('\\', "/")
        .trim()
        .to_string();
    if fallback.is_empty() {
        None
    } else {
        Some(fallback)
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
    use crate::domain::JournalMode;
    use std::path::{Path, PathBuf};

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

    #[test]
    fn test_section_one_label_for_daily_note_uses_rendered_template_header() {
        let template = Template::from_builtin("daily.md").unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap();

        let label = section_one_label_for_note(
            Some(&template),
            JournalMode::Daily,
            Some(date),
            Path::new("2025-01-17.md"),
        );

        assert_eq!(label.as_deref(), Some("January 17, 2025"));
    }

    #[test]
    fn test_section_one_label_for_single_note_falls_back_to_filename() {
        let label =
            section_one_label_for_note(None, JournalMode::Single, None, Path::new("journal.md"));

        assert_eq!(label.as_deref(), Some("journal.md"));
    }

    #[test]
    fn test_source_excerpt_prefers_tagged_line() {
        let excerpt = source_excerpt("Title\n\nBody #work\nNext line");

        assert_eq!(excerpt.focus_line, Some(3));
        assert!(excerpt.context.contains("   3 | Body #work"));
        assert!(excerpt.context.contains("   4 | Next line"));
    }

    #[test]
    fn test_parse_panic_is_reported_as_parse_error() {
        let result = with_parse_panic_context(
            Path::new("2025-01-15.md"),
            "Title\n\nBody #work\n",
            "work",
            || -> Vec<TaggedContent> { panic!("synthetic parser failure") },
        );

        match result {
            Err(DjourError::Parse {
                file,
                line,
                message,
                context,
            }) => {
                assert_eq!(file, PathBuf::from("2025-01-15.md"));
                assert_eq!(line, Some(3));
                assert!(message.contains("synthetic parser failure"));
                assert!(message.contains("work"));
                assert!(context.contains("Body #work"));
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    // Integration tests would require setting up a FileSystemRepository with temp directories
    // Those will be covered in the integration test file
}
