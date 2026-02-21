//! Integration tests for mode migration (daily <-> weekly)

#![allow(deprecated)]

use chrono::{Datelike, NaiveDate};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::djour_cmd;

fn write_daily(dir: &std::path::Path, date: NaiveDate, body: &str) {
    let filename = format!("{}.md", date.format("%Y-%m-%d"));
    let content = format!("# {}\n\n{}", date.format("%B %d, %Y"), body);
    fs::write(dir.join(filename), content).unwrap();
}

fn expected_weekly_filename(week_start: NaiveDate) -> String {
    let iso = week_start.iso_week();
    format!(
        "{}-W{:02}-{}.md",
        iso.year(),
        iso.week(),
        week_start.format("%Y-%m-%d")
    )
}

fn build_weekly_template(
    week_start: NaiveDate,
    tuesday_body: &str,
    inject_preface: bool,
) -> String {
    let week_end = week_start + chrono::Duration::days(6);
    let week_num = week_start.iso_week().week();
    let header = format!(
        "# Week {:02}, {} ({} - {})",
        week_num,
        week_start.year(),
        week_start.format("%B %d, %Y"),
        week_end.format("%B %d, %Y")
    );

    let mut s = String::new();
    s.push_str(&header);
    s.push_str("\n\n");

    if inject_preface {
        s.push_str("PREFACE\n\n");
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

    for (i, name) in names.iter().enumerate() {
        let day = week_start + chrono::Duration::days(i as i64);
        s.push_str(&format!("## {} ({})\n\n", name, day.format("%B %d, %Y")));

        if *name == "Tuesday" {
            s.push_str(tuesday_body);
            if !tuesday_body.ends_with('\n') {
                s.push('\n');
            }
            s.push('\n');
        } else {
            s.push('\n');
        }
    }

    s
}

fn find_latest_archive_dir(root: &std::path::Path) -> std::path::PathBuf {
    let archive_root = root.join(".djour").join("archive");
    let mut candidates = fs::read_dir(&archive_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("mode-migration-")
        })
        .map(|e| e.path())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .pop()
        .expect("expected at least one mode-migration-* archive directory")
}

#[test]
fn test_mode_daily_to_weekly_migrates_and_archives() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    let monday = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
    let tuesday = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
    write_daily(temp.path(), monday, "Some Monday note\n");
    write_daily(temp.path(), tuesday, "Some Tuesday note\n");

    djour_cmd()
        .current_dir(temp.path())
        .arg("mode")
        .arg("weekly")
        .assert()
        .success();

    // Config updated.
    let cfg = fs::read_to_string(temp.path().join(".djour/config.toml")).unwrap();
    assert!(cfg.contains("mode = \"weekly\""));

    // Weekly file created and contains injected markers.
    let ws = monday; // Monday is week start
    let weekly_name = expected_weekly_filename(ws);
    let weekly_path = temp.path().join(&weekly_name);
    assert!(weekly_path.exists());

    let weekly = fs::read_to_string(&weekly_path).unwrap();
    assert!(weekly.contains("<!-- djour:migrated-from=2025-01-13.md:start -->"));
    assert!(weekly.contains("Some Monday note"));
    assert!(weekly.contains("<!-- djour:migrated-from=2025-01-14.md:start -->"));
    assert!(weekly.contains("Some Tuesday note"));

    // Daily files moved out of root.
    assert!(!temp.path().join("2025-01-13.md").exists());
    assert!(!temp.path().join("2025-01-14.md").exists());

    // Archive contains originals + config backup.
    let archive_dir = find_latest_archive_dir(temp.path());
    assert!(archive_dir.join("2025-01-13.md").exists());
    assert!(archive_dir.join("2025-01-14.md").exists());
    assert!(archive_dir.join("config.toml").exists());
}

#[test]
fn test_mode_weekly_to_daily_splits_and_archives() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    let ws = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(); // Monday
    let weekly_name = expected_weekly_filename(ws);
    let weekly_content = build_weekly_template(ws, "Tuesday body line\n", false);
    fs::write(temp.path().join(&weekly_name), weekly_content).unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("mode")
        .arg("daily")
        .assert()
        .success();

    // Config updated.
    let cfg = fs::read_to_string(temp.path().join(".djour/config.toml")).unwrap();
    assert!(cfg.contains("mode = \"daily\""));

    // Weekly file moved to archive.
    assert!(!temp.path().join(&weekly_name).exists());
    let archive_dir = find_latest_archive_dir(temp.path());
    assert!(archive_dir.join(&weekly_name).exists());

    // Only Tuesday daily file created (Monday is empty in our weekly input).
    assert!(!temp.path().join("2025-01-13.md").exists());
    let tuesday_path = temp.path().join("2025-01-14.md");
    assert!(tuesday_path.exists());
    let tuesday_content = fs::read_to_string(tuesday_path).unwrap();
    assert!(tuesday_content.contains("# January 14, 2025"));
    assert!(tuesday_content.contains("Tuesday body line"));
}

#[test]
fn test_mode_migration_refuses_custom_templates() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    let templates_dir = temp.path().join(".djour").join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(templates_dir.join("weekly.md"), "# custom").unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("mode")
        .arg("weekly")
        .assert()
        .failure()
        .stderr(predicate::str::contains("built-in templates"));
}

#[test]
fn test_mode_weekly_to_daily_aborts_on_preface_outside_sections() {
    let temp = TempDir::new().unwrap();

    djour_cmd()
        .arg("init")
        .arg(temp.path())
        .arg("--mode")
        .arg("weekly")
        .assert()
        .success();

    let ws = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(); // Monday
    let weekly_name = expected_weekly_filename(ws);
    let weekly_content = build_weekly_template(ws, "Tuesday body line\n", true /* preface */);
    fs::write(temp.path().join(&weekly_name), weekly_content).unwrap();

    djour_cmd()
        .current_dir(temp.path())
        .arg("mode")
        .arg("daily")
        .assert()
        .failure()
        .stderr(predicate::str::contains("between the header and Monday"));

    // Config unchanged.
    let cfg = fs::read_to_string(temp.path().join(".djour/config.toml")).unwrap();
    assert!(cfg.contains("mode = \"weekly\""));

    // Weekly file still present (nothing moved).
    assert!(temp.path().join(&weekly_name).exists());
}

#[test]
fn test_mode_warns_recursive_is_omitted() {
    let temp = TempDir::new().unwrap();

    djour_cmd().arg("init").arg(temp.path()).assert().success();

    djour_cmd()
        .current_dir(temp.path())
        .arg("mode")
        .arg("weekly")
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("--recursive is omitted"));
}
