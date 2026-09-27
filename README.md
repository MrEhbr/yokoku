# yokoku

[![CI](https://github.com/mrehbr/yokoku/actions/workflows/checks.yml/badge.svg)](https://github.com/mrehbr/yokoku/actions)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024+-orange)](https://www.rust-lang.org)

TBD

## Quick Start

```bash
# Build
just build

# Run
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
just run [ARGS] # Run the application
```



### Nix Development Shell

```bash
nix develop # Enter dev environment
# or
direnv allow # Auto-load with direnv
```



## Project Structure

```
crates/
  yokoku/             # Binary: composition root and CLI
    src/
      args.rs         # CLI root, global flags, command dispatch
      config.rs       # Config type and layered loading
      logging.rs      # tracing setup
      commands/       # One module per subcommand
    benches/          # Criterion benchmarks
    tests/            # Integration tests
  <name>/             # Library crates, see docs/ARCHITECTURE.md §3
config/               # Configuration files
docs/                 # Requirements and architecture
```

## Adding a Command

`crates/yokoku/src/commands/calendar.rs` is a small worked example. Paths
below are relative to `crates/yokoku/`. To add your own:

1. Copy it to `src/commands/<name>.rs` and adjust its `Args` and `run`.
2. Register the module in `src/commands/mod.rs`.
3. Add a variant to `Command` in `src/args.rs` and dispatch it in `route`.
4. Add its config section to `Config` in `src/config.rs`.

Configuration and logging are resolved once in `route`, so a command only
receives `&Config` and its own parsed `Args`. `--config` and `--verbosity`
are global and work on either side of the subcommand.

A command owns its own config section next to its code, the way
`logging.rs` owns `LogConfig`. Declare optional flags and layer them over
the loaded values in `apply_overrides`:

```rust
#[arg(long, short = 'd')]
pub days: Option<u16>,

fn apply_overrides(&self, config: &CalendarConfig) -> CalendarConfig {
    let mut resolved = config.clone();

    if let Some(days) = self.days {
        resolved.days = Some(days);
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
- **Publish Release**: Automatic binary releases on tags

## Data sources

<a href="https://www.themoviedb.org"><img src="https://www.themoviedb.org/assets/v4/logos/v2/blue_short-8e7b30f73a4020692ccca9c88bafe5dcb6f8a62a4c6bc55cd9ba82bb2cd95f6c.svg" alt="TMDB" height="16"></a>

This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.

<a href="https://thetvdb.com"><img src="https://thetvdb.com/images/attribution/logo1.png" alt="TheTVDB" height="32"></a>

Metadata provided by [TheTVDB](https://thetvdb.com). Please consider adding missing information or [subscribing](https://thetvdb.com/subscribe).

## License

Licensed under MIT. See [LICENSE](LICENSE) for details.
