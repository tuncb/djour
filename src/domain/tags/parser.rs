//! Tag parsing from markdown

use chrono::NaiveDate;
use pulldown_cmark::{CodeBlockKind, Event, Parser as MdParser, Tag, TagEnd};
use regex::Regex;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

/// Regex for matching hashtags: #word, #word-with-dashes, #word_with_underscores
fn tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap())
}

fn html_attr_double_quote_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?i)\b(src|href)\s*=\s*"([^"]+)""#).unwrap())
}

fn html_attr_single_quote_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?i)\b(src|href)\s*=\s*'([^']+)'"#).unwrap())
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

#[allow(clippy::too_many_arguments)]
fn append_to_active(
    fragment: &str,
    in_code_block: bool,
    in_heading: bool,
    in_paragraph: bool,
    code_block_text: &mut String,
    current_heading_text: &mut String,
    current_paragraph: &mut String,
    item_stack: &mut [String],
) {
    if in_code_block {
        code_block_text.push_str(fragment);
    } else if in_heading {
        current_heading_text.push_str(fragment);
    } else if in_paragraph {
        current_paragraph.push_str(fragment);
    } else if let Some(item_text) = item_stack.last_mut() {
        item_text.push_str(fragment);
    }
}

fn has_uri_scheme(target: &str) -> bool {
    let mut chars = target.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() {
        return false;
    }

    for c in chars {
        match c {
            ':' => return true,
            'a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '.' => {}
            _ => return false,
        }
    }

    false
}

fn split_target_suffix(target: &str) -> (&str, &str) {
    let query_pos = target.find('?');
    let fragment_pos = target.find('#');

    let split_pos = match (query_pos, fragment_pos) {
        (Some(q), Some(f)) => Some(q.min(f)),
        (Some(q), None) => Some(q),
        (None, Some(f)) => Some(f),
        (None, None) => None,
    };

    match split_pos {
        Some(pos) => (&target[..pos], &target[pos..]),
        None => (target, ""),
    }
}

fn normalize_components(path: &Path) -> (Option<OsString>, bool, Vec<OsString>) {
    let mut prefix: Option<OsString> = None;
    let mut has_root = false;
    let mut segments: Vec<OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(p) => prefix = Some(p.as_os_str().to_os_string()),
            Component::RootDir => has_root = true,
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = segments.last() {
                    if last != OsStr::new("..") {
                        segments.pop();
                        continue;
                    }
                }

                if !has_root {
                    segments.push(OsString::from(".."));
                }
            }
            Component::Normal(seg) => segments.push(seg.to_os_string()),
        }
    }

    (prefix, has_root, segments)
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let (from_prefix, from_root, from_segments) = normalize_components(from);
    let (to_prefix, to_root, to_segments) = normalize_components(to);

    if from_prefix != to_prefix || from_root != to_root {
        return None;
    }

    let mut common_len = 0usize;
    let max_common = from_segments.len().min(to_segments.len());
    while common_len < max_common && from_segments[common_len] == to_segments[common_len] {
        common_len += 1;
    }

    let mut rel = PathBuf::new();
    for _ in common_len..from_segments.len() {
        rel.push("..");
    }
    for segment in &to_segments[common_len..] {
        rel.push(segment);
    }

    if rel.as_os_str().is_empty() {
        rel.push(".");
    }

    Some(rel)
}

fn rewrite_link_target(target: &str, source_file: &Path, output_file: Option<&Path>) -> String {
    let Some(output_file) = output_file else {
        return target.to_string();
    };

    if target.is_empty()
        || target.starts_with('#')
        || target.starts_with('?')
        || target.starts_with("//")
        || target.starts_with('/')
        || target.starts_with('\\')
        || has_uri_scheme(target)
    {
        return target.to_string();
    }

    let (path_part, suffix) = split_target_suffix(target);
    if path_part.is_empty() {
        return target.to_string();
    }

    let source_dir = source_file.parent().unwrap_or_else(|| Path::new(""));
    let output_dir = output_file.parent().unwrap_or_else(|| Path::new(""));
    let target_abs = source_dir.join(path_part);

    if let Some(rel) = relative_path(output_dir, &target_abs) {
        let mut rewritten = rel.to_string_lossy().replace('\\', "/");
        rewritten.push_str(suffix);
        rewritten
    } else {
        target.to_string()
    }
}

fn rewrite_html_targets(html: &str, source_file: &Path, output_file: Option<&Path>) -> String {
    let rewritten_double = html_attr_double_quote_regex()
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let attr = &caps[1];
            let destination = rewrite_link_target(&caps[2], source_file, output_file);
            format!(r#"{attr}="{destination}""#)
        })
        .to_string();

    html_attr_single_quote_regex()
        .replace_all(&rewritten_double, |caps: &regex::Captures<'_>| {
            let attr = &caps[1];
            let destination = rewrite_link_target(&caps[2], source_file, output_file);
            format!(r#"{attr}='{destination}'"#)
        })
        .to_string()
}

fn link_or_image_tail(destination: &str, title: &str) -> String {
    if title.is_empty() {
        format!("]({})", destination)
    } else {
        format!("]({} \"{}\")", destination, title.replace('"', "\\\""))
    }
}

fn format_list_item_markdown(item_text: &str, depth: usize) -> String {
    let trimmed = item_text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let base_indent = "  ".repeat(depth.saturating_sub(1));
    let continuation_indent = format!("{}  ", base_indent);

    let mut lines = trimmed.lines();
    let first_line = lines.next().unwrap_or_default();
    let mut rendered = if first_line.is_empty() {
        format!("{}-", base_indent)
    } else {
        format!("{}- {}", base_indent, first_line)
    };

    for line in lines {
        rendered.push('\n');
        if line.is_empty() {
            continue;
        }
        rendered.push_str(&continuation_indent);
        rendered.push_str(line);
    }

    rendered
}

fn trim_section_body(raw: &str) -> &str {
    raw.trim_matches(|c| c == '\r' || c == '\n')
}

fn parse_atx_heading_level(line: &str) -> Option<usize> {
    let line = line.trim_end_matches(['\r', '\n']);
    let trimmed = line.trim_start_matches(' ');
    let leading_spaces = line.len().saturating_sub(trimmed.len());
    if leading_spaces > 3 {
        return None;
    }

    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    let rest = &trimmed[level..];
    if !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    Some(level)
}

fn parse_fence_marker(line: &str) -> Option<(char, usize)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let trimmed = line.trim_start_matches(' ');
    let leading_spaces = line.len().saturating_sub(trimmed.len());
    if leading_spaces > 3 {
        return None;
    }

    let mut chars = trimmed.chars();
    let fence_char = chars.next()?;
    if fence_char != '`' && fence_char != '~' {
        return None;
    }

    let count = trimmed.chars().take_while(|c| *c == fence_char).count();
    if count < 3 {
        return None;
    }

    Some((fence_char, count))
}

#[derive(Debug, Clone)]
struct HeadingSpan {
    level: usize,
    line_start: usize,
    line_end: usize,
}

fn collect_heading_spans(content: &str) -> Vec<HeadingSpan> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    let mut active_fence: Option<(char, usize)> = None;

    for line in content.split_inclusive('\n') {
        if let Some((fence_char, min_len)) = active_fence {
            if let Some((marker_char, marker_len)) = parse_fence_marker(line) {
                if marker_char == fence_char && marker_len >= min_len {
                    active_fence = None;
                }
            }

            offset += line.len();
            continue;
        }

        if let Some((fence_char, marker_len)) = parse_fence_marker(line) {
            active_fence = Some((fence_char, marker_len));
            offset += line.len();
            continue;
        }

        if let Some(level) = parse_atx_heading_level(line) {
            let line_start = offset;
            let line_end = offset + line.len();
            spans.push(HeadingSpan {
                level,
                line_start,
                line_end,
            });
        }

        offset += line.len();
    }

    spans
}

fn extract_section_bodies_in_order(content: &str) -> Vec<String> {
    let headings = collect_heading_spans(content);
    let mut bodies = Vec::with_capacity(headings.len());

    for (idx, heading) in headings.iter().enumerate() {
        let start = heading.line_end.min(content.len());
        let mut end = content.len();

        for next in headings.iter().skip(idx + 1) {
            if next.level <= heading.level {
                end = next.line_start.min(content.len());
                break;
            }
        }

        bodies.push(trim_section_body(&content[start..end]).to_string());
    }

    bodies
}

#[derive(Debug, Clone)]
enum InlineConstruct {
    Link { destination: String, title: String },
    Image { destination: String, title: String },
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

    /// The original content text (tags preserved)
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

    /// Whether we are currently inside any section that has explicit heading tags.
    fn in_explicit_tagged_section(&self) -> bool {
        self.stack.iter().any(|section| !section.tags.is_empty())
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
        Self::extract_from_markdown_for_output(content, source_file, date, None)
    }

    /// Extract tagged content from markdown, optionally rebasing markdown paths
    /// for links and images so they resolve from the compilation output file.
    pub fn extract_from_markdown_for_output(
        content: &str,
        source_file: &Path,
        date: Option<NaiveDate>,
        output_file: Option<&Path>,
    ) -> Vec<TaggedContent> {
        let section_bodies = extract_section_bodies_in_order(content);
        let mut results = Vec::new();
        let mut section_stack = SectionStack::new();
        let mut list_tag_stack: Vec<Vec<String>> = Vec::new();
        let mut item_stack: Vec<String> = Vec::new();
        let mut item_tag_stack: Vec<Vec<String>> = Vec::new();
        let mut item_children_stack: Vec<Vec<TaggedContent>> = Vec::new();
        let mut pending_list_tags: Option<Vec<String>> = None;

        let parser = MdParser::new(content);
        let mut current_paragraph = String::new();
        let mut in_paragraph = false;
        let mut in_heading = false;
        let mut current_heading_text = String::new();
        let mut current_heading_level = 0;
        let mut in_code_block = false;
        let mut code_block_info = String::new();
        let mut code_block_text = String::new();
        let mut pending_code_block_target: Option<usize> = None;
        let mut inline_stack: Vec<InlineConstruct> = Vec::new();
        let mut heading_index = 0usize;

        let extend_unique = |dest: &mut Vec<String>, tags: Vec<String>| {
            for tag in tags {
                if !dest.contains(&tag) {
                    dest.push(tag);
                }
            }
        };

        for event in parser {
            match event {
                Event::Start(Tag::List(_)) => {
                    pending_code_block_target = None;
                    // Establish list-level inherited tags (from parent list item, if any)
                    let mut inherited = if item_stack.is_empty() {
                        pending_list_tags.take().unwrap_or_default()
                    } else {
                        list_tag_stack.last().cloned().unwrap_or_default()
                    };
                    if !item_stack.is_empty() {
                        if let Some(item_tags) = item_tag_stack.last().cloned() {
                            extend_unique(&mut inherited, item_tags);
                        }
                        if let Some(item_text) = item_stack.last() {
                            let text_tags = extract_tags(item_text);
                            extend_unique(&mut inherited, text_tags);
                        }
                    }
                    list_tag_stack.push(inherited);
                }

                Event::End(TagEnd::List(_)) => {
                    list_tag_stack.pop();
                }

                Event::Start(Tag::Item) => {
                    pending_code_block_target = None;
                    item_stack.push(String::new());
                    item_tag_stack.push(Vec::new());
                    item_children_stack.push(Vec::new());
                }

                Event::End(TagEnd::Item) => {
                    let depth = item_stack.len();
                    let item_text = item_stack.pop().unwrap_or_default();
                    let mut item_tags = item_tag_stack.pop().unwrap_or_default();
                    let child_items = item_children_stack.pop().unwrap_or_default();
                    let text_tags = extract_tags(&item_text);
                    extend_unique(&mut item_tags, text_tags);

                    let section_tags = section_stack.current_tags();
                    let list_tags = list_tag_stack.last().cloned().unwrap_or_default();
                    let mut local_tags = list_tags.clone();
                    extend_unique(&mut local_tags, item_tags.clone());

                    let inside_explicit_section = section_stack.in_explicit_tagged_section();
                    let mut all_tags = section_tags;
                    extend_unique(&mut all_tags, local_tags.clone());

                    let should_emit = if inside_explicit_section {
                        !local_tags.is_empty()
                    } else {
                        !all_tags.is_empty()
                    };

                    let content_clean = strip_tags(&item_text);
                    let content_raw = item_text.trim().to_string();
                    let current_item = if !content_clean.trim().is_empty() && should_emit {
                        Some(TaggedContent::new(
                            all_tags,
                            format_list_item_markdown(&content_raw, depth),
                            source_file.to_path_buf(),
                            date,
                            section_stack
                                .current_context()
                                .unwrap_or(TagContext::Paragraph),
                        ))
                    } else {
                        None
                    };

                    if depth > 1 {
                        if let Some(parent_children) = item_children_stack.last_mut() {
                            if let Some(item) = current_item {
                                parent_children.push(item);
                            }
                            parent_children.extend(child_items);
                        }
                    } else {
                        if let Some(item) = current_item {
                            results.push(item);
                        }
                        results.extend(child_items);
                    }
                }

                Event::Start(Tag::Heading { level, .. }) => {
                    pending_code_block_target = None;
                    in_heading = true;
                    current_heading_level = level as usize;
                    current_heading_text.clear();
                    pending_list_tags = None;
                }

                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;

                    // Extract tags from heading
                    let heading_tags = extract_tags(&current_heading_text);
                    let heading_clean = strip_tags(&current_heading_text);
                    let section_body = section_bodies
                        .get(heading_index)
                        .map(|s| s.as_str())
                        .unwrap_or_default();
                    heading_index += 1;

                    // Update section stack
                    section_stack.push_heading(
                        current_heading_level,
                        &heading_clean,
                        heading_tags.clone(),
                    );

                    // For tagged headings, compile the full section body verbatim.
                    if !heading_tags.is_empty() && !section_body.trim().is_empty() {
                        results.push(TaggedContent::new(
                            section_stack.current_tags(),
                            section_body.to_string(),
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
                    pending_code_block_target = None;
                    in_paragraph = true;
                    current_paragraph.clear();
                    pending_list_tags = None;
                }

                Event::End(TagEnd::Paragraph) => {
                    in_paragraph = false;

                    // Extract paragraph-level tags (at end of paragraph)
                    let para_tags = extract_tags(&current_paragraph);

                    if let Some(item_tags) = item_tag_stack.last_mut() {
                        extend_unique(item_tags, para_tags.clone());
                    }

                    let content_clean = strip_tags(&current_paragraph);
                    let content_raw = current_paragraph.trim().to_string();
                    let list_tags = list_tag_stack.last().cloned().unwrap_or_default();
                    let mut local_tags = list_tags.clone();
                    extend_unique(&mut local_tags, para_tags.clone());
                    let inside_explicit_section = section_stack.in_explicit_tagged_section();

                    if !item_stack.is_empty() {
                        if !content_clean.trim().is_empty() {
                            if let Some(item_text) = item_stack.last_mut() {
                                if !item_text.trim().is_empty() {
                                    item_text.push_str("\n\n");
                                }
                                item_text.push_str(&content_raw);
                            }
                        }
                        pending_list_tags = None;
                    } else if content_clean.trim().is_empty() && !para_tags.is_empty() {
                        // Tag-only paragraph can act as a list tag context for a following list
                        pending_list_tags = Some(para_tags);
                    } else {
                        // Paragraph has tags or inherits tags from section or list
                        let mut all_tags = section_stack.current_tags();
                        extend_unique(&mut all_tags, local_tags.clone());

                        let should_emit = if inside_explicit_section {
                            !local_tags.is_empty()
                        } else {
                            !all_tags.is_empty()
                        };

                        if !content_clean.trim().is_empty() && should_emit {
                            results.push(TaggedContent::new(
                                all_tags,
                                content_raw,
                                source_file.to_path_buf(),
                                date,
                                section_stack
                                    .current_context()
                                    .unwrap_or(TagContext::Paragraph),
                            ));
                            pending_code_block_target = Some(results.len() - 1);
                        }
                        pending_list_tags = None;
                    }
                }

                Event::Start(Tag::Link {
                    dest_url, title, ..
                }) => {
                    let destination = rewrite_link_target(&dest_url, source_file, output_file);
                    append_to_active(
                        "[",
                        in_code_block,
                        in_heading,
                        in_paragraph,
                        &mut code_block_text,
                        &mut current_heading_text,
                        &mut current_paragraph,
                        &mut item_stack,
                    );
                    inline_stack.push(InlineConstruct::Link {
                        destination,
                        title: title.to_string(),
                    });
                }

                Event::End(TagEnd::Link) => {
                    if let Some(InlineConstruct::Link { destination, title }) = inline_stack.pop() {
                        let tail = link_or_image_tail(&destination, &title);
                        append_to_active(
                            &tail,
                            in_code_block,
                            in_heading,
                            in_paragraph,
                            &mut code_block_text,
                            &mut current_heading_text,
                            &mut current_paragraph,
                            &mut item_stack,
                        );
                    }
                }

                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    let destination = rewrite_link_target(&dest_url, source_file, output_file);
                    append_to_active(
                        "![",
                        in_code_block,
                        in_heading,
                        in_paragraph,
                        &mut code_block_text,
                        &mut current_heading_text,
                        &mut current_paragraph,
                        &mut item_stack,
                    );
                    inline_stack.push(InlineConstruct::Image {
                        destination,
                        title: title.to_string(),
                    });
                }

                Event::End(TagEnd::Image) => {
                    if let Some(InlineConstruct::Image { destination, title }) = inline_stack.pop()
                    {
                        let tail = link_or_image_tail(&destination, &title);
                        append_to_active(
                            &tail,
                            in_code_block,
                            in_heading,
                            in_paragraph,
                            &mut code_block_text,
                            &mut current_heading_text,
                            &mut current_paragraph,
                            &mut item_stack,
                        );
                    }
                }

                Event::Text(text) => {
                    append_to_active(
                        &text,
                        in_code_block,
                        in_heading,
                        in_paragraph,
                        &mut code_block_text,
                        &mut current_heading_text,
                        &mut current_paragraph,
                        &mut item_stack,
                    );
                }

                Event::Code(code) => {
                    let snippet = format!("`{}`", code);
                    append_to_active(
                        &snippet,
                        in_code_block,
                        in_heading,
                        in_paragraph,
                        &mut code_block_text,
                        &mut current_heading_text,
                        &mut current_paragraph,
                        &mut item_stack,
                    );
                }

                Event::Html(html) => {
                    let rewritten_html = rewrite_html_targets(&html, source_file, output_file);
                    if in_code_block || in_heading || in_paragraph || !item_stack.is_empty() {
                        append_to_active(
                            &rewritten_html,
                            in_code_block,
                            in_heading,
                            in_paragraph,
                            &mut code_block_text,
                            &mut current_heading_text,
                            &mut current_paragraph,
                            &mut item_stack,
                        );
                    } else {
                        let html_tags = extract_tags(&rewritten_html);
                        let list_tags = list_tag_stack.last().cloned().unwrap_or_default();
                        let mut local_tags = list_tags.clone();
                        extend_unique(&mut local_tags, html_tags.clone());
                        let inside_explicit_section = section_stack.in_explicit_tagged_section();

                        let mut all_tags = section_stack.current_tags();
                        extend_unique(&mut all_tags, local_tags.clone());

                        let content_clean = strip_tags(&rewritten_html);
                        let content_raw = rewritten_html.trim().to_string();
                        let should_emit = if inside_explicit_section {
                            !local_tags.is_empty()
                        } else {
                            !all_tags.is_empty()
                        };

                        if !content_clean.trim().is_empty() && should_emit {
                            results.push(TaggedContent::new(
                                all_tags,
                                content_raw,
                                source_file.to_path_buf(),
                                date,
                                section_stack
                                    .current_context()
                                    .unwrap_or(TagContext::Paragraph),
                            ));
                        }
                    }
                }

                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_text.clear();
                    code_block_info = match kind {
                        CodeBlockKind::Fenced(info) => info.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                }

                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;

                    let mut fenced = String::new();
                    fenced.push_str("```");
                    if !code_block_info.trim().is_empty() {
                        fenced.push_str(code_block_info.trim());
                    }
                    fenced.push('\n');
                    fenced.push_str(&code_block_text);
                    if !code_block_text.ends_with('\n') {
                        fenced.push('\n');
                    }
                    fenced.push_str("```");

                    if let Some(item_text) = item_stack.last_mut() {
                        if !item_text.trim().is_empty() {
                            item_text.push_str("\n\n");
                        }
                        item_text.push_str(&fenced);
                    } else if let Some(idx) = pending_code_block_target {
                        if !results[idx].content.trim().is_empty() {
                            results[idx].content.push_str("\n\n");
                        }
                        results[idx].content.push_str(&fenced);
                    } else {
                        let list_tags = list_tag_stack.last().cloned().unwrap_or_default();
                        let local_tags = list_tags.clone();
                        let inside_explicit_section = section_stack.in_explicit_tagged_section();

                        let mut all_tags = section_stack.current_tags();
                        extend_unique(&mut all_tags, local_tags.clone());

                        let should_emit = if inside_explicit_section {
                            !local_tags.is_empty()
                        } else {
                            !all_tags.is_empty()
                        };

                        if should_emit {
                            results.push(TaggedContent::new(
                                all_tags,
                                fenced,
                                source_file.to_path_buf(),
                                date,
                                section_stack
                                    .current_context()
                                    .unwrap_or(TagContext::Paragraph),
                            ));
                        }
                    }
                }

                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        code_block_text.push('\n');
                    } else if in_heading {
                        current_heading_text.push(' ');
                    } else if in_paragraph {
                        current_paragraph.push('\n');
                    } else if let Some(item_text) = item_stack.last_mut() {
                        item_text.push('\n');
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
    use tempfile::TempDir;

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
        stack.push_heading(1, "Main", vec!["work".to_string(), "urgent".to_string()]);
        stack.push_heading(2, "Sub", vec!["urgent".to_string(), "meeting".to_string()]);

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

        let heading = results
            .iter()
            .find(|r| {
                matches!(&r.context, TagContext::Section { heading, .. } if heading == "Meeting Notes")
            })
            .unwrap();
        assert_eq!(heading.tags, vec!["work", "urgent"]);
        assert_eq!(
            heading.content,
            "Discussed project timeline.\nAction items assigned."
        );
        assert_eq!(results.len(), 1);
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
        assert_eq!(results[0].content, "This paragraph has tags. #idea #garden");
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

        let project = results
            .iter()
            .find(|r| {
                matches!(&r.context, TagContext::Section { heading, .. } if heading == "Project Alpha")
            })
            .unwrap();
        assert_eq!(project.tags, vec!["project-alpha"]);

        let sprint = results
            .iter()
            .find(|r| {
                matches!(&r.context, TagContext::Section { heading, .. } if heading == "Sprint Planning")
            })
            .unwrap();
        assert_eq!(sprint.tags, vec!["project-alpha", "work"]);

        let tasks = results
            .iter()
            .find(
                |r| matches!(&r.context, TagContext::Section { heading, .. } if heading == "Tasks"),
            )
            .unwrap();
        assert_eq!(tasks.tags, vec!["project-alpha", "work", "urgent"]);
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
            .find(|r| r.tags.contains(&"meeting".to_string()))
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

        // Tagged headings with no section body should not create empty entries.
        assert_eq!(results.len(), 0);
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
    fn test_markdown_link_preserved() {
        let markdown = r#"
See [Design Doc](./docs/design.md). #work
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .content
            .contains("[Design Doc](./docs/design.md)"));
    }

    #[test]
    fn test_markdown_image_preserved() {
        let markdown = r#"
![Diagram](./images/diagram.png) #work
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .content
            .contains("![Diagram](./images/diagram.png)"));
    }

    #[test]
    fn test_markdown_paths_rewritten_for_output_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("2025-01-15.md");
        let output = temp.path().join(".compilations").join("work.md");
        let markdown = r#"
See [Design Doc](./docs/design.md). #work

![Diagram](./images/diagram.png) #work
"#;

        let results =
            TagParser::extract_from_markdown_for_output(markdown, &source, None, Some(&output));

        let combined = results
            .iter()
            .map(|r| r.content.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        assert!(combined.contains("[Design Doc](../docs/design.md)"));
        assert!(combined.contains("![Diagram](../images/diagram.png)"));
    }

    #[test]
    fn test_external_link_not_rewritten() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("2025-01-15.md");
        let output = temp.path().join(".compilations").join("work.md");
        let markdown = r#"
See [Website](https://example.com/docs). #work
"#;

        let results =
            TagParser::extract_from_markdown_for_output(markdown, &source, None, Some(&output));

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .content
            .contains("[Website](https://example.com/docs)"));
    }

    #[test]
    fn test_date_preserved() {
        let date = Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
        let markdown = "## Notes #work\n\nBody content.";

        let results = TagParser::extract_from_markdown(markdown, Path::new("2025-01-17.md"), date);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].date, date);
    }

    #[test]
    fn test_multi_line_paragraph_with_tags() {
        // Test that SoftBreak (line continuation in source) is preserved
        let content = "Line one #tag\nLine two";
        let results = TagParser::extract_from_markdown(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Line one"));
        assert!(results[0].content.contains("Line two"));
        assert!(results[0].content.contains('\n')); // Newline preserved
        assert_eq!(results[0].tags, vec!["tag"]);
    }

    #[test]
    fn test_multi_line_paragraph_tag_not_merged_with_next_line() {
        // Reproduces the bug where "#diary" merged with "I" to form "#diaryI"
        let content = "This was a very sad day. #tostos #diary\nI am planning to go.";
        let results = TagParser::extract_from_markdown(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        // Tags should be correctly extracted
        assert!(results[0].tags.contains(&"tostos".to_string()));
        assert!(results[0].tags.contains(&"diary".to_string()));
        // The "I" should not be consumed by the tag
        assert!(results[0].content.contains("I am planning"));
    }

    #[test]
    fn test_heading_tag_applies_to_list_items() {
        let markdown = r#"
#tag
  - item 1
  - item 2
  - item 3
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        let has_item_1 = results
            .iter()
            .any(|r| r.tags.contains(&"tag".to_string()) && r.content.contains("item 1"));
        let has_item_2 = results
            .iter()
            .any(|r| r.tags.contains(&"tag".to_string()) && r.content.contains("item 2"));
        let has_item_3 = results
            .iter()
            .any(|r| r.tags.contains(&"tag".to_string()) && r.content.contains("item 3"));

        assert!(has_item_1);
        assert!(has_item_2);
        assert!(has_item_3);
    }

    #[test]
    fn test_list_item_tag_applies_to_subitems() {
        let markdown = r#"
- #tag
  - item 1
  - item 2
  - item 3
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        let tagged: Vec<&TaggedContent> = results
            .iter()
            .filter(|r| r.tags.contains(&"tag".to_string()))
            .collect();
        assert!(!tagged.is_empty());

        let combined = tagged
            .iter()
            .map(|r| r.content.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        assert!(combined.contains("item 1"));
        assert!(combined.contains("item 2"));
        assert!(combined.contains("item 3"));
    }

    #[test]
    fn test_section_tag_preserves_nested_list_markdown_and_order() {
        let markdown = r#"
### #section

- parent
  - child
- sibling
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);
        let section = String::from("section");
        let list_items: Vec<String> = results
            .iter()
            .filter(|r| r.tags.contains(&section))
            .map(|r| r.content.clone())
            .collect();

        assert_eq!(
            list_items,
            vec!["- parent\n  - child\n- sibling".to_string()]
        );
    }

    #[test]
    fn test_section_tag_includes_untagged_paragraphs() {
        let markdown = r#"
### #crs

lskfjlskdjflksdjflk
lsdkfjlskdjflksdjflk
lksdjflksjdlfkjsldfkj
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        let has_paragraph = results.iter().any(|r| {
            r.tags.contains(&"crs".to_string())
                && r.content.contains("lskfjlskdjflksdjflk")
                && r.content.contains("lsdkfjlskdjflksdjflk")
                && r.content.contains("lksdjflksjdlfkjsldfkj")
        });
        assert!(has_paragraph);
    }

    #[test]
    fn test_heading_only_tag_does_not_create_empty_content() {
        let markdown = r#"
### #codex

Some content under the heading.
"#;

        let results = TagParser::extract_from_markdown(markdown, Path::new("test.md"), None);

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .content
            .contains("Some content under the heading."));
        assert!(!results[0].content.trim().is_empty());
    }
}
