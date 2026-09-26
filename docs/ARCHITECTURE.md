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
  media/          yokoku-media         Import pipeline, review, scan, rename, delete
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
- `Series` → `Season` → `Episode` and `Movie`: aggregates with public fields. Seasons and episodes are kept ordered by number. Each carries its root folder path (`root`) and its folder name in that root (`folder`), both set when it is added and never changed.
- `RootFolder { kind, path }` and `RootKind { Series, Movies }` (FR-8.1): `library` checks an item's root against its kind; `media` stores the root folders.
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
  - `MetadataSync` (+ metadata source): search, add (applying a monitor preset, into a root folder of the item's kind, which the caller takes from `media`), refresh one item or all.
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
  - Pick up torrents added directly in the client (FR-3.3, `[transmission] pick_up_labels`, `pick_up_folder`; off unless set): the sync lists every torrent and takes on each one it has never stored that carries one of the labels or downloads at or under the folder, as an unlinked download with `TorrentAdded`, so detection works out what it is. A download that was removed is never taken on again; two syncs taking on the same torrent conflict on its hash and one of them skips it.
  - Remove a torrent once seeding is finished (FR-3.7, `[transmission] remove_after_seeding`, off by default): a sync removes a download with its data from the client when it was imported (`imported_at`, set by the `FilesImported` that names it) and the client reports seeding finished (Transmission `isFinished`: the ratio or idle limit was reached), then marks it `Removed` and emits `TorrentRemoved`. The library keeps its own hard link or copy; a download never imported, or still in review, stays.
- **Ports:** `DownloadRepo`, `DownloadClient` (version, add, torrents by info hash, all torrents, remove), `Clock`.
- **Sync:** every download not yet `Removed` takes the client's status (state, bytes done, rate, ETA, folder). The first sync that sees it complete sets `completed_at` and emits `DownloadCompleted` in the same transaction; a torrent missing from the client becomes `Removed` and is no longer synced. Adding a torrent syncs it at once, so a torrent that is already complete emits both events.
- **Emits:** `TorrentAdded`, `DownloadCompleted` (once per download; the sync is idempotent), `TorrentRemoved`.
- **Subscribes to:** `FilesImported` (marks the download imported so it can be removed after seeding).

Transmission runs on the same host as Yokoku. The paths it reports are used as-is; no path mapping.

`yokoku-transmission` speaks Transmission's RPC: it repeats a call once with the session id a 409 answer carries, sends basic auth when configured, and labels added torrents `yokoku`. A torrent counts as complete when its metadata is known, its selected size is above zero, nothing is left and it is not being checked. Tests replay recorded answers with wiremock; an ignored live test starts `transmission-daemon`, adds a torrent made from local data and syncs it to completion.

### 5.3 `media`

Owns library files, root folders, naming settings and imports.

- **Use cases:**
  - Plan an import from a download or from files found by a scan.
  - Review: match a row to episodes or a movie in the library, or skip it; approve once every other row has a conflict-free match. Skipped files are not offered again by later scans.
  - Run an approved import.
  - Root folders: add (absolute, existing, not overlapping another root), list, remove (refused while items belong to it).
  - Scan item folders (FR-8.2, 8.3, 8.7): the files under each item's `root/folder` are detected against that item alone (`Target::Series` / `Target::Movie`); nothing else in a root is read. New files that are `Certain`, conflict-free and hold nothing already linked are linked in place; the rest of the folder becomes one import in review. Linked files missing from disk are forgotten with `FileDeleted { reason: External }`. A root that cannot be read fails the scan, so an unmounted disk never looks empty.
  - Rename with preview (FR-5.7): `Renamer::preview(scope)` lists the moves naming asks for, for the whole library, a series or a movie; `apply` makes them file by file, inside the item's `root/folder`. Subtitles beside a video (named after it) move with it and get normalised language tags. Files outside every root, whose item is gone, or that would share a path are skipped; a file already at the new path is never replaced, and that move is reported as failed. Folders left empty are removed up to the root.
  - Delete files (FR-8.4, 8.5): `Deleter::delete(target)` removes the files holding an episode span or movie, with their subtitles and folders left empty, committing each file with `FileDeleted { reason: User }`. Removing a series or movie with `delete_files` does the same for all its files (`ItemRemoved`).
  - Scan an added item (FR-8.8): the `media.scan_added` subscriber scans the new item's folder as above.
  - Retry a failed import.
  - File details (FR-8.6): the `media.probe` subscriber probes the files of `FilesFound` and `FilesImported` with `ffprobe` (`[files] ffprobe`) and stores duration, the video stream (codec, size; cover art is skipped) and each audio (language, codec, channels) and subtitle stream (language, forced) per file; details go with the file when it is removed and stay through renames. A file that cannot be probed, or a missing `ffprobe`, is only logged; `Prober::probe_missing` (`yokoku files probe`) reads every file never probed. `Prober::details(item)` adds the subtitle files beside each video.
- **Library lock:** scan, import, rename and delete change files on disk before they commit, so each holds the `LibraryLock` from its first read of library files until its last commit; a scan never sees a file that is placed but not yet stored. `LockFile` in `system` takes an exclusive `flock` on `<database>.lock`, so the CLI and `serve` wait for each other as well. An import holds it per import, from its claim to its commit.
- **Ports:** `MediaRepo` (root folders, files, imports; one `save(changes, events)` so a use case commits everything in one transaction), `Catalog` (read-only view of `library` data), `FileSystem`, `LibraryLock`, `MediaProbe` (`FfProbe` in `system`), `Clock`; later `ImportQueue`.
- **Emits:** `FilesFound`, `ImportNeedsReview`, `FilesImported`, `FileDeleted`, `FileRenamed`; later `ImportFailed`.
- **Subscribes to:** `DownloadCompleted` (plans an import), `SeriesAdded`, `MovieAdded` (scan the item's folder), `SeriesRemoved`, `MovieRemoved` (delete files when asked).

### 5.4 `integrations`

- Rescans Jellyfin after `FilesImported`, `FileRenamed` and `FileDeleted` (FR-10.4). The `Rescans` subscriber only records that a rescan is due (the latest request time, one row); `run_due(quiet)` rescans once no request arrived for the quiet period and clears the request only if it was not renewed meanwhile, so a burst leads to one rescan, a request made during a rescan is kept, and a failed rescan stays pending. `serve` checks every 10 s with a 30 s quiet period; the CLI rescans right after delivering events and only warns when Jellyfin cannot be reached. Off unless `[jellyfin] url` is set; the API key comes from `APP__JELLYFIN__API_KEY`.
- **Ports:** `MediaServer` (`JellyfinClient` in `system`: `POST /Library/Refresh`, `GET /System/Info`, `Authorization: MediaBrowser Token`), `RescanStore`.
- Future notifications (REQUIREMENTS §6) go here.

### 5.5 Settings

Each module owns its settings section: metadata provider in `library`, Transmission in `downloads`, root folders, naming and import mode in `media`, Jellyfin in `integrations`. The binary maps each section to its module's types.

Settings are layered, later over earlier: defaults, the TOML file, values stored in the database (FR-10.3), then `APP__*` environment variables. Stored values live in `settings (key, value)` by dotted key (`import.mode`) as JSON; `yokoku settings set|unset|list|get` edits them, and the settings screen will too. `set` loads the whole configuration with the new value and validates it (types, naming patterns, schedules, time zone) before storing, so a stored value cannot stop the app. Stored values are read once at start, so `serve` picks up a change when restarted; a stored value that no longer loads fails every command except `settings`, which can unset it.

Not stored: bootstrap values needed before the database opens (`database`, `log`), and secrets (TMDB token, Transmission password, Jellyfin API key), which come only from `APP__*` variables and are never printed. Commands that need no database never create one to read settings.

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
                         ▼ ExecuteImports job claims one approved import at a time
                     Importing ──▶ Done    (FilesImported)
                         └──────▶ Failed   (ImportFailed) ── retry ──▶ Approved
```

- **`detect::plan`** is a pure function (FR-4.1 – 4.13):
  - **Classify:** videos; subtitles attached by name prefix, by a folder named after the video, or to the only video; samples, extras and other files ignored.
  - **Parse:** `hunch` reads the whole path, folders included; own rules mark bare-number files, read `Specials` as season 0, keep Jellyfin `Title (Year)` titles as written and let a date take priority over years. A corpus of real-style names is the test table.
  - **Series:** a linked series is trusted. Otherwise titles are compared without accents or punctuation against the title, original title and alternate titles (TMDB `alternative_titles`, such as romanisations; refreshed with the metadata); a year more than one off rules an item out; a tie means no match.
  - **Episodes:** `S01E02` must exist in the series; numbers without a season use absolute numbering; a date matches the one episode airing that day; a name without numbers matches a unique episode title.
  - **Confidence:** `Certain` only when both the item and the episode were determined firmly; otherwise `Guess`; nothing found is `Unknown`.
  - **Conflicts:** two rows for the same episode or movie, or a target that already has a file.
  - **Movies:** the largest video is the movie; other videos are ignored as extras. An unlinked download is tried as a series first, then as a movie.
- **`naming`** renders, from the `[naming]` patterns (FR-5.6), an item's folder name (once, when it is added) and each file's path relative to that folder; an invalid pattern stops the app from starting and names the pattern:
  - One pattern per path component with tokens `{title}`, `{year}`, `{season}`, `{episodes}`, `{episode_title}`. A `[...]` group is dropped when a token inside has no value.
  - Defaults follow Jellyfin: `Title (Year)/Season 01/Title (Year) - S01E01 - Episode Title.ext` and `Title (Year)/Title (Year).ext`.
  - Every component is sanitised for Linux, macOS, Windows and SMB. File stems are capped at 200 bytes so subtitle suffixes always fit; folders at 255.
  - Subtitles take the video stem plus `.language[.sdh][.forced].ext`.
- **Planning** (`ImportPlanner`, on `DownloadCompleted`): the download's files are read relative to the download's parent folder, so the torrent's folder name counts as a title, and matched against the linked item, or the whole library when there is none or it was removed. One import per download (a unique index); a redelivered event changes nothing. The import is `Approved` when every row is `Certain`, conflict-free and takes no episode or movie that already has a file, `NeedsReview` otherwise, and `Failed` (with `ImportFailed`) when the download holds no video.
- **Execution** (`Importer`) places each row that is not skipped, with its subtitles, at the naming path in the item's `root/folder`. `[import] mode` is `hardlink` (default; keeps seeding), `copy` or `move`. It is idempotent per file: a destination holding the same data or the same size, or the moved file itself, counts as placed, so re-running after a crash is safe. Success commits the new files, `Done`, `FileDeleted { reason: Replaced }` for replaced files and `FilesImported` in one transaction; any failure stores the reason as `Failed` and emits `ImportFailed`; `retry` queues it again.
- **Replace** (FR-4.12): a review row of a download can replace the library file holding its target; the old file is deleted before the new one is placed. Keeping both is not offered, since an episode holds one file.
- **One at a time:** `claim_next_approved` moves the oldest `Approved` import to `Importing` in one statement, so the CLI and `serve` never run the same import, and the job runs one tick at a time. `serve` moves imports left `Importing` by a stopped process back to `Approved` when it starts.
- **Hard links** that fail across filesystems fall back to copy, with a warning; moves across filesystems copy and then delete.
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
- **Failures.** Retried with exponential backoff. After N attempts the failure is recorded in `failed_deliveries` and the subscriber moves on. Recorded failures are tried again later: every `retry_interval` (10 min) in `serve`, and at the start of every CLI catch-up; each further attempt updates the record, success removes it, and the position never moves back. Handlers are idempotent, so an event retried after newer ones is safe. The CLI gives up after 3 quick attempts, since the retry comes later anyway.
- **Wake-up.** `Database::commit` signals a `tokio::sync::watch` channel after each commit that wrote events. A signal sent while a subscriber is busy is not lost. A slow periodic poll is the fallback, and it also picks up events written by CLI commands running in another process.
- **Shutdown.** Delivery stops at the next await point. An event interrupted mid-handler is delivered again on the next run.
- **Rebuild.** A projection is rebuilt by deleting its row in `subscriber_positions`.
- **History (FR-9.1)** is a query over the event log (`events::History`): newest first via `EventLog::read_before`, filtered by `Event::items()` when one series or movie is asked for. There is no separate history table. Failed imports show their reason and can be retried (FR-9.2, `import list` / `import retry`).

### 7.2 Contract

`events` defines one `Event` enum, serialised with an explicit `type` tag. A stored event never changes meaning. A breaking change adds a new variant.

### 7.3 Catalog

| Event | Emitted by | Subscribers |
|---|---|---|
| `SeriesAdded { series, title }` | library | — (history) |
| `MovieAdded { movie, title }` | library | — (history) |
| `SeriesRemoved { series, title, delete_files }` | library | media |
| `MovieRemoved { movie, title, delete_files }` | library | media |
| `TorrentAdded { download, name, item }` | downloads | — (history) |
| `DownloadCompleted { download, name, content_path, item }` | downloads | media |
| `TorrentRemoved { download, name, item }` | downloads | — (history) |
| `ImportNeedsReview { import, source }` | media | — (history) |
| `FilesFound { files }` | media (scan) | library, media (probe), integrations |
| `FilesImported { import, download, files }` | media | library, media (probe), downloads, integrations |
| `ImportFailed { import, source, reason }` | media | — (history) |
| `FileRenamed { file, from, to, target }` | media | integrations |
| `FileDeleted { file, path, target, reason, recycled }` | media | library, integrations |

Events carry the titles and paths that history needs to display, so history still reads correctly after the item is removed. File events list each file as `LinkedFile { file, path, target }`, where `target` is a `FileTarget`.

`FileDeleted.reason` is `External`, `Replaced` (an import replaced the file), `User` or `ItemRemoved`; `recycled` says the file went to the recycle folder. Fields added to a variant later default when older events are read (`recycled: false`, `FileRenamed.target: None`, `FilesImported.download: None`), so stored events keep their meaning. `Event::items()` names the series and movies an event concerns, for history by item. A file removed outside the app is reported by the next scan with `External` (FR-8.7).

### 7.4 Registry

All subscriptions are declared in one file in the `yokoku` binary (`subscriptions.rs`), so every reaction in the system can be read in one place.

---

## 8. Jobs (`yokoku-jobs`)

apalis runs **work to do**: long-running, retryable jobs and schedules. It is not used to deliver events.

| Job | Trigger | Calls |
|---|---|---|
| `SyncDownloads` | cron, every 30 s, one tick at a time | `Downloads::sync` |
| `RefreshMetadata` | cron, every 6 h; only with a TMDB token | `MetadataSync::refresh_all` (one item's failure is logged and the rest continue) |
| `ExecuteImports` | cron, every 5 s, one tick at a time | `Importer::run_pending` |
| `ScanLibrary` | cron, daily at 05:00; on demand with `yokoku scan` | `Scanner::scan` (FR-8.7) |
| `RescanMediaServer` | cron, every 10 s; only with Jellyfin | `Rescans::run_due(30 s)` |

Job handlers are thin. They decode the job and call one use case. Schedules are cron expressions with seconds, set in `[serve]` (`sync_downloads = "*/30 * * * * *"`); `yokoku_jobs::monitor` registers the workers and `serve` runs them with `Monitor::run_with_signal`. Imports need no queue: approved rows in `imports` are the queue, and each run claims one import at a time. Modules that need to hand work to a job later get their own port, which `jobs` implements.

---

## 9. Cross-cutting

### Runtime

One binary.
- `yokoku serve` runs the event subscribers, the apalis `Monitor` and, later, the web server. All of them shut down gracefully on SIGINT/SIGTERM. Each subscriber gets its own `Delivery` loop; on a signal the monitor stops first, then the deliveries are cancelled and awaited.
- Other subcommands (`search`, `add`, `refresh`, `upcoming`, `missing`, `scan`, `review`, `rename`, `download`, `import`, `history`, `delete`, `files`, `jellyfin`, `settings`) call the same use cases against the same database. They let every feature be used and tested before the UI exists. A command that writes events delivers them to every subscriber (`Delivery::catch_up`) before it exits, so the CLI needs no background process.

### Storage

- `yokoku_db::Database` implements every repository port, so wiring passes one `Arc<Database>` for each.
- SQL strings are literals with bind parameters; sqlx 0.9 rejects dynamically built SQL.
- Queries are checked by tests against real SQLite, not by `query!` macros, so no `DATABASE_URL` or `.sqlx` data is needed to build.
- Ids are stored as UUID text, timestamps as RFC 3339 text, dates as ISO 8601 text, enums as lowercase text. Paths are UTF-8 text; the filesystem adapter skips names that are not UTF-8.
- Saving an aggregate upserts its rows and deletes rows no longer present, so child ids (episodes) stay stable.
- Series, movies and downloads carry a `revision` (optimistic concurrency). A save claims the next revision with `UPDATE … SET revision = revision + 1 WHERE id = ? AND revision = ?` in the same transaction; no match, because another save came first or the row was removed, fails with `StorageError::Conflict` and changes nothing. Library use cases reload and reapply their change on a conflict (up to five attempts); a download sync leaves a download another sync just saved to that sync, so `DownloadCompleted` stays exactly once.

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
| Approved imports in our table are the job queue, claimed atomically | apalis storage-backed `ExecuteImport` queue | Import state lives in one place, and the claim lets the CLI and `serve` run imports side by side. |
| No actor framework | kameo, ractor | Mailboxes are in memory (not durable), and it would be a third messaging model next to events and jobs. The one real race (concurrent imports) is solved by concurrency 1. |
| One lock file around library file changes | In-process mutex; locking rows in SQLite | A mutex does not reach the CLI in another process; a write transaction held while a file copies would block every other writer. |
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
7. **History, Jellyfin:** history query, `integrations`.
8. **Web UI:** `web` on Topcoat.
