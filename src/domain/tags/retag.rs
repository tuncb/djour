//! Markdown tag replacement helpers.

use pulldown_cmark::{Event, Parser as MdParser, Tag, TagEnd};
use regex::Regex;
use std::ops::Range;
use std::sync::OnceLock;

fn markdown_tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap())
}

/// Result of a tag replacement operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetagResult {
    pub content: String,
    pub replacements: usize,
}

/// Replace a tag in markdown while skipping fenced code blocks and inline code spans.
pub fn retag_markdown(markdown: &str, from_tag: &str, to_tag: &str) -> RetagResult {
    if markdown.is_empty() || from_tag.eq_ignore_ascii_case(to_tag) {
        return RetagResult {
            content: markdown.to_string(),
            replacements: 0,
        };
    }

    let excluded = excluded_ranges(markdown);
    let mut replacements = 0usize;
    let mut rewritten = String::with_capacity(markdown.len());
    let mut cursor = 0usize;

    for range in excluded {
        if range.start > cursor {
            let chunk = &markdown[cursor..range.start];
            rewritten.push_str(&retag_chunk(chunk, from_tag, to_tag, &mut replacements));
        }

        rewritten.push_str(&markdown[range.start..range.end]);
        cursor = range.end;
    }

    if cursor < markdown.len() {
        let chunk = &markdown[cursor..];
        rewritten.push_str(&retag_chunk(chunk, from_tag, to_tag, &mut replacements));
    }

    RetagResult {
        content: rewritten,
        replacements,
    }
}

fn retag_chunk(chunk: &str, from_tag: &str, to_tag: &str, replacements: &mut usize) -> String {
    markdown_tag_regex()
        .replace_all(chunk, |captures: &regex::Captures<'_>| {
            let matched_tag = &captures[1];
            if matched_tag.eq_ignore_ascii_case(from_tag) {
                *replacements += 1;
                format!("#{}", to_tag)
            } else {
                captures[0].to_string()
            }
        })
        .to_string()
}

fn excluded_ranges(markdown: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut code_block_start: Option<usize> = None;

    for (event, range) in MdParser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                code_block_start = Some(range.start);
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_block_start.take() {
                    ranges.push(start..range.end);
                }
            }
            Event::Code(_) => {
                ranges.push(range.start..range.end);
            }
            _ => {}
        }
    }

    if let Some(start) = code_block_start.take() {
        ranges.push(start..markdown.len());
    }

    merge_ranges(ranges)
}

fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    if ranges.is_empty() {
        return ranges;
    }

    ranges.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));

    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        if let Some(last) = merged.last_mut() {
            if range.start <= last.end {
                if range.end > last.end {
                    last.end = range.end;
                }
            } else {
                merged.push(range);
            }
        } else {
            merged.push(range);
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_matching_tags_case_insensitively() {
        let input = "One #work, two #WORK, keep #workshop.";
        let result = retag_markdown(input, "work", "project");

        assert_eq!(
            result.content,
            "One #project, two #project, keep #workshop."
        );
        assert_eq!(result.replacements, 2);
    }

    #[test]
    fn preserves_duplicate_tags() {
        let input = "#work #work #work";
        let result = retag_markdown(input, "work", "focus");

        assert_eq!(result.content, "#focus #focus #focus");
        assert_eq!(result.replacements, 3);
    }

    #[test]
    fn skips_fenced_code_blocks() {
        let input = r#"
Outside #work

```rust
// #work
```
"#;

        let result = retag_markdown(input, "work", "focus");
        assert!(result.content.contains("Outside #focus"));
        assert!(result.content.contains("// #work"));
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn skips_inline_code_spans() {
        let input = "Use `#work` here, but change #work.";
        let result = retag_markdown(input, "work", "focus");

        assert_eq!(result.content, "Use `#work` here, but change #focus.");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn no_op_when_tags_identical() {
        let input = "Keep #work unchanged.";
        let result = retag_markdown(input, "work", "work");

        assert_eq!(result.content, input);
        assert_eq!(result.replacements, 0);
    }
}
