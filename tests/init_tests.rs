//! Integration tests for init and config commands

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_creates_config() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Check .djour directory exists
    assert!(temp.path().join(".djour").exists());

    // Check config.toml exists
    let config_path = temp.path().join(".djour/config.toml");
    assert!(config_path.exists());

    // Check config content
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("mode = \"daily\""));
}

#[test]
fn test_init_with_weekly_mode() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("djour")
        .unwrap()
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
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Second init fails
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .failure();
}

#[test]
fn test_config_get_mode() {
    let temp = TempDir::new().unwrap();

    // Initialize
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Get mode
    Command::cargo_bin("djour")
        .unwrap()
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
    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    // Set mode to weekly
    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("mode")
        .arg("weekly")
        .assert()
        .success();

    // Verify it was set
    Command::cargo_bin("djour")
        .unwrap()
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

    Command::cargo_bin("djour")
        .unwrap()
        .arg("init")
        .arg(temp.path())
        .assert()
        .success();

    Command::cargo_bin("djour")
        .unwrap()
        .current_dir(temp.path())
        .arg("config")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("mode"))
        .stdout(predicate::str::contains("editor"));
}
