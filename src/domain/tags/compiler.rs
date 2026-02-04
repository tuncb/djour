//! Tag compilation logic - filtering and grouping tagged content
//!
//! This module provides functionality to filter, sort, and format tagged content
//! into markdown compilations.

use super::{TagContext, TagQuery, TaggedContent};
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;

/// Format for compiled output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationFormat {
    /// Chronological order (by date)
    Chronological,
    /// Grouped by source file
    Grouped,
}

/// How to display dates in compiled output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationDateStyle {
    /// Display a single date (e.g., 15-01-2025)
    SingleDate,
    /// Display a week range (start to end)
    WeekRange,
    /// Display a month range (start to end)
    MonthRange,
}

/// Compiler for filtering and organizing tagged content
pub struct TagCompiler;

impl TagCompiler {
    /// Filter tagged content by query
    ///
    /// # Examples
    ///
    /// ```
    /// use djour::domain::tags::{TagCompiler, TagQuery, TaggedContent, TagContext};
    /// use std::path::PathBuf;
    ///
    /// let content = vec![
    ///     TaggedContent {
    ///         tags: vec!["work".to_string(), "urgent".to_string()],
    ///         content: "Important meeting".to_string(),
    ///         source_file: PathBuf::from("2025-01-15.md"),
    ///         date: None,
    ///         context: TagContext::Paragraph,
    ///     },
    /// ];
    ///
    /// let query = TagQuery::parse("work").unwrap();
    /// let filtered = TagCompiler::filter(content, &query);
    /// assert_eq!(filtered.len(), 1);
    /// ```
    pub fn filter(content: Vec<TaggedContent>, query: &TagQuery) -> Vec<TaggedContent> {
        content
            .into_iter()
            .filter(|tc| query.matches(&tc.tags))
            .collect()
    }

    /// Sort content chronologically (by date, then by source file)
    ///
    /// Items without dates are sorted last.
    pub fn sort_chronological(mut content: Vec<TaggedContent>) -> Vec<TaggedContent> {
        content.sort_by(|a, b| match (a.date, b.date) {
            (Some(da), Some(db)) => da.cmp(&db).then_with(|| a.source_file.cmp(&b.source_file)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.source_file.cmp(&b.source_file),
        });
        content
    }

    /// Group content by source file
    ///
    /// Returns a vector of (filename, content) tuples, sorted by filename.
    pub fn group_by_file(content: Vec<TaggedContent>) -> Vec<(String, Vec<TaggedContent>)> {
        let mut groups: HashMap<String, Vec<TaggedContent>> = HashMap::new();

        for tc in content {
            let filename = tc
                .source_file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            groups.entry(filename.clone()).or_default().push(tc);
        }

        // Sort by filename
        let mut result: Vec<_> = groups.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Generate markdown output for compiled content
    ///
    /// # Arguments
    ///
    /// * `content` - Tagged content to include
    /// * `query` - The query that was used (for the title)
    /// * `format` - Output format (chronological or grouped)
    /// * `date_style` - How to display dates in headers
    /// * `include_context` - Whether to include parent section headings
    ///
    /// # Examples
    ///
    /// ```
    /// use djour::domain::tags::{
    ///     CompilationDateStyle, CompilationFormat, TagCompiler, TagContext, TagQuery, TaggedContent,
    /// };
    /// use std::path::PathBuf;
    /// use chrono::NaiveDate;
    ///
    /// let content = vec![
    ///     TaggedContent {
    ///         tags: vec!["work".to_string()],
    ///         content: "Meeting notes".to_string(),
    ///         source_file: PathBuf::from("2025-01-15.md"),
    ///         date: NaiveDate::from_ymd_opt(2025, 1, 15),
    ///         context: TagContext::Section {
    ///             heading: "Work Notes".to_string(),
    ///             level: 1,
    ///         },
    ///     },
    /// ];
    ///
    /// let query = TagQuery::parse("work").unwrap();
    /// let markdown = TagCompiler::to_markdown(
    ///     content,
    ///     &query,
    ///     CompilationFormat::Chronological,
    ///     CompilationDateStyle::SingleDate,
    ///     false,
    /// );
    /// assert!(markdown.contains("# Compilation: #work"));
    /// assert!(markdown.contains("## 15-01-2025"));
    /// ```
    pub fn to_markdown(
        content: Vec<TaggedContent>,
        query: &TagQuery,
        format: CompilationFormat,
        date_style: CompilationDateStyle,
        include_context: bool,
    ) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("# Compilation: {}\n\n", query));

        if content.is_empty() {
            output.push_str("*No matching content found.*\n");
            return output;
        }

        match format {
            CompilationFormat::Chronological => {
                Self::markdown_chronological(content, date_style, include_context, &mut output);
            }
            CompilationFormat::Grouped => {
                Self::markdown_grouped(content, date_style, include_context, &mut output);
            }
        }

        output
    }

    /// Generate chronological markdown output
    fn markdown_chronological(
        content: Vec<TaggedContent>,
        date_style: CompilationDateStyle,
        include_context: bool,
        output: &mut String,
    ) {
        let sorted = Self::sort_chronological(content);
        let mut current_date: Option<NaiveDate> = None;

        for tc in sorted {
            // Date header (if changed)
            if tc.date != current_date {
                if let Some(date) = tc.date {
                    let header = Self::format_date_header(date, date_style);
                    output.push_str(&format!("\n## {}\n\n", header));
                    current_date = tc.date;
                } else if current_date.is_some() {
                    // Switch to undated section
                    output.push_str("\n## Undated\n\n");
                    current_date = None;
                }
            }

            // Context heading (if available and requested)
            if include_context {
                if let TagContext::Section { heading, level } = &tc.context {
                    if !heading.trim().is_empty() {
                        let prefix = "#".repeat(*level + 2); // Base level 2 (##) + section level
                        output.push_str(&format!("{} {}\n\n", prefix, heading));
                    }
                }
            }

            // Content
            output.push_str(&tc.content);
            output.push_str("\n\n");
        }
    }

    /// Generate grouped markdown output
    fn markdown_grouped(
        content: Vec<TaggedContent>,
        date_style: CompilationDateStyle,
        include_context: bool,
        output: &mut String,
    ) {
        let groups = Self::group_by_file(content);

        for (filename, items) in groups {
            if date_style != CompilationDateStyle::SingleDate {
                if let Some(date) = items.iter().find_map(|tc| tc.date) {
                    let header = Self::format_date_header(date, date_style);
                    output.push_str(&format!("\n## From: {} ({})\n\n", filename, header));
                } else {
                    output.push_str(&format!("\n## From: {}\n\n", filename));
                }
            } else {
                output.push_str(&format!("\n## From: {}\n\n", filename));
            }

            for tc in items {
                // Context heading (if available and requested)
                if include_context {
                    if let TagContext::Section { heading, level } = &tc.context {
                        if !heading.trim().is_empty() {
                            let prefix = "#".repeat(*level + 2); // Base level 2 (##) + section level
                            output.push_str(&format!("{} {}\n\n", prefix, heading));
                        }
                    }
                }

                // Content
                output.push_str(&tc.content);
                output.push_str("\n\n");
            }
        }
    }

    fn format_date_header(date: NaiveDate, date_style: CompilationDateStyle) -> String {
        match date_style {
            CompilationDateStyle::SingleDate => date.format("%d-%m-%Y").to_string(),
            CompilationDateStyle::WeekRange => {
                let end = date + Duration::days(6);
                format!("{} to {}", date.format("%d-%m-%Y"), end.format("%d-%m-%Y"))
            }
            CompilationDateStyle::MonthRange => {
                let end = Self::end_of_month(date);
                format!("{} to {}", date.format("%d-%m-%Y"), end.format("%d-%m-%Y"))
            }
        }
    }

    fn end_of_month(date: NaiveDate) -> NaiveDate {
        let year = date.year();
        let month = date.month();
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid month");
        first_next - Duration::days(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn create_test_content(
        tags: Vec<&str>,
        content: &str,
        filename: &str,
        date: Option<NaiveDate>,
    ) -> TaggedContent {
        TaggedContent {
            tags: tags.iter().map(|s| s.to_string()).collect(),
            content: content.to_string(),
            source_file: PathBuf::from(filename),
            date,
            context: TagContext::Paragraph,
        }
    }

    #[test]
    fn test_filter_single_tag() {
        let content = vec![
            create_test_content(vec!["work"], "Work content", "a.md", None),
            create_test_content(vec!["personal"], "Personal content", "b.md", None),
        ];

        let query = TagQuery::parse("work").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].tags, vec!["work".to_string()]);
    }

    #[test]
    fn test_filter_and_query() {
        let content = vec![
            create_test_content(vec!["work", "urgent"], "Urgent work", "a.md", None),
            create_test_content(vec!["work"], "Regular work", "b.md", None),
            create_test_content(vec!["urgent"], "Urgent personal", "c.md", None),
        ];

        let query = TagQuery::parse("work AND urgent").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "Urgent work");
    }

    #[test]
    fn test_filter_or_query() {
        let content = vec![
            create_test_content(vec!["work"], "Work content", "a.md", None),
            create_test_content(vec!["personal"], "Personal content", "b.md", None),
            create_test_content(vec!["hobby"], "Hobby content", "c.md", None),
        ];

        let query = TagQuery::parse("work OR personal").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_not_query() {
        let content = vec![
            create_test_content(vec!["work", "meeting"], "Meeting", "a.md", None),
            create_test_content(vec!["work", "coding"], "Coding", "b.md", None),
            create_test_content(vec!["personal"], "Personal", "c.md", None),
        ];

        let query = TagQuery::parse("work AND NOT meeting").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "Coding");
    }

    #[test]
    fn test_sort_chronological() {
        let content = vec![
            create_test_content(
                vec!["work"],
                "Content C",
                "c.md",
                NaiveDate::from_ymd_opt(2025, 1, 17),
            ),
            create_test_content(
                vec!["work"],
                "Content A",
                "a.md",
                NaiveDate::from_ymd_opt(2025, 1, 15),
            ),
            create_test_content(
                vec!["work"],
                "Content B",
                "b.md",
                NaiveDate::from_ymd_opt(2025, 1, 16),
            ),
        ];

        let sorted = TagCompiler::sort_chronological(content);

        assert_eq!(sorted[0].content, "Content A");
        assert_eq!(sorted[1].content, "Content B");
        assert_eq!(sorted[2].content, "Content C");
    }

    #[test]
    fn test_sort_chronological_with_none() {
        let content = vec![
            create_test_content(vec!["work"], "No date", "z.md", None),
            create_test_content(
                vec!["work"],
                "Has date",
                "a.md",
                NaiveDate::from_ymd_opt(2025, 1, 15),
            ),
        ];

        let sorted = TagCompiler::sort_chronological(content);

        // Dated items should come first
        assert_eq!(sorted[0].content, "Has date");
        assert_eq!(sorted[1].content, "No date");
    }

    #[test]
    fn test_group_by_file() {
        let content = vec![
            create_test_content(vec!["work"], "Content 1", "a.md", None),
            create_test_content(vec!["work"], "Content 2", "b.md", None),
            create_test_content(vec!["work"], "Content 3", "a.md", None),
        ];

        let grouped = TagCompiler::group_by_file(content);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, "a.md");
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[1].0, "b.md");
        assert_eq!(grouped[1].1.len(), 1);
    }

    #[test]
    fn test_to_markdown_chronological() {
        let content = vec![create_test_content(
            vec!["work"],
            "Meeting notes",
            "2025-01-15.md",
            NaiveDate::from_ymd_opt(2025, 1, 15),
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::SingleDate,
            false,
        );

        assert!(markdown.contains("# Compilation: #work"));
        assert!(markdown.contains("## 15-01-2025"));
        assert!(markdown.contains("Meeting notes"));
    }

    #[test]
    fn test_to_markdown_week_range() {
        let content = vec![create_test_content(
            vec!["work"],
            "Weekly notes",
            "2025-W03-2025-01-13.md",
            NaiveDate::from_ymd_opt(2025, 1, 13),
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::WeekRange,
            false,
        );

        assert!(markdown.contains("## 13-01-2025 to 19-01-2025"));
        assert!(markdown.contains("Weekly notes"));
    }

    #[test]
    fn test_to_markdown_month_range() {
        let content = vec![create_test_content(
            vec!["work"],
            "Monthly notes",
            "2025-02.md",
            NaiveDate::from_ymd_opt(2025, 2, 1),
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::MonthRange,
            false,
        );

        assert!(markdown.contains("## 01-02-2025 to 28-02-2025"));
        assert!(markdown.contains("Monthly notes"));
    }

    #[test]
    fn test_to_markdown_grouped() {
        let content = vec![create_test_content(
            vec!["work"],
            "Meeting notes",
            "2025-01-15.md",
            None,
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Grouped,
            CompilationDateStyle::SingleDate,
            false,
        );

        assert!(markdown.contains("# Compilation: #work"));
        assert!(markdown.contains("## From: 2025-01-15.md"));
        assert!(markdown.contains("Meeting notes"));
    }

    #[test]
    fn test_to_markdown_empty() {
        let content = vec![];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::SingleDate,
            false,
        );

        assert!(markdown.contains("# Compilation: #work"));
        assert!(markdown.contains("*No matching content found.*"));
    }

    #[test]
    fn test_to_markdown_with_context() {
        let content = vec![TaggedContent {
            tags: vec!["work".to_string()],
            content: "Meeting notes".to_string(),
            source_file: PathBuf::from("2025-01-15.md"),
            date: NaiveDate::from_ymd_opt(2025, 1, 15),
            context: TagContext::Section {
                heading: "Work Notes".to_string(),
                level: 1,
            },
        }];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::SingleDate,
            true,
        );

        assert!(markdown.contains("### Work Notes")); // Level 1 + 2 = ###
        assert!(markdown.contains("Meeting notes"));
    }

    #[test]
    fn test_to_markdown_multiple_dates() {
        let content = vec![
            create_test_content(
                vec!["work"],
                "First day",
                "a.md",
                NaiveDate::from_ymd_opt(2025, 1, 15),
            ),
            create_test_content(
                vec!["work"],
                "Second day",
                "b.md",
                NaiveDate::from_ymd_opt(2025, 1, 16),
            ),
        ];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(
            content,
            &query,
            CompilationFormat::Chronological,
            CompilationDateStyle::SingleDate,
            false,
        );

        assert!(markdown.contains("## 15-01-2025"));
        assert!(markdown.contains("## 16-01-2025"));
        assert!(markdown.contains("First day"));
        assert!(markdown.contains("Second day"));
    }
}
