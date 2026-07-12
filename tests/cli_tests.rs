//! CLI integration tests for the `sensitive-rs` binary.
//!
//! Only compiled under the `cli` feature. Run with:
//!   cargo test --features cli --test cli_tests
//! or
//!   cargo test --all-features
//!
//! Each test drives the compiled binary directly (no `cargo run` per test) and
//! pins a tiny fixture dictionary via `--dict` so output is deterministic
//! regardless of the 13k-word default dictionary.

#![cfg(feature = "cli")]

use std::process::Command;

/// Args selecting the fixture dictionary (a global flag, placed before the subcommand).
const DICT_ARGS: [&str; 2] = ["--dict", "tests/fixtures/test_dict.txt"];

fn sensitive() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sensitive-rs"))
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn test_cli_check_clean() {
    let output = sensitive().args(DICT_ARGS).args(["check", "正常内容"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let out = stdout(&output);
    assert!(out.contains("No sensitive words found"), "stdout: {out}");
}

#[test]
fn test_cli_check_found() {
    let output = sensitive().args(DICT_ARGS).args(["check", "含有赌博"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let out = stdout(&output);
    assert!(out.contains("赌博"), "stdout: {out}");
    assert!(out.contains("Found"), "stdout: {out}");
}

#[test]
fn test_cli_validate_clean_exits_zero() {
    let output = sensitive().args(DICT_ARGS).args(["validate", "正常内容"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_cli_validate_found_exits_nonzero() {
    let output = sensitive().args(DICT_ARGS).args(["validate", "含有赌博"]).output().unwrap();
    // `validate` exits 1 when a sensitive word is found.
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("赌博"), "stdout: {out}");
}

#[test]
fn test_cli_replace() {
    let output = sensitive().args(DICT_ARGS).args(["replace", "*", "赌博内容"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    // Filter::replace repeats the char per matched character: "赌博"(2) -> "**".
    assert_eq!(stdout(&output).trim(), "**内容");
}

#[test]
fn test_cli_filter() {
    let output = sensitive().args(DICT_ARGS).args(["filter", "赌博内容"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(stdout(&output).trim(), "内容");
}

#[test]
fn test_cli_json_output() {
    let output = sensitive().args(DICT_ARGS).args(["--json", "check", "赌博"]).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let out = stdout(&output);
    // JSON mode emits a pretty-printed array of {source, result} objects.
    assert!(out.trim_start().starts_with('['), "expected JSON array, got: {out}");
    assert!(out.contains(r#""word""#), "expected a `word` field, got: {out}");
    assert!(out.contains("赌博"), "expected the match in output, got: {out}");
}
