//! Tag compilation logic - filtering and rendering tagged content.

use super::{TagContext, TagQuery, TaggedContent};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
struct SectionOneGroup {
    intro: Vec<TaggedContent>,
    section_two_order: Vec<String>,
    section_two_items: HashMap<String, Vec<TaggedContent>>,
}

/// Compiler for filtering and organizing tagged content.
pub struct TagCompiler;

impl TagCompiler {
    /// Filter tagged content by query.
    pub fn filter(content: Vec<TaggedContent>, query: &TagQuery) -> Vec<TaggedContent> {
        let matched: Vec<TaggedContent> = content
            .into_iter()
            .filter(|tc| query.matches(&tc.tags))
            .collect();

        Self::dedupe_contained_in_section(matched)
    }

    fn dedupe_contained_in_section(content: Vec<TaggedContent>) -> Vec<TaggedContent> {
        let mut deduped: Vec<TaggedContent> = Vec::new();

        'candidate_loop: for candidate in content {
            for container in &deduped {
                if Self::is_contained_in_section(&candidate, container) {
                    continue 'candidate_loop;
                }
            }
            deduped.push(candidate);
        }

        deduped
    }

    fn is_contained_in_section(candidate: &TaggedContent, container: &TaggedContent) -> bool {
        if !matches!(container.context, TagContext::Section { .. }) {
            return false;
        }

        if candidate.source_file != container.source_file {
            return false;
        }

        let candidate_content = candidate.raw_payload_content();
        if candidate_content.trim().is_empty() {
            return false;
        }

        let container_content = container.raw_payload_content();
        (container_content.len() > candidate_content.len()
            && container_content.contains(candidate_content))
            || (candidate.raw_heading_line.is_none()
                && container.raw_heading_line.is_some()
                && container_content.trim() == candidate_content.trim())
    }

    /// Sort content by section 1 key, then by source file, then by source position.
    pub fn sort_for_render(mut content: Vec<TaggedContent>) -> Vec<TaggedContent> {
        content.sort_by(|a, b| match (a.date, b.date) {
            (Some(da), Some(db)) => da
                .cmp(&db)
                .then_with(|| a.source_file.cmp(&b.source_file))
                .then_with(|| a.span_start().cmp(&b.span_start())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a
                .source_file
                .cmp(&b.source_file)
                .then_with(|| a.span_start().cmp(&b.span_start())),
        });
        content
    }

    /// Generate markdown output for compiled content.
    pub fn to_markdown(content: Vec<TaggedContent>, query: &TagQuery) -> String {
        Self::to_markdown_for_output(content, query, None)
    }

    /// Generate markdown output for compiled content with optional output path context.
    pub fn to_markdown_for_output(
        content: Vec<TaggedContent>,
        _query: &TagQuery,
        output_file: Option<&Path>,
    ) -> String {
        if content.is_empty() {
            return "*No matching content found.*\n".to_string();
        }

        let sorted = Self::sort_for_render(content);
        let mut output = String::new();
        let mut current_section_one_key: Option<(Option<NaiveDate>, String)> = None;
        let mut current_group = SectionOneGroup::default();

        for item in sorted {
            let label = Self::section_one_label(&item);
            let key = (item.date, label.clone());

            if current_section_one_key.as_ref() != Some(&key) {
                if let Some((_, section_one_label)) = current_section_one_key.take() {
                    Self::render_section_one_group(
                        &section_one_label,
                        &current_group,
                        output_file,
                        &mut output,
                    );
                    current_group = SectionOneGroup::default();
                }

                current_section_one_key = Some(key);
            }

            if item.before_first_h2 {
                current_group.intro.push(item);
                continue;
            }

            let section_two_label = item.section_two.clone().unwrap_or_else(|| label.clone());
            if !current_group
                .section_two_items
                .contains_key(&section_two_label)
            {
                current_group
                    .section_two_order
                    .push(section_two_label.clone());
            }
            current_group
                .section_two_items
                .entry(section_two_label)
                .or_default()
                .push(item);
        }

        if let Some((_, section_one_label)) = current_section_one_key {
            Self::render_section_one_group(
                &section_one_label,
                &current_group,
                output_file,
                &mut output,
            );
        }

        if output.ends_with("\n\n") {
            output.pop();
        }

        output
    }

    fn render_section_one_group(
        label: &str,
        group: &SectionOneGroup,
        output_file: Option<&Path>,
        output: &mut String,
    ) {
        output.push_str(&format!("# {}\n\n", label));

        for (idx, item) in group.intro.iter().enumerate() {
            output.push_str(&Self::render_item(item, output_file));
            output.push_str(&Self::content_separator(&group.intro, idx));
        }

        for section_two_label in &group.section_two_order {
            output.push_str(&format!("## {}\n\n", section_two_label));

            let items = group
                .section_two_items
                .get(section_two_label)
                .expect("section two order must stay aligned");
            for (idx, item) in items.iter().enumerate() {
                output.push_str(&Self::render_item(item, output_file));
                output.push_str(&Self::content_separator(items, idx));
            }
        }
    }

    fn render_item(item: &TaggedContent, output_file: Option<&Path>) -> String {
        let body = item.rendered_content_for_output(output_file);

        match &item.context {
            TagContext::Section { level, .. } if *level >= 3 => {
                if let Some(raw_heading) = item.rendered_heading_line_for_output(output_file) {
                    return format!("{}\n\n{}", raw_heading, body);
                }
                body
            }
            _ => body,
        }
    }

    fn section_one_label(item: &TaggedContent) -> String {
        match item.date {
            Some(date) => date.format("%d-%m-%Y").to_string(),
            None => {
                let filename = item
                    .source_file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim()
                    .to_string();
                if filename.is_empty() {
                    "unknown".to_string()
                } else {
                    filename
                }
            }
        }
    }

    fn content_separator(items: &[TaggedContent], idx: usize) -> String {
        if idx + 1 >= items.len() {
            return "\n\n".to_string();
        }

        if let Some(gap) = items[idx].span_gap_to(&items[idx + 1]) {
            return gap.to_string();
        }

        if Self::should_keep_tight_spacing(&items[idx], &items[idx + 1]) {
            "\n".to_string()
        } else {
            "\n\n".to_string()
        }
    }

    fn should_keep_tight_spacing(current: &TaggedContent, next: &TaggedContent) -> bool {
        current.source_file == next.source_file
            && current.date == next.date
            && matches!(current.context, TagContext::Paragraph)
            && matches!(next.context, TagContext::Paragraph)
            && Self::looks_like_list_item(current.raw_payload_content())
            && Self::looks_like_list_item(next.raw_payload_content())
    }

    fn looks_like_list_item(content: &str) -> bool {
        let first_non_empty = content
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or_default()
            .trim_start();

        if first_non_empty.starts_with("- ")
            || first_non_empty.starts_with("* ")
            || first_non_empty.starts_with("+ ")
        {
            return true;
        }

        let mut chars = first_non_empty.chars().peekable();
        let mut has_digit = false;

        while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
            has_digit = true;
            chars.next();
        }

        if !has_digit {
            return false;
        }

        matches!(chars.next(), Some('.' | ')')) && matches!(chars.next(), Some(' '))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tags::{ContentPayload, SourceSpan};
    use chrono::NaiveDate;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn create_test_content(
        tags: Vec<&str>,
        content: &str,
        filename: &str,
        date: Option<NaiveDate>,
    ) -> TaggedContent {
        TaggedContent::new(
            tags.iter().map(|s| s.to_string()).collect(),
            content.to_string(),
            PathBuf::from(filename),
            date,
            TagContext::Paragraph,
        )
    }

    fn create_test_section_content(
        tags: Vec<&str>,
        content: &str,
        filename: &str,
        date: Option<NaiveDate>,
        heading: &str,
        level: usize,
    ) -> TaggedContent {
        TaggedContent::new(
            tags.iter().map(|s| s.to_string()).collect(),
            content.to_string(),
            PathBuf::from(filename),
            date,
            TagContext::Section {
                heading: heading.to_string(),
                level,
            },
        )
    }

    fn create_test_span_content(
        tags: Vec<&str>,
        source: Arc<str>,
        span: SourceSpan,
        filename: &str,
        date: Option<NaiveDate>,
    ) -> TaggedContent {
        TaggedContent::with_payload(
            tags.iter().map(|s| s.to_string()).collect(),
            ContentPayload::Span { span, source },
            PathBuf::from(filename),
            date,
            TagContext::Paragraph,
        )
    }

    fn with_section_two(
        mut item: TaggedContent,
        section_two: Option<&str>,
        before_first_h2: bool,
    ) -> TaggedContent {
        item.apply_compile_context(
            section_two.map(|value| value.to_string()),
            before_first_h2,
            None,
        );
        item
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
    fn test_filter_dedupes_content_contained_by_section() {
        let content = vec![
            create_test_section_content(
                vec!["work"],
                "Line one.\nLine two.",
                "a.md",
                None,
                "Work",
                2,
            ),
            create_test_content(vec!["work"], "Line one.", "a.md", None),
        ];

        let query = TagQuery::parse("work").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "Line one.\nLine two.");
    }

    #[test]
    fn test_filter_dedupes_equal_paragraph_when_section_preserves_heading() {
        let mut section = create_test_section_content(
            vec!["work", "focus"],
            "Line one.",
            "a.md",
            None,
            "Deep focus",
            3,
        );
        section.apply_compile_context(
            Some("Work #work".to_string()),
            false,
            Some("### Deep focus #work #focus".to_string()),
        );

        let content = vec![
            section,
            create_test_content(vec!["work", "focus"], "Line one.", "a.md", None),
        ];

        let query = TagQuery::parse("work AND focus").unwrap();
        let filtered = TagCompiler::filter(content, &query);

        assert_eq!(filtered.len(), 1);
        assert!(matches!(
            filtered[0].context,
            TagContext::Section { level: 3, .. }
        ));
    }

    #[test]
    fn test_sort_for_render_orders_by_date_file_and_span() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let source: Arc<str> = Arc::from("alpha #work\n\nbeta #work");
        let second_start = source.find("beta").unwrap();
        let content = vec![
            create_test_content(
                vec!["work"],
                "later date",
                "b.md",
                NaiveDate::from_ymd_opt(2025, 1, 16),
            ),
            TaggedContent::with_payload(
                vec!["work".to_string()],
                ContentPayload::Span {
                    span: SourceSpan::new(second_start, second_start + "beta #work".len()),
                    source: Arc::clone(&source),
                },
                PathBuf::from("a.md"),
                date,
                TagContext::Paragraph,
            ),
            TaggedContent::with_payload(
                vec!["work".to_string()],
                ContentPayload::Span {
                    span: SourceSpan::new(0, "alpha #work".len()),
                    source,
                },
                PathBuf::from("a.md"),
                date,
                TagContext::Paragraph,
            ),
        ];

        let sorted = TagCompiler::sort_for_render(content);

        assert_eq!(sorted[0].raw_payload_content(), "alpha #work");
        assert_eq!(sorted[1].raw_payload_content(), "beta #work");
        assert_eq!(sorted[2].content, "later date");
    }

    #[test]
    fn test_to_markdown_groups_by_date_and_section_two() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let content = vec![
            with_section_two(
                create_test_content(vec!["work"], "Intro #work", "2025-01-15.md", date),
                None,
                true,
            ),
            with_section_two(
                create_test_content(vec!["work"], "Work body #work", "2025-01-15.md", date),
                Some("Work #work"),
                false,
            ),
        ];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert!(markdown.contains("# 15-01-2025"));
        assert!(markdown.contains("Intro #work\n\n## Work #work"));
        assert!(markdown.contains("## Work #work\n\nWork body #work"));
    }

    #[test]
    fn test_to_markdown_uses_synthetic_section_two_when_file_has_no_h2() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let content = vec![create_test_content(
            vec!["work"],
            "Standalone #work",
            "2025-01-15.md",
            date,
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert!(markdown.contains("# 15-01-2025"));
        assert!(markdown.contains("## 15-01-2025"));
        assert!(markdown.contains("Standalone #work"));
    }

    #[test]
    fn test_to_markdown_preserves_tagged_subsection_heading() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let mut content = create_test_section_content(
            vec!["work", "focus"],
            "Deep task body. #work #focus",
            "2025-01-15.md",
            date,
            "Deep Task",
            3,
        );
        content.apply_compile_context(
            Some("Work #work".to_string()),
            false,
            Some("### Deep Task #work #focus".to_string()),
        );

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(vec![content], &query);

        assert!(markdown.contains("## Work #work"));
        assert!(markdown.contains("### Deep Task #work #focus"));
        assert!(markdown.contains("Deep task body. #work #focus"));
    }

    #[test]
    fn test_to_markdown_merges_same_section_two_across_files() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let content = vec![
            with_section_two(
                create_test_content(vec!["work"], "First file #work", "2025-01-15.md", date),
                Some("Work #work"),
                false,
            ),
            with_section_two(
                create_test_content(
                    vec!["work"],
                    "Second file #work",
                    "nested/2025-01-15.md",
                    date,
                ),
                Some("Work #work"),
                false,
            ),
        ];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert_eq!(markdown.matches("## Work #work").count(), 1);
        assert!(markdown.contains("First file #work"));
        assert!(markdown.contains("Second file #work"));
    }

    #[test]
    fn test_to_markdown_falls_back_to_filename_when_undated() {
        let content = vec![create_test_content(
            vec!["work"],
            "Single stream task. #work",
            "journal.md",
            None,
        )];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert!(markdown.contains("# journal.md"));
        assert!(markdown.contains("## journal.md"));
    }

    #[test]
    fn test_to_markdown_keeps_tight_spacing_for_list_items() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let content = vec![
            with_section_two(
                create_test_content(vec!["work"], "- bla", "2025-01-15.md", date),
                Some("Work #work"),
                false,
            ),
            with_section_two(
                create_test_content(vec!["work"], "- bla1", "2025-01-15.md", date),
                Some("Work #work"),
                false,
            ),
            with_section_two(
                create_test_content(vec!["work"], "- bla2", "2025-01-15.md", date),
                Some("Work #work"),
                false,
            ),
        ];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert!(markdown.contains("- bla\n- bla1\n- bla2"));
        assert!(!markdown.contains("- bla\n\n- bla1"));
        assert!(!markdown.contains("- bla1\n\n- bla2"));
    }

    #[test]
    fn test_to_markdown_keeps_source_gap_for_adjacent_spans() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15);
        let source: Arc<str> = Arc::from("* first\n* second");
        let second_start = source.find("* second").unwrap();
        let content = vec![
            with_section_two(
                create_test_span_content(
                    vec!["work"],
                    Arc::clone(&source),
                    SourceSpan::new(0, "* first".len()),
                    "2025-01-15.md",
                    date,
                ),
                Some("Work #work"),
                false,
            ),
            with_section_two(
                create_test_span_content(
                    vec!["work"],
                    Arc::clone(&source),
                    SourceSpan::new(second_start, second_start + "* second".len()),
                    "2025-01-15.md",
                    date,
                ),
                Some("Work #work"),
                false,
            ),
        ];

        let query = TagQuery::parse("work").unwrap();
        let markdown = TagCompiler::to_markdown(content, &query);

        assert!(markdown.contains("* first\n* second"));
        assert!(!markdown.contains("* first\n\n* second"));
    }
}
