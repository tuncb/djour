//! Output formatting utilities

use crate::infrastructure::NoteEntry;

/// Format a list of note entries for display
pub fn format_note_list(notes: &[NoteEntry]) -> String {
    if notes.is_empty() {
        return "No notes found".to_string();
    }

    let mut output = String::new();
    for entry in notes {
        if let Some(date) = entry.date {
            output.push_str(&format!("{}  {}\n", date.format("%Y-%m-%d"), entry.filename));
        } else {
            // No date (single mode) - use spacing for alignment
            output.push_str(&format!("           {}\n", entry.filename));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_format_empty_list() {
        let notes = vec![];
        let output = format_note_list(&notes);
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

        let output = format_note_list(&notes);
        assert!(output.contains("2025-01-17  2025-01-17.md"));
        assert!(output.contains("2025-01-16  2025-01-16.md"));
    }

    #[test]
    fn test_format_single_mode_entry() {
        let notes = vec![NoteEntry::new("journal.md".to_string(), None)];

        let output = format_note_list(&notes);
        assert!(output.contains("journal.md"));
        // Should have spacing for alignment
        assert!(output.contains("           journal.md"));
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

        let output = format_note_list(&notes);
        assert!(output.contains("2025-01-17  2025-01-17.md"));
        assert!(output.contains("           journal.md"));
    }
}
