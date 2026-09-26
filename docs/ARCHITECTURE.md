# Yokoku — Architecture

This document describes **how** Yokoku is built. For **what** it does, see [REQUIREMENTS.md](REQUIREMENTS.md).

---

## 1. Principles

1. **MVP limits features, not architecture.** Boundaries are right from the first commit.
2. **The core knows nothing about infrastructure.** Business logic never imports sqlx, reqwest, apalis or the web framework. It talks to the outside world through ports (traits) it owns.
3. **Modules are independent.** Feature modules never depend on each other. They share only value types (`domain`) and the event contract (`events`). The compiler enforces this through crate boundaries.
4. **Consistency where it matters, decoupling everywhere else.** A command inside a module is synchronous and transactional. Reactions across modules happen through a durable event log.
5. **Pure logic is isolated.** Filename detection and naming are pure functions with no IO and no async, tested with plain input/output tables.
6. **No speculative generality.** Every abstraction sits on a real boundary: IO, time, or a third-party library.

---

## 2. Technology

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust, edition 2024 | |
| Async runtime | `tokio` | |
| Web / UI | `topcoat` | Experimental ("expect breaking changes"). Kept in a thin adapter crate. Added after the core works. |
| Database | SQLite through `sqlx` (0.9) | Version set by `apalis-sqlite`. WAL mode, `foreign_keys=ON`, `busy_timeout`. |
| Migrations | `sqlx::migrate!` | One ordered set, owned by `db`. |
| Background jobs, cron | `apalis`, `apalis-sqlite`, `apalis-cron` | Jobs share the app's SQLite database. `apalis-workflow` is not used (see §10). |
| HTTP client | `reqwest` (rustls) | |
| Metadata | Own TMDB client (TVDB later) | Needs only a handful of endpoints. |
| Download client | `transmission-rpc` | Wrapped behind the `DownloadClient` port. |
| Filename parsing | `hunch`, wrapped in our own `ParsedName` | Tested against a corpus of real filenames. Can be replaced without touching callers. |
| Title matching | `strsim`, `unicode-normalization` | Case, punctuation and year ignored; works with Cyrillic. |
| Time | `jiff` | Time-zone aware. If sqlx has no `jiff` support, values are stored as ISO-8601 TEXT through a newtype. |
| Filesystem walk | `walkdir` | |
| Media probing | `ffprobe` subprocess | Optional; missing `ffprobe` only hides file details. |
| iCal feed | `icalendar` | |
| Errors | `thiserror` in libraries, `anyhow` only in the binary | |
| Async ports | `async-trait`, used as `Arc<dyn Port>` | Boxing cost is irrelevant next to network and disk IO. |
| Logging | `tracing` | A correlation id follows a command through its events, handlers and jobs. |
| Tests | `rstest`, `proptest`, `wiremock`, `tempfile`, `cargo-nextest` | `rstest` for case tables and fixtures, `proptest` for invariants. |

Versions are pinned in `[workspace.dependencies]` when the workspace is set up.

---

## 3. Crates

```
crates/
  domain/         yokoku-domain        Shared value types and rules
  events/         yokoku-events        Event contract, Subscriber trait, delivery loop
  detect/         yokoku-detect        Pure: downloaded files → ImportPlan
  naming/         yokoku-naming        Pure: NamingTemplate parse/render, sanitising

  library/        yokoku-library       Catalog, monitoring, metadata refresh, schedule queries
  downloads/      yokoku-downloads     Torrents, Transmission sync, seeding cleanup
  media/          yokoku-media         Import pipeline, review, scan, rename, delete, recycle
  integrations/   yokoku-integrations  Jellyfin rescan

  db/             yokoku-db            sqlx: migrations, all repository impls, event store
  metadata/       yokoku-metadata      MetadataProvider impls: TMDB, TVDB
  transmission/   yokoku-transmission  DownloadClient impl
  jobs/           yokoku-jobs          apalis workers and cron; queue port impls
  system/         yokoku-system        FileSystem, Clock, MediaProbe (ffprobe), MediaServer (Jellyfin HTTP)
  web/            yokoku-web           Topcoat pages (later)

  yokoku/         yokoku (bin)         Composition root, subscription registry, CLI
```

### Dependency rules

```
              domain ◀──────────── detect, naming
                ▲                      ▲
   events ──────┤                      │
     ▲          │                      │
     └── library · downloads · media · integrations      (feature modules)
                          ▲
   db · metadata · transmission · jobs · system · web    (adapters)
                          ▲
                       yokoku (bin)
```

| Crate kind | May depend on | Must not depend on |
|---|---|---|
| `domain` | std, `serde`, `jiff`, `thiserror` | anything else in the workspace |
| `detect`, `naming` | `domain` | IO, async, any module |
| `events` | `domain` | any module or adapter |
| Feature module | `domain`, `events`, pure crates it needs | other modules, any adapter, sqlx/reqwest/apalis/topcoat |
| Adapter | modules whose ports it implements | other adapters |
| `yokoku` | everything | — |

When a module needs another module's data, it declares its own narrow **read port**, and `db` implements it. Example: `media` needs a series' episode list for detection, so it declares `media::ports::Catalog`.

---

## 4. Domain (`yokoku-domain`)

Value types and rules shared by all modules. Examples:

- Identifiers: `SeriesId`, `MovieId`, `EpisodeId`, and later `DownloadId`, `ImportId`, `MediaFileId`. UUIDv7 newtypes created by the domain, so an aggregate and its events are complete before they are saved. Users refer to items by source id (`tmdb:1396`).
- `ExternalId { Tmdb(u64), Tvdb(u64) }`. An item stays bound to the provider it was added with.
- `Series` → `Season` → `Episode` and `Movie`: aggregates with public fields. Seasons and episodes are kept ordered by number.
- `SeriesMetadata`, `MovieMetadata`: an item as its source describes it. `library::MetadataProvider` returns these.
- `EpisodeRef { season, episode }` (`S01E02`) and `EpisodeSpan`, consecutive episodes of one season for multi-episode files (`S01E01-E03`).
- `FileTarget { Episodes { series, span }, Movie(movie) }`: what a video file holds. `detect` produces it; `media` stores it.
- `Numbering { Standard, Absolute }` (FR-1.8).
- Air dates are dates only (`jiff::civil::Date`). TMDB provides no time; a nullable time column is added when a source provides one.
- `MonitorPreset { All, Future, LatestSeason, None }` (FR-2.2).
- `SeriesStatus { Continuing, OnBreak, Ended }`, `MovieStatus { Announced, InCinemas, Released }`, `FileStatus { Downloaded, Missing, Upcoming }`: derived from dates, files and today, never stored.
- `Confidence { Unknown, Guess, Certain }` (FR-4.10) and `SubtitleTags` (language, SDH, forced), shared by `detect` and `naming`.
- `Clock`: the one port every module needs, so it lives here rather than in a module.
- Later: `ImportMode { HardLink, Copy, Move }` (FR-3.6).

### Rules

| Rule | Behaviour |
|---|---|
| `Series::status(today)` | `Ended` if the source says ended or canceled; `Continuing` if a followed episode airs today or later; otherwise `OnBreak`. |
| `Series::next_episode` / `last_aired` | Over followed episodes: every regular episode, and specials only when their season and the episode are monitored. The library list sorts by the same next date. |
| `Movie::status(today)` | `Released` from the digital or physical date, `InCinemas` from the cinema date, otherwise `Announced`. |
| `Episode::file_status(today)` | `Downloaded` with a file; `Missing` from the day after its air date; otherwise `Upcoming`. |
| `Movie::file_status(today)` | `Missing` only once the movie is `Released`. |
| `Series::monitored_episodes()` | Monitored at series, season and episode level (FR-2.1, 2.3). |
| `Series::add(metadata, preset, today, now)` | Applies the preset. Specials are never monitored by a preset. |
| `Series::refresh(metadata, now)` | Matches episodes by source id, so renumbered episodes keep id, flags and file. New seasons follow the series flag (specials excepted); new episodes follow their season. Episodes gone from the source are dropped. |
| `Series::absolute_to_ref(n)` | Counts episodes in order, excluding specials, which matches Jellyfin's default TMDB order (FR-4.9, FR-5.8). |

"Today" is the date in the user's configured time zone.

---

## 5. Feature modules

Each module has the same internal shape:

```
src/
  lib.rs        public API: the use cases
  model.rs      module-owned entities and state machines
  ports.rs      traits the module needs; implemented by adapters
  subscriber.rs reactions to events from other modules (if any)
  error.rs
```

### 5.1 `library`

Owns movies, series, seasons, episodes, monitoring flags, and a projection of the media file holding each episode and movie (`file: Option<MediaFileId>`). Tracking the id rather than a flag keeps the projection correct whatever order file events arrive in.

- **Use cases**, split by what they depend on:
  - `Library` (repositories + clock): list with filter and sort, series and movie details, set monitoring, set numbering, remove.
  - `MetadataSync` (+ metadata source): search, add (applying a monitor preset), refresh one item or all.
  - `Schedule` (repositories + clock): calendar for a date range, upcoming, missing grouped by series. Only monitored items appear (FR-2.3).
- **Later:** iCal feed (served by `web`).
- **Ports:** `SeriesRepo`, `MovieRepo` (whole aggregates, events in the same transaction), `MetadataProvider`, `Clock`. The list is built from the aggregates; a dedicated query port comes only if the library grows large enough to need one.
- **Emits:** `SeriesAdded`, `MovieAdded`, `SeriesRemoved`, `MovieRemoved`.
- **Subscribes to:** `FilesFound`, `FilesImported`, `FileDeleted` (`FileTracker` updates the file projection). `FileRenamed` keeps the file id, so the projection needs no change.

### 5.2 `downloads`

Owns the downloads Yokoku knows about and the Transmission connection settings.

- **Use cases:**
  - Test the connection.
  - Add a torrent (magnet link or .torrent file), linked to a movie or series or left unlinked.
  - Sync with the client: progress, state, completion, and optionally torrents picked up by label or folder.
  - Remove a torrent once seeding is finished.
- **Ports:** `DownloadRepo`, `DownloadClient`, `Clock`.
- **Emits:** `TorrentAdded`, `DownloadCompleted` (once per download; the sync is idempotent).
- **Subscribes to:** `FilesImported` (marks the download imported so it can be removed after seeding).

Transmission runs on the same host as Yokoku. The paths it reports are used as-is; no path mapping.

### 5.3 `media`

Owns library files, root folders, naming settings, imports and the recycle folder.

- **Use cases:**
  - Plan an import from a download or from files found by a scan.
  - Review: match a row to episodes or a movie in the library, or skip it; approve once every other row has a conflict-free match. Skipped files are not offered again by later scans.
  - Run an approved import.
  - Root folders: add (absolute, existing, not overlapping another root), list, remove.
  - Scan root folders (FR-8.2, 8.3, 8.7): each entry of a root is detected on its own against series (series root) or movies (movie root). New files that are `Certain`, conflict-free and hold nothing already linked are linked in place; the rest of the entry becomes one import in review. Linked files missing from disk are forgotten with `FileDeleted { reason: External }`. A root that cannot be read fails the scan, so an unmounted disk never looks empty.
  - Rename with preview (FR-5.7): `Renamer::preview(scope)` lists the moves naming asks for, for the whole library, a series or a movie; `apply` makes them file by file. Subtitles beside a video (named after it) move with it and get normalised language tags. Files outside every root, whose item is gone, or that would share a path are skipped; a file already at the new path is never replaced, and that move is reported as failed. Folders left empty are removed up to the root.
  - Delete or recycle files; clean up the recycle folder.
  - Retry a failed import.
- **Ports:** `MediaRepo` (root folders, files, imports; one `save(changes, events)` so a use case commits everything in one transaction), `Catalog` (read-only view of `library` data), `FileSystem`, `Clock`; later `MediaProbe`, `ImportQueue`.
- **Emits:** `FilesFound`, `ImportNeedsReview`, `FilesImported`, `FileDeleted`, `FileRenamed`; later `ImportFailed`.
- **Subscribes to:** `DownloadCompleted` (plans an import), `SeriesRemoved`, `MovieRemoved` (delete or recycle files when asked).

### 5.4 `integrations`

- Rescans Jellyfin after `FilesImported`, `FileRenamed` and `FileDeleted`. A burst of events is debounced into one rescan.
- **Ports:** `MediaServer`.
- Future notifications (REQUIREMENTS §6) go here.

### 5.5 Settings

Each module owns its settings section: metadata provider and API keys in `library`, Transmission in `downloads`, root folders, naming, import mode and recycle in `media`, Jellyfin in `integrations`. All sections are stored in the database (FR-10.3). The settings screen composes them.

Bootstrap values that are needed before the database exists (database path, bind address, log format) come from the existing TOML file and `APP__*` environment variables.

---

## 6. Import pipeline

Imports of finished downloads and of scanned files that were not recognised go through the same pipeline and the same review screen (FR-4.11, FR-8.3).

```
DownloadCompleted ─┐
                   ├─▶ list files ─▶ detect::plan(files, catalog) ─▶ ImportPlan
scan: unknown file ┘                                                  │
                         ┌────────────────────────────────────────────┤
                         │ all rows Certain, no conflicts             │ otherwise
                         ▼                                            ▼
                      Approved ◀──────── user edits rows ──────── NeedsReview
                         │
                         ▼ ExecuteImport job (apalis, concurrency 1)
                     Importing ──▶ Done    (FilesImported)
                         └──────▶ Failed   (ImportFailed) ── retry ──▶ Approved
```

- **`detect::plan`** is a pure function (FR-4.1 – 4.13):
  - **Classify:** videos; subtitles attached by name prefix, by a folder named after the video, or to the only video; samples, extras and other files ignored.
  - **Parse:** `hunch` reads scene names; own rules add Russian `сезон`/`серия`, bare-number files, season from folders (`S02`, `Season 2`, `Specials`) and a date taking priority over years. A corpus of real-style names is the test table.
  - **Series:** a linked series is trusted. Otherwise titles are compared without accents or punctuation against title and original title; a year more than one off rules an item out; a tie means no match.
  - **Episodes:** `S01E02` must exist in the series; seasonless numbers use absolute numbering or the folder season; a date matches the one episode airing that day; a name without numbers matches a unique episode title.
  - **Confidence:** `Certain` only when both the item and the episode were determined firmly; otherwise `Guess`; nothing found is `Unknown`.
  - **Conflicts:** two rows for the same episode or movie, or a target that already has a file.
  - **Movies:** the largest video is the movie; other videos are ignored as extras. An unlinked download is tried as a series first, then as a movie.
- **`naming`** renders the target path, relative to the root folder, from patterns validated when settings are saved (FR-5):
  - One pattern per path component with tokens `{title}`, `{year}`, `{season}`, `{episodes}`, `{episode_title}`. A `[...]` group is dropped when a token inside has no value.
  - Defaults follow Jellyfin: `Title (Year)/Season 01/Title (Year) - S01E01 - Episode Title.ext` and `Title (Year)/Title (Year).ext`.
  - Every component is sanitised for Linux, macOS, Windows and SMB. File stems are capped at 200 bytes so subtitle suffixes always fit; folders at 255.
  - Subtitles take the video stem plus `.language[.sdh][.forced].ext`.
- **Execution** is idempotent per file. A row whose target already exists with the expected size, and whose source is gone (move mode), counts as done. Re-running after a crash is safe.
- **Concurrency 1** for `ExecuteImport` means two imports can never race on the same episode.
- **Hard links** that fail across filesystems fall back to copy, with a warning.
- **Unlinked torrents** are matched only against items already in the library. Anything unmatched goes to review.

---

## 7. Events

### 7.1 Model

The event log records **what happened**. It is not event sourcing: state tables remain the source of truth, and events are written next to them.

```
events               (id INTEGER PRIMARY KEY AUTOINCREMENT, payload JSON, occurred_at)
subscriber_positions (subscriber TEXT PRIMARY KEY, last_event_id)
failed_deliveries    (subscriber, event_id, error, attempts, failed_at)
```

The payload carries the event's `type` tag, so no separate kind column is needed; `json_extract(payload, '$.type')` filters by type.

- **Atomic append.** A module passes the events of a command to its repository, for example `ImportRepo::complete(&import, &events)`. The `db` adapter writes the state and the events in **one transaction**. Modules never see transactions.
- **Ordered delivery.** Each subscriber runs as one task that reads events after its saved position, in id order, and advances its position after each success.
- **Order is safe.** SQLite allows one writer at a time, so ids are always committed in id order. A reader can never skip an event whose transaction commits late.
- **At-least-once.** Handlers are idempotent.
- **Failures.** Retried with exponential backoff. After N attempts the failure is recorded in `failed_deliveries` and the subscriber moves on.
- **Wake-up.** `Database::commit` signals a `tokio::sync::watch` channel after each commit that wrote events. A signal sent while a subscriber is busy is not lost. A slow periodic poll is the fallback, and it also picks up events written by CLI commands running in another process.
- **Shutdown.** Delivery stops at the next await point. An event interrupted mid-handler is delivered again on the next run.
- **Rebuild.** A projection is rebuilt by deleting its row in `subscriber_positions`.
- **History (FR-9.1)** is a query over the event log. There is no separate history table.

### 7.2 Contract

`events` defines one `Event` enum, serialised with an explicit `type` tag. A stored event never changes meaning. A breaking change adds a new variant.

### 7.3 Catalog

| Event | Emitted by | Subscribers |
|---|---|---|
| `SeriesAdded { series, title }` | library | — (history) |
| `MovieAdded { movie, title }` | library | — (history) |
| `SeriesRemoved { series, title, delete_files }` | library | media |
| `MovieRemoved { movie, title, delete_files }` | library | media |
| `TorrentAdded { download, linked_item }` | downloads | — (history) |
| `DownloadCompleted { download, content_path, linked_item }` | downloads | media |
| `ImportNeedsReview { import, source }` | media | — (history) |
| `FilesFound { files }` | media (scan) | library, integrations |
| `FilesImported { import, files }` | media | library, downloads, integrations |
| `ImportFailed { import, reason }` | media | — (history) |
| `FileRenamed { file, from, to }` | media | integrations |
| `FileDeleted { file, path, target, reason }` | media | library, integrations |

Events carry the titles and paths that history needs to display, so history still reads correctly after the item is removed. File events list each file as `LinkedFile { file, path, target }`, where `target` is a `FileTarget`.

`FileDeleted.reason` is `External` for now; `User`, `ItemRemoved` and `Replaced` (with a `recycled` flag) come with deleting files from the app. A file removed outside the app is reported by the next scan with `External` (FR-8.7).

### 7.4 Registry

All subscriptions are declared in one file in the `yokoku` binary (`subscriptions.rs`), so every reaction in the system can be read in one place.

---

## 8. Jobs (`yokoku-jobs`)

apalis runs **work to do**: long-running, retryable jobs and schedules. It is not used to deliver events.

| Job | Trigger | Calls |
|---|---|---|
| `SyncDownloads` | cron, every 30 s | `downloads::sync` |
| `RefreshMetadata` | cron, every 6 h | enqueues `RefreshItem` for each item |
| `RefreshItem { item }` | queue | `library::refresh` |
| `ExecuteImport { import }` | queue, concurrency 1 | `media::execute_import` |
| `ScanLibrary` | cron, daily; on demand | `media::scan` |
| `CleanupRecycle` | cron, daily | `media::cleanup_recycle` |
| `RescanMediaServer` | queue, debounced | `integrations::rescan` |

Job handlers are thin. They decode the job and call one use case. Modules enqueue work through their own ports (for example `media::ports::ImportQueue`), which `jobs` implements.

---

## 9. Cross-cutting

### Runtime

One binary.
- `yokoku serve` runs the event subscribers, the apalis `Monitor` and, later, the web server. All of them shut down gracefully on SIGINT/SIGTERM.
- Other subcommands (`search`, `add`, `refresh`, `upcoming`, `missing`, `detect --dry-run`, `import`, `scan`) call the same use cases against the same database. They let every feature be used and tested before the UI exists. A command that writes events delivers them to every subscriber (`Delivery::catch_up`) before it exits, so the CLI needs no background process.

### Storage

- `yokoku_db::Database` implements every repository port, so wiring passes one `Arc<Database>` for each.
- SQL strings are literals with bind parameters; sqlx 0.9 rejects dynamically built SQL.
- Queries are checked by tests against real SQLite, not by `query!` macros, so no `DATABASE_URL` or `.sqlx` data is needed to build.
- Ids are stored as UUID text, timestamps as RFC 3339 text, dates as ISO 8601 text, enums as lowercase text. Paths are UTF-8 text; the filesystem adapter skips names that are not UTF-8.
- Saving an aggregate upserts its rows and deletes rows no longer present, so child ids (episodes) stay stable.

### Errors

- Each crate defines its errors with `thiserror`.
- Module errors separate domain failures (for example "episode does not exist", which the user can act on) from port failures (IO, network).
- Only the binary uses `anyhow`.
- User-facing messages never contain internal paths or SQL.

### Testing

| Layer | How |
|---|---|
| `domain`, `detect`, `naming` | Case tables with `rstest`, invariants with `proptest`. `detect` has a corpus of real release names with expected results. |
| Modules | Use cases against real adapters: storage through `yokoku-db` on SQLite `:memory:`, the real filesystem on a temporary directory, and a fixed `Clock`. No in-memory fakes of storage. |
| `db` | In-memory SQLite with real migrations. |
| `metadata`, `transmission`, `system` | HTTP adapters against `wiremock`. |
| End-to-end | Binary with a temporary directory, a real SQLite file and Transmission RPC served by `wiremock`. |

Tests that use `yokoku-db` from a crate that `db` depends on (`events`, the modules) take it as a dev-dependency and live in `tests/`. Cargo allows that cycle for integration tests. `#[cfg(test)]` unit tests would compile the crate a second time, so their trait implementations would not match.

### Workspace conventions

- Virtual workspace: the root `Cargo.toml` holds only `[workspace]`, `[workspace.dependencies]` and `[workspace.lints]`.
- `unsafe_code = "forbid"` in all crates.
- Existing tooling stays: `just`, `cargo-nextest`, `cargo-deny`, `typos`, `prek`.

---

## 10. Decisions and rejected alternatives

| Decision | Rejected alternative | Reason |
|---|---|---|
| Ports owned by modules; adapters depend on modules | Modules depend on `db`, `metadata`, `transmission` | Business logic would change with infrastructure. |
| Event log in SQLite, one position per subscriber, own code (~200 lines) | evento, cqrs-es/sqlite-es, hexeract-outbox, cratestack-outbox, eventsdb | Each fails at least one requirement: works with SQLite, joins our sqlx transaction, keeps a position per subscriber, no event sourcing, sqlx 0.9. |
| State tables are the source of truth | Event sourcing | TMDB is the real source of metadata. Rebuilding from events adds no value, and stored event schemas are costly to migrate. |
| In-process delivery | External broker (NATS, Redis, Kafka) | Adds deployment weight for a single-user, self-hosted app. |
| Import state machine in our tables | `apalis-workflow` | Review can pause for days, and the UI must query import state. |
| No actor framework | kameo, ractor | Mailboxes are in memory (not durable), and it would be a third messaging model next to events and jobs. The one real race (concurrent imports) is solved by concurrency 1. |
| Own TMDB client | `tmdb-api` crate | Few endpoints needed; low adoption. |
| `Arc<dyn Port>` + `async-trait` | Generic `App<I: Infra>` | Generics would spread through every signature. |
| SQLite | PostgreSQL | Single user, self-hosted, one file to back up. |
| Topcoat behind a thin web crate | — | Experimental; a breaking upgrade affects only `web`. |

---

## 11. Requirement map

| Requirement | Where |
|---|---|
| FR-1 Library | `library`, `metadata` |
| FR-2 Monitoring | `domain` (presets), `library` |
| FR-3 Download client | `downloads`, `transmission` |
| FR-4 Detection | `detect` (pure), `media` (plan, review, conflicts) |
| FR-5 Naming | `naming` (pure), `media` (apply, rename preview) |
| FR-6 Next episode | `domain` (`FileStatus`), `library` queries |
| FR-7 Calendar | `library` queries; iCal in `web` |
| FR-8 File management | `media`, `system` |
| FR-9 History | event log (`events`, `db`) |
| FR-10 General | `yokoku`, `web`, settings per module |

---

## 12. Build order

Follows REQUIREMENTS §5, with the foundation first.

0. **Workspace skeleton:** crates, dependency rules, lints, `db` with migrations, `events` delivery loop.
1. **Library + metadata:** `domain`, `library`, `metadata` (TMDB), CLI `search` / `add` / `refresh` / `show`.
2. **Next episode + calendar:** `library` queries, CLI `upcoming` / `missing`.
3. **Root folders + scan + manual match:** `media` scan, `detect` basics, `system`.
4. **Renaming:** `naming`, rename preview.
5. **Transmission:** `downloads`, `transmission`, `jobs` (`SyncDownloads`).
6. **Detection + review + auto import:** full `detect` corpus, the import pipeline.
7. **History, recycle folder, Jellyfin:** history query, `CleanupRecycle`, `integrations`.
8. **Web UI:** `web` on Topcoat.
