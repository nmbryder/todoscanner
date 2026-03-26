use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn cmd() -> Command {
    Command::cargo_bin("todoscan").unwrap()
}

#[test]
fn test_default_scan_finds_all_tags() {
    cmd()
        .arg(fixtures_dir())
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("FIXME"))
        .stdout(predicate::str::contains("HACK"))
        .stdout(predicate::str::contains("XXX"))
        .stdout(predicate::str::contains("BUG"));
}

#[test]
fn test_ext_filter_rs_only() {
    cmd()
        .arg(fixtures_dir())
        .args(["-e", "rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample.rs"))
        .stdout(predicate::str::contains("sample.py").not());
}

#[test]
fn test_ext_filter_txt_no_results() {
    cmd()
        .arg(fixtures_dir())
        .args(["-e", "txt"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_single_file_mode() {
    let sample = fixtures_dir().join("sample.rs");
    cmd()
        .args(["-f", sample.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample.rs"))
        .stdout(predicate::str::contains("sample.py").not());
}

#[test]
fn test_custom_pattern_todo_only() {
    cmd()
        .arg(fixtures_dir())
        .args(["-p", "TODO"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("FIXME").not());
}

#[test]
fn test_ignore_case() {
    // Our fixtures use uppercase tags; create a temp file with lowercase to verify.
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.rs");
    std::fs::write(&f, "// todo: lowercase\n").unwrap();

    cmd()
        .arg(dir.path())
        .args(["-p", "todo", "-i"])
        .assert()
        .success()
        .stdout(predicate::str::contains("todo"));
}

#[test]
fn test_json_output() {
    let out = cmd()
        .arg(fixtures_dir())
        .args(["-o", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("output should be valid JSON");
    let arr = parsed.as_array().expect("JSON should be an array");
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.get("path").is_some());
    assert!(first.get("line_number").is_some());
    assert!(first.get("tag").is_some());
}

#[test]
fn test_csv_output() {
    let out = cmd()
        .arg(fixtures_dir())
        .args(["-o", "csv"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    let mut lines = text.lines();
    let header = lines.next().expect("should have header line");
    assert_eq!(header, "path,line_number,column,tag,line_content");
    // Each data line should have at least 4 commas separating 5 fields.
    let first_data = lines.next().expect("should have at least one data line");
    assert!(first_data.chars().filter(|&c| c == ',').count() >= 4);
}

#[test]
fn test_context_lines() {
    let sample = fixtures_dir().join("sample.rs");
    // sample.rs line 2 is the TODO; line 1 is "fn foo() -> i32 {" and line 3 is "    42"
    let out = cmd()
        .args(["-f", sample.to_str().unwrap()])
        .args(["-c", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("fn foo()") || text.contains("42"));
}

#[test]
fn test_no_color_flag() {
    cmd()
        .arg(fixtures_dir())
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn test_nonexistent_path() {
    cmd()
        .arg("/nonexistent/path/xyz/abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist").or(predicate::str::contains("Error")));
}

#[test]
fn test_scan_txt_no_results() {
    let sample = fixtures_dir().join("sample.txt");
    cmd()
        .args(["-f", sample.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_no_gitignore_flag() {
    cmd()
        .arg(fixtures_dir())
        .arg("--no-gitignore")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("todoscan"))
        .stdout(predicate::str::contains("pattern"));
}
