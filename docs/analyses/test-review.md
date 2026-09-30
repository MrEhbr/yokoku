# Test review

A review of all 839 tests in the workspace, as nextest counts them with each rstest case counted separately. Six reviewers each read one area in full, together with the code under test. Every finding below was re-opened at its cited lines before it went in. For every proposed removal, the named covering test was opened to confirm it asserts the same behaviour.

The review follows seven decisions made beforehand:
1. Test each behaviour at the lowest level that proves it; higher levels keep one test per wiring path.
2. Replace a hand-written fake with mockall only where the mock comes out shorter.
3. Put shared fakes and fixtures in a new `yokoku-test-support` crate.
4. Use proptest for invariants of pure logic, and keep readable example tables as rstest cases.
5. Keep the sentence-style test names.
6. Fix flaky tests.
7. List gaps at low priority.

## 1. Summary

**Overall.** The suite is already in good shape. rstest tables and fixtures are used throughout. Proptest already covers these invariants:
- the round-trips of `EpisodeSpan`, ids and events;
- `sanitize`, both that its output is valid and that sanitizing twice changes nothing;
- detect's partition of every file into exactly one place, and its naming↔detect round-trip;
- the series monitoring presets;
- the db storage round-trips;
- delivery backoff.

The review found little to delete. The value is in a handful of duplicated CLI checks, two slow metadata tests, one flaky service test, shared test support, and web tests.

**Test counts per crate.** "Now" counts rstest cases separately. "After" is the estimate if every finding is applied, gaps excluded.

| Crate | Now | After (est.) | Change |
|---|---|---|---|
| domain | 178 | 181 | +1 event case, +2 properties |
| naming | 53 | 55 | +2 properties |
| detect | 117 | 117 | none |
| config | 9 | 10 | +1 bare-section case |
| db | 58 | 62 | +4 `read_before` cases |
| events | 33 | 33 | none |
| jobs | 1 | 1 | none |
| metadata | 58 | 58 | faster only |
| library | 61 | 59 | −2 folder cases |
| integrations | 7 | 7 | none |
| media | 93 | 93 | helpers only |
| system | 33 | 35 | probe mapping moves to unit tests |
| downloads | 22 | 22 | none |
| download-clients | 21 | 21 | none |
| media-servers | 6 | 6 | none |
| yokoku | 61 | 55 | −5 CLI cases, −1 test |
| web | 28 | 37 | +9 settings-field cases |

**Findings by category.**

| Category | High | Medium | Low |
|---|---|---|---|
| redundant | 1 | 2 | 3 |
| slow | 2 | 2 | — |
| flaky | 1 | — | — |
| test-support | — | 4 | 2 |
| fixture / clarity | — | — | 9 |
| proptest | — | 1 | 1 |
| gap | — | 2 | 15 |
| web tests | — | 1 | 1 |

**Top 10.**

| ID | Finding | Effect |
|---|---|---|
| T-X1 | Five of six CLI "bad setting" cases re-prove config and naming rules | 5 process spawns fewer |
| T-C1 | `slow_answers_time_out` waits through 3 s of retry backoff it doesn't test | ≈3.1 s → ≈0.05 s |
| T-C2 | `endless_episode_pages_are_invalid` walks 100 real pages | ≈3.0 s → <0.1 s |
| T-F7 | Flaky `a_torrent_can_be_added_for_an_item`: background sync races the first add | Deterministic |
| T-F8 | `a_running_service_reloads…` waits for the fixed 5 s delivery poll, twice | 5.2 s → <1 s (needs a small source change) |
| T-S1 | `yokoku-test-support`: clocks, `MemoryLog`, the Transmission handshake, the JSON fixture loader, sample metadata, publisher | Removes about 10 copies |
| T-W1 | Web: the settings field logic gets tests, including pulling the "value outside the list" code into a function | +9 cases |
| T-D2 | ffprobe report → `MediaInfo` mapping is tested only through a stand-in shell script | Unit tests, no subprocess |
| T-A1 | `SettingsChanged` is the only event without a stored-format case | Closes a contract hole |
| T-B4 | `EventLog::read_before` has no db-level test, while `read_after` has a 4-case table | Closes a gap |

**Slowest tests.**
- `settings_commands::a_running_service_reloads_a_setting_changed_from_the_command_line`, 5.2 s: see T-F8.
- `metadata http::tests::slow_answers_time_out`, 3.2 s: see T-C1.
- `metadata::tvdb endless_episode_pages_are_invalid`, 3.0 s: see T-C2.
- The `yokoku` CLI tests, 2.6–3.4 s each: each spawn of the real binary pays process start-up and migrations. T-X1, T-F3 and T-F4 remove 7 spawns. The rest are single-spawn wiring tests and stay.
- The db round-trip proptests, about 1.1 s each: these run 64 cases against real SQLite, which is the right level, so they stay.

nextest runs tests in parallel, so the whole suite's wall time (about 12.5 s) is set by the longest tests. Fixing T-F8, T-C1 and T-C2 is what shortens it.

## 2. Cross-cutting findings

### T-S1: the `yokoku-test-support` crate

It is a dev-dependency only, and each module is used by two or more crates.

| Module | Contents | Moves from | Used by |
|---|---|---|---|
| `clock` | `TODAY` (2026-09-26) and `TestClock` (a mutex around the time, with `advance`); a fixed clock is a `TestClock` never advanced | `downloads/tests/downloads.rs:25-34` and `media/tests/common/mod.rs:26-40` (identical bodies), `library/tests/common/mod.rs:26-44` and `integrations/tests/rescans.rs:15-28` (advancing variants) | downloads, media, library, integrations |
| `events` | `MemoryLog`, the `EventLog` fake: stateful, kept hand-written, with the `failing`/`recover`/`events`/`correlations` helpers; and `publisher(db, dir)` | `events/tests/publisher.rs:14-79` and `system/tests/spool.rs:17-64` (same fields and impl); `Publisher::new(…FileSpool…)` at `downloads/tests/downloads.rs:147`, `media/tests/common/mod.rs:70,158`, `library/tests/common/mod.rs:193`, `download-clients/tests/transmission_live.rs:86` | events, system, downloads, media, library, download-clients |
| `transmission` | `session_handshake(&server)`, which mounts the 409 + `X-Transmission-Session-Id` pair | `download-clients/tests/transmission.rs:14-42`, `yokoku/tests/service.rs:764-795`, and an inline second copy at `service.rs:838-850`. The copies also disagree on the session id literal | download-clients, yokoku |
| `jellyfin` | `system_info(api_key, version) -> Mock`, unmounted | `media-servers/tests/jellyfin.rs:33-38`, `yokoku/tests/media_commands.rs:131-137` | media-servers, yokoku |
| `metadata` | `fixture(name)`, the TMDB/TVDB JSON loader; and the sample builders `series_metadata(source, title, status, seasons)` and `movie_metadata(…)` | loader: `metadata/tests/tmdb.rs:14-16`, `yokoku/tests/metadata_commands.rs:25-28`, `media_commands.rs:25-28`, and inline at `service.rs:936-939`. Builders: `library/tests/common/mod.rs:123-160`, with media's hard-coded `frieren_metadata`/`dune_metadata` at `media/tests/common/mod.rs:170-194` | metadata, yokoku, library, media |

These stay where they are, because only one crate uses them: `ScriptedClient`, `MemorySpool`, `GatedLog`, `Recorder`/`Harness`, `Servers`, `Files`, `StaticMetadata`, `Stub`, `RecordingServer`, config's `MemoryStore`, and `ScriptedProbe`.

### T-M: mockall adoption

**None.** Every hand-written fake falls into one of three groups:
- Stateful, and driven by the tests like the real thing: `ScriptedClient`, `ScriptedProbe`, `MemoryLog`, `Files`, `StaticMetadata`, `RecordingServer`.
- A synchronisation gate: `GatedLog`.
- A tiny stub reused unchanged across many tests: `Stub`, `Failing`, `Servers`. Here the mockall expectations would be longer than the fake.

Mockall would only add a dependency, so none of these is converted.

### House idioms to keep

- Finite round-trip properties are written as `prop_oneof![Just(…)]`.
- Async tests use `#[rstest] #[tokio::test]` with `#[future(awt)]` fixtures.
- Assertions carry messages, as in `assert!(cond, "{value}")`.

## 3. Findings by crate

### domain

#### T-A1: `SettingsChanged` has no stored-format case

- **Category**: gap.
- **Value**: medium.
- **Locations**: `domain/tests/events.rs:83-219` (`stored_format_is_stable`, 14 cases) and `:301-374` (the `any_event` strategy).
- The only place `SettingsChanged` appears in a test is `yokoku/tests/settings_commands.rs`, which checks that the event fires, not how it is stored.
- **Fix**: add `case::settings_changed` to the table and a `SettingsChanged` variant to the strategy.

#### T-A2: property that `Secret::masked` reveals at most 8 characters

- **Category**: proptest.
- **Value**: low–medium.
- **Location**: `domain/src/secret.rs:52-58`.
- **Property**: for any `s`, `Secret::new(&s).masked()` contains at most 8 characters of `s` besides `…`.
- Keep the 6-case table at `secret.rs:124-133`. It documents the exact boundaries.

#### T-A3: `Series::span` and `absolute_span` have no domain-level test

- **Category**: gap.
- **Value**: low.
- **Location**: `series.rs:394-408`.
- They are covered only through `detect/tests/plan.rs`.

#### T-A4: `SubtitleTags` `Display` is untested

- **Category**: gap.
- **Value**: low.
- **Location**: `subtitle.rs:14-24`.

### naming

#### T-A6: properties for `subtitle_path`

- **Category**: proptest.
- **Value**: medium.
- **Location**: `naming/src/subtitle.rs:6-23`.
- **Properties**:
  1. The subtitle's parent folder equals the video's parent folder.
  2. The subtitle's file name starts with the video's file stem, which is the pairing that detect's classify relies on.
- Keep the table at `naming/tests/subtitles.rs:11-22`.

### config

#### T-A11: env-over-stored precedence is tested only through the CLI

- **Category**: gap.
- **Value**: low.
- It is tested only by `yokoku/tests/settings_commands.rs:74-91`.
- **Fix**: add a `Config::load` test.

#### T-A12: the log config's custom level (de)serialize is untested

- **Category**: gap.
- **Value**: low.
- **Location**: `config/src/log.rs:6-25`.

### db

#### T-B4: `EventLog::read_before` has no db-level test

- **Category**: gap.
- **Value**: medium.
- **Location**: `db/src/event_log.rs:81`.
- **Fix**: add a table matching `read_after_returns_later_events_up_to_the_limit` (`db/tests/event_log.rs:53-70`): before a cursor, no cursor, limited, and before the start.

#### T-B1: shared helpers into `db/tests/support/mod.rs`

- **Category**: fixture.
- **Value**: low.
- `block_on` is copied at `catalog.rs:235`, `downloads.rs:63` and `media.rs:133`, and inlined at `media_info.rs:102`.
- `now()` is copied at `catalog.rs:25-27` and `media.rs:20-22`.

#### T-B2: `#[future(awt)]` everywhere

- **Category**: clarity.
- **Value**: low.
- Replace `#[future] db` followed by `let db = db.await;` in `catalog.rs:261,278,297,311` and in all 9 tests of `media.rs`.

#### T-B3: use the crate's `#[fixture] db` pattern

- **Category**: fixture.
- **Value**: low.
- `media_info.rs:41,56,66` and `downloads.rs:101,112` call `open_in_memory` inline instead.

#### T-B5: `RescanStore` has no db test file

- **Category**: gap.
- **Value**: low.
- It is covered only through `integrations/tests/rescans.rs`.

#### T-B6: name the `1 << 33` size

- **Category**: clarity.
- **Value**: low.
- It appears unnamed at `media_info.rs:17` and `media.rs:29`. Call it `SIZE_BEYOND_U32`.

### jobs

#### T-B9: the default cron strings are never parsed in a test

- **Category**: gap.
- **Value**: low.
- **Location**: `jobs/src/lib.rs:60-66,78-89`.
- **Fix**: assert that `ScheduleSettings::default().schedules()` is `Ok`.

### metadata

#### T-C1: `slow_answers_time_out` pays for retry backoff it isn't testing

- **Category**: slow.
- **Value**: high.
- **Location**: `metadata/src/http.rs:184-196`.
- A timeout is retried with `backoff` of 1 s, then 2 s (`http.rs:156-158`, `MAX_ATTEMPTS = 3`), so the test takes about 3.15 s.
- **Fix**: make the attempt count a field, with a test constructor next to `with_timeout`, and use 1 attempt in this test.
- The retry policy stays covered by `tmdb.rs:342` (`temporary_failures_are_retried`) and `:359` (`failing_requests_leave_the_source_unavailable`).
- **Risk**: this changes source, adding a test constructor.

#### T-C2: `endless_episode_pages_are_invalid` walks 100 pages

- **Category**: slow.
- **Value**: high.
- **Location**: `metadata/tests/tvdb.rs:315-331`.
- `MAX_EPISODE_PAGES = 100` (`tvdb.rs:22`), and each request is spaced 25 ms apart.
- **Fix**: make the page cap an instance field with a test constructor, and use 2 in this test.
- **Risk**: this changes source.

#### T-C3: bare `assert!(matches!(…))` hides the actual error

- **Category**: clarity.
- **Value**: low.
- **Locations**: `tvdb.rs:330,341,352-353`; `tmdb.rs:315,366,379,388`.
- **Fix**: add `"{error:?}"`.

#### T-C4: a tvdb `logged_in` fixture

- **Category**: fixture.
- **Value**: low.
- `MockServer::start()` followed by `mount_login` repeats at `tvdb.rs:122,173,195,210,226,318,335`.

#### T-C5: TVDB's no-API-key branch is untested

- **Category**: gap.
- **Value**: low.
- **Location**: `tvdb.rs:68`.

### library

#### T-C7: `a_folder_name_must_be_one_path_component` repeats domain's table

- **Category**: redundant.
- **Value**: medium.
- **Location**: `library/tests/metadata.rs:90-106`.
- Its cases (empty, nested, parent) are a subset of `domain/tests/folder.rs:15-24` `anything_else_is_rejected`, which asserts `assert_eq!(ItemFolder::new(…), Err(InvalidFolderName(…)))` over 6 cases.
- **Fix**: keep `nested` only, as the wiring case (it becomes `LibraryError::InvalidFolder` and emits no event).

#### T-C8: `refresh_series_stores_new_episodes_and_keeps_changes` re-derives the domain merge

- **Category**: redundant.
- **Value**: low–medium.
- **Location**: `library/tests/metadata.rs:176-190`.
- Its monitored assertion is proven in domain by `domain/tests/series.rs:243-260` (`refresh_monitors_new_episodes_like_their_season`) and `:187-214`.
- **Fix**: narrow it to the stored episode count plus one event.

#### T-C9: bare `matches!` assertions in library

- **Category**: clarity.
- **Value**: low.
- **Locations**: `library/tests/metadata.rs:47,229`, `library/tests/library.rs:138,144`.

#### T-C6: artwork `content_type` has no webp case

- **Category**: gap.
- **Value**: low.
- **Location**: `library/src/artwork.rs:85-90`.

#### T-C10: listing sort and filter are tested only through a full `App` with SQLite

- **Category**: gap.
- **Value**: low.
- **Location**: `listing.rs:85-105`.
- **Fix**: add pure comparator tests.

#### T-C11: the retry's give-up after 5 conflicts is untested

- **Category**: gap.
- **Value**: low.
- **Location**: `library/src/retry.rs:6-24`.

### media

#### T-D1: one `linked` helper

- **Category**: fixture.
- **Value**: low–medium.
- Three near-identical helpers sit at `deleter.rs:15-20`, `rename.rs:18-22` and `renumber.rs:11-15`.
- **Fix**: move one `common::linked(app, paths) -> Vec<MediaFile>` into common, and rename the unrelated pure converter at `scan.rs:14` to `linked_files`.

#### T-D3: `common::episode()` is never called

- **Category**: useless.
- **Value**: low.
- **Location**: `media/tests/common/mod.rs:166-168`.
- The `#![allow(dead_code)]` in that file hides it. Delete it.

#### T-D4: split compound `assert!(a && b)`

- **Category**: clarity.
- **Value**: low.
- **Locations**: `deleter.rs:34,71,91,110`.
- Split each so a failure names the path in the wrong state.

#### T-D5: `Deleter`'s `MovieRemoved` handler has no media-level test

- **Category**: gap.
- **Value**: low.
- **Location**: `media/src/deleter.rs:107-113`.

#### T-D6: renumbering a series no longer in the library is untested

- **Category**: gap.
- **Value**: low.
- **Location**: `media/src/scan/renumber.rs:17`.

#### T-D7: permission tests assume a non-root runner

- **Category**: gap.
- **Value**: low.
- **Locations**: `deleter.rs:104`, `rename.rs:176`, `system/tests/fs.rs:99`.
- **Fix**: skip these tests when running as root.

### system

#### T-D2: `impl From<Report> for MediaInfo` is tested only through a stand-in ffprobe script

- **Category**: slow.
- **Value**: medium.
- **Locations**: the mapping is at `system/src/probe.rs:110-140`; the tests are `system/tests/probe.rs:40-49,51-67`.
- **Fix**: move the cases into unit tests in `probe.rs` that build `Report` from JSON. Covered cases: cover art skipped, `und` becomes no language, forced flag, duration. Keep one subprocess test for the wiring, plus the error-path tests.

### downloads, download-clients, media-servers

These are already at the right level.
- The tests in `downloads` never touch Transmission's wire format; that is tested only in `download-clients`.
- Only the shared-support items in §2 apply here.

Gaps:
- **T-E6**: a plain 5xx answer from Transmission is untested (`transmission.rs:76-79`).
- **T-E5**: two syncs racing to adopt the same outside torrent are untested (`downloads.rs:175-181`), while the matching race for known downloads is tested.

### yokoku

#### T-X1: `bad_settings_are_refused_and_not_stored` re-proves config and naming rules

- **Category**: redundant.
- **Value**: high.
- **Location**: `yokoku/tests/settings_commands.rs:58-71`, 6 cases, each a subprocess spawn.
- What already covers each case:

  | Case | Covered by |
  |---|---|
  | `unknown_mode`, `unknown_key`, `bootstrap` | `config/tests/settings.rs:117-130` `a_value_that_does_not_load_is_refused_and_not_stored`, same keys, asserting `contains("teleport")`, `contains("not a setting")` and `contains("before the database opens")` |
  | `bad_schedule` | `config/tests/settings.rs:81-92`, same input `"every day"`, asserting `contains("Invalid schedule")` |
  | `pattern_without_episodes` | `naming/tests/templates.rs:41-44`, `case::episodes_required` |
  | `section` (`"naming"`) | Nothing lower down |

- **Fix**:
  1. Add the bare-section case to `config/tests/settings.rs`.
  2. Keep one CLI case as the wiring test: it is refused, and nothing is stored.

#### T-F3: `a_missing_secret_file_is_reported` duplicates lower-level tests

- **Category**: redundant.
- **Value**: medium, about 2.6 s of one spawn.
- **Location**: `media_commands.rs:162-173`.
- The same error text is asserted by `domain/src/secret.rs:110-115` `a_missing_file_fails`, as `contains("failed to read secret file …")`.
- The wiring (the CLI fails cleanly on a bad config value) is covered by `settings_commands.rs:134-145` `loading_the_configuration_does_not_create_the_database`.
- **Fix**: remove the test.

#### T-F4: `the_jellyfin_api_key_can_be_read_from_a_file` spawns the CLI twice

- **Category**: redundant.
- **Value**: low–medium.
- **Location**: `media_commands.rs:128-160`.
- It runs once with the key from the config file and once from the environment, which proves the same file-secret mechanism twice.
- The rest is covered by the domain `Secret` file tests and `media-servers/tests/jellyfin.rs:30-41` `reports_the_server_version`.
- **Fix**: keep the environment variable run.

#### T-F5: the "still holds 1 library items" assertion is covered lower down

- **Category**: redundant.
- **Value**: low.
- **Location**: `media_commands.rs:108-114`.
- `media/tests/roots.rs:101` `a_root_folder_holding_items_is_not_removed` covers it.
- **Fix**: drop that block and keep the add/list/remove happy path.

### web

See §4.

## 4. Web crate testing plan

The approach follows the Dioxus 0.7 guide (learn/0.7, "Testing"):
- **Component tests** render through a `VirtualDom` and `dioxus_ssr` and assert on the HTML: `VirtualDom::new_with_props(C, props)`, then `rebuild_in_place()`, then `dioxus_ssr::render(&vdom)`. `dioxus_ssr::render_element(rsx!{…})` works for plain elements.
- **Hook tests** use a small host component, driven with `wait_for_work().now_or_never()` and `render_immediate(&mut NoOpMutations)`.
- **End-to-end tests** with Playwright are not proposed. Nothing here needs a real browser.

Server-function behaviour stays in `yokoku/tests/service.rs`.

The filtering and keyboard selection in `components/combobox.rs` belong to `dioxus_primitives`, not to our code, so they are not tested here.

Dev-dependencies: `dioxus-ssr = "=0.7.10"`, pinned to match `dioxus`, and `futures` for `now_or_never`. The tests live in `#[cfg(test)]` modules and run on the host under `cargo nextest run --workspace`.

#### T-W1: `pages/settings/fields.rs` `text()` table

- **Location**: `fields.rs:224-231`.
- No rendering is needed. The cases:

  | Control | Value | Expected |
  |---|---|---|
  | Secret | any | `""` |
  | any | `Null` | `""` |
  | List | `["a", "b"]` | `"a, b"` |
  | List | `[]` | `""` |
  | Text | `"x"` | `"x"` |
  | Switch | `true` | `"true"` |

#### T-W2: pull the "value outside the list" code into a function and test it

- **Location**: `fields.rs:110-116`.
- **Fix**: move it into `fn choice_options(choices, current) -> Vec<(String, String)>`, which changes source.
- **Cases**:
  - The current value is in the list: nothing is inserted.
  - It is absent and not empty: it is inserted once, at index 0, with value and text equal.
  - It is empty: nothing is inserted.

#### T-W3 (optional): one SSR render test for `SettingField` with `Control::Switch`

- It needs an `Unsaved` context provider in a host component.
- **Assertion**: the rendered switch shows as checked when the setting's value is `true`.
- This proves our prop wiring, not the primitive.

Web gaps:
- `typed()` splitting a list on commas (`fields.rs:76-81`).
- The combobox's pointerdown guard (`combobox.rs:49-54`), which is trivial.

## 5. Flaky tests

### `yokoku/tests/service.rs:797-817` `a_torrent_can_be_added_for_an_item`

**Cause.**
- The mock Transmission's `torrent-get` lists the torrent with the `yokoku` label from the moment the mock starts (`service.rs:776-793`).
- `DownloadOptions::takes_on` adopts any torrent with that label (`downloads/src/downloads.rs:44-48`).
- Two background jobs can run before the test's first POST: `sync_downloads` (`*/30 * * * * *`) and `sync_active_downloads` (`*/5 * * * * *`), both in `jobs/src/lib.rs:36-37`.
- If either runs first, it records the torrent, and the test's first POST answers "already added".

**Fix.** Pin both job schedules, not just `sync_downloads` as two reviewers suggested. The broader fix is in `Service::start_with` (`service.rs:41-65`): set every `APP__SERVE__*` schedule to a far-future cron such as `0 0 0 1 1 *` before `.envs(env)`. Every service test then opts in to the jobs it needs, the way `service.rs:422` already sets `SCAN_LIBRARY` to run every second.

**Verification.** Run the test 20 times in a row.

The other service tests were checked for the same pattern, and none depend on background timing.

## 6. Slow tests needing source changes

### T-F8: `settings_commands.rs:170-191` `a_running_service_reloads_a_setting_changed_from_the_command_line`

This test takes 5.2 s.
- The CLI's `settings set` runs in another process, so the service only sees the `SettingsChanged` event on its next delivery poll.
- The poll interval is fixed at 5 s by `DeliveryConfig::default()` (`events/src/delivery.rs:25`), used in `yokoku/src/app.rs:193-204`.

**Fix.** Make the poll interval configurable, for example `[events] poll_interval_ms`, and set it to about 200 ms in this test.

**Risk.** This changes source in the config and yokoku crates. It needs a yes before it is applied.

T-C1 and T-C2 (§3, metadata) also add test constructors to source.

## 7. Gaps

All low priority, except T-A1 and T-B4, which are listed in §3 at medium.

- T-A3 `Series::span` and `absolute_span` at the domain level.
- T-A4 `SubtitleTags` `Display`.
- T-A11 env-over-stored precedence in `config`.
- T-A12 the log config's (de)serialize.
- T-B5 the db `RescanStore`.
- T-B9 the default cron strings.
- T-C5 TVDB with no API key.
- T-C6 webp artwork.
- T-C10 pure tests for listing sort and filter.
- T-C11 the retry giving up.
- T-D5 `Deleter` handling `MovieRemoved`.
- T-D6 renumbering a series that is gone.
- T-D7 permission tests running as root.
- T-E5 two syncs adopting the same outside torrent.
- T-E6 a Transmission 5xx answer.
- CLI `files show movie`.
- CLI root add/list for movies.
- CLI `settings get` on an unknown key.
- Web `typed()` comma splitting.

## 8. Rejected candidates

- **Removing the 50 ms sleeps in `events/tests/delivery.rs:177,281`.** They let delivery finish its startup catch-up read. Without them, the "wakes up" tests pass through the catch-up read even when wake-up is broken.
- **Dropping the final probe step of `media_commands.rs:220-235`.** It is the only CLI proof that `files probe` succeeds, which makes it a wiring path.
- **Moving a single-crate fake to test-support**: `ScriptedClient`, `MemorySpool`, `GatedLog`, `Recorder`, `Harness`, `Servers`, `Files`, `StaticMetadata`, `Stub`, `RecordingServer`, `MemoryStore`, `ScriptedProbe`.
- **Mockall for any fake**: see §2.
- **Proptest for readable tables**:
  - detect's 49-case `parse` corpus and its `plan` decision table;
  - the `templates.rs` error table;
  - `artwork_name`;
  - `Known::under`;
  - `week_of` and `month_of`.
- **Tests that are already right**:
  - detect `round_trip.rs`, which is already proptest;
  - the finite-enum round-trip proptests;
  - the paths and sanitize proptests;
  - the db proptests, which are about 1.1 s each and at the right level;
  - the backoff table plus its property.
- **`system/tests/lock.rs` vs `media/tests/lock.rs`.** They test different things: the flock itself, and whether each use case takes the lock.
- **The sample file in `media/tests/scan.rs`.** It checks scanner wiring, not the classification rule.
- **The `media`/`system` timing tests.** Their bounds assert something is still blocked, backed by a real block, so a slow runner can't make them fail.
- **`file_links_survive_concurrent_refreshes`.** It is single-threaded and its assertion doesn't depend on order.
- **CLI tests that prove CLI-only behaviour and stay**: `integration_test.rs` (5 tests), `found_files_are_probed_and_shown_with_their_details`, `an_unreachable_jellyfin_does_not_fail_the_change`, `metadata_commands_need_a_token`, `refresh_updates_one_item_or_the_whole_library`.
- **The web `format.rs` and `route.rs` tests.** They are already well structured.
- **Sharing the two "stalled server times out" unit tests** in transmission and jellyfin. Each serves one crate, and a shared helper would have to be generic over the client.
- **`transmission_live.rs`.** It is `#[ignore]`d and duplicates nothing.

## 9. Suggested apply order

1. `test: add a shared test-support crate`: the §2 modules, each wired to its first users.
2. `fix(test): run a service test's scheduled jobs only when it asks`: the flaky fix. It must pass 20 runs in a row.
3. One `test(<crate>):` commit per crate, in dependency order:
   domain → naming → config → db → jobs → metadata → library → media → system → downloads → download-clients → media-servers → yokoku → web.
   - A crate's move to test-support goes in that crate's commit.
   - The yokoku commit includes T-X1 only after config has the bare-section case.
4. Source changes that need a yes, each in its own commit:
   - T-C1 and T-C2, the metadata test constructors;
   - T-F8, the delivery poll interval setting;
   - T-W2, `choice_options` pulled out of the component.

## 10. Outcomes

Before: 839 tests, and a wall time of about 12.5 s. After: 888 tests, and about 10.2 s.

The slowest tests are now the single-spawn `yokoku` CLI wiring tests, at 2.7–3.5 s each, which this review kept on purpose.

| Finding | Outcome |
|---|---|
| T-S1, the test-support crate | Applied, `78ea529`. Its modules are `clock`, `events`, `transmission`, `jellyfin` and `metadata`. Each crate moved over in its own commit |
| Flaky `a_torrent_can_be_added_for_an_item` | Applied, `d828e9d`. Every job schedule is set to never by default; the test passed 20 of 20 runs |
| T-A1, T-A2, T-A3, T-A4 | Applied, `32f1db0` |
| T-A6 | Applied, `ca853a1`. The first version generated a stem of `.` and failed; `eb355fc` generates stems the way the naming patterns do (passed 20 000 cases) |
| T-A11 | Skipped. `Config::load` reads the environment, and setting environment variables in-process is `unsafe` in edition 2024, which the workspace forbids. The CLI test stays |
| T-A12, and the bare-section case of T-X1 | Applied, `9972d90` |
| T-B1–T-B6 | Applied, `14f118e` |
| T-B9 | Applied, `82e7c7e` |
| T-C1 | Applied as a unit test on tokio's paused clock, with no production change, 3.2 s → 0.09 s. Commit `aed792b` |
| T-C2 | The page cap became a field, tested as a unit test through struct update, 3.0 s → 0.09 s. Commit `aed792b` |
| T-C3, T-C4, T-C5, the shared fixture loader | Applied, but they landed in `eb355fc`, whose message names only the naming fix; a split of that commit was not allowed |
| T-C6–T-C11 | Applied, `7e013ad`. T-C9 skips the three multi-line `matches!` in `library/tests/library.rs`, where the value would first need its own binding |
| T-D1, T-D3, T-D4, T-D5, T-D6 | Applied, `4311645` |
| T-D7 | Skipped. Checking for root needs `unsafe` libc or a new dependency, and nothing runs these tests as root |
| T-D2 | Applied, `b5397c8` |
| T-E3, T-E4, T-E5 | Applied, `b24b68c` |
| T-E1, T-E6 | Applied, `380afa0` |
| T-E2 | Applied, `a3c96ec` |
| T-X1, T-F3, T-F4, T-F5, the CLI gaps (a movie's files, a movies root, `settings get` on an unknown key) | Applied, `f365cf0` |
| T-F8 | `[events] poll_interval_ms` added, `765545d`; the test runs with 100 ms, 5.2 s → 0.5 s, `00e4010` |
| T-W2, and the extracted `typed()` | Applied, `53371a9` (structural) |
| T-W1, T-W3, and the `typed()` gap | Applied, `ac85b92`. `dioxus-ssr` renders `SettingField` inside an `Unsaved` context |
| The combobox pointerdown guard gap | Skipped. It is trivial and would only restate `!open() && !disabled()` |
| T-M (mockall) | Not adopted, as §2 recommends |
