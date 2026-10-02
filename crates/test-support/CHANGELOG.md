# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [0.1.0] - 2026-10-02

### Features

- **(events)** keep refused events in memory - ([dcfff45](https://github.com/MrEhbr/yokoku/commit/dcfff45f63993d364638b617ba962b25da70f147)) `+177 / -507 across 27 file(s)` - Aleksei Burmistrov
- **(nix)** run Jellyfin and Transmission for development and integration tests - ([ac038bb](https://github.com/MrEhbr/yokoku/commit/ac038bb11943b69589467d2ff00bdb28be925e1c)) `+479 / -39 across 10 file(s)` - Aleksei Burmistrov

### Refactoring

- **(db)** open transactions from the pool directly - ([88d0afd](https://github.com/MrEhbr/yokoku/commit/88d0afda241db649f44b3ef0827d3f821d8f6deb)) `+6 / -10 across 6 file(s)` - Aleksei Burmistrov
- **(events)** make the event log concrete - ([83a9f38](https://github.com/MrEhbr/yokoku/commit/83a9f38e6fbf91cf018ef9dfdc940e592d518fe4)) `+462 / -518 across 30 file(s)` - Aleksei Burmistrov
- **(events)** put the event log's storage behind EventStore - ([501d506](https://github.com/MrEhbr/yokoku/commit/501d5069bf0305202761fb2218319efaeac94c28)) `+250 / -182 across 23 file(s)` - Aleksei Burmistrov
- merge the adapter crates into yokoku-infra - ([c14927a](https://github.com/MrEhbr/yokoku/commit/c14927a5ef1b652597d5db64914ce296dd6b36b1)) `+12086 / -12247 across 190 file(s)` - Aleksei Burmistrov
- merge the feature crates into yokoku-core - ([e91cb85](https://github.com/MrEhbr/yokoku/commit/e91cb85b0e074602ca182b8f769e690522b656bb)) `+11538 / -11700 across 262 file(s)` - Aleksei Burmistrov

### Tests

- add a shared test-support crate - ([39608ef](https://github.com/MrEhbr/yokoku/commit/39608eff47a63898ab26f0f21aa5f8a9a4a01b4a)) `+294 / -74 across 11 file(s)` - Aleksei Burmistrov

### Statistics

- 8 commit(s) contributed to the release.
- 1 day(s) between first and last commit.
- 8 commit(s) parsed as conventional.
- Diff totals: +25292 / -25277 across 559 file change(s) (sum across commits, may double-count files touched in multiple commits).


