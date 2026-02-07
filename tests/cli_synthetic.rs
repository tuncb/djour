//! Fixture-driven CLI synthetic tests.
//!
//! Each case under `tests/fixtures/synthetic/<case>/` provides:
//! - `input/`    initial journal tree copied to a temp directory
//! - `scenario.toml` command list and command-level assertions
//! - `expected/` expected final journal tree after executing scenario

use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct Scenario {
    #[serde(rename = "command")]
    commands: Vec<CommandSpec>,
}

#[derive(Debug, Deserialize)]
struct CommandSpec {
    args: Vec<String>,
    #[serde(default = "default_exit_code")]
    expect_exit: i32,
    #[serde(default)]
    stdout_contains: Vec<String>,
    #[serde(default)]
    stdout_not_contains: Vec<String>,
    #[serde(default)]
    stderr_contains: Vec<String>,
    #[serde(default)]
    stderr_not_contains: Vec<String>,
}

fn default_exit_code() -> i32 {
    0
}

#[test]
fn test_synthetic_fixtures() {
    let root = Path::new("tests").join("fixtures").join("synthetic");
    assert!(
        root.exists(),
        "Synthetic fixture root missing: {}",
        root.display()
    );

    let mut case_dirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    case_dirs.sort();
    assert!(!case_dirs.is_empty(), "No synthetic test cases found");

    for case_dir in case_dirs {
        run_case(&case_dir);
    }
}

fn run_case(case_dir: &Path) {
    let case_name = case_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unknown-case>");

    let input_dir = case_dir.join("input");
    let expected_dir = case_dir.join("expected");
    let scenario_path = case_dir.join("scenario.toml");

    assert!(
        input_dir.exists(),
        "Case '{}' is missing input directory: {}",
        case_name,
        input_dir.display()
    );
    assert!(
        expected_dir.exists(),
        "Case '{}' is missing expected directory: {}",
        case_name,
        expected_dir.display()
    );
    assert!(
        scenario_path.exists(),
        "Case '{}' is missing scenario.toml: {}",
        case_name,
        scenario_path.display()
    );

    let scenario_content = fs::read_to_string(&scenario_path).unwrap_or_else(|e| {
        panic!(
            "Case '{}' failed to read scenario file {}: {}",
            case_name,
            scenario_path.display(),
            e
        )
    });
    let scenario: Scenario = toml::from_str(&scenario_content).unwrap_or_else(|e| {
        panic!(
            "Case '{}' has invalid scenario TOML in {}: {}",
            case_name,
            scenario_path.display(),
            e
        )
    });

    let temp = tempfile::TempDir::new().unwrap();
    copy_tree(&input_dir, temp.path());

    for (idx, command) in scenario.commands.iter().enumerate() {
        let output = run_djour(temp.path(), &command.args);
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        assert_eq!(
            code,
            command.expect_exit,
            "Case '{}', command #{} ({:?}) exit code mismatch.\nstdout:\n{}\nstderr:\n{}",
            case_name,
            idx + 1,
            command.args,
            stdout,
            stderr
        );

        for needle in &command.stdout_contains {
            assert!(
                stdout.contains(needle),
                "Case '{}', command #{} ({:?}) expected stdout to contain {:?}.\nstdout:\n{}",
                case_name,
                idx + 1,
                command.args,
                needle,
                stdout
            );
        }

        for needle in &command.stdout_not_contains {
            assert!(
                !stdout.contains(needle),
                "Case '{}', command #{} ({:?}) expected stdout to NOT contain {:?}.\nstdout:\n{}",
                case_name,
                idx + 1,
                command.args,
                needle,
                stdout
            );
        }

        for needle in &command.stderr_contains {
            assert!(
                stderr.contains(needle),
                "Case '{}', command #{} ({:?}) expected stderr to contain {:?}.\nstderr:\n{}",
                case_name,
                idx + 1,
                command.args,
                needle,
                stderr
            );
        }

        for needle in &command.stderr_not_contains {
            assert!(
                !stderr.contains(needle),
                "Case '{}', command #{} ({:?}) expected stderr to NOT contain {:?}.\nstderr:\n{}",
                case_name,
                idx + 1,
                command.args,
                needle,
                stderr
            );
        }
    }

    assert_trees_match(case_name, &expected_dir, temp.path());
}

fn run_djour(cwd: &Path, args: &[String]) -> Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_djour"));
    cmd.current_dir(cwd)
        .env_remove("DJOUR_ROOT")
        .env_remove("DJOUR_MODE")
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .args(args);

    cmd.output().unwrap_or_else(|e| {
        panic!(
            "Failed to execute djour in {} with args {:?}: {}",
            cwd.display(),
            args,
            e
        )
    })
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(from).unwrap();
        if rel_path.as_os_str().is_empty() {
            continue;
        }

        let dest_path = to.join(rel_path);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path).unwrap();
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(src_path, &dest_path).unwrap();
        }
    }
}

fn assert_trees_match(case_name: &str, expected_root: &Path, actual_root: &Path) {
    let expected_files = collect_relative_files(expected_root);
    let actual_files = collect_relative_files(actual_root);

    let missing: Vec<_> = expected_files.difference(&actual_files).cloned().collect();
    let extra: Vec<_> = actual_files.difference(&expected_files).cloned().collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "Case '{}' tree mismatch.\nMissing files: {:?}\nExtra files: {:?}",
        case_name,
        missing,
        extra
    );

    for rel in expected_files {
        let expected_path = expected_root.join(&rel);
        let actual_path = actual_root.join(&rel);
        assert_file_matches(case_name, &rel, &expected_path, &actual_path);
    }
}

fn collect_relative_files(root: &Path) -> BTreeSet<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().strip_prefix(root).unwrap().to_path_buf())
        .collect()
}

fn assert_file_matches(case_name: &str, rel: &Path, expected_path: &Path, actual_path: &Path) {
    let expected = fs::read(expected_path).unwrap();
    let actual = fs::read(actual_path).unwrap();

    if expected == actual {
        return;
    }

    if let (Ok(expected_text), Ok(actual_text)) = (
        String::from_utf8(expected.clone()),
        String::from_utf8(actual.clone()),
    ) {
        let normalized_expected = normalize_newlines(&expected_text);
        let normalized_actual = normalize_newlines(&actual_text);

        if normalized_expected == normalized_actual {
            return;
        }

        panic!(
            "Case '{}' file mismatch at {}.\n{}",
            case_name,
            rel.display(),
            first_text_diff(&normalized_expected, &normalized_actual)
        );
    }

    panic!(
        "Case '{}' binary file mismatch at {} ({} expected bytes vs {} actual bytes).",
        case_name,
        rel.display(),
        expected.len(),
        actual.len()
    );
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn first_text_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let min_len = expected_lines.len().min(actual_lines.len());

    for i in 0..min_len {
        if expected_lines[i] != actual_lines[i] {
            return format!(
                "First difference at line {}.\nexpected: {:?}\nactual:   {:?}",
                i + 1,
                expected_lines[i],
                actual_lines[i]
            );
        }
    }

    if expected_lines.len() != actual_lines.len() {
        return format!(
            "Line count differs.\nexpected: {} lines\nactual:   {} lines",
            expected_lines.len(),
            actual_lines.len()
        );
    }

    "Content differs, but no line-level difference could be determined.".to_string()
}
