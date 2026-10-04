# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Yokoku is a self-hosted manager for a movie and TV library: it tracks releases from TMDB/TVDB, imports
torrents from Transmission and organizes files for Jellyfin. One binary serves a Dioxus web UI, delivers
events and runs scheduled jobs.

## Commands

The dev shell (`nix develop` / `direnv allow`) provides the toolchain, `just`, `dx` (Dioxus CLI 0.7),
nextest and a nightly rustfmt (`rustfmt.toml` uses nightly-only import options).

```bash
just build                     # cargo build
just test                      # cargo nextest run --workspace
just test -p yokoku-core       # one crate
just test -E 'test(/name/)'    # tests matching a filter (nextest filterset)
just lint                      # clippy -D warnings (applies --fix) + rustfmt check
just fmt
just run <args>                # the yokoku CLI
just bench [regex]             # criterion benches in crates/yokoku/benches
just docs                      # serve the mdBook user docs (docs/) with live reload

just web serve                 # app at http://127.0.0.1:8080, hot-reloading rsx!
just web gallery               # component gallery
just web fmt                   # dx fmt + cargo fmt for crates/web
just web lint                  # clippy for the wasm (feature web) and server (feature server) builds
just web add <component>...    # vendor a Dioxus component at the pinned dioxus-primitives rev
```

Integration tests are `#[ignore]`d and run against shared services from `just services` (Jellyfin on
:18096, Transmission on :19091, state in `data/services`), plus real TMDB/TVDB and ffprobe. Export the
`YOKOKU__JELLYFIN__*`, `YOKOKU__TRANSMISSION__URL` and `YOKOKU__METADATA__TMDB__TOKEN` variables listed in
CONTRIBUTING.md, then `just test-integration [filter]`.

Pre-commit (prek) runs fmt, clippy and the full nextest suite; CI also runs `typos`, `cargo deny` and
`cargo shear` (unused dependencies).

## Architecture

Six crates under `crates/`, layered strictly inward:

- `domain` — value types, rules, naming templates, the event contract (`yokoku_domain::events`) and shared
  ports (`Clock`, `SettingsStore` in `ports.rs`). No I/O.
- `core` — use cases (`Library`, `Downloads`, `Scanner`, `Importer`, `Renamer`, ...) grouped by module
  (`library`, `downloads`, `media`, `integrations`, `events`). Each module declares its own `ports.rs`
  traits; use cases take `Arc<dyn Port>`.
- `infra` — adapters implementing those ports: SQLite via sqlx (runtime queries, migrations in
  `crates/infra/migrations`), TMDB/TVDB, Transmission, Jellyfin, filesystem/ffprobe/ffmpeg. The single
  `Database` type implements most storage ports.
- `web` — Dioxus 0.7 fullstack UI. Feature `web` builds the wasm client; feature `server` adds server
  functions (`src/api/`) that call use cases through `AppState` (`src/state.rs`), pulled from an axum
  `Extension`. Components in `src/components/` are vendored Dioxus Components restyled to the "Paper"
  design (`styles/`, Tailwind). `gallery/` is a separate bin for previewing components.
- `yokoku` — the binary and composition root. `app.rs` wires every use case to its adapters;
  `service.rs` runs the web server, event deliveries and jobs (`jobs.rs`, cron schedules) until
  SIGINT/SIGTERM; `subscriptions.rs` lists every event subscriber; `config/` handles configuration;
  `cli/` holds setup/maintenance commands.
- `test-support` — fakes, fixtures, wiremock servers and live-service helpers shared by tests.

### Events

Use cases save their change, then publish events through `Publisher` to an SQLite event log — there is no
transactional outbox, and repositories/aggregates never carry events. A `Delivery` loop per `Subscription`
(declared in `crates/yokoku/src/subscriptions.rs`) polls the log and calls handlers. A subscription's name
keys its stored delivery position, so renaming one redelivers the whole log to it. Events carry a
`CorrelationId` propagated through task-local context (`core::events::correlation`) so History can group
related changes. `QueueChanges` is an in-process signal that wakes live UI streams.

### Configuration

Settings layer as: TOML file (`--config`, every key documented in `config/app.toml`) → settings stored in
the database (edited on the Settings page) → `YOKOKU__<SECTION>__<KEY>` env vars. Use cases receive
`Live<T>` values (`Settings::live`) that read the current config on each use, so setting changes apply
without restart; a `SettingsChanged` event reloads them.

## Conventions

- Features belong in the web UI; the CLI is for setup and maintenance.
- Add modules to existing crates rather than new crates.
- Tests live in each crate's `tests/` directory; use `rstest` for tables/fixtures and `proptest` for
  invariants (regressions are committed under `proptest-regressions`).
- In `rsx!`, a conditional `class: if x { .. }` without `else { "" }` wipes merged base classes.
- `crates/web/assets/tailwind.css` is generated by `dx` from `crates/web/tailwind.css` and gitignored; it
  loads through `option_asset!`.
- Conventional Commits (`feat:`, `fix:`, `refactor:`, ...) feed the changelog (git-cliff); keep
  refactors and behavior changes in separate commits. Update `docs/` when user-facing behavior changes.
- Release bundles are defined in `.goreleaser.yaml`, not the Justfile.
