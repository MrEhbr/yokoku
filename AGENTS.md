# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 workspace. `crates/domain` holds value types and rules; `crates/core` holds use cases and ports; `crates/infra` implements adapters for SQLite, metadata providers, media services, and the file system. `crates/web` contains the Dioxus UI, and `crates/yokoku` wires the binary, jobs, configuration, and CLI. Shared test fakes live in `crates/test-support`. Look for integration tests in each crate's `tests/` directory, web assets in `crates/web/assets/`, configuration examples in `config/`, and mdBook sources in `docs/src/`.

## Build, Test, and Development Commands

Enter the development shell with `nix develop` (or `direnv allow`); it supplies the toolchain and project tools. Use `just build` to compile, `just run --help` to run the CLI, and `just web serve` to run the web UI with hot reload. `just test` runs workspace tests through cargo-nextest; `just test-doc` runs documentation tests. `just docs` serves the user guide. Run `just --list` for the full task list.

## Coding Style & Naming Conventions

Follow the existing crate boundaries and keep feature behavior in the web UI; the CLI handles setup and maintenance. Rust uses four spaces and standard `snake_case` modules and functions, `PascalCase` types, and `SCREAMING_SNAKE_CASE` constants. Use `just fmt` for Rust formatting and `just web fmt` for Dioxus `rsx!` code. `just lint` runs Clippy with fixes and checks rustfmt, so review its diff afterward. The workspace forbids unsafe Rust.

## Testing Guidelines

Place focused unit tests beside the code and broader behavior tests in `crates/<crate>/tests/`. Name tests for the behavior they verify. Existing tests use Tokio, rstest, proptest, and wiremock where appropriate. Run `just test` before a pull request. Tests that need Jellyfin, Transmission, Jackett, ffprobe, or live metadata credentials are ignored by default; start `just services`, configure the required environment variables described in `CONTRIBUTING.md`, then run `just test-integration [filter]`.

## Commit & Pull Request Guidelines

Use Conventional Commit subjects such as `feat(web): add a page` or `fix: handle missing metadata`; the changelog is generated from them. Keep refactors separate from behavior changes. For larger changes, open an issue to agree on the approach first. Pull requests should explain the behavior, link the relevant issue, include screenshots for UI changes, and update `docs/` when user behavior changes. Confirm `just lint` and `just test` pass; CI also checks typos and dependencies.
