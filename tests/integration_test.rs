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
fn test_greet_command() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["greet", "Ada"]);

    cmd.assert().success().stdout(predicate::str::contains("Hello, Ada!"));
}

#[test]
fn test_greet_repeats() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["greet", "Ada", "--count", "3"]);

    cmd.assert().success().stdout(predicate::str::contains("Hello, Ada!").count(3));
}

#[test]
fn test_greet_uses_config_file_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    std::fs::write(&path, "[greet]\ngreeting = \"Howdy\"\ncount = 2\n").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["greet", "Ada", "-c"]).arg(&path);

    cmd.assert().success().stdout(predicate::str::contains("Howdy, Ada!").count(2));
}

#[test]
fn test_greet_flags_override_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    std::fs::write(&path, "[greet]\ngreeting = \"Howdy\"\ncount = 2\n").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["greet", "Ada", "-c"]).arg(&path).args(["--greeting", "Hi", "--count", "1"]);

    cmd.assert().success().stdout(predicate::str::contains("Hi, Ada!").count(1));
}

#[test]
fn test_greet_rejects_empty_name() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.args(["greet", "   "]);

    cmd.assert().failure().stderr(predicate::str::contains("name must not be empty"));
}

#[test]
fn test_global_flags_accepted_either_side() {
    for args in [["-v", "greet", "Ada"], ["greet", "Ada", "-v"]] {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        cmd.args(args);

        cmd.assert().success().stdout(predicate::str::contains("Hello, Ada!"));
    }
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
    cmd.arg("nonexistent");

    cmd.assert().failure().stderr(predicate::str::contains("unrecognized subcommand"));
}
