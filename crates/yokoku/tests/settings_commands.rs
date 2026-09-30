use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_events::{EventLog, SettingsChanged};
use yokoku_test_support::events::refuse_events;

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

#[test]
fn a_bad_setting_is_refused_and_not_stored_and_an_unknown_one_is_not_read() {
    let setup = setup();

    let refused = setup.command().args(["settings", "set", "import.mode", "sideways"]).assert().failure();
    let unknown = setup.command().args(["settings", "get", "import.speed"]).assert().failure();

    refused.stderr(predicate::str::contains("import.mode cannot be \"sideways\""));
    unknown.stderr(predicate::str::contains("import.speed is not a setting"));

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
fn secrets_are_shown_masked() {
    let setup = setup();

    let unset = setup
        .command()
        .args(["settings", "get", "metadata.tmdb.token"])
        .env_remove("APP__METADATA__TMDB__TOKEN")
        .output()
        .unwrap();
    let set = setup
        .command()
        .args(["settings", "get", "metadata.tmdb.token"])
        .env("APP__METADATA__TMDB__TOKEN", "very-secret")
        .output()
        .unwrap();

    assert_eq!(String::from_utf8(unset.stdout).unwrap(), "null\n");
    assert_eq!(String::from_utf8(set.stdout).unwrap(), "\"very…cret\"\n");
}

#[test]
fn secrets_can_be_stored_but_are_shown_masked() {
    let setup = setup();

    let run = |args: &[&str]| {
        let output = setup.command().args(args).env_remove("APP__METADATA__TMDB__TOKEN").output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };

    let stored = run(&["settings", "set", "metadata.tmdb.token", "very-secret"]);
    let listed = run(&["settings", "list"]);
    let effective = run(&["settings", "get", "metadata.tmdb.token"]);

    assert_eq!(stored, "Set metadata.tmdb.token = \"very…cret\"\n");
    assert_eq!(listed, "metadata.tmdb.token = \"very…cret\"\n");
    assert_eq!(effective, "\"very…cret\"\n");
}

#[test]
fn loading_the_configuration_does_not_create_the_database() {
    let setup = setup();

    setup
        .command()
        .env("APP__SERVE__SCAN_LIBRARY", "not a schedule")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid schedule"));

    assert!(!setup.database.exists());
}

impl Setup {
    /// Sets `import.mode` to `mode` and waits until the service reloaded on its `SettingsChanged`.
    async fn set_and_await_reload(&self, mode: &str) -> bool {
        self.stdout(&["settings", "set", "import.mode", mode]);
        let log = EventLog::new(Database::open(&self.database).await.unwrap());
        let changed = log
            .read_after(None, 100)
            .await
            .unwrap()
            .into_iter()
            .rev()
            .find_map(|recorded| recorded.event.get::<SettingsChanged>().is_some().then_some(recorded.id));
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if changed.is_some() && log.last_delivered("config.settings").await.unwrap() == changed {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        false
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_service_reloads_a_setting_changed_from_the_command_line() {
    let setup = setup();
    setup.stdout(&["settings", "list"]);
    let assets = tempfile::tempdir().unwrap();
    let mut serve = setup
        .command()
        .env("DIOXUS_PUBLIC_PATH", assets.path())
        .env("APP__WEB__PORT", "0")
        .env("APP__EVENTS__POLL_INTERVAL_MS", "100")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let started = setup.set_and_await_reload("copy").await;
    let reloaded = setup.set_and_await_reload("move").await;
    let killed = Command::new("kill").args(["-TERM", &serve.id().to_string()]).status().unwrap();
    let status = serve.wait().unwrap();

    assert!(started && reloaded, "the service did not reload the settings in time");
    assert!(killed.success() && status.success(), "{status}");
}

#[tokio::test]
async fn a_change_whose_event_cannot_be_recorded_is_stored_but_fails_the_command() {
    let setup = setup();
    setup.stdout(&["settings", "list"]);
    refuse_events(&Database::open(&setup.database).await.unwrap()).await;

    let set = setup.command().args(["settings", "set", "import.mode", "copy"]).assert().failure();

    set.stderr(predicate::str::contains("its events could not be recorded"));
    assert_eq!(setup.stdout(&["settings", "get", "import.mode"]), "\"copy\"\n");
}
