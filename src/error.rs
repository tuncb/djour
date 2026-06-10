//! Error types for djour

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for djour application
#[derive(Debug, Error)]
pub enum DjourError {
    #[error("Not a djour directory: {0}")]
    NotDjourDirectory(PathBuf),

    #[error("Invalid time reference: {0}")]
    InvalidTimeReference(String),

    #[error("Time reference '{time_ref}' is only valid in {required_mode:?} mode, but current mode is {current_mode:?}")]
    TimeReferenceModeMismatch {
        time_ref: String,
        required_mode: crate::domain::JournalMode,
        current_mode: crate::domain::JournalMode,
        period: &'static str,
    },

    #[error("Tag not found: {0}")]
    TagNotFound(String),

    #[error("Parse error in {}: {}", file.display(), message)]
    Parse {
        file: PathBuf,
        line: Option<usize>,
        message: String,
        context: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Editor error: {0}")]
    Editor(String),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

impl DjourError {
    /// Get the exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            DjourError::NotDjourDirectory(_) => 2,
            DjourError::InvalidTimeReference(_) | DjourError::TimeReferenceModeMismatch { .. } => 3,
            DjourError::TagNotFound(_) => 4,
            _ => 1,
        }
    }

    /// Get a user-friendly error message with suggestions
    pub fn display_with_suggestions(&self) -> String {
        match self {
            DjourError::NotDjourDirectory(path) => {
                format!(
                    "Not a djour directory: {}\n\n\
                    Suggestions:\n\
                    • Run 'djour init' in this directory to create a new journal\n\
                    • Navigate to an existing djour directory\n\
                    • Set DJOUR_ROOT environment variable to your journal path",
                    path.display()
                )
            }
            DjourError::InvalidTimeReference(ref_str) => {
                format!(
                    "Invalid time reference: '{}'\n\n\
                    Valid time references:\n\
                    • today, yesterday, tomorrow\n\
                    • monday, tuesday, ..., sunday (most recent)\n\
                    • last monday, next friday, etc.\n\
                    • day +2, day -2 (daily mode only)\n\
                    • last week, week +2, week -2 (weekly mode only)\n\
                    • Specific dates: DD-MM-YYYY (e.g., 17-01-2025)\n\n\
                    Examples:\n\
                    djour today\n\
                    djour last monday\n\
                    djour day +2\n\
                    djour week -2\n\
                    djour 17-01-2025",
                    ref_str
                )
            }
            DjourError::TimeReferenceModeMismatch {
                time_ref,
                required_mode,
                current_mode,
                period,
            } => {
                format!(
                    "Time reference '{}' is only valid in {} mode, but current mode is {}.\n\n\
                    Suggestions:\n\
                    • Use this {} reference in a {} journal\n\
                    • Change mode with 'djour config mode {}'\n\
                    • Use a general time reference such as today, yesterday, tomorrow, a weekday, or a DD-MM-YYYY date",
                    time_ref,
                    format!("{:?}", required_mode).to_lowercase(),
                    format!("{:?}", current_mode).to_lowercase(),
                    period,
                    format!("{:?}", required_mode).to_lowercase(),
                    format!("{:?}", required_mode).to_lowercase()
                )
            }
            DjourError::TagNotFound(tag) => {
                format!(
                    "No content found matching query: '{}'\n\n\
                    Suggestions:\n\
                    • Check your tag spelling (tags are case-insensitive)\n\
                    • Use 'djour list' to see available notes\n\
                    • Tags must start with # in your notes (e.g., #work)\n\
                    • Try a broader query (e.g., 'work OR personal')",
                    tag
                )
            }
            DjourError::Parse {
                file,
                line,
                message,
                context,
            } => {
                let location = match line {
                    Some(line) => format!("{}:{}", file.display(), line),
                    None => file.display().to_string(),
                };
                format!(
                    "Failed to parse tagged content in {}.\n\n\
                    Context:\n{}\n\n\
                    Reason: {}\n\n\
                    Suggestions:\n\
                    • Check the nearby Markdown heading, list, code fence, or front matter syntax\n\
                    • If the Markdown looks valid, report this with the file and context above",
                    location, context, message
                )
            }
            DjourError::Editor(msg) => {
                format!(
                    "{}\n\n\
                    Suggestions:\n\
                    • Check that your editor is installed and in PATH\n\
                    • Set EDITOR environment variable (e.g., export EDITOR=nano)\n\
                    • Configure editor: djour config editor 'vim'\n\
                    • Try a different editor: djour config editor 'notepad'",
                    msg
                )
            }
            DjourError::Config(msg) => {
                if msg.contains("Invalid mode") {
                    format!(
                        "{}\n\n\
                        Valid modes: daily, weekly, monthly, single\n\
                        Example: djour config mode weekly",
                        msg
                    )
                } else if msg.contains("date format") {
                    format!(
                        "{}\n\n\
                        Expected format: DD-MM-YYYY\n\
                        Example: djour list --from 17-01-2025 --to 31-01-2025",
                        msg
                    )
                } else {
                    msg.clone()
                }
            }
            _ => self.to_string(),
        }
    }
}

/// Result type using DjourError
pub type Result<T> = std::result::Result<T, DjourError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_djour_directory_suggestion() {
        let err = DjourError::NotDjourDirectory(PathBuf::from("/tmp/test"));
        let msg = err.display_with_suggestions();
        assert!(msg.contains("djour init"));
        assert!(msg.contains("DJOUR_ROOT"));
        assert!(msg.contains("Suggestions"));
    }

    #[test]
    fn test_invalid_time_reference_examples() {
        let err = DjourError::InvalidTimeReference("baddate".to_string());
        let msg = err.display_with_suggestions();
        assert!(msg.contains("today"));
        assert!(msg.contains("DD-MM-YYYY"));
        assert!(msg.contains("Examples"));
        assert!(msg.contains("djour today"));
    }

    #[test]
    fn test_time_reference_mode_mismatch_suggestion() {
        let err = DjourError::TimeReferenceModeMismatch {
            time_ref: "week -2".to_string(),
            required_mode: crate::domain::JournalMode::Weekly,
            current_mode: crate::domain::JournalMode::Daily,
            period: "week",
        };
        let msg = err.display_with_suggestions();
        assert!(msg.contains("weekly mode"));
        assert!(msg.contains("current mode is daily"));
        assert!(msg.contains("djour config mode weekly"));
    }

    #[test]
    fn test_tag_not_found_suggestions() {
        let err = DjourError::TagNotFound("nonexistent".to_string());
        let msg = err.display_with_suggestions();
        assert!(msg.contains("djour list"));
        assert!(msg.contains("case-insensitive"));
        assert!(msg.contains("broader query"));
    }

    #[test]
    fn test_parse_error_shows_file_context_and_reason() {
        let err = DjourError::Parse {
            file: PathBuf::from("2025-01-15.md"),
            line: Some(5),
            message: "parser panicked: test failure".to_string(),
            context: "   5 | Button #work".to_string(),
        };
        let msg = err.display_with_suggestions();

        assert!(msg.contains("2025-01-15.md:5"));
        assert!(msg.contains("Button #work"));
        assert!(msg.contains("parser panicked"));
        assert!(msg.contains("Markdown"));
    }

    #[test]
    fn test_editor_error_suggestions() {
        let err = DjourError::Editor("Editor not found".to_string());
        let msg = err.display_with_suggestions();
        assert!(msg.contains("EDITOR environment variable"));
        assert!(msg.contains("djour config editor"));
        assert!(msg.contains("PATH"));
    }

    #[test]
    fn test_config_invalid_mode_suggestions() {
        let err = DjourError::Config("Invalid mode: xyz".to_string());
        let msg = err.display_with_suggestions();
        assert!(msg.contains("daily, weekly, monthly, single"));
        assert!(msg.contains("djour config mode weekly"));
    }

    #[test]
    fn test_config_date_format_suggestions() {
        let err = DjourError::Config("Invalid date format: 2025-01-17".to_string());
        let msg = err.display_with_suggestions();
        assert!(msg.contains("DD-MM-YYYY"));
        assert!(msg.contains("17-01-2025"));
    }

    #[test]
    fn test_other_errors_fallback() {
        let err = DjourError::Template("Template error".to_string());
        let msg = err.display_with_suggestions();
        // Thiserror prefixes with the error type
        assert_eq!(msg, "Template error: Template error");
    }
}
