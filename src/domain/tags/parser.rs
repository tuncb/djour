//! Tag parsing from markdown

use chrono::NaiveDate;
use pulldown_cmark::{Event, Parser as MdParser, Tag, TagEnd};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Regex for matching hashtags: #word, #word-with-dashes, #word_with_underscores
fn tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap())
}

/// Extract all tags from a string (case-insensitive, normalized to lowercase)
fn extract_tags(text: &str) -> Vec<String> {
    tag_regex()
        .captures_iter(text)
        .map(|cap| cap[1].to_lowercase())
        .collect()
}

/// Remove tags from text, returning cleaned text
fn strip_tags(text: &str) -> String {
    tag_regex().replace_all(text, "").trim().to_string()
}

/// Context information about where tagged content came from
#[derive(Debug, Clone, PartialEq)]
pub enum TagContext {
    /// Content from a section (heading)
    Section {
        heading: String, // Original heading text (without tags)
        level: usize,    // Heading level (1-6)
    },
    /// Content from a standalone paragraph
    Paragraph,
}

/// A piece of content with associated tags
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedContent {
    /// All tags applying to this content (including inherited)
    pub tags: Vec<String>,

    /// The actual content text (tags removed)
    pub content: String,

    /// Source file this came from
    pub source_file: PathBuf,

    /// Date extracted from filename (if applicable)
    pub date: Option<NaiveDate>,

    /// Context about where this content came from
    pub context: TagContext,
}

impl TaggedContent {
    pub fn new(
        tags: Vec<String>,
        content: String,
        source_file: PathBuf,
        date: Option<NaiveDate>,
        context: TagContext,
    ) -> Self {
        Self {
            tags,
            content,
            source_file,
            date,
            context,
        }
    }
}

/// Represents a section in the document hierarchy
#[derive(Debug, Clone)]
struct Section {
    level: usize,
    heading: String,
    tags: Vec<String>,
}

/// Tracks the current section hierarchy stack
#[derive(Debug)]
struct SectionStack {
    stack: Vec<Section>,
}

impl SectionStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Enter a new heading, popping sections at same or higher level
    fn push_heading(&mut self, level: usize, heading: &str, tags: Vec<String>) {
        // Pop all sections at the same level or deeper
        self.stack.retain(|s| s.level < level);

        // Push new section
        self.stack.push(Section {
            level,
            heading: heading.to_string(),
            tags,
        });
    }

    /// Get all tags from current section hierarchy (union of all parent tags)
    fn current_tags(&self) -> Vec<String> {
        let mut all_tags = Vec::new();
        for section in &self.stack {
            all_tags.extend(section.tags.clone());
        }
        // Deduplicate while preserving order
        let mut unique_tags = Vec::new();
        for tag in all_tags {
            if !unique_tags.contains(&tag) {
                unique_tags.push(tag);
            }
        }
        unique_tags
    }

    /// Get the current section context (innermost section)
    fn current_context(&self) -> Option<TagContext> {
        self.stack.last().map(|s| TagContext::Section {
            heading: s.heading.clone(),
            level: s.level,
        })
    }
}

pub struct TagParser;

impl TagParser {
    /// Extract tagged content from markdown
    pub fn extract_from_markdown(
        content: &str,
        source_file: &Path,
        date: Option<NaiveDate>,
    ) -> Vec<TaggedContent> {
        let mut results = Vec::new();
        let mut section_stack = SectionStack::new();

        let parser = MdParser::new(content);
        let mut current_paragraph = String::new();
        let mut in_paragraph = false;
        let mut in_heading = false;
        let mut current_heading_text = String::new();
        let mut current_heading_level = 0;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    current_heading_level = level as usize;
                    current_heading_text.clear();
                }

                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;

                    // Extract tags from heading
                    let heading_tags = extract_tags(&current_heading_text);
                    let heading_clean = strip_tags(&current_heading_text);

                    // Update section stack
                    section_stack.push_heading(
                        current_heading_level,
                        &heading_clean,
                        heading_tags.clone(),
                    );

                    // If heading has tags, create tagged content for the heading itself
                    if !heading_tags.is_empty() {
                        results.push(TaggedContent::new(
                            section_stack.current_tags(),
                            heading_clean.clone(),
                            source_file.to_path_buf(),
                            date,
                            TagContext::Section {
                                heading: heading_clean,
                                level: current_heading_level,
                            },
                        ));
                    }
                }

                Event::Start(Tag::Paragraph) => {
                    in_paragraph = true;
                    current_paragraph.clear();
                }

                Event::End(TagEnd::Paragraph) => {
                    in_paragraph = false;

                    // Extract paragraph-level tags (at end of paragraph)
                    let para_tags = extract_tags(&current_paragraph);

                    if !para_tags.is_empty() {
                        // Paragraph has its own tags - combine with inherited section tags
                        let mut all_tags = section_stack.current_tags();
                        for tag in para_tags {
                            if !all_tags.contains(&tag) {
                                all_tags.push(tag);
                            }
                        }

                        let content_clean = strip_tags(&current_paragraph);

                        if !content_clean.trim().is_empty() {
                            results.push(TaggedContent::new(
                                all_tags,
                                content_clean,
                                source_file.to_path_buf(),
                                date,
                                section_stack
                                    .current_context()
                                    .unwrap_or(TagContext::Paragraph),
                            ));
                        }
                    }
                }

                Event::Text(text) => {
                    if in_heading {
                        current_heading_text.push_str(&text);
                    } else if in_paragraph {
                        current_paragraph.push_str(&text);
                    }
                }

                Event::Code(code) => {
                    if in_paragraph {
                        current_paragraph.push('`');
                        current_paragraph.push_str(&code);
                        current_paragraph.push('`');
                    }
                }

                _ => {}
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        assert_eq!(extract_tags("Hello #world"), vec!["world"]);
        assert_eq!(extract_tags("#work #urgent"), vec!["work", "urgent"]);
        assert_eq!(
            extract_tags("#project-alpha #task_1"),
            vec!["project-alpha", "task_1"]
        );
        assert_eq!(extract_tags("#Work #WORK"), vec!["work", "work"]); // Case insensitive
        assert_eq!(extract_tags("No tags here"), Vec::<String>::new());
    }

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("Text #work #urgent"), "Text");
        assert_eq!(strip_tags("#work Meeting notes #urgent"), "Meeting notes");
        assert_eq!(strip_tags("No tags"), "No tags");
    }

    #[test]
    fn test_tagged_content_creation() {
        let tc = TaggedContent::new(
            vec!["work".to_string()],
            "Meeting notes".to_string(),
            PathBuf::from("2025-01-17.md"),
            Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap()),
            TagContext::Section {
                heading: "Daily Standup".to_string(),
                level: 2,
            },
        );

        assert_eq!(tc.tags, vec!["work"]);
        assert_eq!(tc.content, "Meeting notes");
    }

    #[test]
    fn test_section_stack() {
        let mut stack = SectionStack::new();

        // Push level 1 heading
        stack.push_heading(1, "Main", vec!["tag1".to_string()]);
        assert_eq!(stack.current_tags(), vec!["tag1"]);

        // Push level 2 heading - inherits from level 1
        stack.push_heading(2, "Sub", vec!["tag2".to_string()]);
        assert_eq!(stack.current_tags(), vec!["tag1", "tag2"]);

        // Push another level 2 - replaces previous level 2
        stack.push_heading(2, "Sub2", vec!["tag3".to_string()]);
        assert_eq!(stack.current_tags(), vec!["tag1", "tag3"]);

        // Push level 1 - clears all
        stack.push_heading(1, "Main2", vec!["tag4".to_string()]);
        assert_eq!(stack.current_tags(), vec!["tag4"]);
    }

    #[test]
    fn test_section_stack_deduplication() {
        let mut stack = SectionStack::new();
        stack.push_heading(
            1,
            "Main",
            vec!["work".to_string(), "urgent".to_string()],
        );
        stack.push_heading(
            2,
            "Sub",
            vec!["urgent".to_string(), "meeting".to_string()],
        );

        let tags = stack.current_tags();
        assert_eq!(tags, vec!["work", "urgent", "meeting"]); // "urgent" not duplicated
    }

    #[test]
    fn test_section_level_tags() {
        let markdown = r#"
## Meeting Notes #work #urgent

Discussed project timeline.
Action items assigned.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Should have one tagged content for the heading
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tags, vec!["work", "urgent"]);
        assert_eq!(results[0].content, "Meeting Notes");
    }

    #[test]
    fn test_paragraph_level_tags() {
        let markdown = r#"
This is a regular paragraph.

This paragraph has tags. #idea #garden
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tags, vec!["idea", "garden"]);
        assert_eq!(results[0].content, "This paragraph has tags.");
        assert_eq!(results[0].context, TagContext::Paragraph);
    }

    #[test]
    fn test_tag_inheritance() {
        let markdown = r#"
# Project Alpha #project-alpha

## Sprint Planning #work

Planning for sprint 3.

### Tasks #urgent

Critical path items.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Should have 3 headings with tags
        assert!(results.len() >= 3);

        // First heading: just #project-alpha
        assert_eq!(results[0].tags, vec!["project-alpha"]);

        // Second heading: inherits #project-alpha, adds #work
        assert_eq!(results[1].tags, vec!["project-alpha", "work"]);

        // Third heading: inherits both, adds #urgent
        assert_eq!(results[2].tags, vec!["project-alpha", "work", "urgent"]);
    }

    #[test]
    fn test_sibling_sections() {
        let markdown = r#"
# Main #main

## Section A #tag-a

Content A.

## Section B #tag-b

Content B.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Find Section A and Section B headings
        let section_a = results
            .iter()
            .find(|r| {
                matches!(&r.context, TagContext::Section { heading, .. } if heading == "Section A")
            })
            .unwrap();

        let section_b = results
            .iter()
            .find(|r| {
                matches!(&r.context, TagContext::Section { heading, .. } if heading == "Section B")
            })
            .unwrap();

        // Section A should have main + tag-a
        assert_eq!(section_a.tags, vec!["main", "tag-a"]);

        // Section B should have main + tag-b (NOT tag-a)
        assert_eq!(section_b.tags, vec!["main", "tag-b"]);
    }

    #[test]
    fn test_paragraph_inherits_section_tags() {
        let markdown = r#"
## Work Notes #work

This paragraph should inherit work tag. #meeting
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Find the paragraph
        let paragraph = results
            .iter()
            .find(|r| r.content.contains("This paragraph"))
            .unwrap();

        // Should have both inherited #work and paragraph #meeting
        assert_eq!(paragraph.tags, vec!["work", "meeting"]);
    }

    #[test]
    fn test_case_insensitive_tags() {
        let markdown = r#"
## Notes #Work #URGENT #Project-Alpha

Content here.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results[0].tags, vec!["work", "urgent", "project-alpha"]);
    }

    #[test]
    fn test_no_tags() {
        let markdown = r#"
## Regular Heading

Regular paragraph with no tags.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Should have no tagged content
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_empty_content_ignored() {
        let markdown = r#"
## Heading #tag

"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        // Heading itself should be captured
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Heading");
    }

    #[test]
    fn test_inline_code_preserved() {
        let markdown = r#"
Use the `git commit` command here. #git #tutorial
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("`git commit`"));
    }

    #[test]
    fn test_date_preserved() {
        let date = Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
        let markdown = "## Notes #work";

        let results =
            TagParser::extract_from_markdown(markdown, Path::new("2025-01-17.md"), date);

        assert_eq!(results[0].date, date);
    }
}
