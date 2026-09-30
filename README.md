# yokoku

[![CI](https://github.com/mrehbr/yokoku/actions/workflows/checks.yml/badge.svg)](https://github.com/mrehbr/yokoku/actions)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024+-orange)](https://www.rust-lang.org)

TBD

## Quick Start

```bash
# Build
just build

# Run the service: events and scheduled jobs
cargo run

# Command-line interface
just run --help

# Test
just test
```

## Development

### Prerequisites

- Rust 2024 edition or later
- [just](https://github.com/casey/just) - Task runner
- Nix (for reproducible environment)
- Docker (for containerization)

### Common Tasks

```bash
just build      # Build the project
just test       # Run tests
just lint       # Run linters
just fmt        # Format code
just run [ARGS] # Run a CLI command
```

The web UI is built on Dioxus in `crates/web` and served by the service:

```bash
just web serve          # Run the app (browser build + service) at http://127.0.0.1:8080
just web gallery        # Browse the UI components at http://127.0.0.1:8080
just web fmt            # Format Rust and rsx!
just web lint           # Clippy for the browser and server builds
just web add <name>...  # Vendor a Dioxus component into src/components/, then restyle it
```

`just web serve` runs the service from the repository root with `config/app.toml`, so its data
lives in `data/` (ignored by git). `dx` hot-reloads `rsx!` edits and rebuilds on other changes.
Add root folders with the CLI, then search and add series and movies in the web UI:

```bash
mkdir -p data/library/series data/library/movies
just run root add series "$PWD/data/library/series"
just run root add movies "$PWD/data/library/movies"
```

How web code is organized and written: [ARCHITECTURE.md §3.1](docs/ARCHITECTURE.md#31-where-things-live).



### Nix Development Shell

```bash
nix develop # Enter dev environment
# or
direnv allow # Auto-load with direnv
```



## Project Structure

```
crates/
  yokoku/             # Binary: composition root, service and CLI
    src/
      app.rs          # Wires the adapters into use cases
      service.rs      # The service: web UI, events, scheduled jobs
      subscriptions.rs  # Every event subscription
      logging.rs      # tracing setup
      jobs.rs         # Scheduled jobs
      config/         # Configuration: layered settings
      cli/
        args.rs       # CLI root, global flags, command dispatch
        commands/     # One module per subcommand
    benches/          # Criterion benchmarks
    tests/            # Integration tests
  domain/             # Value types, rules, event contract, naming
  core/               # Use cases and their ports
  infra/              # Adapters: SQLite, TMDB/TVDB, Transmission, Jellyfin, local system
  web/                # Web UI on Dioxus: pages, server functions, components, gallery
  test-support/       # Fakes and fixtures shared by tests
config/               # Configuration files
docs/                 # Requirements and architecture
```

## Adding a Command

The CLI is for setup and operations; features belong in the web UI.
`crates/yokoku/src/cli/commands/jellyfin.rs` is a small worked example: its
own config section and two subcommands. Paths below are relative to
`crates/yokoku/`. To add your own:

1. Copy it to `src/cli/commands/<name>.rs` and adjust its `Args` and `run`.
2. Register the module in `src/cli/commands/mod.rs`.
3. Add a variant to `Command` in `src/cli/args.rs` and dispatch it in `dispatch`.
4. Add its config section to `Config` in `src/config/mod.rs` (binary-only sections go in `src/config/sections.rs`).

Configuration and logging are resolved once in `route`, so a command only
receives `&Config` and its own parsed `Args`. `--config` and `--verbosity`
are global and work on either side of the subcommand.

A command's config section lives in `src/config/`, the way
`src/config/log.rs` owns `LogConfig`. Declare optional flags and layer them over
the loaded values in `apply_overrides`:

```rust
/// Most items to show, overriding `example.limit`
#[arg(long)]
pub limit: Option<u16>,

fn apply_overrides(&self, config: &ExampleConfig) -> ExampleConfig {
    let mut resolved = config.clone();

    if let Some(limit) = self.limit {
        resolved.limit = limit;
    }

    resolved
}
```

Configuration precedence is command flags > `APP__*` env vars > config file
> defaults.

## CI/CD

GitHub Actions workflows:

- **Checks**: Runs on every PR (build, test, lint, format)
- **Prepare Release**: Manual workflow to create version tags
- **Publish Release**: On tags, GoReleaser builds Linux x86_64 and arm64 archives (binary and web assets) and a multi-arch Docker image

## Data sources

<a href="https://www.themoviedb.org"><img src="https://www.themoviedb.org/assets/v4/logos/v2/blue_short-8e7b30f73a4020692ccca9c88bafe5dcb6f8a62a4c6bc55cd9ba82bb2cd95f6c.svg" alt="TMDB" height="16"></a>

This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.

<a href="https://thetvdb.com"><img src="https://thetvdb.com/images/attribution/logo1.png" alt="TheTVDB" height="32"></a>

Metadata provided by [TheTVDB](https://thetvdb.com). Please consider adding missing information or [subscribing](https://thetvdb.com/subscribe).

## License

Licensed under MIT. See [LICENSE](LICENSE) for details.
