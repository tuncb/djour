//! Integration tests for list command

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_list_no_notes() {
    let temp = TempDir::new().unwrap();

    // Initialize
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // List should show no notes
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No notes found"));
}

#[test]
fn test_list_with_notes() {
    let temp = TempDir::new().unwrap();

    // Initialize
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Create some note files
    fs::write(temp.path().join("2025-01-17.md"), "note 1").unwrap();
    fs::write(temp.path().join("2025-01-16.md"), "note 2").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "note 3").unwrap();

    // List should show all notes
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("17-01-2025"))
        .stdout(predicate::str::contains("16-01-2025"))
        .stdout(predicate::str::contains("15-01-2025"));
}

#[test]
fn test_list_sorted_newest_first() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-10.md"), "note").unwrap();

    let output = Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    // Should be sorted newest first
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("20-01-2025"));
    assert!(lines[1].contains("15-01-2025"));
    assert!(lines[2].contains("10-01-2025"));
}

#[test]
fn test_list_with_date_range() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    fs::write(temp.path().join("2025-01-10.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "note").unwrap();

    // List with date range
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--from")
        .arg("12-01-2025")
        .arg("--to")
        .arg("18-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("15-01-2025"))
        .stdout(predicate::str::contains("10-01-2025").not())
        .stdout(predicate::str::contains("20-01-2025").not());
}

#[test]
fn test_list_with_from_only() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    fs::write(temp.path().join("2025-01-10.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "note").unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--from")
        .arg("15-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("15-01-2025"))
        .stdout(predicate::str::contains("20-01-2025"))
        .stdout(predicate::str::contains("10-01-2025").not());
}

#[test]
fn test_list_with_to_only() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    fs::write(temp.path().join("2025-01-10.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "note").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "note").unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--to")
        .arg("15-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("10-01-2025"))
        .stdout(predicate::str::contains("15-01-2025"))
        .stdout(predicate::str::contains("20-01-2025").not());
}

#[test]
fn test_list_with_limit() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Create 5 notes
    for day in 1..=5 {
        let filename = format!("2025-01-{:02}.md", day);
        fs::write(temp.path().join(filename), "note").unwrap();
    }

    // List with limit 2
    let output = Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--limit")
        .arg("2")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let line_count = stdout.lines().count();

    // Should only show 2 entries (newest ones)
    assert_eq!(line_count, 2);
    assert!(stdout.contains("05-01-2025"));
    assert!(stdout.contains("04-01-2025"));
}

#[test]
fn test_list_default_limit() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Create 15 notes
    for day in 1..=15 {
        let filename = format!("2025-01-{:02}.md", day);
        fs::write(temp.path().join(filename), "note").unwrap();
    }

    // List without limit should default to 10
    let output = Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let line_count = stdout.lines().count();

    // Should show 10 entries (default limit)
    assert_eq!(line_count, 10);
}

#[test]
fn test_list_invalid_date_format() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Invalid date format should error
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--from")
        .arg("2025/01/15")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid date format"));
}

#[test]
fn test_list_weekly_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize with weekly mode
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    // Create weekly notes
    fs::write(temp.path().join("2025-W03.md"), "week 3").unwrap();
    fs::write(temp.path().join("2025-W02.md"), "week 2").unwrap();

    // Should list both
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-W03"))
        .stdout(predicate::str::contains("2025-W02"));
}

#[test]
fn test_list_monthly_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize with monthly mode
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("monthly")
        .assert()
        .success();

    // Create monthly notes
    fs::write(temp.path().join("2025-01.md"), "jan").unwrap();
    fs::write(temp.path().join("2024-12.md"), "dec").unwrap();

    // Should list both
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01"))
        .stdout(predicate::str::contains("2024-12"));
}

#[test]
fn test_list_single_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize with single mode
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("single")
        .assert()
        .success();

    fs::write(temp.path().join("journal.md"), "entry").unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("journal.md"));
}

#[test]
fn test_list_ignores_other_files() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Create note files and other files
    fs::write(temp.path().join("2025-01-17.md"), "note").unwrap();
    fs::write(temp.path().join("readme.txt"), "text").unwrap();
    fs::write(temp.path().join("invalid.md"), "bad").unwrap();

    let output = Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should only list valid note
    assert!(stdout.contains("2025-01-17"));
    assert!(!stdout.contains("readme.txt"));
    assert!(!stdout.contains("invalid.md"));
}

#[test]
fn test_list_not_in_djour_directory() {
    let temp = TempDir::new().unwrap();

    // Try to list without initializing
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .failure();
}

#[test]
fn test_list_combined_filters() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Create several notes
    for day in 10..=20 {
        let filename = format!("2025-01-{:02}.md", day);
        fs::write(temp.path().join(filename), "note").unwrap();
    }

    // Test combining date range and limit
    let output = Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("list")
        .arg("--from")
        .arg("12-01-2025")
        .arg("--to")
        .arg("18-01-2025")
        .arg("--limit")
        .arg("3")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    // Should have 3 entries from the range, newest first
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("2025-01-18"));
    assert!(lines[1].contains("2025-01-17"));
    assert!(lines[2].contains("2025-01-16"));
}
