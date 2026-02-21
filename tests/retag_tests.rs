//! Integration tests for retag command

#![allow(deprecated)]

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::djour_cmd;

fn init_journal(temp: &TempDir) {
    djour_cmd().arg("init").arg(temp.path()).assert().success();
}

#[test]
fn test_retag_replaces_exact_tags_and_preserves_duplicates() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    fs::write(
        temp.path().join("2025-01-15.md"),
        "One #work, two #WORK, keep #workshop, and #work #work.",
    )
    .unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work")
        .arg("project")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated 1 file(s) with 4 replacement(s).",
        ));

    let content = fs::read_to_string(temp.path().join("2025-01-15.md")).unwrap();
    assert!(content.contains("One #project, two #project, keep #workshop, and #project #project."));
    assert_eq!(content.matches("#project").count(), 4);
    assert!(content.contains("#workshop"));
}

#[test]
fn test_retag_skips_fenced_and_inline_code() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    fs::write(
        temp.path().join("2025-01-15.md"),
        r#"Outside #work
And also #work.

Inline `#work` should remain.

```md
Inside block #work
```
"#,
    )
    .unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work")
        .arg("focus")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated 1 file(s) with 2 replacement(s).",
        ));

    let content = fs::read_to_string(temp.path().join("2025-01-15.md")).unwrap();
    assert!(content.contains("Outside #focus"));
    assert!(content.contains("And also #focus."));
    assert!(content.contains("Inline `#work` should remain."));
    assert!(content.contains("Inside block #work"));
}

#[test]
fn test_retag_dry_run_does_not_write_files() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    fs::write(temp.path().join("2025-01-15.md"), "Task #work").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work")
        .arg("focus")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Dry run: 1 file(s) would be updated with 1 replacement(s).",
        ));

    let content = fs::read_to_string(temp.path().join("2025-01-15.md")).unwrap();
    assert_eq!(content, "Task #work");
}

#[test]
fn test_retag_with_date_filters() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    fs::write(temp.path().join("2025-01-10.md"), "Old #work").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "New #work").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work")
        .arg("focus")
        .arg("--from")
        .arg("15-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated 1 file(s) with 1 replacement(s).",
        ));

    let old_content = fs::read_to_string(temp.path().join("2025-01-10.md")).unwrap();
    let new_content = fs::read_to_string(temp.path().join("2025-01-20.md")).unwrap();
    assert_eq!(old_content, "Old #work");
    assert_eq!(new_content, "New #focus");
}

#[test]
fn test_retag_recursive_includes_nested_and_skips_dot_dirs() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    fs::write(temp.path().join("2025-01-15.md"), "Root #work").unwrap();

    let nested = temp.path().join("projects");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("2025-01-16.md"), "Nested #work").unwrap();

    let hidden = temp.path().join(".hidden");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("2025-01-17.md"), "Hidden #work").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work")
        .arg("focus")
        .arg("--recursive")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated 2 file(s) with 2 replacement(s).",
        ));

    let root_content = fs::read_to_string(temp.path().join("2025-01-15.md")).unwrap();
    let nested_content = fs::read_to_string(nested.join("2025-01-16.md")).unwrap();
    let hidden_content = fs::read_to_string(hidden.join("2025-01-17.md")).unwrap();
    assert_eq!(root_content, "Root #focus");
    assert_eq!(nested_content, "Nested #focus");
    assert_eq!(hidden_content, "Hidden #work");
}

#[test]
fn test_retag_invalid_tag_fails() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    djour_cmd()
        .current_dir(temp.path())
        .arg("retag")
        .arg("work@email")
        .arg("focus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid tag"));
}
