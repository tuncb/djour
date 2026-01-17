//! Editor integration for opening note files

use crate::error::{DjourError, Result};
use std::path::Path;
use std::process::Command;

/// Session for opening files in an external editor
pub struct EditorSession {
    command: String,
}

impl EditorSession {
    /// Create a new editor session with the given command
    pub fn new(editor_command: String) -> Self {
        EditorSession {
            command: editor_command,
        }
    }

    /// Open a file in the editor and return immediately
    pub fn open(&self, file_path: &Path) -> Result<()> {
        let (program, args) = self.parse_command();

        // Add file path as final argument
        let mut all_args = args;
        all_args.push(file_path.to_string_lossy().to_string());

        // On Windows, use cmd /c to ensure .bat and .cmd files are found
        #[cfg(windows)]
        {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(&program).args(&all_args);
            cmd.spawn().map_err(|e| {
                DjourError::Editor(format!("Failed to launch editor '{}': {}", program, e))
            })?;
        }

        // On Unix, use the program directly
        #[cfg(not(windows))]
        {
            Command::new(&program)
                .args(&all_args)
                .spawn()
                .map_err(|e| {
                    DjourError::Editor(format!("Failed to launch editor '{}': {}", program, e))
                })?;
        }

        Ok(())
    }

    /// Parse command into program and arguments
    fn parse_command(&self) -> (String, Vec<String>) {
        let parts: Vec<&str> = self.command.split_whitespace().collect();

        if parts.is_empty() {
            // Fallback to notepad if command is empty
            return ("notepad".to_string(), vec![]);
        }

        let program = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        (program, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        let session = EditorSession::new("vim".to_string());
        let (program, args) = session.parse_command();

        assert_eq!(program, "vim");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_parse_command_with_args() {
        let session = EditorSession::new("code -w".to_string());
        let (program, args) = session.parse_command();

        assert_eq!(program, "code");
        assert_eq!(args, vec!["-w"]);
    }

    #[test]
    fn test_parse_command_multiple_args() {
        let session = EditorSession::new("vim +10 -c startinsert".to_string());
        let (program, args) = session.parse_command();

        assert_eq!(program, "vim");
        assert_eq!(args, vec!["+10", "-c", "startinsert"]);
    }

    #[test]
    fn test_parse_command_empty() {
        let session = EditorSession::new("".to_string());
        let (program, args) = session.parse_command();

        // Empty command falls back to notepad
        assert_eq!(program, "notepad");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_parse_command_with_spaces() {
        let session = EditorSession::new("  vim  -n  ".to_string());
        let (program, args) = session.parse_command();

        assert_eq!(program, "vim");
        assert_eq!(args, vec!["-n"]);
    }
}
