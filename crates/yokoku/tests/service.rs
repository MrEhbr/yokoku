use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn the_service_stops_at_startup_without_web_assets() {
    let dir = tempfile::tempdir().unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .env("DIOXUS_PUBLIC_PATH", dir.path().join("public"))
        .env("APP__WEB__PORT", "0")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to start the web server")
                .and(predicate::str::contains("web assets not found")),
        );
}

#[test]
fn the_service_stops_at_startup_when_the_web_port_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("yokoku"))
        .env("APP__DATABASE__PATH", dir.path().join("yokoku.db"))
        .env("DIOXUS_PUBLIC_PATH", dir.path())
        .env("APP__WEB__PORT", taken.local_addr().unwrap().port().to_string())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to start the web server"));
}
