# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [0.1.0] - 2026-10-02

### Bug Fixes

- **(detect)** read episodes after a Jellyfin title, not inside it - ([d1c27d2](https://github.com/MrEhbr/yokoku/commit/d1c27d28a5150ba3aefdb352dfe8a0bc7552a0dd)) `+23 / -8 across 3 file(s)` - Aleksei Burmistrov

### Features

- **(integrations)** sync watched files from the Jellyfin user's played items - ([92e7573](https://github.com/MrEhbr/yokoku/commit/92e75734adcc561019e8cada7c63ebf6e57e9e72)) `+760 / -16 across 20 file(s)` - Aleksei Burmistrov
- **(library)** count a series' episode files and name a movie's file status - ([f8bec7f](https://github.com/MrEhbr/yokoku/commit/f8bec7fbb64938e65c505d0b9bd17121ad6425ef)) `+104 / -21 across 7 file(s)` - Aleksei Burmistrov
- **(library)** carry a season's monitoring to its episodes - ([f415e36](https://github.com/MrEhbr/yokoku/commit/f415e367ee55ef710d988661dbf806f3a53f9dd6)) `+36 / -2 across 3 file(s)` - Aleksei Burmistrov
- **(library)** show and filter by watched state - ([854bf73](https://github.com/MrEhbr/yokoku/commit/854bf73a852d96cab8d01580cd41ca331821983d)) `+351 / -60 across 17 file(s)` - Aleksei Burmistrov
- **(library)** group the library by root folder - ([3f683bf](https://github.com/MrEhbr/yokoku/commit/3f683bf1efbaa84c59a237af643412b0d5fb48a6)) `+126 / -11 across 4 file(s)` - Aleksei Burmistrov
- **(review)** match imports in a table that keeps partial matches - ([cc29a06](https://github.com/MrEhbr/yokoku/commit/cc29a068f167dbfe4ec9c8b7114c0ce36881291c)) `+1397 / -663 across 25 file(s)` - Aleksei Burmistrov
- **(web)** choose which files to delete when removing or freeing space - ([6b22e61](https://github.com/MrEhbr/yokoku/commit/6b22e615224d36654c0b2163a5ca8890ccc6a9b2)) `+492 / -132 across 13 file(s)` - Aleksei Burmistrov

### Performance

- **(media)** read an item's files, details and folders once - ([31d8cc6](https://github.com/MrEhbr/yokoku/commit/31d8cc68431cc0a655ecb42a9e5995734c66017b)) `+161 / -39 across 7 file(s)` - Aleksei Burmistrov
- **(media)** look up subtitle language names in a cached map - ([65e1f2d](https://github.com/MrEhbr/yokoku/commit/65e1f2dff503c088bbc06eb201549cfde571de47)) `+33 / -3 across 3 file(s)` - Aleksei Burmistrov
- **(media)** read only the item's files when deleting - ([705b53f](https://github.com/MrEhbr/yokoku/commit/705b53f0ba285bb3f1615d2e71808e5df84bc15e)) `+3 / -2 across 1 file(s)` - Aleksei Burmistrov
- **(media)** read only the affected items' files for renames and reviews - ([c0346a8](https://github.com/MrEhbr/yokoku/commit/c0346a8650d02de01be87288fe0f586031a95d86)) `+20 / -12 across 2 file(s)` - Aleksei Burmistrov

### Refactoring

- **(core)** delete library files only by their ids - ([19326f5](https://github.com/MrEhbr/yokoku/commit/19326f57a3aa5b6d8ed934b2e8419f804a896034)) `+22 / -49 across 3 file(s)` - Aleksei Burmistrov
- **(domain)** name items through ItemName - ([200d1cb](https://github.com/MrEhbr/yokoku/commit/200d1cbe03c735df3c70ef87d9f211d227329feb)) `+48 / -31 across 10 file(s)` - Aleksei Burmistrov
- **(domain)** construct series and movies with new - ([08c4d21](https://github.com/MrEhbr/yokoku/commit/08c4d2116b6f9630a2a0135b1c8c18790fb08c0d)) `+69 / -69 across 18 file(s)` - Aleksei Burmistrov
- **(downloads)** remove the unused Downloads::test_connection - ([d73e7a5](https://github.com/MrEhbr/yokoku/commit/d73e7a52d9d2e18089b7f42cb12a9755731ea8c0)) `+0 / -7 across 2 file(s)` - Aleksei Burmistrov
- **(events)** import the event contract from yokoku-domain only - ([f0bfcbc](https://github.com/MrEhbr/yokoku/commit/f0bfcbcee182c836f334783c83ee2f8ecde8322a)) `+174 / -133 across 51 file(s)` - Aleksei Burmistrov
- **(fs)** build every FsError with FsError::new - ([46d83e8](https://github.com/MrEhbr/yokoku/commit/46d83e89a6c3fcf39be1dae1335f020b48212f73)) `+39 / -32 across 3 file(s)` - Aleksei Burmistrov
- **(library)** name new item folders with Naming directly - ([cadd1f0](https://github.com/MrEhbr/yokoku/commit/cadd1f0a71c8f37b3e829f7c89eb6974d4ac3f48)) `+29 / -52 across 5 file(s)` - Aleksei Burmistrov
- **(library)** drop LibrarySort's serde derives - ([a0f0957](https://github.com/MrEhbr/yokoku/commit/a0f0957aee541b6cd18ea251c669e0ff145c5791)) `+1 / -3 across 1 file(s)` - Aleksei Burmistrov
- **(media)** group the import pipeline under media::import - ([cc6461d](https://github.com/MrEhbr/yokoku/commit/cc6461dfafad6b4fadc1b42ea130b3fafb904d99)) `+868 / -863 across 8 file(s)` - Aleksei Burmistrov
- **(media)** move files across devices through FileSystem::move_file - ([277c607](https://github.com/MrEhbr/yokoku/commit/277c6076ef1b9e23841cff7860c88437938d9222)) `+13 / -14 across 3 file(s)` - Aleksei Burmistrov
- **(media)** split the sidecar subtitle lookup from the folder listing - ([dda78f1](https://github.com/MrEhbr/yokoku/commit/dda78f135b71db4badc85c2718654343c71bf209)) `+18 / -7 across 1 file(s)` - Aleksei Burmistrov
- merge the feature crates into yokoku-core - ([e91cb85](https://github.com/MrEhbr/yokoku/commit/e91cb85b0e074602ca182b8f769e690522b656bb)) `+11538 / -11700 across 262 file(s)` - Aleksei Burmistrov

### Tests

- keep proptest regressions where multi-file test binaries look - ([8981395](https://github.com/MrEhbr/yokoku/commit/8981395eb5a52a3c61182794964e8a7932c14f02)) `+22 / -22 across 6 file(s)` - Aleksei Burmistrov

### Statistics

- 25 commit(s) contributed to the release.
- 2 day(s) between first and last commit.
- 25 commit(s) parsed as conventional.
- Diff totals: +16347 / -13951 across 478 file change(s) (sum across commits, may double-count files touched in multiple commits).


