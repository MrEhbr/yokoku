# Web crate idiom review

A review of `crates/web` (src and gallery, about 14.2k lines) for idiomatic Rust and Dioxus 0.7. The
aim is cleaner, more compact code that leans on std traits, iterators, combinators and Dioxus's
reactive primitives. Five reviewers read one area each in full. Every finding below was re-opened at
its cited lines before it went in; corrections to the reviewers' claims are noted where they apply.

Verdicts:
- **Confirmed**: read at the cited lines.
- **Plausible**: needs a build or a browser check before the fix.

## 1. Summary

37 findings, including 5 cross-cutting ones that merge sites from several areas.

| Category | High | Medium | Low |
|---|---|---|---|
| duplication | 4 | 3 | 2 |
| std-trait | 1 | | |
| dioxus-reactivity | 1 | | 1 |
| dioxus-props | 1 | | 1 |
| ownership | | 3 | 7 |
| iterator / combinator | | | 5 |
| types / pattern | | | 4 |
| dead-code | | 2 | |
| correctness | | 1 | 1 |
| **Total** | **7** | **9** | **21** |

The estimated net reduction is about 260 lines, not counting dead code.

Top 10 by value:

| ID | Finding | Net |
|---|---|---|
| W-A4 | 17 identical `Provides<T>` impls become a `provides!` table | −80 |
| W-X2 | `Option<EventHandler>` plus `if let` forwarding becomes `#[props(default)] EventHandler` (5 components) | −40 |
| W-D2 | Detail pages hand-roll what `Resource::restart()` does | −28 |
| W-E1 | Three dialogs repeat the same header and open/close wrapper | −25 |
| W-D1 | Six pages repeat the "could not be loaded" alert | −20 |
| W-X1 | Seven hand-written singular/plural branches | −15 |
| W-X4 | The "log it, show a generic message" error mapping is repeated 6 times in `api/` | −14 |
| W-B4 | Tooltip and hover card share an identical 11-line position class block | −11 |
| W-A1 | `Kind` ↔ `MediaKind`/`RootKind` free fns and inline matches become `From` impls | −10 |
| W-X3 | `pause()` split by target, and a runtime `cfg!` check | −10 |

## 2. Cross-cutting findings

### W-X1: One plural helper instead of seven hand-written branches

- **Category**: duplication.
- **Value**: medium, about −15 lines.
- **Risk**: none. Rendered text stays the same.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/pages/missing/groups.rs:105-108`: an existing private `count(n, one, many)`, the one to promote.
  - `crates/web/src/pages/downloads/torrents.rs:31-35`
  - `crates/web/src/pages/settings/roots.rs:68-73` (a closure) and `:170`
  - `crates/web/src/components/remove_item.rs:28`
  - `crates/web/src/components/unrecognised_files.rs:9-13`
  - `crates/web/src/dialogs/import_review/mod.rs:111`
  - `crates/web/src/format.rs:27`: only the `unit` in `relative`. That one stays inline, because it shares `count` with the direction text.

Before:

```rust
if count == 1 { "1 torrent" } else { "{count} torrents" }
let its_files = if files == 1 { "its file".to_owned() } else { format!("its {files} files") };
```

After, with the helper in `format.rs`:

```rust
/// `1 episode`, `3 episodes`.
pub fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}
// call sites
TableCaption { {format::count(count, "torrent", "torrents")} }
```

The sentence-shaped sites (`unrecognised_files`, `remove_item`) keep their own wording where "1"
reads as "its file". Apply the helper only where the text is exactly "N noun".

**Why it's better**: ARCHITECTURE.md already says a formatting helper that a second page needs moves
to `format.rs`, and one rule replaces seven copies that could drift.

### W-X2: `#[props(default)] EventHandler<T>` instead of `Option<EventHandler<T>>` plus forwarding

- **Category**: dioxus-props.
- **Value**: high, about −40 lines.
- **Risk**: none. Call sites stay the same, because a closure converts into either type.
  `Callback<T>` implements `Default` as a no-op (`dioxus-core-0.7.10/src/events.rs:296`).
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/components/button.rs:55-58, 72-92`: four `if let Some(f) = &x { f.call(e) }` blocks.
  - `crates/web/src/components/pagination.rs:91-93, 122-136, 145-147, 169-171`
  - `crates/web/src/components/input.rs:6-24, 38-56`: 18 lines of `_ = x.map(|callback| callback(e))`.
  - `crates/web/src/components/textarea.rs:8-26, 41-59`: 19 lines of the same.
  - `crates/web/src/components/sidebar.rs:336, 353-358`
- **Idiom already used in the crate**: `table.rs:90,101` and `select.rs:38`.

Before:

```rust
onclick: Option<EventHandler<MouseEvent>>,
onclick: move |event| {
    if let Some(f) = &onclick {
        f.call(event);
    }
},
```

After:

```rust
#[props(default)] onclick: EventHandler<MouseEvent>,
onclick: move |event| onclick.call(event),
```

The explicit event fields are still needed, because `#[props(extends = button)]` does not carry DOM
events. Only the forwarding body shrinks.

**Why it's better**: this drops the `Option` and `if let` handling where the type already provides a
no-op default, and it matches the crate's own `table.rs`.

### W-X3: `live_downloads` gates the stream at compile time; the gallery shares nothing

- **Category**: dead-code.
- **Value**: medium, about −10 lines.
- **Risk**: low (see the note below).
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/layout/live_downloads.rs:16-39`
  - `crates/web/gallery/patterns/live.rs:66-71`: a byte-identical `pause()`.

Before:

```rust
use_future(move || async move {
    if !cfg!(target_arch = "wasm32") {
        return;
    }
    loop { … pause(RECONNECT).await; }
});
async fn pause(duration: Duration) { /* gloo on wasm32, tokio elsewhere */ }
```

After:

```rust
#[cfg(target_arch = "wasm32")]
use_future(move || async move {
    let mut latest = latest;
    loop { … gloo_timers::future::sleep(RECONNECT).await; }
});
```

Note: `let mut latest` at the top becomes an unused `mut` on the server build, and `-D warnings`
fails on that. So bind it immutably and rebind it inside the future, as shown. The hook count
differs by target, but it is fixed per build, never per render. The gallery's `pause` stays as it
is: its server-side demo sleeps on tokio, so it needs both arms.

**Why it's better**: the server build no longer carries a loop that returns at once, or a tokio
sleep it never runs.

### W-X4: One "unexpected error" mapper for the server functions

- **Category**: duplication.
- **Value**: medium, about −14 lines. This merges the reviewer's W-A2 and W-A3.
- **Risk**: low. The helpers are private to `api`.
- **Verdict**: confirmed.
- **Locations**:
  - Generic "Something went wrong; the server log has the cause":
    - `crates/web/src/api/mod.rs:55-58`
    - `crates/web/src/api/rename.rs:92-95`: the whole `failure` fn. It also fixes pedantic `needless_pass_by_value`.
    - `crates/web/src/api/review.rs:289-292`
    - `crates/web/src/api/settings.rs:227-231`
  - Identical "listing root folders failed" closures:
    - `crates/web/src/api/add.rs:127-130`
    - `crates/web/src/api/settings.rs:176-179`

After:

```rust
/// An unexpected error as the user's message; the error and `doing` go to the log.
fn unexpected(error: &dyn Display, doing: &str) -> ServerFnError {
    error!(%error, "{doing} failed");
    ServerFnError::new("Something went wrong; the server log has the cause")
}
fn root_listing_failed(error: MediaError) -> ServerFnError {
    error!(%error, "listing root folders failed");
    ServerFnError::new("The root folders could not be loaded")
}
```

The catch-all arms become `error => unexpected(&error, "reviewing the import")`.
`library_failure` builds each `ServerFnError` per arm instead of going through a `message: String`.

**Why it's better**: a change to the generic message, or to how it is logged, touches one place.

### W-X5: `Kind` conversions as `From` impls

This is the reviewer's W-A1.

- **Category**: std-trait.
- **Value**: high, about −10 lines.
- **Risk**: low. Everything is crate-private.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/api/add.rs:194-213`: `wire_kind`, `media_kind`, `root_kind`. Their call sites are at `:102, :110, :153, :155`.
  - `crates/web/src/api/settings.rs:183-186, 193-196`: inline matches for `RootKind` → `Kind` and `Kind` → `RootKind`.
  - `crates/web/src/api/library/mod.rs:176-183`: `impl From<Kind> for MediaKind` already exists.

After: add `From<MediaKind> for Kind`, `From<Kind> for RootKind` and `From<RootKind> for Kind`
beside the existing impl, then delete the three free fns. Call sites become `kind.into()` or
`RootKind::from(item.kind)`.

**Why it's better**: there is one conversion per direction, and the crate already uses this pattern
in `library/mod.rs`.

## 3. Findings by area

### A: api and server

A1, A2 and A3 are merged into W-X5 and W-X4 above.

#### W-A4: `AppState`'s 17 `Provides<T>` impls become one table

- **Category**: duplication.
- **Value**: high, about −80 lines.
- **Risk**: low. Mechanical, but not yet build-checked; `$ty:ty` accepts `dyn Clock`.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/state.rs:75-175`.

Before, repeated 17 times:

```rust
impl Provides<Library> for AppState {
    fn provide(&self) -> Arc<Library> {
        self.library.clone()
    }
}
```

After:

```rust
macro_rules! provides {
    ($($ty:ty => $field:ident),* $(,)?) => {
        $(impl Provides<$ty> for AppState {
            fn provide(&self) -> Arc<$ty> { self.$field.clone() }
        })*
    };
}
provides! {
    Library => library, Artworks => artworks, Calendar => calendar, Prober => prober,
    History => history, dyn Clock => clock, Downloads => downloads, Reviewer => reviewer,
    Importer => importer, QueueChanges => queue_changes, MetadataService => metadata,
    RootFolders => roots, Deleter => deleter, Renamer => renamer, Scanner => scanner,
    dyn SettingsAccess => settings, AddSettings => add,
}
```

**Why it's better**: the field-to-type pairing is stated once. The trade-off is that
go-to-definition lands on the macro rather than on a per-type impl.

### B: vendored components

B3 is W-X2 above.

#### W-B2: `Label` and `Badge` don't merge a caller's `class`

- **Category**: correctness.
- **Value**: medium.
- **Risk**: low.
- **Verdict**: plausible. Check in the browser that the `review-all` label loses `font-medium` before fixing it.
- **Locations**:
  - `crates/web/src/components/label.rs:5-14`: sets `class` as its own prop, next to `attributes: props.attributes`.
  - `crates/web/src/components/badge.rs:47-58`: a literal `class` followed by `..props.attributes`.
  - A live caller that passes `class`: `crates/web/src/dialogs/import_review/mod.rs:137`.

Every other wrapper merges through `attributes!` and `merge_attributes`. The rsx macro only merges
static attributes, so a spread `class` is not concatenated.

After (`label.rs`):

```rust
let base = attributes!(label { class: "text-caption font-medium" });
let merged = merge_attributes(vec![base, props.attributes]);
rsx! { label::Label { html_for: props.html_for, attributes: merged, {props.children} } }
```

**Why it's better**: this is the same pattern as the other 28 wrappers, and a caller's `class` then
adds to the component's own classes instead of replacing them.

#### W-B4: Tooltip and hover card duplicate the popover position classes

- **Category**: duplication.
- **Value**: high, about −11 lines.
- **Risk**: none. Tailwind still sees the literal in one file.
- **Verdict**: confirmed. The block is byte-identical.
- **Locations**: `crates/web/src/components/tooltip.rs:41-52`, `crates/web/src/components/hover_card.rs:43-53`.

After: `pub(super) const POPOVER_POSITION: &str = "data-[state=open]:animate-popover-in … data-[side=right]:data-[align=end]:bottom-0";`,
used through `class: "… {POPOVER_POSITION}"` in both components.

#### W-B5: Vendored components that only the gallery uses

- **Category**: dead-code.
- **Value**: medium. Informational; no deletion is proposed.
- **Verdict**: confirmed, after correction. The reviewer listed `alert_dialog` as unused, but
  `delete_file.rs` and `remove_item.rs` use it.

These modules have no caller in the app, only their gallery stories:
- accordion, avatar, breadcrumb, dropdown_menu, hover_card, kbd, pagination, radio_group
- spinner, tabs, textarea, toggle (the `Toggle` component; `MonitorToggle` is separate)
- sidebar, and through it sheet, separator and tooltip

Partly unused: `select.rs` `SelectGroupLabel` and `table.rs:90` `TableSortHead`.

The gallery is the design-system catalogue, so this is a product call, not cleanup. Note that no
busy button renders a `Spinner`: `aria_busy` is set with no visual cue.

#### W-B6: `sidebar.rs` has two copies of its window-listener setup

- **Category**: duplication.
- **Value**: low. The sidebar is kept by decision.
- **Verdict**: confirmed.
- **Locations**: `crates/web/src/components/sidebar.rs:129-165`, `:187-218`.

Both copies use `use_effect` + `spawn` + `document::eval`, with a `use_drop` for cleanup. Unify them
only if the sidebar is ever adopted.

#### W-B7: Unneeded `r#"…"#` hashes

- **Category**: pattern (pedantic).
- **Value**: low, 0 lines.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/components/sidebar.rs:135,157,190,213,221`.

#### W-B8: `&self` → `self` on small `Copy` enums

- **Category**: ownership (pedantic).
- **Value**: low, 0 lines.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/components/sidebar.rs:592,618,817`.

`button.rs`, `badge.rs` and `alert.rs` already take `self`.

#### W-B9: The same change for the `as_str` methods

- **Category**: ownership.
- **Value**: low, 0 lines.
- **Locations**: `crates/web/src/components/sidebar.rs:28,44,61,79,585,610,810`, `crates/web/src/components/sheet.rs:20-21`.

### C: app components and layout

C1 is W-X3 above.

#### W-C2: The busy-guarded `on_open_change` is duplicated

- **Category**: duplication.
- **Value**: low, 0 to −4 lines.
- **Verdict**: confirmed.
- **Locations**: `crates/web/src/components/delete_file.rs:31-37`, `crates/web/src/components/remove_item.rs:40-46`.

```rust
on_open_change: move |next| { if !busy() { open.set(next); } },
```

Two sites don't justify a helper. Leave it unless a third appears.

#### W-C3: `by_day` accumulates in a loop

- **Category**: iterator.
- **Value**: low, ±0 lines.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/components/history_list.rs:96-105`.

A `fold` holds the same match. This is marginal, and the loop is already clear.

### D: pages

D3 is W-X1 above.

#### W-D1: A shared `LoadFailed` alert

- **Category**: duplication.
- **Value**: high, about −20 lines.
- **Risk**: none.
- **Verdict**: confirmed. There are exactly 6 copies.
- **Locations**:
  - `crates/web/src/pages/downloads/mod.rs:41-44`
  - `crates/web/src/pages/missing/mod.rs:31-35`
  - `crates/web/src/pages/library/mod.rs:82-86`
  - `crates/web/src/pages/movie_detail/mod.rs:45-49`
  - `crates/web/src/pages/series_detail/mod.rs:49-53`
  - `crates/web/src/pages/upcoming/mod.rs:35-40` (with `mt-6`)

Before:

```rust
Some(Err(_)) => rsx! {
    Alert { variant: AlertVariant::Danger,
        AlertTitle { "The library could not be loaded" }
        AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
    }
},
```

After:

```rust
#[component]
pub fn LoadFailed(subject: &'static str, #[props(default)] class: &'static str) -> Element {
    rsx! {
        Alert { class, variant: AlertVariant::Danger,
            AlertTitle { "{subject} could not be loaded" }
            AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
        }
    }
}
Some(Err(_)) => rsx! { LoadFailed { subject: "The library" } },
```

`settings/mod.rs:252-257` shows the server's own message and stays separate.

#### W-D2: Detail pages reload with `Resource::restart()`

- **Category**: dioxus-reactivity.
- **Value**: high, about −28 lines.
- **Risk**: low, but this is a **behavior change** and needs a decision.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/pages/movie_detail/mod.rs:1, 68-84`
  - `crates/web/src/pages/series_detail/mod.rs:5, 70-86`

Before:

```rust
let mut current = use_signal(|| movie);
let mut reading = use_signal(|| None::<Task>);
let reload = use_callback(move |()| {
    if let Some(task) = reading.take() { task.cancel(); }
    reading.set(Some(spawn(async move {
        match detail::movie(id).await {
            Ok(Some(movie)) => current.set(movie),
            Ok(None) => warn!("the movie is gone from the library"),
            Err(error) => warn!(%error, "reading the movie again failed"),
        }
    })));
});
```

After: pass the page's `Resource` down, and set `let reload = use_callback(move |()| movie.restart());`.
`restart()` cancels the running future and keeps the old value until the new one resolves.
`settings/roots.rs:22,34,85` already works this way.

Behavior change: today a failed reload, or an item that has vanished, only logs a warning and keeps
the stale page. With `restart()` the page switches to the error alert or to "not found". This makes
a separate behavioral commit.

#### W-D4: `Torrent` clones `download.item` to read it

- **Category**: ownership.
- **Value**: medium, −1 clone.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/pages/downloads/torrents.rs:58, 64-70`.

Use `if let Some(item) = &download.item`.

#### W-D5: `latest.read().clone()`

- **Category**: dioxus-reactivity.
- **Value**: low.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/pages/downloads/mod.rs:24`.

Call it as `latest()`, like every other signal in the file.

#### W-D6: Add's `OptionsForm` re-collects static data on every render

- **Category**: ownership.
- **Value**: medium: −1 clone and 2 allocations per render.
- **Risk**: none. `OptionFields` is private and used once.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/pages/add/dialog.rs:75-79`: `hit.clone()` where `hit` is not used afterwards.
  - `crates/web/src/pages/add/dialog.rs:168-172`: `presets`, already `&'static [..]`, is re-collected into a `Vec`, and `roots` into `Vec<String>`.

Have `OptionFields` take `&'static [(MonitorPreset, &str, &str)]` and `Vec<RootChoice>` directly.

#### W-D7: `Action`'s `label: String` forces `.to_owned()` in every arm

- **Category**: dioxus-props.
- **Value**: low, −4 `.to_owned()` calls.
- **Verdict**: confirmed. `DownloadState::label()` returns `&'static str` (`api/downloads.rs:69`).
- **Location**: `crates/web/src/pages/downloads/torrents.rs:104-149`.

Use `#[props(into)] label: String`, and let the match arms yield `&'static str`. The two `format!`
arms need `.into()` or a `Cow`. Leave this as is if that ends up longer.

#### W-D8: `fn(&RootChoice) -> &Vec<String>`

- **Category**: types.
- **Value**: low.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/pages/add/dialog.rs:119-122`.

Use `&[String]`.

#### W-D9: `.map(f).unwrap_or_else(g)`

- **Category**: combinator (pedantic).
- **Value**: low.
- **Verdict**: confirmed.
- **Locations**: `crates/web/src/pages/library/table.rs:49`, `crates/web/src/pages/movie_detail/mod.rs:147`.

Use `map_or_else(g, f)`.

#### W-D10: A redundant closure

- **Category**: combinator (pedantic).
- **Value**: low.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/pages/settings/fields.rs:78`.

Replace `.map(|item| item.into())` with `.map(Into::into)`.

#### W-D11: `Results` clones the query to display it

- **Category**: ownership.
- **Value**: low.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/pages/add/mod.rs:108`.

Use `let searched = &query.0;`. Separately, the page shows the untrimmed text while the search uses
the trimmed text.

### E: dialogs and gallery

#### W-E1: One closable dialog shell

- **Category**: duplication.
- **Value**: high, about −25 lines.
- **Risk**: low.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/dialogs/add_torrent.rs:50-60`
  - `crates/web/src/dialogs/rename.rs:28-38`
  - `crates/web/src/dialogs/import_review/mod.rs:43-53`

Before, identical in all three except the title:

```rust
Dialog { open: Some(open()), on_open_change: move |next| open.set(next),
    div { class: "flex items-start justify-between gap-4",
        DialogTitle { "Add torrent" }
        Button { variant: ButtonVariant::Quiet, size: ButtonSize::Icon, aria_label: "Close",
            onclick: move |_| open.set(false), X {} }
    }
    …
```

After, in `dialogs/mod.rs`:

```rust
#[component]
pub(super) fn ClosableDialog(title: &'static str, open: Signal<bool>, children: Element) -> Element {
    rsx! {
        Dialog { open: Some(open()), on_open_change: move |next| open.set(next),
            div { class: "flex items-start justify-between gap-4",
                DialogTitle { "{title}" }
                Button { variant: ButtonVariant::Quiet, size: ButtonSize::Icon, aria_label: "Close",
                    onclick: move |_| open.set(false), X {} }
            }
            {children}
        }
    }
}
```

The reviewer proposed a header-only component that closes through a callback. Taking the
`Signal<bool>` directly, and wrapping the `Dialog` as well, removes more lines.

#### W-E3: The gallery's `Catalog::label` re-implements `title_with_year`

- **Category**: duplication.
- **Value**: medium, −3 lines.
- **Risk**: none. This is demo code, and it picks up the real "year already in the title" rule.
- **Verdict**: confirmed. The year is `Option<i16>` in both.
- **Location**: `crates/web/gallery/patterns/import.rs:123-128`.

#### W-E4: A whole `SeriesDetail` is cloned to read one season

- **Category**: ownership.
- **Value**: medium.
- **Verdict**: confirmed.
- **Locations**:
  - `crates/web/src/dialogs/import_review/bulk.rs:164-170`
  - `crates/web/src/dialogs/import_review/mod.rs:388-406`

Before: `.read().clone().flatten().and_then(|d| d.seasons.into_iter().find(…))`.

After: `match &*detail.read() { Some(Some(detail)) => detail.seasons.iter().find(…).map(…clone titles…).unwrap_or_default(), _ => Vec::new() }`.
This is the same shape as `mod.rs:383-386`, next to it.

#### W-E5: `rows.clone()` and `files.clone()` at their last use

- **Category**: ownership.
- **Value**: low, −2 clones.
- **Verdict**: plausible, needs a compile.
- **Location**: `crates/web/src/dialogs/import_review/bulk.rs:79,82`.

#### W-E6: `Form { item: item.clone() }` is the only use of `item`

- **Category**: ownership.
- **Value**: low.
- **Verdict**: plausible, needs a compile.
- **Location**: `crates/web/src/dialogs/add_torrent.rs:62`.

#### W-E7: `Vec<&ReviewFile>` is collected only to be counted

- **Category**: iterator.
- **Value**: low, −1 allocation.
- **Verdict**: confirmed.
- **Location**: `crates/web/src/dialogs/import_review/mod.rs:106-109`.

Count straight from the iterator instead.

#### W-E8: `episode_options = episodes.clone()` just before its only use

- **Category**: ownership.
- **Value**: low.
- **Verdict**: confirmed. The reviewer cited the wrong file.
- **Location**: `crates/web/src/dialogs/import_review/bulk.rs:209,216,225`.

#### W-E9: `map_or("—".to_owned(), …)` builds its default eagerly

- **Category**: combinator.
- **Value**: low.
- **Verdict**: confirmed.
- **Location**: `crates/web/gallery/patterns/import.rs:442,444`.

#### W-E10: `episode_title` can underflow

- **Category**: correctness.
- **Value**: low. It is demo code.
- **Verdict**: confirmed.
- **Location**: `crates/web/gallery/patterns/import.rs:134-138`.

Use `usize::from(season) - 1`. Its sibling methods use `checked_sub(1)?`.

#### W-E2: `StoryPage` and `Story` take `String`

- **Category**: ownership.
- **Value**: low, 0 lines. The reviewer rated it high.
- **Verdict**: confirmed.
- **Location**: `crates/web/gallery/main.rs:223,238`.

Every caller passes a literal, so `&'static str` fits.

#### W-E11: A missing `;` after `set(…)` in two closures

- **Category**: pattern (pedantic).
- **Value**: low.
- **Locations**: `crates/web/src/dialogs/import_review/mod.rs:133-135`, `crates/web/src/dialogs/rename.rs:143-145`.

#### W-E12: A bare URL in a doc comment

- **Category**: pattern (pedantic).
- **Value**: low.
- **Location**: `crates/web/gallery/main.rs:2`.

## 4. Rejected candidates

- **Pedantic `must_use` (about 100) and `# Errors` (about 65)**: API-doc lints for a crate with no external users.
- **Unused `async`**:
  - `crates/web/src/api/downloads.rs:96` and `crates/web/gallery/patterns/backend.rs:34,39,45,65,72,76,84,89`
  - These are `#[get]`/`#[post]` server functions, which must be `async`.
- **`similar_names`**: `crates/web/src/api/downloads.rs:160,176` and `crates/web/src/api/rename.rs:62,64`. Fixing them only renames.
- **Precision-loss cast**: `crates/web/src/format.rs:50`. The value is rounded to one decimal for display.
- **Casts that could truncate**: `gallery/patterns/import.rs:487,516` and `gallery/stories/mod.rs:286,377`. They cover demo data with a handful of items.
- **Wildcard import**: `gallery/main.rs:12` (`stories::*`). The route table already names each story.
- **`Option<Option<ItemId>>`**: `crates/web/src/dialogs/add_torrent.rs:203,232`. `Select<T>` takes `Option<ReadSignal<Option<T>>>` with `T = Option<ItemId>`.
- **`attributes!` + `merge_attributes` boilerplate**: this is the upstream vendoring shape. A wrapper would only rename it.
- **`as_str`/`class` methods → `Display`**: they return `&'static str` without allocating, and `Display` would add `to_string()` on every render.
- **`.to_string()` on vendored content `class` props**: the upstream prop is `Option<String>` without `into`.
- **`avatar.rs` `Option<EventHandler>` fields**: pure pass-through to upstream props with the same type.
- **`select.rs` `Option<ReadSignal<Option<T>>>`**: it mirrors the upstream prop.
- **`sidebar.rs` `merged.clone()` in the tooltip closure**: required, because the closure runs on more than one render.
- **`sheet.rs:45-49` `data-side` default**: callers override it through attributes, and that works.
- **Deleting `sidebar.rs`**: you chose to keep it.
- **`combobox.rs:33-34` `use_memo(move || Some(x()))`**: these adapt a signal to the `ReadSignal<Option<T>>` prop. The same holds for `pickers.rs:21,75` and `import_review/mod.rs:387`.
- **`monitor_toggle.rs:29-32` `_ = monitored`**: this is the crate's idiom for "depend on this without reading it".
- **`nav_badges.rs:40-55` two counts**: two named filters read better than a `fold`.
- **`item_hero.rs:18,40` double `is_some()`**: trivial.
- **`file_info.rs:34-39` filter then map**: `filter_map` + `then` is no shorter.
- **`NavItem` calling `use_route` itself**: cheap, and it avoids a prop.
- **`history_list.rs:44-53` holding a `Ref`**: there is no `.await` in between.
- **`busy()` read twice**: `Signal` is `Copy`, so there is nothing to clone.
- **`upcoming/agenda.rs:14-20` grouping loop**: `chunk_by` works on slices and would need clones.
- **`settings/fields.rs` `Unsaved` effect**: it writes a shared context that sibling fields add to, which a memo can't do.
- **`numbering.rs:25` and `add/mod.rs:28` `use_reactive` resync**: this is a controlled value with a local override, not derived state.
- **`add/dialog.rs` `folder_hint` branches**: each state has its own sentence.
- **Stories passing `.to_string()` to `RadioItem`/`Tabs`**: the upstream props are `String`.

## 5. Suggested batches

Each batch is one commit. Structural and behavioral changes stay apart, and API-risk items get
their own batches.

| # | Batch | Findings | Kind | Risk |
|---|---|---|---|---|
| 1 | Server wiring table | W-A4 | refactor | none |
| 2 | Server error mapping and kind conversions | W-X4, W-X5 | refactor | none |
| 3 | Event handler props | W-X2 | refactor | api (props; call sites unchanged) |
| 4 | Popover position constant | W-B4 | refactor | none |
| 5 | Shared dialog shell | W-E1 | refactor | none |
| 6 | Shared load-failure alert | W-D1 | refactor | none |
| 7 | Plural helper | W-X1 | refactor | none |
| 8 | Live downloads gating | W-X3 | refactor | low (both builds) |
| 9 | Ownership and combinator cleanups in app code | W-D4, D5, D6, D7, D8, D9, D10, D11, E4, E5, E6, E7, E8, E11 | refactor | none |
| 10 | Gallery cleanups | W-E2, E3, E9, E10, E12 | refactor | none |
| 11 | Sidebar and sheet tidy-ups (the sidebar is kept) | W-B7, B8, B9 | refactor | none |
| 12 | `Label`/`Badge` class merge | W-B2 | fix | low (browser check first) |
| 13 | Detail pages reload through `restart()` | W-D2 | refactor with a behavior change | low (needs your yes) |
| — | Informational only | W-B5, W-B6, W-C2, W-C3 | — | — |
