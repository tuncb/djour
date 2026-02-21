//! Integration tests for folder command

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

fn editor_command_for_test() -> &'static str {
    if cfg!(windows) {
        "cmd /c exit 0"
    } else {
        "sh -c true"
    }
}

#[test]
fn test_folder_prints_journal_root_path() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    let expected = temp.path().display().to_string();

    djour_cmd()
        .current_dir(temp.path())
        .arg("folder")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_folder_from_nested_directory_prints_journal_root_path() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    let nested_dir = temp.path().join("nested").join("deep");
    fs::create_dir_all(&nested_dir).unwrap();

    let expected = temp.path().display().to_string();

    djour_cmd()
        .current_dir(&nested_dir)
        .arg("folder")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_folder_with_open_flag_opens_editor_and_prints_root_path() {
    let temp = TempDir::new().unwrap();
    init_journal(&temp);

    let expected = temp.path().display().to_string();

    djour_cmd()
        .current_dir(temp.path())
        .env("EDITOR", editor_command_for_test())
        .arg("folder")
        .arg("--open")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_folder_not_in_djour_directory_fails() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("folder")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a djour directory"));
}
