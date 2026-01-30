//! Integration tests for compile command

#![allow(deprecated)]

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::djour_cmd;

/// Helper to initialize a test journal
fn init_journal(temp: &TempDir) {
    djour_cmd().arg("init").arg(temp.path()).assert().success();
}

/// Helper to create a note file with content
fn create_note(temp: &TempDir, filename: &str, content: &str) {
    let note_path = temp.path().join(filename);
    fs::write(note_path, content).unwrap();
}

#[test]
fn test_compile_single_tag() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test note with tagged content
    create_note(
        &temp,
        "2025-01-15.md",
        "## Work Notes #work

Meeting at 10am with team. #work",
    );

    // Compile work tag
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .success()
        .stdout(predicate::str::contains("Compiled tags to:"));

    // Verify output file created
    let output = temp.path().join("compilations/work.md");
    assert!(output.exists());

    // Verify content
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("# Compilation: #work"));
    assert!(content.contains("## 15-01-2025"));
    assert!(content.contains("Meeting at 10am with team"));
}

#[test]
fn test_compile_and_query() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes
    create_note(
        &temp,
        "2025-01-15.md",
        "## Work Notes #work

Urgent task here. #work #urgent",
    );
    create_note(
        &temp,
        "2025-01-16.md",
        "## Work Notes #work

Regular task here. #work",
    );

    // Compile with AND query
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work AND urgent")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work-and-urgent.md");
    assert!(output.exists());

    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("Urgent task here"));
    assert!(!content.contains("Regular task here"));
}

#[test]
fn test_compile_or_query() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes
    create_note(&temp, "2025-01-15.md", "Work content. #work");
    create_note(&temp, "2025-01-16.md", "Personal content. #personal");
    create_note(&temp, "2025-01-17.md", "Hobby content. #hobby");

    // Compile with OR query
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work OR personal")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work-or-personal.md");
    assert!(output.exists());

    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("Work content"));
    assert!(content.contains("Personal content"));
    assert!(!content.contains("Hobby content"));
}

#[test]
fn test_compile_not_query() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes
    create_note(&temp, "2025-01-15.md", "Meeting notes. #work #meeting");
    create_note(&temp, "2025-01-16.md", "Coding notes. #work #coding");

    // Compile with NOT query
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work AND NOT meeting")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work-and-not-meeting.md");
    assert!(output.exists());

    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("Coding notes"));
    assert!(!content.contains("Meeting notes"));
}

#[test]
fn test_compile_with_date_filtering() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes on different dates
    create_note(&temp, "2025-01-10.md", "Old work. #work");
    create_note(&temp, "2025-01-15.md", "Mid work. #work");
    create_note(&temp, "2025-01-20.md", "New work. #work");

    // Compile with date range
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--from")
        .arg("12-01-2025")
        .arg("--to")
        .arg("18-01-2025")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work.md");
    assert!(output.exists());

    let content = fs::read_to_string(output).unwrap();
    assert!(!content.contains("Old work"));
    assert!(content.contains("Mid work"));
    assert!(!content.contains("New work"));
}

#[test]
fn test_compile_format_chronological() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes
    create_note(&temp, "2025-01-15.md", "## Work #work\nFirst day.");
    create_note(&temp, "2025-01-16.md", "## Work #work\nSecond day.");

    // Compile with chronological format (default)
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--format")
        .arg("chronological")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("## 15-01-2025"));
    assert!(content.contains("## 16-01-2025"));
}

#[test]
fn test_compile_format_grouped() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test notes
    create_note(&temp, "2025-01-15.md", "## Work #work\nFirst file.");
    create_note(&temp, "2025-01-16.md", "## Work #work\nSecond file.");

    // Compile with grouped format
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--format")
        .arg("grouped")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("## From: 2025-01-15.md"));
    assert!(content.contains("## From: 2025-01-16.md"));
}

#[test]
fn test_compile_with_context() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test note with sections
    create_note(
        &temp,
        "2025-01-15.md",
        "# Daily Log

## Work Section #work

Meeting notes here.",
    );

    // Compile with context
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--include-context")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    // Should include section heading with adjusted level
    assert!(content.contains("Work Section"));
}

#[test]
fn test_compile_custom_output_path() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test note
    create_note(&temp, "2025-01-15.md", "## Work #work\nSome work.");

    // Create custom output directory
    let custom_dir = temp.path().join("reports");
    fs::create_dir(&custom_dir).unwrap();

    // Compile with custom output
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--output")
        .arg("reports/work-summary.md")
        .assert()
        .success();

    // Verify output at custom path
    let output = temp.path().join("reports/work-summary.md");
    assert!(output.exists());

    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("# Compilation: #work"));
}

#[test]
fn test_compile_empty_query_fails() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Try to compile with empty query
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Empty query"));
}

#[test]
fn test_compile_no_matches_fails() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create note without matching tags
    create_note(&temp, "2025-01-15.md", "## Personal #personal\nSome notes.");

    // Try to compile non-existent tag
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No content found matching query"));
}

#[test]
fn test_compile_invalid_format_fails() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test note
    create_note(&temp, "2025-01-15.md", "## Work #work\nSome work.");

    // Try to compile with invalid format
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--format")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid format"));
}

#[test]
fn test_compile_invalid_date_fails() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create test note
    create_note(&temp, "2025-01-15.md", "## Work #work\nSome work.");

    // Try to compile with invalid date
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .arg("--from")
        .arg("invalid-date")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid date format"));
}

#[test]
fn test_compile_not_djour_directory_fails() {
    let temp = TempDir::new().unwrap();
    // Don't initialize - not a djour directory

    // Try to compile
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a djour directory"));
}

#[test]
fn test_compile_paragraph_level_tags() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create note with paragraph-level tags
    create_note(
        &temp,
        "2025-01-15.md",
        "## Notes

First thought about work. #work

Second thought about personal stuff. #personal

Third thought about work again. #work",
    );

    // Compile work tag
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .success();

    // Verify output
    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("First thought about work"));
    assert!(!content.contains("Second thought about personal"));
    assert!(content.contains("Third thought about work again"));
}

#[test]
fn test_compile_tagged_caption_with_fenced_code_block() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    create_note(
        &temp,
        "2025-01-15.md",
        r#"## Snippets #work

Code sample below. #work

```rust
fn hello() {
    println!("hello");
}
```
"#,
    );

    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .success();

    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("Code sample below."));
    assert!(content.contains("```rust"));
    assert!(content.contains("fn hello()"));
    assert!(content.contains("```"));
}

#[test]
fn test_compile_list_item_with_fenced_code_block() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    create_note(
        &temp,
        "2025-01-15.md",
        r#"## Snippets #work

- Code sample #work

  ```bash
  echo "hi"
  ```
"#,
    );

    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .success();

    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("Code sample"));
    assert!(content.contains("```bash"));
    assert!(content.contains("echo \"hi\""));
    assert!(content.contains("```"));
}

#[test]
fn test_compile_tag_inheritance() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    // Create note with nested sections (tag inheritance)
    create_note(
        &temp,
        "2025-01-15.md",
        "## Work Section #work

### Subsection #subsection

This inherits the work tag from parent section. #note",
    );

    // Compile work tag
    djour_cmd()
        .current_dir(temp.path())
        .arg("compile")
        .arg("work")
        .assert()
        .success();

    // Verify output includes content with inherited work tag
    let output = temp.path().join("compilations/work.md");
    let content = fs::read_to_string(output).unwrap();
    // The paragraph has #note tag, but it should also inherit #work from the parent section
    // So compiling "work" should include this paragraph
    assert!(content.contains("This inherits the work tag from parent section"));
}
