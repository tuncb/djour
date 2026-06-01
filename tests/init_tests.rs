//! Integration tests for init and config commands

#![allow(deprecated)]

use chrono::{Datelike, Duration, Local};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::djour_cmd;

#[test]
fn test_init_creates_config() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    // Check .djour directory exists
    assert!(temp.path().join(".djour").exists());

    // Check config.toml exists
    let config_path = temp.path().join(".djour/config.toml");
    assert!(config_path.exists());

    // Check config content
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("mode = \"daily\""));
    assert!(!content.contains("created"));
}

#[test]
fn test_init_with_weekly_mode() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    let config_path = temp.path().join(".djour/config.toml");
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("mode = \"weekly\""));
}

#[test]
fn test_init_already_initialized_fails() {
    let temp = TempDir::new().unwrap();

    // First init succeeds
    djour_cmd().arg("init").arg(temp.path()).assert().success();

    // Second init fails
    djour_cmd().arg("init").arg(temp.path()).assert().failure();
}

#[test]
fn test_config_get_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize
    djour_cmd().arg("init").arg(temp.path()).assert().success();

    // Get mode
    djour_cmd()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .assert()
        .success()
        .stdout(predicate::str::contains("daily"));
}

#[test]
fn test_config_set_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize
    djour_cmd().arg("init").arg(temp.path()).assert().success();

    // Set mode to weekly
    djour_cmd()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .arg("weekly")
        .assert()
        .success();

    // Verify it was set
    djour_cmd()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .assert()
        .success()
        .stdout(predicate::str::contains("weekly"));
}

#[test]
fn test_config_list() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("config")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("mode"))
        .stdout(predicate::str::contains("editor"))
        .stdout(predicate::str::contains("created").not());
}

#[test]
fn test_config_get_created_fails() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("config")
        .arg("created")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown config key: 'created'"));
}

#[test]
fn test_time_ref_prints_absolute_note_path() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    let today = Local::now().date_naive();
    let expected = temp
        .path()
        .join(format!("{}.md", today.format("%Y-%m-%d")))
        .display()
        .to_string();

    djour_cmd()
        .current_dir(temp.path())
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_day_offset_prints_absolute_note_path() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    let target = Local::now().date_naive() + Duration::days(2);
    let expected = temp
        .path()
        .join(format!("{}.md", target.format("%Y-%m-%d")))
        .display()
        .to_string();

    djour_cmd()
        .current_dir(temp.path())
        .arg("day")
        .arg("+2")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_week_offset_prints_absolute_note_path_in_weekly_mode() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    let target = Local::now().date_naive() - Duration::weeks(2);
    let iso = target.iso_week();
    let week_start = target - Duration::days(target.weekday().num_days_from_monday() as i64);
    let expected = temp
        .path()
        .join(format!(
            "{}-W{:02}-{}.md",
            iso.year(),
            iso.week(),
            week_start.format("%Y-%m-%d")
        ))
        .display()
        .to_string();

    djour_cmd()
        .current_dir(temp.path())
        .arg("week")
        .arg("-2")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_week_offset_fails_in_daily_mode() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("week")
        .arg("-2")
        .assert()
        .failure()
        .stderr(predicate::str::contains("only valid in weekly mode"));
}
