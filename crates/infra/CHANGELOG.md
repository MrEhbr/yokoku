# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [0.1.0] - 2026-10-02

### Bug Fixes

- **(db)** check the revision in the save's own upsert - ([a0b5aa7](https://github.com/MrEhbr/yokoku/commit/a0b5aa72c327be08502196c2edac0e73de22a79e)) `+89 / -40 across 6 file(s)` - Aleksei Burmistrov
- **(fs)** let a panic in blocking file work reach its caller - ([6cb6d31](https://github.com/MrEhbr/yokoku/commit/6cb6d319effb715efff9cf0b6dd638249bdbb65c)) `+20 / -1 across 1 file(s)` - Aleksei Burmistrov

### Features

- **(integrations)** sync watched files from the Jellyfin user's played items - ([92e7573](https://github.com/MrEhbr/yokoku/commit/92e75734adcc561019e8cada7c63ebf6e57e9e72)) `+760 / -16 across 20 file(s)` - Aleksei Burmistrov
- **(nix)** run Jellyfin and Transmission for development and integration tests - ([ac038bb](https://github.com/MrEhbr/yokoku/commit/ac038bb11943b69589467d2ff00bdb28be925e1c)) `+479 / -39 across 10 file(s)` - Aleksei Burmistrov
- **(review)** match imports in a table that keeps partial matches - ([cc29a06](https://github.com/MrEhbr/yokoku/commit/cc29a068f167dbfe4ec9c8b7114c0ce36881291c)) `+1397 / -663 across 25 file(s)` - Aleksei Burmistrov

### Performance

- **(media)** read an item's files, details and folders once - ([31d8cc6](https://github.com/MrEhbr/yokoku/commit/31d8cc68431cc0a655ecb42a9e5995734c66017b)) `+161 / -39 across 7 file(s)` - Aleksei Burmistrov

### Refactoring

- **(db)** open transactions from the pool directly - ([88d0afd](https://github.com/MrEhbr/yokoku/commit/88d0afda241db649f44b3ef0827d3f821d8f6deb)) `+6 / -10 across 6 file(s)` - Aleksei Burmistrov
- **(db)** bind file target columns at each statement - ([6d59ae0](https://github.com/MrEhbr/yokoku/commit/6d59ae033ad5f00d60506ccad4dc909c46309be8)) `+31 / -19 across 1 file(s)` - Aleksei Burmistrov
- **(db)** name storage modules after their tables - ([76daa22](https://github.com/MrEhbr/yokoku/commit/76daa2267057284155a59e26adac4a5adc7c1d2c)) `+1410 / -1413 across 17 file(s)` - Aleksei Burmistrov
- **(domain)** construct series and movies with new - ([08c4d21](https://github.com/MrEhbr/yokoku/commit/08c4d2116b6f9630a2a0135b1c8c18790fb08c0d)) `+69 / -69 across 18 file(s)` - Aleksei Burmistrov
- **(events)** import the event contract from yokoku-domain only - ([f0bfcbc](https://github.com/MrEhbr/yokoku/commit/f0bfcbcee182c836f334783c83ee2f8ecde8322a)) `+174 / -133 across 51 file(s)` - Aleksei Burmistrov
- **(fs)** build every FsError with FsError::new - ([46d83e8](https://github.com/MrEhbr/yokoku/commit/46d83e89a6c3fcf39be1dae1335f020b48212f73)) `+39 / -32 across 3 file(s)` - Aleksei Burmistrov
- **(jellyfin)** inline the unavailable error helper - ([0a9fe61](https://github.com/MrEhbr/yokoku/commit/0a9fe61a301669b13a9b4f92c3051a61a2e391ca)) `+3 / -7 across 1 file(s)` - Aleksei Burmistrov
- **(jellyfin)** read JSON answers through one get helper - ([e6eed27](https://github.com/MrEhbr/yokoku/commit/e6eed274df18486d849596542b546ac803011388)) `+7 / -3 across 1 file(s)` - Aleksei Burmistrov
- **(media)** share the conversion of stored media info - ([e2d41e9](https://github.com/MrEhbr/yokoku/commit/e2d41e9f43f3758f89b3871d67a1ab10c7e883e1)) `+23 / -21 across 1 file(s)` - Aleksei Burmistrov
- **(metadata)** inline the unavailable and invalid error helpers - ([e012a00](https://github.com/MrEhbr/yokoku/commit/e012a00be305d9eec2a4aad3d323232d9525f33d)) `+9 / -16 across 3 file(s)` - Aleksei Burmistrov
- **(metadata)** parse source dates while deserializing - ([7cff418](https://github.com/MrEhbr/yokoku/commit/7cff418fca0cc70a8120abaddcbe17e75c159f77)) `+48 / -35 across 7 file(s)` - Aleksei Burmistrov
- **(transmission)** inline the unavailable error helper - ([bac1bfd](https://github.com/MrEhbr/yokoku/commit/bac1bfd547bd16f4f4a33e72900a13c9930e320c)) `+3 / -6 across 1 file(s)` - Aleksei Burmistrov
- merge the adapter crates into yokoku-infra - ([c14927a](https://github.com/MrEhbr/yokoku/commit/c14927a5ef1b652597d5db64914ce296dd6b36b1)) `+12086 / -12247 across 190 file(s)` - Aleksei Burmistrov
- merge the feature crates into yokoku-core - ([e91cb85](https://github.com/MrEhbr/yokoku/commit/e91cb85b0e074602ca182b8f769e690522b656bb)) `+11538 / -11700 across 262 file(s)` - Aleksei Burmistrov

### Tests

- keep proptest regressions where multi-file test binaries look - ([8981395](https://github.com/MrEhbr/yokoku/commit/8981395eb5a52a3c61182794964e8a7932c14f02)) `+22 / -22 across 6 file(s)` - Aleksei Burmistrov

### Statistics

- 21 commit(s) contributed to the release.
- 1 day(s) between first and last commit.
- 21 commit(s) parsed as conventional.
- Diff totals: +28374 / -26531 across 637 file change(s) (sum across commits, may double-count files touched in multiple commits).


