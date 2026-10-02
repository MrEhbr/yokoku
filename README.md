# yokoku

[![CI](https://github.com/mrehbr/yokoku/actions/workflows/checks.yml/badge.svg)](https://github.com/mrehbr/yokoku/actions)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-user%20guide-blue)](https://mrehbr.github.io/yokoku/)

Self-hosted manager for a movie and TV library: tracks releases, imports torrents from Transmission and organizes files for Jellyfin.

See the [user guide](https://mrehbr.github.io/yokoku/) for installation, configuration and troubleshooting.

## Install

- **Docker**: `ghcr.io/mrehbr/yokoku`, web UI on port 8080.
- **Archive**: static Linux binaries on the [releases](https://github.com/mrehbr/yokoku/releases) page.
- **NixOS**: the `services.yokoku` module from [nur-packages](https://github.com/MrEhbr/nur-packages).

## Configuration

Settings come from a TOML file (`--config`); [`config/app.toml`](config/app.toml) lists every one.
Any of them can be set with `YOKOKU__<SECTION>__<KEY>`, like `YOKOKU__WEB__PORT`, and most can be
changed on the Settings page. A TMDB token is required to search and add items.

## Development

```bash
nix develop        # or direnv allow
just web serve     # the app at http://127.0.0.1:8080
just test
just lint
just services      # Jellyfin and Transmission for integration tests
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the rest.

## Data sources

<a href="https://www.themoviedb.org"><img src="https://www.themoviedb.org/assets/v4/logos/v2/blue_short-8e7b30f73a4020692ccca9c88bafe5dcb6f8a62a4c6bc55cd9ba82bb2cd95f6c.svg" alt="TMDB" height="16"></a>

This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.

<a href="https://thetvdb.com"><img src="https://thetvdb.com/images/attribution/logo1.png" alt="TheTVDB" height="32"></a>

Metadata provided by [TheTVDB](https://thetvdb.com). Please consider adding missing information or [subscribing](https://thetvdb.com/subscribe).

## License

MIT. See [LICENSE](LICENSE).
