//! Mode migration helpers (daily <-> weekly).
//!
//! This module is intentionally I/O-free: it validates and transforms note contents.

use crate::error::{DjourError, Result};
use chrono::{Datelike, Duration, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    Crlf,
}

impl NewlineStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            NewlineStyle::Lf => "\n",
            NewlineStyle::Crlf => "\r\n",
        }
    }
}

fn detect_newline_style(s: &str) -> NewlineStyle {
    // Use the first newline we see as the "file style".
    if let Some(pos) = s.find('\n') {
        if pos > 0 && s.as_bytes()[pos - 1] == b'\r' {
            NewlineStyle::Crlf
        } else {
            NewlineStyle::Lf
        }
    } else {
        NewlineStyle::Lf
    }
}

fn normalize_newlines(s: &str, style: NewlineStyle) -> String {
    // First normalize to LF, then convert if needed.
    let lf = s.replace("\r\n", "\n");
    match style {
        NewlineStyle::Lf => lf,
        NewlineStyle::Crlf => lf.replace('\n', "\r\n"),
    }
}

#[derive(Debug, Clone, Copy)]
struct LineIdx {
    start: usize,
    end: usize,              // without newline
    end_with_newline: usize, // includes newline (if present)
}

fn scan_lines(s: &str) -> Vec<LineIdx> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\n' {
            // Line ends at i (exclusive) or i-1 if preceded by \r.
            let end = if i > 0 && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            out.push(LineIdx {
                start,
                end,
                end_with_newline: i + 1,
            });
            start = i + 1;
        }
        i += 1;
    }

    // Final line (no trailing newline).
    if start <= bytes.len() {
        out.push(LineIdx {
            start,
            end: bytes.len(),
            end_with_newline: bytes.len(),
        });
    }

    out
}

pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

#[derive(Debug, Clone)]
pub struct WeeklyExpected {
    pub header_variants: Vec<String>,
    pub weekday_headings: Vec<String>, // Monday..Sunday
}

pub fn expected_weekly(week_start: NaiveDate) -> WeeklyExpected {
    let week_end = week_start + Duration::days(6);
    let week_num = week_start.iso_week().week();

    let ws = week_start.format("%B %d, %Y").to_string();
    let we = week_end.format("%B %d, %Y").to_string();

    // NOTE: The built-in weekly template uses {YEAR} which is derived from the date used to render
    // the template, not the ISO week-year. For weeks spanning a year boundary, the header could
    // plausibly contain either the start year or the end year, depending on which day created it.
    let mut header_variants = vec![format!(
        "# Week {:02}, {} ({} - {})",
        week_num,
        week_start.year(),
        ws,
        we
    )];
    let alt = format!(
        "# Week {:02}, {} ({} - {})",
        week_num,
        week_end.year(),
        ws,
        we
    );
    if alt != header_variants[0] {
        header_variants.push(alt);
    }

    let names = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let weekday_headings = (0..7)
        .map(|i| {
            let day = week_start + Duration::days(i);
            format!("## {} ({})", names[i as usize], day.format("%B %d, %Y"))
        })
        .collect::<Vec<_>>();

    WeeklyExpected {
        header_variants,
        weekday_headings,
    }
}

#[derive(Debug, Clone)]
pub struct DaySection {
    pub heading: String,
    pub heading_start: usize,
    pub heading_end_with_newline: usize,
    pub content_start: usize,
    pub content_end: usize,
}

#[derive(Debug, Clone)]
pub struct WeeklyParsed {
    pub newline: NewlineStyle,
    pub header_line: String,
    pub header_start: usize,
    pub header_end_with_newline: usize,
    pub days: Vec<DaySection>, // Monday..Sunday
}

pub fn parse_weekly(content: &str, week_start: NaiveDate) -> Result<WeeklyParsed> {
    let expected = expected_weekly(week_start);
    let newline = detect_newline_style(content);
    let lines = scan_lines(content);

    // First non-empty line must match one of the expected header variants.
    let mut header_line_idx: Option<usize> = None;
    for (i, li) in lines.iter().enumerate() {
        let line = &content[li.start..li.end];
        if line.trim().is_empty() {
            continue;
        }
        header_line_idx = Some(i);
        if !expected.header_variants.iter().any(|h| h == line) {
            return Err(DjourError::Config(format!(
                "Weekly note header does not match built-in template for week starting {}. Expected one of: {:?}. Found: '{}'",
                week_start.format("%Y-%m-%d"),
                expected.header_variants,
                line
            )));
        }
        break;
    }

    let header_line_idx = header_line_idx
        .ok_or_else(|| DjourError::Config("Weekly note is empty; cannot migrate".to_string()))?;
    let header_li = &lines[header_line_idx];
    let header_line = content[header_li.start..header_li.end].to_string();

    // Find weekday headings (exact match, once each, in order).
    let mut found: Vec<(usize, &LineIdx)> = Vec::new();
    let mut seen = std::collections::HashMap::<String, usize>::new();

    for li in &lines {
        let line = &content[li.start..li.end];
        if expected.weekday_headings.iter().any(|h| h == line) {
            if let Some(prev) = seen.insert(line.to_string(), li.start) {
                return Err(DjourError::Config(format!(
                    "Weekly note has duplicate weekday heading '{}' at byte offsets {} and {}",
                    line, prev, li.start
                )));
            }
        }
    }

    for heading in &expected.weekday_headings {
        let mut match_idx: Option<(usize, &LineIdx)> = None;
        for (i, li) in lines.iter().enumerate() {
            let line = &content[li.start..li.end];
            if line == heading {
                match_idx = Some((i, li));
                break;
            }
        }
        let (i, li) = match_idx.ok_or_else(|| {
            DjourError::Config(format!(
                "Weekly note is missing expected heading '{}'",
                heading
            ))
        })?;
        found.push((i, li));
    }

    // Ensure headings are in increasing line order.
    for w in found.windows(2) {
        if w[0].0 >= w[1].0 {
            return Err(DjourError::Config(
                "Weekly weekday headings are not in the expected order (Monday..Sunday)"
                    .to_string(),
            ));
        }
    }

    let mut days: Vec<DaySection> = Vec::with_capacity(7);
    for (idx, (_line_no, li)) in found.iter().enumerate() {
        let heading = expected.weekday_headings[idx].clone();
        let heading_start = li.start;
        let heading_end_with_newline = li.end_with_newline;
        let content_start = heading_end_with_newline;
        let content_end = if idx + 1 < found.len() {
            found[idx + 1].1.start
        } else {
            content.len()
        };

        days.push(DaySection {
            heading,
            heading_start,
            heading_end_with_newline,
            content_start,
            content_end,
        });
    }

    Ok(WeeklyParsed {
        newline,
        header_line,
        header_start: header_li.start,
        header_end_with_newline: header_li.end_with_newline,
        days,
    })
}

pub fn validate_weekly_no_outside_content(content: &str, parsed: &WeeklyParsed) -> Result<()> {
    // Only whitespace is allowed before the header.
    if !content[..parsed.header_start].trim().is_empty() {
        return Err(DjourError::Config(
            "Weekly note has non-whitespace content before the header; aborting migration"
                .to_string(),
        ));
    }

    // Only whitespace is allowed between the header and the Monday section heading.
    let monday_start = parsed
        .days
        .first()
        .ok_or_else(|| DjourError::Config("Weekly note has no weekday sections".to_string()))?
        .heading_start;
    if !content[parsed.header_end_with_newline..monday_start]
        .trim()
        .is_empty()
    {
        return Err(DjourError::Config(
            "Weekly note has content between the header and Monday section; aborting migration"
                .to_string(),
        ));
    }

    Ok(())
}

pub fn daily_prefix(date: NaiveDate) -> String {
    // Built-in daily template is just the date heading and a blank line.
    format!("# {}{}", date.format("%B %d, %Y"), "\n\n")
}

pub fn strip_daily_prefix(content: &str, date: NaiveDate) -> Result<String> {
    let prefix_lf = daily_prefix(date);
    let prefix_crlf = prefix_lf.replace("\n", "\r\n");

    let rest = if content.starts_with(&prefix_lf) {
        &content[prefix_lf.len()..]
    } else if content.starts_with(&prefix_crlf) {
        &content[prefix_crlf.len()..]
    } else {
        return Err(DjourError::Config(format!(
            "Daily note '{}' does not start with the expected built-in header '{}'",
            date.format("%Y-%m-%d"),
            prefix_lf.trim_end()
        )));
    };

    Ok(rest.to_string())
}

fn marker_start(source_filename: &str) -> String {
    format!("<!-- djour:migrated-from={}:start -->", source_filename)
}

fn marker_end(source_filename: &str) -> String {
    format!("<!-- djour:migrated-from={}:end -->", source_filename)
}

pub fn strip_migration_markers(block: &str) -> String {
    let mut out = String::new();
    for line in block.replace("\r\n", "\n").lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!-- djour:migrated-from=") && trimmed.ends_with("-->") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Preserve no trailing newline preference by trimming one final '\n' if the input didn't end with it.
    out
}

pub fn inject_daily_into_weekly(
    weekly_content: &str,
    week_start: NaiveDate,
    day: NaiveDate,
    source_filename: &str,
    daily_body: &str,
) -> Result<String> {
    let parsed = parse_weekly(weekly_content, week_start)?;
    let newline = parsed.newline;

    let expected_heading = expected_weekly(week_start).weekday_headings
        [day.weekday().num_days_from_monday() as usize]
        .clone();

    let (day_idx, section) = parsed
        .days
        .iter()
        .enumerate()
        .find(|(_, s)| s.heading == expected_heading)
        .ok_or_else(|| {
            DjourError::Config(format!(
                "Weekly note is missing expected section heading for {}",
                day.format("%Y-%m-%d")
            ))
        })?;

    // Idempotency: if we already injected this source file into this weekday section, skip.
    let section_text = &weekly_content[section.content_start..section.content_end];
    let start_marker = marker_start(source_filename);
    let end_marker = marker_end(source_filename);
    if section_text.contains(&start_marker) {
        if !section_text.contains(&end_marker) {
            return Err(DjourError::Config(format!(
                "Weekly note contains start marker but missing end marker for '{}'",
                source_filename
            )));
        }
        return Ok(weekly_content.to_string());
    }

    // Insert before trailing whitespace in the weekday section to keep the blank-line padding before
    // the next weekday heading.
    let section_trimmed_len = section_text.trim_end().len();
    let insert_at = section.content_start + section_trimmed_len;

    let nl = newline.as_str();
    let mut block = String::new();

    // Ensure we start on a new line.
    if insert_at > 0 {
        let before = &weekly_content[..insert_at];
        if !(before.ends_with('\n') || before.ends_with("\r\n")) {
            block.push_str(nl);
        }
    }

    block.push_str(&start_marker);
    block.push_str(nl);

    let normalized_body = normalize_newlines(daily_body, newline);
    let body_trimmed = normalized_body.trim_end_matches(['\n', '\r']);
    if !body_trimmed.is_empty() {
        block.push_str(body_trimmed);
        block.push_str(nl);
    }

    block.push_str(&end_marker);
    block.push_str(nl);

    // If we're injecting at the very end of the weekday section (no trailing whitespace),
    // add an extra newline to avoid gluing the end marker to the next heading.
    if day_idx < parsed.days.len() - 1 {
        let after = &weekly_content[insert_at..section.content_end];
        if after.is_empty() {
            block.push_str(nl);
        }
    }

    let mut out = String::new();
    out.push_str(&weekly_content[..insert_at]);
    out.push_str(&block);
    out.push_str(&weekly_content[insert_at..]);
    Ok(out)
}

pub fn split_weekly_into_daily_bodies(
    weekly_content: &str,
    week_start: NaiveDate,
) -> Result<Vec<(NaiveDate, String)>> {
    let parsed = parse_weekly(weekly_content, week_start)?;
    validate_weekly_no_outside_content(weekly_content, &parsed)?;

    let mut out = Vec::with_capacity(7);
    for (i, section) in parsed.days.iter().enumerate() {
        let day = week_start + Duration::days(i as i64);
        let raw = &weekly_content[section.content_start..section.content_end];
        let cleaned = strip_migration_markers(raw);
        out.push((day, cleaned));
    }
    Ok(out)
}
