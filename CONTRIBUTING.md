# Contributing

Bug reports and pull requests are welcome. For a larger change, open an issue first to agree on
the approach.

## Setup

The dev shell has the toolchain and every tool the tasks below use:

```bash
nix develop     # or: direnv allow
```

Without Nix, install the toolchain from `rust-toolchain.toml`, [just](https://github.com/casey/just)
and the [Dioxus CLI](https://dioxuslabs.com) 0.7, then run `just setup`.

## Tasks

```bash
just build      # Build
just test       # Unit and integration tests
just lint       # Clippy and rustfmt
just fmt        # Format
just run [ARGS] # Run a CLI command
just docs       # Serve the user docs
```

The web UI is built with Dioxus in `crates/web`:

```bash
just web serve          # The app at http://127.0.0.1:8080, hot-reloading rsx! edits
just web gallery        # The UI components at http://127.0.0.1:8080
just web fmt            # Format Rust and rsx! in crates/web
just web lint           # Clippy for the browser and server builds
just web add <name>...  # Vendor a Dioxus component into src/components/, then restyle it
```

`just web serve` runs the service from the repository root with `config/app.toml`, so its data
lives in `data/`, which git ignores.

## Integration Tests

`just services` runs Jellyfin (http://127.0.0.1:18096, user `dev` without a password),
Transmission (http://127.0.0.1:19091) and Jackett (http://127.0.0.1:19117, no trackers added) from
the dev shell, with their state in `data/services`.
The ignored tests use them, ffprobe, and the real TMDB and TVDB:

```bash
just services                     # Leave it running
export YOKOKU__JELLYFIN__URL=http://127.0.0.1:18096 YOKOKU__JELLYFIN__API_KEY=yokoku-dev-key YOKOKU__JELLYFIN__USER=dev
export YOKOKU__TRANSMISSION__URL=http://127.0.0.1:19091/transmission/rpc
export YOKOKU__JACKETT__URL=http://127.0.0.1:19117 YOKOKU__JACKETT__API_KEY=yokoku-dev-key
export YOKOKU__METADATA__TMDB__TOKEN=...
just test-integration [filter]
just services down                # From another terminal
```

## Layout

```
crates/
  domain/        Value types, rules, events, naming
  core/          Use cases and their ports
  infra/         Adapters: SQLite, TMDB/TVDB, Transmission, Jellyfin, the file system
  web/           Web UI: pages, server functions, components, gallery
  yokoku/        The binary: wiring, service, jobs, configuration and CLI
  test-support/  Fakes and fixtures shared by tests
config/          Configuration files
docs/            User docs (mdBook)
```

The CLI is for setup and maintenance; features belong in the web UI.

## Pull Requests

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org): `feat:`,
  `fix:`, `refactor:`, `docs:`, `test:`, `chore:`. The changelog is generated from them.
- Keep refactors and behavior changes in separate commits.
- `just lint` and `just test` pass; CI also runs typos and `cargo deny`.
- Update the user docs in `docs/` when behavior changes.
