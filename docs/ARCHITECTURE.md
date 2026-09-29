# Yokoku — Architecture

This document describes **how** Yokoku is built. For **what** it does, see [REQUIREMENTS.md](REQUIREMENTS.md).

---

## 1. Principles

1. **MVP limits features, not architecture.** Boundaries are right from the first commit.
2. **The core knows nothing about infrastructure.** Business logic never imports sqlx, reqwest, apalis or the web framework. It talks to the outside world through ports (traits) it owns.
3. **Modules are independent.** Feature modules never depend on each other. They share only value types and the event contract (`domain`) and event delivery (`events`). The compiler enforces this through crate boundaries.
4. **Consistency where it matters, decoupling everywhere else.** A command inside a module is synchronous and transactional. Reactions across modules happen through a durable event log, appended after the command's change is saved.
5. **Pure logic is isolated.** Filename detection and naming are pure functions with no IO and no async, tested with plain input/output tables.
6. **No speculative generality.** Every abstraction sits on a real boundary: IO, time, or a third-party library.

---

## 2. Technology

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust, edition 2024 | |
| Async runtime | `tokio` | |
| Web / UI | Dioxus 0.7 (fullstack) | Pre-1.0. Pages render on the server and hydrate in the browser (WASM); server functions call use cases. Components are Dioxus Components restyled to Paper (`docs/design-system`). Kept in one adapter crate, `web`, which the service serves (§9, Runtime). |
| Database | SQLite through `sqlx` (0.9) | Version set by `apalis-sqlite`. WAL mode, `foreign_keys=ON`, `busy_timeout`. |
| Migrations | `sqlx::migrate!` | One ordered set, owned by `db`. |
| Background jobs, cron | `apalis`, `apalis-sqlite`, `apalis-cron` | Jobs share the app's SQLite database. `apalis-workflow` is not used (see §10). |
| HTTP client | `reqwest` (rustls) | |
| Metadata | Own TMDB and TVDB v4 clients | Need only a handful of endpoints. |
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
  domain/            yokoku-domain            Shared value types and rules, event contract
  events/            yokoku-events            Handlers, subscriptions, delivery loop; re-exports the event contract
  detect/            yokoku-detect            Pure: downloaded files → ImportPlan
  naming/            yokoku-naming            Pure: NamingTemplate parse/render, sanitising

  library/           yokoku-library           Catalog, monitoring, metadata refresh, calendar queries
  downloads/         yokoku-downloads         Torrents, Transmission sync, seeding cleanup
  media/             yokoku-media             Import pipeline, review, scan, rename, delete
  integrations/      yokoku-integrations      Jellyfin rescan

  db/                yokoku-db                sqlx: migrations, all repository impls, event store
  metadata/          yokoku-metadata          MetadataProvider impls: TMDB, TVDB
  download-clients/  yokoku-download-clients  DownloadClient impls (Transmission)
  media-servers/     yokoku-media-servers     MediaServer impls (Jellyfin)
  jobs/              yokoku-jobs              apalis workers and cron; queue port impls
  system/            yokoku-system            FileSystem, LibraryLock, Clock, MediaProbe (ffprobe), EventSpool
  web/               yokoku-web               Web UI on Dioxus: pages, server functions, components, component gallery

  config/            yokoku-config            Configuration: composes every crate's settings section, layers and validates them

  yokoku/            yokoku (bin)             Composition root, service, subscription registry, CLI
```

### Dependency rules

```
              domain ◀──────────── detect, naming
                ▲                      ▲
   events ──────┤                      │
     ▲          │                      │
     └── library · downloads · media · integrations      (feature modules)
                          ▲
   db · metadata · download-clients · media-servers · jobs · system · web    (adapters)
                          ▲
                        config
                          ▲
                       yokoku (bin)
```

| Crate kind | May depend on | Must not depend on |
|---|---|---|
| `domain` | std, `serde`, `serde_json`, `jiff`, `thiserror`, `async-trait`, `secrecy` | anything else in the workspace |
| `detect`, `naming` | `domain` | IO, async, any module |
| `events` | `domain` | any module or adapter |
| Feature module | `domain`, `events`, pure crates it needs | other modules, any adapter, sqlx/reqwest/apalis/dioxus |
| Adapter | modules whose ports it implements | other adapters |
| `web` (driving adapter) | modules whose use cases it calls | other adapters, sqlx/reqwest/apalis |
| `config` | modules and adapters whose settings it composes | `db`, `web` |
| `yokoku` | everything | — |

`web` is the one adapter that drives the core instead of serving it: it calls use cases the way the CLI does, and receives them from `yokoku`, which wires the concrete adapters.

When a module needs another module's data, it declares its own narrow **read port**, and `db` implements it. Example: `media` needs a series' episode list for detection, so it declares `media::ports::Catalog`.

### 3.1 Where things live

A feature's code spans crates by design (§3): its use case in a feature module, its storage in
`db`, its outside services in an adapter crate, its wiring in `yokoku`, its screens in `web`. Four
naming rules make each hop predictable:

- **Feature modules:** one file per use case, named after it: `media/src/scan.rs` holds `Scanner`,
  `library/src/calendar.rs` holds `Calendar`. Event handlers sit next to the use case they call.
  Entities are in `model.rs`, ports in `ports.rs`.
- **`db`:** one file per port, named after the trait in snake case: `series_repo.rs` implements
  `SeriesRepo`, `catalog.rs` implements `media::ports::Catalog`, `settings_store.rs` implements
  `SettingsStore`. `media_info.rs` is the probe-details half of `media_repo.rs`.
- **Adapters for outside services:** one crate per port and one module per product, named after
  the product: `download-clients/src/transmission.rs`, `media-servers/src/jellyfin.rs`,
  `metadata/src/tmdb.rs` and `tvdb.rs`. Response shapes live in `<product>_wire.rs`.
  `system` holds only local-host adapters: filesystem, library lock, clock, ffprobe, event spool.
- **`web`:** one folder per role a file plays in the UI:

  ```
  web/src/
    route.rs      Route enum and App; the only file that names every page
    layout/       the shell around every page: Shell, DocumentHead, BackButton
    pages/        one folder per route: library/, series_detail/, upcoming/, downloads/, history/
    dialogs/      modal flows opened from several pages: import_review.rs, rename.rs
    api/          server functions, one file per feature module: library/, add.rs, downloads.rs, history.rs, media.rs
    components/   Paper components (vendored with `just web add`) and Yokoku widgets, flat
    format.rs     values as every page writes them: year, date
    server.rs     Server: binds and serves the router; server build only
    state.rs      AppState and the Dep extractor for server functions; server build only
    main.rs       browser entry
  ```

  A page is a folder named after its `Route` variant in snake case (`Route::SeriesDetail` is
  `pages/series_detail/`), even when it has one file:

  ```
  pages/library/
    mod.rs        the routed component: its state, the api call, loading, error and empty states
    filters.rs    parts only this page renders, one file each, named after what they render,
    grid.rs         `pub(super)`
    table.rs
    fields.rs     how the page's values render, shared by its parts
  ```

  A part a second page needs moves to `components/`, with a gallery story, and a formatting
  helper a second page needs moves to `format.rs`. A file in `dialogs/` or `api/` becomes a
  folder when it grows, and there are no empty `hooks/` or `utils/` folders.

  How web code is written:

  - **Pages** are the only callers of `api` (with dialogs). Server data lives in the page that
    loads it, with no global store.
  - **State**, by what it is:
    - Server data: `use_server_future`, reading the page's state signals so a change reloads it.
      It renders on the server and hydrates without a second request.
    - Page or component state: `use_signal`, one per concern, not per field. Values that change
      together are one plain struct, `Filters { kind, status, sort }`, whose rules (a type change
      clears a status of the other type) are its methods. View-only state, like grid or table,
      is its own signal.
    - Values computed from signals: `use_memo`, not a signal kept in sync by hand.
    - Large nested state edited per item, like the rows of an import review: a store
      (`#[derive(Store)]`, `use_store`), so a row re-renders alone.
    - State a subtree shares: context (`use_context_provider` / `use_context`), as the sidebar
      does; not for server data.
    - A child that edits page state takes the signal as a prop (`FilterBar { filters }`); a
      child that only shows it takes the value.
  - **Page states** follow the design system (§6 of DESIGN-SYSTEM.md): loading (`Skeleton`),
    failed (`Alert` with a generic message), empty (what to do next), and empty because of
    filters (a Clear filters action) are separate. Each page names itself with
    `document::Title { "Library · Yokoku" }`; the shell's `Yokoku` is the fallback.
  - **Components** take props and send events, never call `api`, so the gallery shows each one
    with fixed data. A server-rendered `Select` with a value passes that value's text as
    `placeholder`, or the page shows "Select…" until the options register in the browser.
    Sections that open and close are `Disclosure`, a native `details` that renders on the
    server; the vendored `Accordion` mounts its content only in the browser.
  - **Navigation:** a main destination is one `NavItem` in `layout/` `Shell`, a router link in
    the sidebar, which is a sheet below `md`. Every other page belongs to one
    (`Route::section`), which stays highlighted, and starts with `BackButton`: back after an
    in-app link, or to its section when opened directly.
  - **Server functions:** a file in `api/` holds, compiled for both builds, its wire types and
    its server function signatures, and in one `#[cfg(feature = "server")] mod server` everything
    that needs the server build: the use-case call, error mapping, and the conversions between
    wire and domain types. A server function names the use cases it calls as extractors,
    `#[get("/api/library?kind&status&sort", library: Dep<Library>)]`, and its body is one call
    into `mod server`.
  - **Wire types** are the api file's own: enums with `#[serde(rename_all = "kebab-case")]`, a
    `label()` for display and an `ALL` list for selects. A domain type crosses the wire only if
    it already derives serde (`ItemId`); the browser build never depends on a feature module.
  - **Errors:** `mod server` logs the use case's error and returns a generic `ServerFnError`
    message; the page shows a generic alert. Internals never reach the browser.
  - **Dependencies:** `Dep<T>` takes one use case from `AppState`, and one that `AppState` does
    not provide fails to compile. A new use case is an `AppState` field (`state.rs`), a
    `Provides<T>` impl for it, and its type in the `#[cfg(feature = "server")] use` of
    `api/mod.rs`; api files take it with `#[cfg(feature = "server")] use super::{Dep, …}`.
    `yokoku` fills `AppState` in `service.rs`.
  - **Tests:** `yokoku/tests/service.rs` starts the binary on a seeded database and fetches pages
    and api routes over HTTP; wire conversions and page rules are covered through it.

Tests mirror sources: `crates/<crate>/tests/<module>.rs` tests `crates/<crate>/src/<module>.rs`.
CLI commands are `yokoku/src/cli/commands/<verb>.rs` for `yokoku <verb>`; their tests are grouped
by module in `yokoku/tests/<module>_commands.rs`.

| Feature | Use case (entry point) | Rules / pure logic | Storage (`db/src`) | Outside world | Driven from |
|---|---|---|---|---|---|
| Search and add (FR-1.1, 2.2) | `library/src/metadata.rs` `MetadataService::search` (with each hit's default folder), `add_series`, `add_movie`; existing and taken folders from `media/src/roots.rs` `RootFolders::folders`, `item_folders` | `domain/src/series.rs` `Series::add`, `movie.rs`; folder name `naming/src/naming.rs` via `yokoku/src/app.rs` `NamedFolders` | `series_repo.rs`, `movie_repo.rs` | `metadata/src/sources.rs`, `tmdb.rs`, `tvdb.rs` | `cli/commands/search.rs`, `add.rs`; web `api/add.rs`, `pages/add/` (the search page at `/add`, with the options in a dialog) |
| List, detail, monitoring, numbering, remove (FR-1, FR-2) | `library/src/library.rs` `Library` | `domain/src/series.rs` (monitoring, numbering), `library/src/listing.rs` | `series_repo.rs`, `movie_repo.rs` | none | `list.rs`, `show.rs`, `monitor.rs`, `numbering.rs`, `remove.rs`; web `api/library/` (changes in `manage.rs`), `pages/library/`, `series_detail/` (numbering in `numbering.rs`), `movie_detail/`, `components/monitor_toggle.rs` |
| Metadata refresh (FR-1.6) | `library/src/metadata.rs` `refresh_*` | `Series::refresh`, `needs_refresh` in `domain/src/series.rs`; `movie.rs` | as above | `metadata` | job `refresh-metadata` (`jobs/src/lib.rs`); `refresh.rs`; web `api/library/manage.rs`, `components/refresh_button.rs` on detail pages |
| Next / last aired (FR-6.1, 6.2) | `library/src/listing.rs` | `domain/src/series.rs` `next_episode`, `last_aired` | as above | none | `list.rs`, `show.rs`; web `pages/series_detail/` |
| Calendar and missing (FR-6.3, 6.4, FR-7) | `library/src/calendar.rs` `Calendar::entries`, `missing` | `domain/src/series.rs`, `movie.rs` | as above | none | `calendar.rs`, `missing.rs`; web `api/library/calendar.rs`, `pages/upcoming/`, `missing/` |
| File projection on items | `library/src/files.rs` `FileTracker` (`library.files`) | none | `media_files.rs` | none | `yokoku/src/subscriptions.rs` |
| Artwork: poster, backdrop, logo (FR-1.2) | `library/src/artwork.rs` `Artworks` (`library.artwork`) | `domain/src/artwork.rs` `Artwork`; choice in `metadata/src/tmdb_wire.rs`, `tvdb_wire.rs` | `series_repo.rs`, `movie_repo.rs` (`artwork` JSON column) | `metadata/src/artwork.rs` `ArtworkFetcher`; `system/src/artwork.rs` `ArtworkFiles` | `web/src/api/artwork.rs`, which also serves search result posters uncached (`Artworks::preview`) |
| Description: overview, genres, runtime; episode overviews | `library/src/metadata.rs` (with refresh) | `domain/src/description.rs` `Description`; `Series::refresh`, `Movie::refresh` | `series_repo.rs`, `movie_repo.rs` (`description` JSON column, episode `overview`) | `metadata/src/tmdb.rs`, `tvdb.rs` | `web/src/api/library/detail.rs` |
| Downloads: add, sync, pick up, seeding cleanup (FR-3) | `downloads/src/downloads.rs` `Downloads` | `downloads/src/model.rs` | `download_repo.rs` | `download-clients/src/transmission.rs` | jobs `sync-downloads`, `sync-active-downloads`; `download.rs`; web `api/downloads.rs` (live over server-sent events, woken by `events::QueueChanges`, which `Downloads`, `ImportPlanner`, `Scanner`, `Reviewer` and `Importer` notify after saving downloads or imports), `pages/downloads/` |
| Detection (FR-4.1–4.10, 4.13) | `detect` `ImportPlan::new` (`plan.rs`) | `classify.rs`, `parse.rs`, `titles.rs` (title and year), `plan.rs` (episodes) | none | none | `media/src/planner.rs`, `scan.rs` |
| Import: plan, review, execute, retry (FR-3.5, 3.6, 4.11, 4.12, 9.2) | `media/src/planner.rs` `ImportPlanner` → `review.rs` `Reviewer` → `importer.rs` `Importer` | `detect`, `naming` | `media_repo.rs` | `system/src/fs.rs` | job `execute-imports`; `review.rs`, `import.rs`; web: status and retry on torrent rows (`api/downloads.rs`, `pages/downloads/`), scan imports on detail pages |
| Episode spans (`S01E01-E03`) | none | `domain/src/episode_span.rs` | none | none | none |
| Naming (FR-5.1–5.6) | none | `naming/src/naming.rs`, `template.rs`, `sanitize.rs`, `subtitle.rs` | none | none | `media` |
| Rename with preview (FR-5.7) | `media/src/rename.rs` `Renamer` | `naming` | `media_repo.rs` | `system/src/fs.rs` | `rename.rs` |
| Root folders (FR-8.1) | `media/src/roots.rs` `RootFolders` | `media/src/model.rs` `RootFolder` | `media_repo.rs` | `system/src/fs.rs` | `root.rs` |
| Scan (FR-8.2, 8.3, 8.7, 8.8) | `media/src/scan.rs` `Scanner` | `detect` | `media_repo.rs`, `catalog.rs` | `system/src/fs.rs` | job `scan-library`; `media.scan_added`; `scan.rs` |
| Retarget files on renumber | `media/src/scan/renumber.rs` (`media.renumbered`) | `domain/src/series.rs` `Series::refresh` | `media_repo.rs` | none | `subscriptions.rs` |
| Delete files (FR-8.4, 8.5, FR-1.7) | `media/src/deleter.rs` `Deleter` | none | `media_repo.rs` | `system/src/fs.rs` | `delete.rs`, `remove.rs` |
| File details (FR-8.6) | `media/src/prober.rs` `Prober` (`media.probe`) | `media/src/model.rs` `MediaInfo` | `media_info.rs` | `system/src/probe.rs` | `files.rs`; web `api/library/detail.rs` |
| Library lock | `media/src/ports.rs` `LibraryLock` | none | none | `system/src/lock.rs` | every media use case |
| Jellyfin rescan (FR-10.4) | `integrations/src/rescans.rs` `Rescans` | none | `rescan_store.rs` | `media-servers/src/jellyfin.rs` | job `rescan-media-server`; `jellyfin.rs` |
| History (FR-9.1) | `events/src/history.rs` `History` | text: `domain/src/events.rs` `Display` for the CLI; the web words events with item links in `web/src/api/history.rs` | `event_log.rs` | none | `history.rs`; web `api/history.rs`, `pages/history/`, `components/history_list.rs` (also on detail pages) |
| Event contract, delivery | `domain/src/events.rs`; `events/src/publisher.rs`, `delivery.rs`, `event_log.rs` | none | `event_log.rs` | `system/src/spool.rs` | `yokoku/src/subscriptions.rs`, `app.rs` |
| Settings (FR-10.3) | `config/src/settings.rs` `Settings`; each crate's `*Settings` next to its code (§5.5) | `config/src/lib.rs` (layering) | `settings_store.rs` | none | `settings.rs` |
| Jobs and schedules | `jobs/src/lib.rs` | none | none | none | `yokoku/src/service.rs` |
| Attribution (FR-10.5) | none | none | none | none | `cli/args.rs` `DATA_SOURCES` |

---

## 4. Domain (`yokoku-domain`)

Value types and rules shared by all modules. Examples:

- Identifiers: `SeriesId`, `MovieId`, `EpisodeId`, and later `DownloadId`, `ImportId`, `MediaFileId`. UUIDv7 newtypes created by the domain, so an aggregate and its events are complete before they are saved. Users refer to items by source id (`tmdb:1396`).
- `ExternalId { Tmdb(u64), Tvdb(u64) }`. An item stays bound to the provider it was added with: `metadata::Sources` looks it up there. Movies come from TMDB; series from TMDB, or from TVDB once a TVDB API key is set, which then also answers series searches.
- `Series` → `Season` → `Episode` and `Movie`: aggregates with public fields. Seasons and episodes are kept ordered by number. Each carries its root folder path (`root`) and its folder name in that root (`folder`), both set when it is added and never changed.
- `RootFolder { kind, path }` and `RootKind { Series, Movies }` (FR-8.1) are in `media` (`media/src/model.rs`), not here: `library` checks an item's root against its kind; `media` stores the root folders.
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
| `Series::refresh(metadata, now)` | Matches episodes by source id, so renumbered episodes keep id, flags and file. New seasons follow the series flag (specials excepted); new episodes follow their season. Episodes gone from the source are dropped. Returns the files whose episodes changed numbers, with their new span; a file whose episodes no longer form one span is unlinked from them. |
| `Series::needs_refresh(now, today)` | Sonarr's rules: refreshed over 30 days ago, or an aired regular episode titled `TBA` or untitled; otherwise not within 6 h of the last refresh, and not ended or with an episode airing in the last 30 days or later. |
| `Movie::needs_refresh(now, today)` | Radarr's rules: refreshed over 180 days ago; otherwise not within 12 h of the last refresh, and not `Released` or with a physical release in the last 30 days or later. TMDB allows keeping its data 6 months at most. |
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
  error.rs
```

A use case that reacts to events from other modules implements `Handler<E>` for each event type, next to its code.

### 5.1 `library`

Owns movies, series, seasons, episodes, monitoring flags, and a projection of the media file holding each episode and movie (`file: Option<MediaFileId>`). Tracking the id rather than a flag, and linking a file only where media holds it when the event is handled, keeps the projection correct whatever order file events arrive in.

- **Use cases**, split by what they depend on:
  - `Library` (repositories + clock): list with filter and sort, series and movie details, set monitoring, set numbering, remove.
  - `MetadataService` (+ metadata source): search, add (applying a monitor preset, into a root folder of the item's kind, which the caller takes from `media`), refresh one item or all.
  - `Calendar` (repositories + clock): calendar for a date range, missing grouped by series. Only monitored items appear (FR-2.3).
  - `Artworks` (repositories + artwork source and cache): an item's poster, backdrop or logo. Metadata gives each item an `Artwork` (`domain`) of paths at its source; the first request for a kind downloads the image and stores it, later ones read it from the cache. The image's file name keys the cache, so a refresh that changes an image downloads the new one, which replaces the old.
- **Later:** iCal feed (served by `web`).
- **Ports:** `SeriesRepo`, `MovieRepo` (whole aggregates), `MediaFiles` (read-only: where media holds a file now), `Publisher` (events, appended after the save), `MetadataProvider`, `ArtworkSource` (`ArtworkFetcher` in `metadata`), `ArtworkCache` (`ArtworkFiles` in `system`), `Clock`. The list is built from the aggregates; a dedicated query port comes only if the library grows large enough to need one.
- **Emits:** `SeriesAdded`, `MovieAdded`, `SeriesRemoved`, `MovieRemoved`, `EpisodesRenumbered` (a refresh renumbered episodes holding files).
- **Subscribes to:** `FilesFound`, `FilesImported`, `FileDeleted` (`FileTracker` updates the file projection). `FileRenamed` keeps the file id, so the projection needs no change. `SeriesRemoved`, `MovieRemoved` (`library.artwork`: `Artworks` drops the item's cached images).

Artwork (FR-1.2): TMDB gives a poster and backdrop path per item and a logo from its `images`, the best-voted one in the metadata language, else one without text. TVDB gives the series image as poster and, from `series/{id}/artworks`, the best-scored background without text (else in the language) and clear logo in the language (else without text). `ArtworkFetcher` downloads from `image.tmdb.org` (posters 342, backdrops 1280, logos 500 pixels wide) and from URLs under `https://artworks.thetvdb.com/`, and refuses any other address. `ArtworkFiles` keeps one folder per item next to the database, `artwork/series/<id>/poster-81189-10.jpg`; `web` serves them at `/artwork/{series|movie}/{id}/{kind}/{name}` with an immutable cache header, since the name changes with the image.

`yokoku-metadata` sends at most 40 requests a second to each source, gives up on a request after 30 s (5 s to connect), and tries a request up to three times on 429, 502, 503, 504, a timeout or a failed connection, waiting as `Retry-After` says (at most 30 s) or 1 s, then 2 s. A 404 for an item is `NotFound`; 401 and 403 are `Refused` and not retried; an answer of another shape is `Invalid`. The TVDB client logs in once for all callers and logs in again once when its token is refused.

### 5.2 `downloads`

Owns the downloads Yokoku knows about and the Transmission connection settings.

- **Use cases:**
  - Test the connection.
  - Add a torrent (magnet link or .torrent file), linked to a movie or series or left unlinked.
  - Sync with the client: progress, state, completion, and optionally torrents picked up by label or folder.
  - Pick up torrents added directly in the client (FR-3.3, `[downloads] pick_up_labels`, `pick_up_folder`; off unless set): the sync lists every torrent and takes on each one it has never stored that carries one of the labels or downloads at or under the folder, as an unlinked download with `TorrentAdded`, so detection works out what it is. A download that was removed is never taken on again; two syncs taking on the same torrent conflict on its hash and one of them skips it. Torrents labelled `yokoku` (`ports::LABEL`, which `DownloadClient::add` puts on every torrent Yokoku adds) are taken on the same way even when pick up is off, so a torrent added whose save then failed is not left behind; it comes back unlinked from its series or movie.
  - Remove a torrent once seeding is finished (FR-3.7, `[downloads] remove_after_seeding`, off by default): a sync removes a download with its data from the client when it was imported (`imported_at`, set by the `FilesImported` that names it) and the client reports seeding finished (Transmission `isFinished`: the ratio or idle limit was reached), then marks it `Removed` and emits `TorrentRemoved`. The library keeps its own hard link or copy; a download never imported, or still in review, stays.
- **Ports:** `DownloadRepo`, `DownloadClient` (version, add, torrents by info hash, all torrents, remove), `Clock`, `Publisher` (events, appended after the save).
- **Sync:** every download not yet `Removed` takes the client's status (state, bytes done, rate, ETA, folder). The first sync that sees it complete sets `completed_at` and, once that is saved, publishes `DownloadCompleted`; a torrent missing from the client becomes `Removed` and is no longer synced. Adding a torrent syncs it at once, so a torrent that is already complete emits both events.
- **Emits:** `TorrentAdded`, `DownloadCompleted` (once per download; the sync is idempotent), `TorrentRemoved`.
- **Subscribes to:** `FilesImported` (marks the download imported so it can be removed after seeding).

Transmission runs on the same host as Yokoku. The paths it reports are used as-is; no path mapping.

`yokoku-download-clients` (module `transmission`) speaks Transmission's RPC: it repeats a call once with the session id a 409 answer carries, sends basic auth when configured, labels added torrents `yokoku`, and gives up on a request after 30 s (5 s to connect). A torrent counts as complete when its metadata is known, its selected size is above zero, nothing is left and it is not being checked. Tests replay recorded answers with wiremock; an ignored live test starts `transmission-daemon`, adds a torrent made from local data and syncs it to completion.

### 5.3 `media`

Owns library files, root folders, naming settings and imports.

- **Use cases:**
  - Plan an import from a download or from files found by a scan.
  - Review: match a row to episodes or a movie in the library, or skip it; approve once every other row has a conflict-free match. Skipped files are not offered again by later scans.
  - Run an approved import.
  - Root folders: add (absolute, existing, not overlapping another root), list, remove (refused while items belong to it).
  - Scan item folders (FR-8.2, 8.3, 8.7): the files under each item's `root/folder` are detected against that item alone (`MatchScope::Series` / `MatchScope::Movie`); nothing else in a root is read. New files that are `Certain`, conflict-free and hold nothing already linked are linked in place; the rest of the folder becomes one import in review. Linked files missing from disk are forgotten with `FileDeleted { reason: External }`. A root that cannot be read fails the scan, so an unmounted disk never looks empty.
  - Rename with preview (FR-5.7): `Renamer::preview(scope)` lists the moves naming asks for, for the whole library, a series or a movie; `apply` makes them file by file, inside the item's `root/folder`. Subtitles beside a video (named after it) move with it and get normalised language tags. Files outside every root, whose item is gone, or that would share a path are skipped; a file already at the new path is never replaced, and that move is reported as failed. Folders left empty are removed up to the root; one that cannot be removed is only logged.
  - Delete files (FR-8.4, 8.5): `Deleter::delete(target)` removes the files holding an episode span or movie, committing each file with `FileDeleted { reason: User }` once its video is gone, then its subtitles and folders left empty; a failure in that cleanup is only logged. Removing a series or movie with `delete_files` does the same for all its files (`ItemRemoved`).
  - Scan an added item (FR-8.8): the `media.scan_added` subscriber scans the new item's folder as above.
  - Retry a failed import.
  - File details (FR-8.6): the `media.probe` subscriber probes the files of `FilesFound` and `FilesImported` with `ffprobe` (`[files] ffprobe`) and stores duration, the video stream (codec, size; cover art is skipped) and each audio (language, codec, channels) and subtitle stream (language, forced) per file; details go with the file when it is removed and stay through renames. A file that cannot be probed, or a missing `ffprobe`, is only logged; `Prober::probe_missing` (`yokoku files probe`) reads every file never probed. `Prober::details(item)` adds the subtitle files beside each video.
- **Library lock:** scan, import, rename and delete change files on disk before they commit, so each holds the `LibraryLock` from its first read of library files until its last commit; a scan never sees a file that is placed but not yet stored. `LockFile` in `system` takes an exclusive `flock` on `<database>.lock`, so the CLI and `serve` wait for each other as well. An import holds it per import, from its claim to its commit.
- **Ports:** `MediaRepo` (root folders, files, imports; one `save(changes)` so a use case commits its state in one transaction), `Publisher` (events, appended after the save), `Catalog` (read-only view of `library` data), `FileSystem`, `LibraryLock`, `MediaProbe` (`FfProbe` in `system`), `Clock`; later `ImportQueue`.
- **Emits:** `FilesFound`, `ImportNeedsReview`, `FilesImported`, `FileDeleted`, `FileRenamed`; later `ImportFailed`.
- **Subscribes to:** `DownloadCompleted` (plans an import), `SeriesAdded`, `MovieAdded` (scan the item's folder), `SeriesRemoved`, `MovieRemoved` (delete the item's files when asked, otherwise drop their records and keep them on disk), `EpisodesRenumbered` (moves each file's target to its new span; a file without one leaves the library and goes to review).

### 5.4 `integrations`

- Rescans Jellyfin after `FilesImported`, `FileRenamed` and `FileDeleted` (FR-10.4). The `Rescans` subscriber only records that a rescan is due (the latest request time, one row); `run_due(quiet)` rescans once no request arrived for the quiet period and clears the request only if it was not renewed meanwhile, so a burst leads to one rescan, a request made during a rescan is kept, and a failed rescan stays pending. `serve` checks every 10 s with a 30 s quiet period; the CLI rescans right after delivering events and only warns when Jellyfin cannot be reached. Off unless `[jellyfin] url` is set; `api_key` is a secret.
- **Ports:** `MediaServer` (`JellyfinClient` in `media-servers`: `POST /Library/Refresh`, `GET /System/Info`, `Authorization: MediaBrowser Token`, 30 s per request, 5 s to connect), `RescanStore`.
- Future notifications (REQUIREMENTS §6) go here.

### 5.5 Settings

Each crate owns the settings its code reads, as a serde type next to that code: `MetadataSettings` in `metadata`, `TransmissionSettings` in `download-clients`, `DownloadOptions` (`[downloads]`) in `downloads`, `ImportSettings` in `media`, `Naming` (parsed from `[naming]`, so a bad pattern fails when the configuration loads) in `naming`, `JellyfinSettings` in `media-servers`, `ClockSettings` and `ProbeSettings` (`[files]`) in `system`, `ScheduleSettings` (`[serve]`) in `jobs`. `yokoku-config` composes them into `Config`, next to the sections only the binary reads (`database`, `log`, `web`, and the CLI defaults `add`, `list`, `calendar`). Stored settings are reached through the `SettingsStore` port in `domain`, which `db` implements, so `config` does not depend on `db`.

No use case or adapter keeps a copy of its settings. Each takes a `Live<T>` (`domain`), which `Settings::live` projects from the configuration in effect and which it reads each time it is used: `Importer` reads the naming patterns and import mode per file, the TMDB, TVDB, Transmission and Jellyfin clients read their URL and credentials per request (TVDB logs in again once the API key or PIN changes), `SystemClock` reads the time zone. A missing TMDB token, TVDB API key or Jellyfin URL is checked when a call needs it, so every service and job is wired even while it is not configured: a rescan stays pending until Jellyfin is set up. `database`, `log`, `serve` and `web` are read once at start.

Settings are layered, later over earlier: defaults, the TOML file, values stored in the database (FR-10.3), then `APP__*` environment variables. Stored values live in `settings (key, value)` by dotted key (`import.mode`) as JSON; `yokoku settings set|unset|list|get` edits them, and the settings screen will too. `set` loads the whole configuration with the new value and validates it (types, naming patterns, schedules, time zone) before storing, so a stored value cannot stop the app. `set` and `unset` publish `SettingsChanged { key }` (without the value, which may be a secret); the `config.settings` subscription reloads `Settings` in every process that delivers events, so a running `serve` applies a change made from the CLI within a delivery poll (5 s), except the settings read once at start. A reload that fails keeps the settings in effect. A stored value that no longer loads fails every command except `settings`, which can unset it.

Not stored: bootstrap values needed before the database opens (`database`, `log`). Secrets (TMDB token, TVDB API key and PIN, Transmission password, Jellyfin API key) are `Secret` fields: any layer gives them as a value or as `{ file = "..." }` (`APP__…__FILE`), read when the configuration loads. A `Secret` serializes as its value, so saved settings load again, and is shown only masked (`Secret::masked`: the first and last four characters, `eyJh…NiJ9`, or for eight or fewer `…` and the last quarter); `Config::setting` serializes within `Secret::masking`, so every `Secret` it prints is masked without a list of secret keys, and `yokoku settings` prints through it; `Debug` shows none of it.

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
- **Execution** (`Importer`) places each row that is not skipped, with its subtitles, at the naming path in the item's `root/folder`. `[import] mode` is `hardlink` (default; keeps seeding), `copy` or `move`. It is idempotent per file: a destination holding the same data or the same size, or the moved file itself, counts as placed, so re-running after a crash is safe. The import commits the files it placed and replaced so far together with its outcome in one transaction: `Done`, or on any failure `Failed` with the reason; it then publishes `FileDeleted { reason: Replaced }`, `FilesImported` and, on failure, `ImportFailed`; `retry` queues it again and skips rows already in the library.
- **Replace** (FR-4.12): a review row of a download can replace the library file holding its target; the old file is deleted before the new one is placed. Keeping both is not offered, since an episode holds one file.
- **One at a time:** `claim_next_approved` moves the oldest `Approved` import to `Importing` in one statement, so the CLI and the service never run the same import, and the job runs one tick at a time. Imports run only under the library lock, so each runner, once it holds the lock, moves imports still `Importing` (left by a stopped process) back to `Approved` before it claims.
- **Hard links** that fail across filesystems fall back to copy, with a warning; moves across filesystems copy and then delete.
- **Unlinked torrents** are matched only against items already in the library. Anything unmatched goes to review.

---

## 7. Events

### 7.1 Model

The event log records **what happened**. It is not event sourcing: state tables remain the source of truth, and events are written next to them.

```
events               (id INTEGER PRIMARY KEY AUTOINCREMENT, payload JSON, occurred_at, correlation)
subscriber_positions (subscriber TEXT PRIMARY KEY, last_event_id)
failed_deliveries    (subscriber, event_id, error, attempts, failed_at)
```

The payload carries the event's `type` tag, so no separate kind column is needed; `json_extract(payload, '$.type')` filters by type.

- **Append after save.** A use case saves its state, then publishes the command's events through `Publisher`, which appends them in a transaction of their own. A failed append never fails the command: the events go to a spool file next to the database (`yokoku.spool`, JSON lines under an `flock`) and are appended, before any newer ones, by the next publish, every CLI catch-up and once a minute in `serve`; a crash between replaying and emptying the spool appends them twice, which idempotent handlers accept. Events neither the log nor the spool takes are logged and lost, and so are events of a crash between the save and the append.
- **Ordered delivery.** Each subscriber runs as one task that reads events after its saved position, in id order, and advances its position after each success.
- **Order is safe.** SQLite allows one writer at a time, so ids are always committed in id order. A reader can never skip an event whose transaction commits late.
- **At-least-once.** Handlers are idempotent.
- **Failures.** Retried with exponential backoff. After N attempts the failure is recorded in `failed_deliveries` and the subscriber moves on. Recorded failures are tried again later: every `retry_interval` (10 min) in `serve`, and at the start of every CLI catch-up; each further attempt updates the record, success removes it, and the position never moves back. Handlers are idempotent, so an event retried after newer ones is safe. The CLI gives up after 3 quick attempts, since the retry comes later anyway.
- **Wake-up.** `EventLog::append` signals a `tokio::sync::watch` channel after each append. A signal sent while a subscriber is busy is not lost. A slow periodic poll is the fallback, and it also picks up events written by CLI commands running in another process.
- **Shutdown.** Delivery stops at the next await point. An event interrupted mid-handler is delivered again on the next run.
- **Correlation.** Every CLI command and job tick runs under a new correlation id (a task-local, also a field of its `command` or `job` span). `Publisher` stores it with each event, in the log and in spooled lines, and a delivery restores it in its `deliver` span, so one id follows a command through its events, their handlers and the events those publish. Events stored before the column have none and get a new id per delivery.
- **Rebuild.** A projection is rebuilt by deleting its row in `subscriber_positions`.
- **History (FR-9.1)** is a query over the event log (`events::History`): newest first via `EventLog::read_before`, filtered by `Event::items()` when one series or movie is asked for. There is no separate history table. Failed imports show their reason and can be retried (FR-9.2, `import list` / `import retry`).

### 7.2 Contract

`domain::events` defines one struct per event and an `Event` enum wrapping them, serialised with an explicit `type` tag; `events` re-exports them. Events are built with `into()` and read with `Event::get::<E>()`. A stored event never changes meaning. A breaking change adds a new variant.

### 7.3 Catalog

| Event | Emitted by | Subscribers |
|---|---|---|
| `SeriesAdded { series, title }` | library | — (history) |
| `MovieAdded { movie, title }` | library | — (history) |
| `SeriesRemoved { series, title, delete_files }` | library | media |
| `MovieRemoved { movie, title, delete_files }` | library | media |
| `EpisodesRenumbered { series, files }` | library | media |
| `TorrentAdded { download, name, item }` | downloads | — (history) |
| `DownloadCompleted { download, name, content_path, item }` | downloads | media |
| `TorrentRemoved { download, name, item }` | downloads | — (history) |
| `ImportNeedsReview { import, source }` | media | — (history) |
| `FilesFound { files }` | media (scan) | library, media (probe), integrations |
| `FilesImported { import, download, files }` | media | library, media (probe), downloads, integrations |
| `ImportFailed { import, source, reason }` | media | — (history) |
| `FileRenamed { file, from, to, target }` | media | integrations |
| `FileDeleted { file, path, target, reason }` | media | library, integrations |
| `SettingsChanged { key }` | `yokoku settings` | config (reloads `Settings`) |

Events carry the titles and paths that history needs to display, so history still reads correctly after the item is removed. File events list each file as `LinkedFile { file, path, target }`, where `target` is a `FileTarget`.

`FileDeleted.reason` is `External`, `Replaced` (an import replaced the file), `User` or `ItemRemoved`. Fields added to a variant later default when older events are read (`FileRenamed.target: None`, `FilesImported.download: None`), and fields dropped from one are ignored, so stored events keep their meaning. `Event::items()` names the series and movies an event concerns, for history by item. A file removed outside the app is reported by the next scan with `External` (FR-8.7).

### 7.4 Registry

All subscriptions are declared in one file in the `yokoku` binary (`subscriptions.rs`), so every reaction in the system can be read in one place. A module reacts to an event by implementing `Handler<E>` for that event type. A `Subscription` names a subscriber and lists its handlers, e.g. `Subscription::new("library.files").on::<FilesFound>(tracker.clone()).on::<FileDeleted>(tracker)`; each event goes to the handlers of its type in the order they were added, and a subscription is delivered as one subscriber with one position. The name keys the stored position, so renaming a subscription delivers the whole log to it again.

---

## 8. Jobs (`yokoku-jobs`)

apalis runs **work to do**: long-running, retryable jobs and schedules. It is not used to deliver events.

| Job | Trigger | Calls |
|---|---|---|
| `SyncDownloads` | cron, every 30 s, one tick at a time | `Downloads::sync` |
| `SyncActiveDownloads` | cron, every 5 s, one tick at a time | `Downloads::sync_active`: syncs only while a download is queued, checking or downloading |
| `ExecuteImports` | cron, every 5 s, one tick at a time | `Importer::run_pending` |
| `RefreshMetadata` | cron, every 12 h; only with a TMDB token; everything on demand with `yokoku refresh` | `MetadataService::refresh_due` (one item's failure is logged and the rest continue) |
| `ScanLibrary` | cron, daily at 05:00; on demand with `yokoku scan` | `Scanner::scan` (FR-8.7) |
| `RescanMediaServer` | cron, every 10 s; only with Jellyfin | `Rescans::run_due(30 s)` |

Job handlers are thin. They decode the job and call one use case. Schedules are cron expressions with seconds, set in `[serve]` (`sync_downloads = "*/30 * * * * *"`); `yokoku_jobs::monitor` registers the workers and the service runs them with `Monitor::run_with_signal`. Imports need no queue: approved rows in `imports` are the queue, and each run claims one import at a time. Modules that need to hand work to a job later get their own port, which `jobs` implements.

---

## 9. Cross-cutting

### Runtime

One binary; the service is the application, and the CLI is a second interface to it.
- `yokoku` without a command runs the service: the web UI, the event subscribers and the apalis `Monitor`. The web server binds first, on `[web] host` and `port` (default `127.0.0.1:8080`; under `dx serve`, the address `dx` assigns), and the service stops at startup when the address is taken or the web assets are missing (`public/` next to the binary, or `DIOXUS_PUBLIC_PATH`). Server functions reach the use cases through `yokoku_web::AppState`, which `yokoku` builds. Each subscriber gets its own `Delivery` loop. A signal stops the monitor first; then the deliveries and the web server are cancelled and awaited.
- `just web serve` runs the whole app in development: `dx` builds `yokoku-web` for the browser and runs the `yokoku` service as its server (`dx serve @client --package yokoku-web @server --package yokoku`), and it provides the web assets.
- Releases are Linux only (x86_64, aarch64). GoReleaser builds the browser part once (`dx bundle --fullstack false` into `target/web/public`) and each server with `cargo zigbuild`, unstripped; `dx tools assets` then writes the asset paths into the server binary, which a plain `cargo` build leaves as placeholders and which need its symbols, and `zig objcopy --strip-all` strips it. The archive and the Docker image hold `yokoku` with `public/` beside it. The image runs with `--config /config/app.toml`, which is `config/docker.toml` (database in `/data`, web on `0.0.0.0`) unless a mounted `/config` replaces it. macOS is not built: linking it needs Apple's SDK frameworks.
- Subcommands are the command-line interface for setup and operations: `settings`, `root`, `scan`, `refresh`, `files`, `jellyfin`. They call the same use cases against the same database, so they need no running service. A command that writes events delivers them to every subscriber (`Delivery::catch_up`) before it exits.
- The feature commands (`search`, `add`, `list`, `show`, `monitor`, `numbering`, `remove`, `calendar`, `missing`, `review`, `rename`, `download`, `import`, `history`, `delete`) predate the web interface. Each is removed in the change that ships its page, together with its CLI tests.
- `delete` and `remove --delete-files` list the files and ask on stdin before deleting (FR-8.5); no answer counts as no, and `--yes` skips the question.

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
| `metadata`, `download-clients`, `media-servers` | HTTP adapters against `wiremock`. |
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
| Ports owned by modules; adapters depend on modules | Modules depend on `db`, `metadata`, `download-clients` | Business logic would change with infrastructure. |
| Event log in SQLite, one position per subscriber, own code (~200 lines) | evento, cqrs-es/sqlite-es, hexeract-outbox, cratestack-outbox, eventsdb | Each fails at least one requirement: works with SQLite, keeps a position per subscriber, no event sourcing, sqlx 0.9. |
| State tables are the source of truth | Event sourcing | TMDB is the real source of metadata. Rebuilding from events adds no value, and stored event schemas are costly to migrate. |
| In-process delivery | External broker (NATS, Redis, Kafka) | Adds deployment weight for a single-user, self-hosted app. |
| Import state machine in our tables | `apalis-workflow` | Review can pause for days, and the UI must query import state. |
| Approved imports in our table are the job queue, claimed atomically | apalis storage-backed `ExecuteImport` queue | Import state lives in one place, and the claim lets the CLI and `serve` run imports side by side. |
| No actor framework | kameo, ractor | Mailboxes are in memory (not durable), and it would be a third messaging model next to events and jobs. The one real race (concurrent imports) is solved by concurrency 1. |
| One lock file around library file changes | In-process mutex; locking rows in SQLite | A mutex does not reach the CLI in another process; a write transaction held while a file copies would block every other writer. |
| Own TMDB client | `tmdb-api` crate | Few endpoints needed; low adoption. |
| `Arc<dyn Port>` + `async-trait` | Generic `App<I: Infra>` | Generics would spread through every signature. |
| SQLite | PostgreSQL | Single user, self-hosted, one file to back up. |
| Dioxus behind a thin web crate | topcoat, Leptos | Client-side state in Rust for selection-heavy dialogs, and the same crates (such as `naming`) on server and browser. Pre-1.0; a breaking upgrade affects only the web crate. |

---

## 11. Requirement map

| Requirement | Where |
|---|---|
| FR-1 Library | `library`, `metadata` |
| FR-2 Monitoring | `domain` (presets), `library` |
| FR-3 Download client | `downloads`, `download-clients` |
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
2. **Next episode + calendar:** `library` queries, CLI `calendar` / `missing`.
3. **Root folders + scan + manual match:** `media` scan, `detect` basics, `system`.
4. **Renaming:** `naming`, rename preview.
5. **Transmission:** `downloads`, `download-clients`, `jobs` (`SyncDownloads`).
6. **Detection + review + auto import:** full `detect` corpus, the import pipeline.
7. **History, Jellyfin:** history query, `integrations`.
8. **Web UI:** `web` on Dioxus.
