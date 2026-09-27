use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn test_help_command() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.arg("--help");

    cmd.assert().success().stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.arg("--version");

    cmd.assert().success().stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_config_file_values_are_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_file(&dir, "[calendar]\ndays = 30\n");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["settings", "get", "calendar.days", "-c"]).arg(&path);

    cmd.assert().success().stdout("30\n");
}

#[test]
fn test_global_flags_accepted_either_side() {
    let dir = tempfile::tempdir().unwrap();
    let path = config_file(&dir, "");
    let path = path.to_str().unwrap();

    for args in [["-v", "-c", path, "settings", "list"], ["settings", "list", "-v", "-c", path]] {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        cmd.args(args);

        cmd.assert().success().stdout("No stored settings.\n");
    }
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.arg("nonexistent");

    cmd.assert().failure().stderr(predicate::str::contains("unrecognized subcommand"));
}

/// A config file in `dir` with `contents` and a database beside it.
fn config_file(dir: &tempfile::TempDir, contents: &str) -> std::path::PathBuf {
    let path = dir.path().join("app.toml");
    let database = dir.path().join("yokoku.db");
    std::fs::write(&path, format!("[database]\npath = {:?}\n\n{contents}", database.display().to_string())).unwrap();
    path
}
