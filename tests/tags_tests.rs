//! Integration tests for tags command

#![allow(deprecated)]

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::djour_cmd;

#[test]
fn test_tags_no_tags_found() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .assert()
        .success()
        .stdout(predicate::str::contains("No tags found"));
}

#[test]
fn test_tags_lists_unique_sorted_tags_with_prefix() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    fs::write(
        temp.path().join("2025-01-15.md"),
        "Today: #Work and #team_ops and #work",
    )
    .unwrap();
    fs::write(
        temp.path().join("2025-01-16.md"),
        "Backlog: #alpha #TEAM_ops",
    )
    .unwrap();

    let output = djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["#alpha", "#team_ops", "#work"]);
}

#[test]
fn test_tags_with_date_range() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    fs::write(temp.path().join("2025-01-10.md"), "Old #old").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "Middle #mid").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "New #new").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .arg("--from")
        .arg("12-01-2025")
        .arg("--to")
        .arg("18-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("#mid"))
        .stdout(predicate::str::contains("#old").not())
        .stdout(predicate::str::contains("#new").not());
}

#[test]
fn test_tags_with_from_only() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    fs::write(temp.path().join("2025-01-10.md"), "Old #old").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "Middle #mid").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "New #new").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .arg("--from")
        .arg("15-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("#mid"))
        .stdout(predicate::str::contains("#new"))
        .stdout(predicate::str::contains("#old").not());
}

#[test]
fn test_tags_with_to_only() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    fs::write(temp.path().join("2025-01-10.md"), "Old #old").unwrap();
    fs::write(temp.path().join("2025-01-15.md"), "Middle #mid").unwrap();
    fs::write(temp.path().join("2025-01-20.md"), "New #new").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .arg("--to")
        .arg("15-01-2025")
        .assert()
        .success()
        .stdout(predicate::str::contains("#old"))
        .stdout(predicate::str::contains("#mid"))
        .stdout(predicate::str::contains("#new").not());
}

#[test]
fn test_tags_invalid_date_format() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .arg("--from")
        .arg("2025/01/15")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid date format"));
}

#[test]
fn test_tags_not_in_djour_directory() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("tags")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a djour directory"));
}
