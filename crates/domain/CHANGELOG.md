# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [0.1.0] - 2026-10-02

### Bug Fixes

- **(domain)** ignore unfollowed specials in next, last and status - ([2f4c35a](https://github.com/MrEhbr/yokoku/commit/2f4c35a61215d2a9801f984ccd0e750e8710dce5)) `+78 / -11 across 5 file(s)` - Aleksei Burmistrov
- **(naming)** keep one year for titles that already end in it - ([962dcc4](https://github.com/MrEhbr/yokoku/commit/962dcc435d52043a6c08764d3f2f08f11753d190)) `+65 / -20 across 6 file(s)` - Aleksei Burmistrov
- stop concurrent saves from undoing each other - ([72d85e4](https://github.com/MrEhbr/yokoku/commit/72d85e407706c908a443abd0c6b9b5b47ad77a51)) `+337 / -115 across 30 file(s)` - Aleksei Burmistrov
- move file targets with renumbered episodes - ([1c7eb07](https://github.com/MrEhbr/yokoku/commit/1c7eb070c5013d875f3ddaf01950b2a75ec38434)) `+291 / -29 across 12 file(s)` - Aleksei Burmistrov

### Features

- **(cli)** add list, show, monitor, numbering and remove - ([cdc55fd](https://github.com/MrEhbr/yokoku/commit/cdc55fd41d414df76ffe80ec1c0e6e8b700f7bac)) `+778 / -9 across 22 file(s)` - Aleksei Burmistrov
- **(config)** read settings each time they are used - ([3160092](https://github.com/MrEhbr/yokoku/commit/31600922bf0e0e29ad2a5f71eee57ab5c3306ffc)) `+595 / -391 across 59 file(s)` - Aleksei Burmistrov
- **(config)** show secrets masked instead of redacted - ([160b9c3](https://github.com/MrEhbr/yokoku/commit/160b9c3dff3da694045e7f6e5347868bdd6e9685)) `+515 / -24 across 9 file(s)` - Aleksei Burmistrov
- **(db)** store alternate titles of series and movies - ([d48bacf](https://github.com/MrEhbr/yokoku/commit/d48bacfd36e1e86b7ffdeefa477386eaca3ed4b4)) `+72 / -19 across 19 file(s)` - Aleksei Burmistrov
- **(detect)** plan imports against the library - ([d23ebc0](https://github.com/MrEhbr/yokoku/commit/d23ebc091d32d0ee03e51a52e7b13bacf98e1dc7)) `+752 / -2 across 10 file(s)` - Aleksei Burmistrov
- **(domain)** add series and movie catalog model - ([ef63d69](https://github.com/MrEhbr/yokoku/commit/ef63d690ae60601513a78b2de8c6aa5c98227d06)) `+798 / -9 across 11 file(s)` - Aleksei Burmistrov
- **(domain)** tell whether two file targets overlap - ([34bc83a](https://github.com/MrEhbr/yokoku/commit/34bc83a116a8d068a18802f0af4b82c4daf22e2e)) `+48 / -0 across 2 file(s)` - Aleksei Burmistrov
- **(domain)** parse episode spans like S01E01-E03 - ([a38c7ff](https://github.com/MrEhbr/yokoku/commit/a38c7ffae9ecf1bcb918b543b2e2145977e9a149)) `+49 / -3 across 3 file(s)` - Aleksei Burmistrov
- **(domain)** parse ids from their UUID text - ([78cfd12](https://github.com/MrEhbr/yokoku/commit/78cfd123639388f5819e21d1f1b30dafb5cc122f)) `+24 / -1 across 2 file(s)` - Aleksei Burmistrov
- **(domain)** show string enums through Display - ([cad2efe](https://github.com/MrEhbr/yokoku/commit/cad2efe1a6f9e90f3bea8369653b49c11e79327a)) `+62 / -2 across 6 file(s)` - Aleksei Burmistrov
- **(domain)** describe events, streams and subtitle tags through Display - ([dd3461e](https://github.com/MrEhbr/yokoku/commit/dd3461ea89f21d8c8176da5757b35ce0a3ea1250)) `+116 / -3 across 3 file(s)` - Aleksei Burmistrov
- **(events)** add file and import events - ([f95c363](https://github.com/MrEhbr/yokoku/commit/f95c363a7befdd43b93be2df213e1162c042844c)) `+195 / -14 across 10 file(s)` - Aleksei Burmistrov
- **(events)** add TorrentAdded and DownloadCompleted - ([cd5926c](https://github.com/MrEhbr/yokoku/commit/cd5926cd06e296f19a2eac62b2c14fb70d625a50)) `+63 / -4 across 4 file(s)` - Aleksei Burmistrov
- **(events)** say which items an event concerns - ([e5494a8](https://github.com/MrEhbr/yokoku/commit/e5494a89cbcfeb04a2698122441104af6d9a367d)) `+158 / -22 across 12 file(s)` - Aleksei Burmistrov
- **(events)** follow a command through its events with a correlation id - ([9060d05](https://github.com/MrEhbr/yokoku/commit/9060d054679ddf1bae3121be2b6fb068862a2d16)) `+331 / -123 across 26 file(s)` - Aleksei Burmistrov
- **(media)** [**breaking**] give every item a root and a fixed folder, scan only item folders - ([8b96148](https://github.com/MrEhbr/yokoku/commit/8b96148dce10a0e99b20a7bea94772a5748a0ba8)) `+944 / -398 across 47 file(s)` - Aleksei Burmistrov
- **(naming)** add Jellyfin naming templates - ([694d5e8](https://github.com/MrEhbr/yokoku/commit/694d5e86e4e5e14c40ae41609e88183396ad196e)) `+773 / -6 across 15 file(s)` - Aleksei Burmistrov
- **(settings)** reload a running service when settings change - ([bee7e67](https://github.com/MrEhbr/yokoku/commit/bee7e676f4159cabb92c1203d108bbc28f43a085)) `+95 / -9 across 10 file(s)` - Aleksei Burmistrov
- add durable event log with ordered delivery - ([da4f758](https://github.com/MrEhbr/yokoku/commit/da4f758a694045ccbf262d574ec76301cde559ec)) `+2273 / -66 across 21 file(s)` - Aleksei Burmistrov
- add upcoming, calendar and missing views - ([55beeee](https://github.com/MrEhbr/yokoku/commit/55beeee99126fc19e53bd71e7c7f0882cf6dda52)) `+700 / -10 across 20 file(s)` - Aleksei Burmistrov
- refresh due metadata on a schedule - ([2b8623c](https://github.com/MrEhbr/yokoku/commit/2b8623ced5f0cb43603fb2b543a5a9a0c6afa3ed)) `+162 / -5 across 14 file(s)` - Aleksei Burmistrov
- download and cache item posters, backdrops and logos - ([a4981b5](https://github.com/MrEhbr/yokoku/commit/a4981b54cd88d714797ac72374eb0b14e4b5afef)) `+1087 / -128 across 53 file(s)` - Aleksei Burmistrov
- store item descriptions and episode overviews - ([9e6e424](https://github.com/MrEhbr/yokoku/commit/9e6e424d41d06fc532999e6b8fbf2f81cc674a61)) `+366 / -62 across 30 file(s)` - Aleksei Burmistrov
- choose the season of a series torrent's files named without one - ([da1f810](https://github.com/MrEhbr/yokoku/commit/da1f810168f4b5e8c75acadc031e5f6c980418f1)) `+252 / -76 across 20 file(s)` - Aleksei Burmistrov

### Miscellaneous Chores

- set up workspace skeleton - ([f6eadfc](https://github.com/MrEhbr/yokoku/commit/f6eadfcdfa3e749f401f5407dcaeebb7a4c9f5a6)) `+738 / -454 across 47 file(s)` - Aleksei Burmistrov

### Performance

- **(media)** read an item's files, details and folders once - ([31d8cc6](https://github.com/MrEhbr/yokoku/commit/31d8cc68431cc0a655ecb42a9e5995734c66017b)) `+161 / -39 across 7 file(s)` - Aleksei Burmistrov

### Refactoring

- **(config)** [**breaking**] move configuration into yokoku-config - ([4dbf314](https://github.com/MrEhbr/yokoku/commit/4dbf314c4d8587ea18331f2b02afa853bc2dbf18)) `+752 / -683 across 70 file(s)` - Aleksei Burmistrov
- **(config)** mask secrets by type instead of by key list - ([ebd3fad](https://github.com/MrEhbr/yokoku/commit/ebd3faddb1fd42193cd012c2fe086454b4ef6a56)) `+53 / -37 across 4 file(s)` - Aleksei Burmistrov
- **(db)** decode columns through typed sqlx wrappers over free fns - ([aeeefd0](https://github.com/MrEhbr/yokoku/commit/aeeefd0830be9310c51fd59821ff1758e646d3ba)) `+476 / -444 across 16 file(s)` - Aleksei Burmistrov
- **(detect)** build import plans through methods on their types - ([77812da](https://github.com/MrEhbr/yokoku/commit/77812da71a9340ae63c9a8ef9948099cc9ede7dc)) `+200 / -203 across 7 file(s)` - Aleksei Burmistrov
- **(domain)** move detect's MatchTarget to domain as FileTarget - ([1b494a7](https://github.com/MrEhbr/yokoku/commit/1b494a7aed504e64bb32fa387fd2e121e4e4761c)) `+28 / -23 across 6 file(s)` - Aleksei Burmistrov
- **(domain)** track the file id on episodes and movies - ([5985d6e](https://github.com/MrEhbr/yokoku/commit/5985d6ec36779af5d8f995f690ba1f2d7a67577b)) `+56 / -45 across 17 file(s)` - Aleksei Burmistrov
- **(domain)** move the Clock port to domain - ([9409879](https://github.com/MrEhbr/yokoku/commit/9409879f608deaaa9b16b58a68323d6713067f1c)) `+20 / -17 across 11 file(s)` - Aleksei Burmistrov
- **(domain)** move ItemId and MediaKind to domain - ([f54ffe3](https://github.com/MrEhbr/yokoku/commit/f54ffe3aecf33cfb287570ffd0e346b91985d84f)) `+59 / -57 across 20 file(s)` - Aleksei Burmistrov
- **(domain)** share one StorageError across all ports - ([1a74624](https://github.com/MrEhbr/yokoku/commit/1a746241e4ab5a938c4669f959115322c68b4605)) `+95 / -153 across 25 file(s)` - Aleksei Burmistrov
- **(domain)** merge FileTarget impl blocks - ([8e617dc](https://github.com/MrEhbr/yokoku/commit/8e617dc4e607ad30ac1f2015864d8ff17bfb1182)) `+0 / -2 across 1 file(s)` - Aleksei Burmistrov
- **(domain)** group the value types into fewer modules - ([ebc7201](https://github.com/MrEhbr/yokoku/commit/ebc7201f25d77971787ae6c7f3114154790ecc89)) `+534 / -556 across 23 file(s)` - Aleksei Burmistrov
- **(domain)** name items through ItemName - ([200d1cb](https://github.com/MrEhbr/yokoku/commit/200d1cbe03c735df3c70ef87d9f211d227329feb)) `+48 / -31 across 10 file(s)` - Aleksei Burmistrov
- **(domain)** construct series and movies with new - ([08c4d21](https://github.com/MrEhbr/yokoku/commit/08c4d2116b6f9630a2a0135b1c8c18790fb08c0d)) `+69 / -69 across 18 file(s)` - Aleksei Burmistrov
- **(domain)** inline has_aired into its two callers - ([95c97d2](https://github.com/MrEhbr/yokoku/commit/95c97d217fc03b114a9b0f1c9ca2aafa4c762130)) `+3 / -6 across 1 file(s)` - Aleksei Burmistrov
- **(domain)** build a span from episode refs with EpisodeSpan::from_refs - ([01059ce](https://github.com/MrEhbr/yokoku/commit/01059ce15d8af999d5c3fbe1fbc6c498f9099b43)) `+25 / -16 across 3 file(s)` - Aleksei Burmistrov
- **(events)** handle each event type in its own Handler - ([b725210](https://github.com/MrEhbr/yokoku/commit/b725210480420c2ac0de91dc5d70388f45ace0e4)) `+321 / -185 across 20 file(s)` - Aleksei Burmistrov
- **(naming)** render an item's title and year through Template - ([c24b53c](https://github.com/MrEhbr/yokoku/commit/c24b53cfe5fecbc4b7b53b8b35e9050b884f7b3b)) `+21 / -23 across 2 file(s)` - Aleksei Burmistrov
-  [**breaking**]use UUIDv7 for domain ids - ([bc0dea1](https://github.com/MrEhbr/yokoku/commit/bc0dea101af19097a90860670bbbfa5a90106997)) `+56 / -16 across 8 file(s)` - Aleksei Burmistrov
- move SubtitleTags to domain - ([5c36a0f](https://github.com/MrEhbr/yokoku/commit/5c36a0f477f755ea6e223fff8c723fa9917c17da)) `+15 / -11 across 5 file(s)` - Aleksei Burmistrov
- use match for file and progress status branching - ([6eeadcc](https://github.com/MrEhbr/yokoku/commit/6eeadcc7e20b8acc2d596b38ec5ef5a186983cc1)) `+18 / -22 across 3 file(s)` - Aleksei Burmistrov
- move the event contract into domain - ([109531d](https://github.com/MrEhbr/yokoku/commit/109531de0be0dd82597e958ff951e84cb1aeed70)) `+544 / -547 across 9 file(s)` - Aleksei Burmistrov
- move title_with_year into the domain - ([4f1989c](https://github.com/MrEhbr/yokoku/commit/4f1989ccc05ae963aa1b5914298ed519358d0e36)) `+35 / -28 across 9 file(s)` - Aleksei Burmistrov
- rename modules after the ports and services they hold - ([a593466](https://github.com/MrEhbr/yokoku/commit/a5934664dcc6399788684f66acabbaab72609d2f)) `+378 / -378 across 18 file(s)` - Aleksei Burmistrov
- split episode spans out of domain series - ([ec52c20](https://github.com/MrEhbr/yokoku/commit/ec52c204464b64742c227d8ef8cf95a3e770f6da)) `+516 / -506 across 7 file(s)` - Aleksei Burmistrov
- move single-crate dependencies out of workspace root - ([085e0e5](https://github.com/MrEhbr/yokoku/commit/085e0e557d8926c39586e5f0f674822545e2d4d1)) `+26 / -52 across 12 file(s)` - Aleksei Burmistrov
- group crate dependencies by shared and local - ([adb6714](https://github.com/MrEhbr/yokoku/commit/adb6714b86982d1068f1c5639eb186a12c10f658)) `+29 / -17 across 10 file(s)` - Aleksei Burmistrov
- merge the feature crates into yokoku-core - ([e91cb85](https://github.com/MrEhbr/yokoku/commit/e91cb85b0e074602ca182b8f769e690522b656bb)) `+11538 / -11700 across 262 file(s)` - Aleksei Burmistrov

### Tests

- **(db)** cover bad stored values and string_enum round-trips - ([7341ad2](https://github.com/MrEhbr/yokoku/commit/7341ad27e8aea7e7f30db5f24d43bc9d1c9ffe97)) `+168 / -0 across 2 file(s)` - Aleksei Burmistrov
- **(domain)** lock the settings event's format and cover spans, masking and subtitle tags - ([b6e91f9](https://github.com/MrEhbr/yokoku/commit/b6e91f95dfe0c1ff995e5b2adfef98fc2fc9b0d2)) `+77 / -0 across 4 file(s)` - Aleksei Burmistrov

### Statistics

- 59 commit(s) contributed to the release.
- 5 day(s) between first and last commit.
- 59 commit(s) parsed as conventional.
- Diff totals: +29038 / -17881 across 1138 file change(s) (sum across commits, may double-count files touched in multiple commits).


