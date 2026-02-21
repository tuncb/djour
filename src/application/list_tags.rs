//! List tags use case

use crate::error::Result;
use crate::infrastructure::repository::JournalRepository;
use crate::infrastructure::FileSystemRepository;
use chrono::NaiveDate;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

fn tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap())
}

fn collect_tags_from_text(text: &str, output: &mut BTreeSet<String>) {
    for captures in tag_regex().captures_iter(text) {
        output.insert(captures[1].to_lowercase());
    }
}

/// List all tags used in notes with optional date filters.
pub fn list_tags(
    repository: &FileSystemRepository,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    recursive: bool,
) -> Result<Vec<String>> {
    let config = repository.load_config()?;
    let notes = repository.list_notes(config.get_mode(), from, to, None, recursive)?;

    let mut tags = BTreeSet::new();
    for note in notes {
        let content = repository.read_note(&note.filename)?;
        collect_tags_from_text(&content, &mut tags);
    }

    Ok(tags.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_tags_normalizes_and_deduplicates() {
        let mut tags = BTreeSet::new();
        collect_tags_from_text("one #Work and #work and #team_ops", &mut tags);
        assert_eq!(
            tags.into_iter().collect::<Vec<String>>(),
            vec!["team_ops".to_string(), "work".to_string()]
        );
    }

    #[test]
    fn collect_tags_supports_dash_and_numbers() {
        let mut tags = BTreeSet::new();
        collect_tags_from_text("Tasks: #project-alpha #task1", &mut tags);
        assert_eq!(
            tags.into_iter().collect::<Vec<String>>(),
            vec!["project-alpha".to_string(), "task1".to_string()]
        );
    }
}
