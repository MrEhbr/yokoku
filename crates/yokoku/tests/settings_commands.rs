use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use rstest::rstest;
use tempfile::TempDir;

struct Setup {
    _dir: TempDir,
    database: PathBuf,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("yokoku.db");
    Setup { _dir: dir, database }
}

impl Setup {
    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("yokoku"));
        command.env("APP__DATABASE__PATH", &self.database).env("APP__CLOCK__TIMEZONE", "UTC");
        command
    }

    fn stdout(&self, args: &[&str]) -> String {
        let output = self.command().args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }
}

#[test]
fn settings_are_stored_listed_read_and_unset() {
    let setup = setup();

    let set_mode = setup.stdout(&["settings", "set", "import.mode", "copy"]);
    let set_days = setup.stdout(&["settings", "set", "calendar.days", "14"]);
    let listed = setup.stdout(&["settings", "list"]);
    let mode = setup.stdout(&["settings", "get", "import.mode"]);
    let unset = setup.stdout(&["settings", "unset", "import.mode"]);

    assert_eq!(set_mode, "Set import.mode = \"copy\"\n");
    assert_eq!(set_days, "Set calendar.days = 14\n");
    assert_eq!(listed, "calendar.days = 14\nimport.mode = \"copy\"\n");
    assert_eq!(mode, "\"copy\"\n");
    assert_eq!(unset, "Unset import.mode\n");
    assert_eq!(setup.stdout(&["settings", "get", "import.mode"]), "\"hardlink\"\n");
    assert_eq!(setup.stdout(&["settings", "unset", "import.mode"]), "import.mode is not stored\n");
}

#[rstest]
#[case::unknown_mode("import.mode", "sideways", "import.mode cannot be \"sideways\"")]
#[case::pattern_without_episodes("naming.episode_file", "{title}", "episode file pattern")]
#[case::bad_schedule("serve.scan_library", "every day", "Invalid schedule")]
#[case::unknown_key("import.speed", "fast", "import.speed is not a setting")]
#[case::section("naming", "x", "naming is not a setting")]
#[case::bootstrap("database.path", "/tmp/other.db", "needed before the database opens")]
fn bad_settings_are_refused_and_not_stored(#[case] key: &str, #[case] value: &str, #[case] reason: &str) {
    let setup = setup();

    setup.command().args(["settings", "set", key, value]).assert().failure().stderr(predicate::str::contains(reason));

    assert_eq!(setup.stdout(&["settings", "list"]), "No stored settings.\n");
}

#[test]
fn environment_variables_take_precedence_over_stored_settings() {
    let setup = setup();

    let set = setup
        .command()
        .args(["settings", "set", "import.mode", "copy"])
        .env("APP__IMPORT__MODE", "move")
        .output()
        .unwrap();
    let effective =
        setup.command().args(["settings", "get", "import.mode"]).env("APP__IMPORT__MODE", "move").output().unwrap();

    assert_eq!(
        String::from_utf8(set.stdout).unwrap(),
        "Set import.mode = \"copy\"\nAPP__IMPORT__MODE is set and takes precedence\n"
    );
    assert_eq!(String::from_utf8(effective.stdout).unwrap(), "\"move\"\n");
}

#[test]
fn secrets_are_never_shown() {
    let setup = setup();

    let unset = setup
        .command()
        .args(["settings", "get", "metadata.tmdb_token"])
        .env_remove("APP__METADATA__TMDB_TOKEN")
        .output()
        .unwrap();
    let set = setup
        .command()
        .args(["settings", "get", "metadata.tmdb_token"])
        .env("APP__METADATA__TMDB_TOKEN", "very-secret")
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(unset.stdout).unwrap(), "null\n");
    assert_eq!(String::from_utf8(set.stdout).unwrap(), "\"<redacted>\"\n");
}

#[test]
fn secrets_can_be_stored_but_are_never_shown() {
    let setup = setup();

    let run = |args: &[&str]| {
        let output = setup.command().args(args).env_remove("APP__METADATA__TMDB_TOKEN").output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };

    let stored = run(&["settings", "set", "metadata.tmdb_token", "very-secret"]);
    let listed = run(&["settings", "list"]);
    let effective = run(&["settings", "get", "metadata.tmdb_token"]);

    assert_eq!(stored, "Set metadata.tmdb_token = \"<redacted>\"\n");
    assert_eq!(listed, "metadata.tmdb_token = \"<redacted>\"\n");
    assert_eq!(effective, "\"<redacted>\"\n");
}

#[test]
fn loading_the_configuration_does_not_create_the_database() {
    let setup = setup();

    setup
        .command()
        .arg("serve")
        .env("APP__SERVE__SCAN_LIBRARY", "not a schedule")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid schedule"));

    assert!(!setup.database.exists());
}
