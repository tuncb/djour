//! Output formatting utilities

use crate::infrastructure::NoteEntry;
use std::path::Path;

/// Format a list of note entries for display
pub fn format_note_list(notes: &[NoteEntry], root: &Path) -> String {
    if notes.is_empty() {
        return "No notes found".to_string();
    }

    let mut output = String::new();
    for entry in notes {
        let full_path = root.join(&entry.filename);
        if let Some(date) = entry.date {
            output.push_str(&format!(
                "{}  {}\n",
                date.format("%d-%m-%Y"),
                full_path.display()
            ));
        } else {
            // No date (single mode) - use spacing for alignment
            output.push_str(&format!("           {}\n", full_path.display()));
        }
    }
    output
}

/// Format a list of tags for display.
pub fn format_tag_list(tags: &[String]) -> String {
    if tags.is_empty() {
        return "No tags found".to_string();
    }

    let mut output = String::new();
    for tag in tags {
        output.push_str(&format!("#{}\n", tag));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::path::Path;

    #[test]
    fn test_format_empty_list() {
        let notes = vec![];
        let output = format_note_list(&notes, Path::new("/tmp/journal"));
        assert_eq!(output, "No notes found");
    }

    #[test]
    fn test_format_note_list() {
        let notes = vec![
            NoteEntry::new(
                "2025-01-17.md".to_string(),
                Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap()),
            ),
            NoteEntry::new(
                "2025-01-16.md".to_string(),
                Some(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap()),
            ),
        ];

        let root = Path::new("/tmp/journal");
        let output = format_note_list(&notes, root);
        assert!(output.contains("17-01-2025"));
        assert!(output.contains(&root.join("2025-01-17.md").display().to_string()));
        assert!(output.contains("16-01-2025"));
        assert!(output.contains(&root.join("2025-01-16.md").display().to_string()));
    }

    #[test]
    fn test_format_single_mode_entry() {
        let notes = vec![NoteEntry::new("journal.md".to_string(), None)];
        let root = Path::new("/tmp/journal");

        let output = format_note_list(&notes, root);
        assert!(output.contains(&root.join("journal.md").display().to_string()));
    }

    #[test]
    fn test_format_mixed_entries() {
        let notes = vec![
            NoteEntry::new(
                "2025-01-17.md".to_string(),
                Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap()),
            ),
            NoteEntry::new("journal.md".to_string(), None),
        ];
        let root = Path::new("/tmp/journal");

        let output = format_note_list(&notes, root);
        assert!(output.contains("17-01-2025"));
        assert!(output.contains(&root.join("2025-01-17.md").display().to_string()));
        assert!(output.contains(&root.join("journal.md").display().to_string()));
    }

    #[test]
    fn test_format_empty_tag_list() {
        let tags = vec![];
        let output = format_tag_list(&tags);
        assert_eq!(output, "No tags found");
    }

    #[test]
    fn test_format_tag_list() {
        let tags = vec!["personal".to_string(), "work".to_string()];
        let output = format_tag_list(&tags);
        assert_eq!(output, "#personal\n#work\n");
    }
}
