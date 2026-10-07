use std::cmp::Ordering;

use dioxus::prelude::*;
use frizbee::{Config, Matcher};
use jiff::civil::Date;
use yokoku_domain::{ItemId, SeriesId, TrackerId, TrackerSet, Trackers};

use crate::{
    api::{
        add::{SearchHit, add_options, search as metadata_search},
        downloads::ItemLink,
        failure,
        library::Kind,
        releases::{Found, ReleaseEntry, TrackerEntry, grab_release, release_trackers, search_releases},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        field::{Field, FieldError, FieldHint},
        input::Input,
        label::Label,
        select::{Select, SelectMulti, SelectOption},
        skeleton::Skeleton,
        spinner::Spinner,
        table::{SortDirection, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableSortHead},
    },
    dialogs::{ClosableDialog, add_torrent::ItemField, pickers::SeasonPicker},
    format::{plural, relative, size},
    pages::add::dialog::AddDialog,
    route::Route,
};

/// A "Search releases" button that opens [`SearchReleases`] for `item`.
#[component]
pub fn SearchReleasesButton(item: ItemLink) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Button { onclick: move |_| open.set(true), "Search releases" }
        ClosableDialog { title: "Search releases", open, wide: true,
            if open() {
                SearchReleases { item: Some(item), on_close: Some(Callback::new(move |()| open.set(false))) }
            }
        }
    }
}

/// The inputs that produced a displayed search result.
#[derive(Clone, PartialEq)]
struct SearchQuery {
    text: String,
    kind: Option<Kind>,
    season: Option<u16>,
    trackers: Trackers,
}

/// Searches configured indexers for releases of `item`, its title to start with; downloading one adds it
/// for `item` and opens the Downloads page.
#[component]
pub(crate) fn SearchReleases(item: Option<ItemLink>, on_close: Option<Callback>) -> Element {
    let mut text = use_signal(|| item.as_ref().map_or(String::new(), |item| item.title.clone()));
    let mut kind = use_signal(|| None::<Kind>);
    let mut season = use_signal(|| None::<u16>);
    let mut searching = use_signal(|| None::<SearchQuery>);
    let mut request = use_signal(|| 0u64);
    let mut found = use_signal(|| None::<(SearchQuery, Result<Found, String>)>);
    let listed = use_resource(release_trackers);
    let picked = use_signal(|| None::<Vec<TrackerId>>);
    let trackers = use_memo(move || {
        let listed = listed.read();
        let every = listed.as_ref().and_then(|listed| listed.as_ref().ok()).map_or(0, Vec::len);
        match picked() {
            Some(ids) if ids.len() < every => TrackerSet::new(ids).map(Trackers::Only),
            _ => Some(Trackers::All),
        }
    });
    let id = item.as_ref().map(|item| item.id);
    let search = move |event: FormEvent| async move {
        event.prevent_default();
        let Some(trackers) = trackers() else { return };
        let query = SearchQuery { text: text(), kind: kind(), season: season(), trackers };
        let generation = request() + 1;
        request.set(generation);
        searching.set(Some(query.clone()));
        found.set(None);
        let result = search_releases(query.text.clone(), id, query.season, query.trackers.clone(), query.kind).await;
        if request() == generation {
            found.set(Some((query, result.map_err(|error| failure(&error)))));
            searching.set(None);
        }
    };
    let current = trackers().map(|trackers| SearchQuery { text: text(), kind: kind(), season: season(), trackers });
    let busy = current.as_ref().is_some_and(|query| searching().as_ref() == Some(query));
    let visible = found().filter(|(query, _)| current.as_ref() == Some(query));
    rsx! {
        form { role: "search", class: "flex flex-col gap-4", onsubmit: search,
            Field {
                Label { html_for: "release-text", "Search for" }
                div { class: "flex gap-2",
                    Input {
                        id: "release-text",
                        r#type: "search",
                        class: "min-w-0 flex-1",
                        value: "{text}",
                        oninput: move |event: FormEvent| text.set(event.value()),
                    }
                    Button {
                        r#type: "submit",
                        variant: ButtonVariant::Primary,
                        disabled: busy || trackers().is_none(),
                        aria_busy: busy,
                        "Search"
                    }
                }
            }
            if item.is_none() {
                div { class: "flex flex-wrap gap-2", role: "group", aria_label: "Media type",
                    for (label, value) in [("All", None), ("Movies", Some(Kind::Movie)), ("Series", Some(Kind::Series))] {
                        Button { key: "{label}", r#type: "button", aria_pressed: kind() == value,
                            onclick: move |_| { kind.set(value); season.set(None); }, "{label}" }
                    }
                }
            }
            if let Some(series) = id.and_then(ItemId::series) {
                SeasonField { series, season }
            } else if item.is_none() && kind() == Some(Kind::Series) {
                Field {
                    Label { html_for: "release-season-number", "Season (optional)" }
                    Input { id: "release-season-number", r#type: "number", min: "0", max: "9999",
                        value: "{season().map(|value| value.to_string()).unwrap_or_default()}",
                        oninput: move |event: FormEvent| season.set(event.value().parse().ok()) }
                }
            }
            if let Some(Ok(trackers)) = &*listed.read()
                && trackers.len() > 1
            {
                TrackerField { trackers: trackers.clone(), picked }
            }
            if let Some(Err(error)) = &*listed.read() {
                p { role: "alert", class: "text-warning", {failure(error)} }
            }
        }
        div { aria_live: "polite",
            match visible {
                None if busy => rsx! {
                    Searching { every: trackers() == Some(Trackers::All) }
                },
                None => rsx! {},
                Some((_, Err(message))) => rsx! {
                    p { role: "alert", class: "text-danger", "{message}" }
                },
                Some((query, Ok(found))) => rsx! {
                    Releases { found, item: id, season: query.season, on_close }
                },
            }
        }
    }
}

/// Seconds after which the wait for slow indexers is explained.
const SLOW_SEARCH: u32 = 10;

/// Which of `trackers` to search; `picked` stays `None`, every source, until changed.
#[component]
fn TrackerField(trackers: Vec<TrackerEntry>, mut picked: Signal<Option<Vec<TrackerId>>>) -> Element {
    let every: Vec<TrackerId> = trackers.iter().map(|tracker| tracker.id.clone()).collect();
    let values = use_memo(move || Some(picked().unwrap_or_else(|| every.clone())));
    let none = values().is_some_and(|values| values.is_empty());
    rsx! {
        Field {
            Label { html_for: "release-trackers", "Indexers" }
            SelectMulti::<TrackerId> {
                id: "release-trackers",
                aria_describedby: "release-trackers-hint",
                values,
                placeholder: "No indexer",
                on_values_change: move |next: Vec<TrackerId>| picked.set(Some(next)),
                for (index, tracker) in trackers.into_iter().enumerate() {
                    SelectOption::<TrackerId> {
                        key: "{tracker.id}",
                        index,
                        value: tracker.id,
                        text_value: tracker.name.clone(),
                        "{tracker.name}"
                    }
                }
            }
            if none {
                FieldError { id: "release-trackers-hint", "Pick an indexer to search." }
            } else {
                FieldHint { id: "release-trackers-hint",
                    "Leave a slow source out to get the others' results sooner."
                }
            }
        }
    }
}

/// How long the search has taken so far, over placeholder rows shaped like the results; `every`
/// when it searches every indexer.
#[component]
fn Searching(every: bool) -> Element {
    let seconds = use_signal(|| 0u32);
    #[cfg(target_arch = "wasm32")]
    use_future(move || {
        let mut seconds = seconds;
        async move {
            loop {
                gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
                seconds += 1;
            }
        }
    });
    rsx! {
        p { class: "flex items-center gap-2 text-muted",
            Spinner { label: "Searching" }
            if every {
                "Searching every indexer…"
            } else {
                "Searching selected indexers…"
            }
            if seconds() > 0 {
                span { class: "tabular-nums", "{seconds} s" }
            }
        }
        if seconds() >= SLOW_SEARCH {
            p { class: "text-caption text-muted", "Slow indexers may take up to a minute." }
        }
        div { aria_hidden: "true", class: "divide-y divide-line border-y border-line",
            for index in 0..6 {
                div { key: "{index}", class: "flex items-center gap-4 py-3",
                    div { class: "flex flex-1 flex-col gap-2",
                        Skeleton { class: "h-4 w-full max-w-xl" }
                        Skeleton { class: "h-3 w-40 sm:hidden" }
                        Skeleton { class: "h-8 w-24 sm:hidden" }
                    }
                    Skeleton { class: "hidden h-4 w-16 sm:block" }
                    Skeleton { class: "hidden h-4 w-10 sm:block" }
                    Skeleton { class: "hidden h-4 w-20 sm:block" }
                    Skeleton { class: "hidden h-8 w-24 sm:block" }
                }
            }
        }
    }
}

#[component]
fn SeasonField(series: SeriesId, season: Signal<Option<u16>>, #[props(default)] initial: Option<u16>) -> Element {
    rsx! {
        Field {
            SeasonPicker {
                id: "release-season",
                series,
                season,
                none: Some("Any season"),
                initial,
                aria_describedby: Some("release-season-hint"),
            }
            FieldHint { id: "release-season-hint",
                "Narrows the search. When linked to a series, this season places files named without one there."
            }
        }
    }
}

/// A column the releases sort by.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Column {
    Title,
    Size,
    Seeders,
    Leechers,
    Downloads,
    Age,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Sort {
    column: Column,
    direction: SortDirection,
}

impl Default for Sort {
    fn default() -> Self {
        Self { column: Column::Seeders, direction: SortDirection::Descending }
    }
}

impl Sort {
    /// Turns the direction of `column` around, or sorts by it: titles A to Z, numbers largest
    /// first, ages newest first. Ages sort by date.
    fn press(&mut self, column: Column) {
        self.direction = match (self.column == column, self.direction, column) {
            (true, SortDirection::Ascending, _) => SortDirection::Descending,
            (true, SortDirection::Descending, _) | (false, _, Column::Title) => SortDirection::Ascending,
            (false, _, _) => SortDirection::Descending,
        };
        self.column = column;
    }

    fn of(self, column: Column) -> Option<SortDirection> {
        (self.column == column).then_some(self.direction)
    }

    /// The orders the phone picker offers, where the column headers are hidden.
    const PICKS: [Self; 8] = [
        Self::new(Column::Seeders, SortDirection::Descending),
        Self::new(Column::Seeders, SortDirection::Ascending),
        Self::new(Column::Downloads, SortDirection::Descending),
        Self::new(Column::Age, SortDirection::Descending),
        Self::new(Column::Age, SortDirection::Ascending),
        Self::new(Column::Size, SortDirection::Descending),
        Self::new(Column::Size, SortDirection::Ascending),
        Self::new(Column::Title, SortDirection::Ascending),
    ];

    const fn new(column: Column, direction: SortDirection) -> Self {
        Self { column, direction }
    }

    fn label(self) -> String {
        let label = match (self.column, self.direction) {
            (Column::Title, SortDirection::Ascending) => "Title, A to Z",
            (Column::Title, SortDirection::Descending) => "Title, Z to A",
            (Column::Size, SortDirection::Ascending) => "Smallest",
            (Column::Size, SortDirection::Descending) => "Largest",
            (Column::Seeders, SortDirection::Ascending) => "Fewest seeders",
            (Column::Seeders, SortDirection::Descending) => "Most seeders",
            (Column::Leechers, SortDirection::Ascending) => "Fewest leechers",
            (Column::Leechers, SortDirection::Descending) => "Most leechers",
            (Column::Downloads, SortDirection::Ascending) => "Least downloaded",
            (Column::Downloads, SortDirection::Descending) => "Most downloaded",
            (Column::Age, SortDirection::Ascending) => "Oldest",
            (Column::Age, SortDirection::Descending) => "Newest",
        };
        label.to_owned()
    }
}

/// Examples of the filter syntax, with what each keeps.
const FILTER_SYNTAX: [(&str, &str); 6] = [
    ("bb 1080", "Letters in this order, anything between them: Breaking Bad … 1080p"),
    ("'web-dl", "Contains exactly web-dl"),
    ("^во", "Starts with во"),
    ("hdtv$", "Ends with hdtv"),
    ("!hevc", "Leaves out titles containing hevc"),
    ("'1080p !hevc 'lostfilm", "Words are separated by spaces; every one must match"),
];

/// How the filter reads what is typed, opened on demand.
#[component]
fn FilterHelp() -> Element {
    rsx! {
        details { id: "release-filter-help", class: "text-caption",
            summary { class: "w-fit cursor-pointer text-muted hover:text-ink", "How to filter" }
            dl { class: "mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1",
                for (example, meaning) in FILTER_SYNTAX {
                    div { key: "{example}", class: "contents",
                        dt {
                            code { class: "yk-code", "{example}" }
                        }
                        dd { class: "text-muted", "{meaning}" }
                    }
                }
            }
            p { class: "mt-2 text-muted",
                "A plain word matches loosely: 1080p also finds 2008 … 720p. Start it with ' to match it exactly. Lowercase words ignore case; a word with a capital letter matches case."
            }
        }
    }
}

/// The releases whose titles match `filter`, in fzf syntax like `1080p !hevc`, ordered by `sort`
/// with unknown values last; ties keep their order.
fn shown(releases: &[ReleaseEntry], filter: &str, sort: Sort) -> Vec<ReleaseEntry> {
    let mut shown: Vec<&ReleaseEntry> = if filter.trim().is_empty() {
        releases.iter().collect()
    } else {
        let titles: Vec<&str> = releases.iter().map(|release| release.title.as_str()).collect();
        let mut matched: Vec<usize> = Matcher::from_query(filter, &Config::default())
            .match_list(&titles)
            .into_iter()
            .map(|found| found.index as usize)
            .collect();
        matched.sort_unstable();
        matched.into_iter().map(|index| &releases[index]).collect()
    };
    shown.sort_by(|a, b| match sort.column {
        Column::Title => ordered(Some(a.title.to_lowercase()), Some(b.title.to_lowercase()), sort.direction),
        Column::Size => ordered(Some(a.size), Some(b.size), sort.direction),
        Column::Seeders => ordered(a.seeders, b.seeders, sort.direction),
        Column::Leechers => ordered(a.leechers, b.leechers, sort.direction),
        Column::Downloads => ordered(a.grabs, b.grabs, sort.direction),
        Column::Age => ordered(a.published, b.published, sort.direction),
    });
    shown.into_iter().cloned().collect()
}

/// `a` against `b` in `direction`, a missing value after any other.
fn ordered<T: Ord>(a: Option<T>, b: Option<T>, direction: SortDirection) -> Ordering {
    match (a, b, direction) {
        (Some(a), Some(b), SortDirection::Ascending) => a.cmp(&b),
        (Some(a), Some(b), SortDirection::Descending) => b.cmp(&a),
        (a, b, _) => b.is_some().cmp(&a.is_some()),
    }
}

#[component]
fn Releases(found: Found, item: Option<ItemId>, season: Option<u16>, on_close: Option<Callback>) -> Element {
    let error = use_signal(|| None::<String>);
    let busy = use_signal(|| None::<String>);
    let mut filter = use_signal(String::new);
    let mut sort = use_signal(Sort::default);
    if found.releases.is_empty() {
        return rsx! {
            for warning in &found.warnings {
                p { role: "alert", class: "text-warning", "{warning}" }
            }
            p { class: "text-muted", "No releases found. Try fewer words, or the original title." }
        };
    }
    let total = found.releases.len();
    let counted = found.releases.iter().any(|release| release.grabs.is_some());
    let releases = shown(&found.releases, &filter.read(), sort());
    let count = releases.len();
    let head = move |label: &'static str, column: Column, class: &'static str| {
        rsx! {
            TableSortHead {
                label,
                class,
                direction: sort().of(column),
                onclick: move |_| sort.write().press(column),
            }
        }
    };
    let picked = use_memo(move || Some(sort()));
    rsx! {
        for warning in &found.warnings {
            p { role: "alert", class: "text-warning", "{warning}" }
        }
        div { class: "flex flex-wrap items-start gap-2",
            p { class: "mr-auto text-caption text-muted max-sm:order-last sm:self-end",
                if count == total {
                    {plural(total, "release", "releases")}
                } else {
                    "{count} of {total} releases"
                }
            }
            div { class: "w-full sm:hidden",
                Select::<Sort> {
                    aria_label: "Sort by",
                    placeholder: sort().label(),
                    value: Some(picked.into()),
                    on_value_change: move |next: Option<Sort>| sort.set(next.unwrap_or_default()),
                    for (index, option) in Sort::PICKS
                        .into_iter()
                        .filter(|pick| counted || pick.column != Column::Downloads)
                        .enumerate()
                    {
                        SelectOption::<Sort> {
                            key: "{index}",
                            index,
                            value: option,
                            text_value: option.label(),
                            "{option.label()}"
                        }
                    }
                }
            }
            div { class: "flex w-full flex-col gap-1 sm:w-96",
                Input {
                    r#type: "search",
                    aria_label: "Filter releases",
                    aria_describedby: "release-filter-help",
                    placeholder: "Filter, like '1080p !hevc",
                    value: "{filter}",
                    oninput: move |event: FormEvent| filter.set(event.value()),
                }
                FilterHelp {}
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        Table {
            class: if counted { "table-fixed sm:min-w-[56rem]" } else { "table-fixed sm:min-w-[50rem]" },
            aria_label: "Releases",
            TableHeader { class: "max-sm:hidden",
                TableRow {
                    {head("Title", Column::Title, "")}
                    {head("Size", Column::Size, "hidden w-24 sm:table-cell")}
                    {head("Seeders", Column::Seeders, "hidden w-24 sm:table-cell")}
                    {head("Leechers", Column::Leechers, "hidden w-24 sm:table-cell")}
                    if counted {
                        {head("Downloads", Column::Downloads, "hidden w-28 sm:table-cell")}
                    }
                    {head("Age", Column::Age, "hidden w-28 sm:table-cell")}
                    TableHead { class: "hidden w-32 sm:table-cell", "Tracker" }
                    TableHead { class: "hidden w-28 sm:table-cell",
                        span { class: "sr-only", "Download" }
                    }
                }
            }
            TableBody {
                for release in releases {
                    Release {
                        key: "{release.link}",
                        release,
                        today: found.today,
                        counted,
                        item,
                        season,
                        busy,
                        error,
                        on_close,
                    }
                }
            }
        }
        if count == 0 {
            p { class: "text-muted", "Nothing matches the filter." }
        }
    }
}

/// One release, with what it is, how well seeded and how old; `busy` holds the link being
/// downloaded. `counted` shows its downloads, for results where a tracker counts them. Below `sm`
/// its download button sits under the facts instead of in its own column.
#[component]
fn Release(
    release: ReleaseEntry,
    today: Date,
    counted: bool,
    item: Option<ItemId>,
    season: Option<u16>,
    busy: Signal<Option<String>>,
    error: Signal<Option<String>>,
    on_close: Option<Callback>,
) -> Element {
    let button = rsx! {
        DownloadButton {
            link: release.link.clone(),
            item,
            season,
            busy,
            error,
            on_close,
        }
    };
    let age = release.published.map(|published| relative(published, today));
    let count = |count: Option<u32>| count.map_or_else(|| "—".to_owned(), |count| count.to_string());
    rsx! {
        TableRow { class: "[&>td]:align-top",
            TableCell {
                match &release.details {
                    Some(details) => rsx! {
                        a {
                            class: "font-medium [overflow-wrap:anywhere] hover:underline",
                            href: "{details}",
                            target: "_blank",
                            rel: "noreferrer",
                            "{release.title}"
                        }
                    },
                    None => rsx! {
                        span { class: "font-medium [overflow-wrap:anywhere]", "{release.title}" }
                    },
                }
                span { class: "mt-1 flex flex-wrap gap-x-3 text-caption text-muted sm:hidden",
                    span { class: "tabular-nums", "{size(release.size)}" }
                    if let Some(seeders) = release.seeders {
                        span { class: "tabular-nums", {plural(seeders as usize, "seeder", "seeders")} }
                    }
                    if let Some(grabs) = release.grabs {
                        span { class: "tabular-nums", {plural(grabs as usize, "download", "downloads")} }
                    }
                    if let Some(age) = &age {
                        span { "{age}" }
                    }
                    if !release.tracker.is_empty() {
                        span { "{release.tracker}" }
                    }
                }
                div { class: "mt-2 sm:hidden", {button.clone()} }
            }
            TableCell { class: "hidden tabular-nums whitespace-nowrap sm:table-cell", "{size(release.size)}" }
            TableCell { class: "hidden tabular-nums sm:table-cell", "{count(release.seeders)}" }
            TableCell { class: "hidden tabular-nums sm:table-cell", "{count(release.leechers)}" }
            if counted {
                TableCell { class: "hidden tabular-nums sm:table-cell", "{count(release.grabs)}" }
            }
            TableCell { class: "hidden whitespace-nowrap sm:table-cell", {age.unwrap_or_else(|| "—".to_owned())} }
            TableCell { class: "hidden truncate sm:table-cell", title: "{release.tracker}", "{release.tracker}" }
            TableCell { class: "hidden sm:table-cell", {button} }
        }
    }
}

/// Adds the release at `link` for `item`, then opens the Downloads page; `busy` holds the link
/// being downloaded, which disables every download button.
#[component]
fn DownloadButton(
    link: String,
    item: Option<ItemId>,
    season: Option<u16>,
    busy: Signal<Option<String>>,
    error: Signal<Option<String>>,
    on_close: Option<Callback>,
) -> Element {
    let mut open = use_signal(|| false);
    if item.is_none() {
        return rsx! {
            Button { size: ButtonSize::Sm, onclick: move |_| open.set(true), "Download" }
            ClosableDialog { title: "Download release", open,
                if open() {
                    GlobalGrab { link, searched_season: season, on_close: move |()| open.set(false) }
                }
            }
        };
    }
    let downloading = busy().as_ref() == Some(&link);
    let download = move |_| {
        let link = link.clone();
        async move {
            busy.set(Some(link.clone()));
            error.set(None);
            match grab_release(link, item, season).await {
                Ok(()) => {
                    if let Some(on_close) = on_close {
                        on_close(());
                    }
                    navigator().push(Route::Downloads {});
                },
                Err(failed) => {
                    error.set(Some(failure(&failed)));
                    busy.set(None);
                },
            }
        }
    };
    rsx! {
        Button {
            size: ButtonSize::Sm,
            disabled: busy().is_some(),
            aria_busy: downloading,
            onclick: download,
            "Download"
        }
    }
}

/// Pick a library item, add one through metadata, or let import detection decide after download.
#[component]
fn GlobalGrab(link: String, searched_season: Option<u16>, on_close: Callback) -> Element {
    let chosen = use_signal(|| Some(None::<ItemId>));
    let refresh = use_signal(|| 0u32);
    let season = use_signal(|| None::<u16>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let series = chosen().flatten().and_then(ItemId::series);
    let add = move |_| {
        let link = link.clone();
        async move {
            busy.set(true);
            error.set(None);
            match grab_release(link, chosen().flatten(), season().filter(|_| series.is_some())).await {
                Ok(()) => {
                    on_close(());
                    navigator().push(Route::Downloads {});
                },
                Err(failed) => {
                    error.set(Some(failure(&failed)));
                    busy.set(false);
                },
            }
        }
    };
    rsx! {
        div { class: "grid gap-4",
            ItemField { key: "{refresh}", chosen }
            AddLibraryItem { chosen, refresh }
            if let Some(series) = series {
                SeasonField { key: "{series}", series, season, initial: searched_season }
            }
            crate::components::free_space::FreeSpace { item: chosen().flatten() }
            if let Some(message) = error() { p { role: "alert", class: "text-danger", "{message}" } }
            crate::components::dialog::DialogFooter {
                Button { onclick: move |_| on_close(()), "Cancel" }
                Button { variant: ButtonVariant::Primary, disabled: busy(), aria_busy: busy(), onclick: add, "Send to Transmission" }
            }
        }
    }
}

/// Search metadata and reuse the library's Add dialog without losing the chosen release.
#[component]
fn AddLibraryItem(chosen: Signal<Option<Option<ItemId>>>, refresh: Signal<u32>) -> Element {
    let mut show = use_signal(|| false);
    let mut kind = use_signal(|| Kind::Movie);
    let mut text = use_signal(String::new);
    let mut searching = use_signal(|| None::<(String, Kind)>);
    let mut request = use_signal(|| 0u64);
    let mut found = use_signal(|| None::<((String, Kind), Result<Vec<SearchHit>, String>)>);
    let picked = use_signal(|| None::<SearchHit>);
    let options = use_resource(add_options);
    let current = (text(), kind());
    let busy = searching() == Some(current.clone());
    let visible = found().filter(|(query, _)| query == &current);
    rsx! {
        if !show() {
            Button { variant: ButtonVariant::Quiet, class: "w-fit", onclick: move |_| show.set(true), "Add a movie or series to the library…" }
        } else {
            div { class: "grid gap-3 rounded border border-line p-3",
                div { class: "flex gap-2",
                    for (label, value) in [("Movies", Kind::Movie), ("Series", Kind::Series)] {
                        Button { key: "{label}", aria_pressed: kind() == value,
                            onclick: move |_| { kind.set(value); found.set(None); }, "{label}" }
                    }
                }
                form { class: "flex gap-2", onsubmit: move |event: FormEvent| async move {
                        event.prevent_default();
                        let query = (text(), kind());
                        let generation = request() + 1;
                        request.set(generation);
                        searching.set(Some(query.clone()));
                        found.set(None);
                        let result = metadata_search(query.0.clone(), query.1).await.map_err(|error| failure(&error));
                        if request() == generation {
                            found.set(Some((query, result)));
                            searching.set(None);
                        }
                    },
                    Input { class: "min-w-0 flex-1", r#type: "search", placeholder: "Movie or series title",
                        value: "{text}", oninput: move |event: FormEvent| text.set(event.value()) }
                    Button { r#type: "submit", disabled: busy || text.read().trim().is_empty(), aria_busy: busy, "Search" }
                }
                match visible {
                    Some((_, Ok(hits))) if hits.is_empty() => rsx! { p { class: "text-muted", "No matching media found." } },
                    Some((_, Ok(hits))) => rsx! { ul { class: "max-h-48 overflow-y-auto",
                        for hit in hits {
                            MetadataHit { key: "{hit.source}", hit, picked, chosen, refresh, show }
                        }
                    } },
                    Some((_, Err(message))) => rsx! { p { role: "alert", class: "text-danger", "{message}" } },
                    None if busy => rsx! { p { role: "status", class: "text-muted", "Searching…" } },
                    None => rsx! {},
                }
            }
        }
        AddDialog { picked, options,
            on_added: Some(Callback::new(move |id| {
                chosen.set(Some(Some(id)));
                refresh.set(refresh() + 1);
                found.set(None);
                show.set(false);
            })) }
    }
}

#[component]
fn MetadataHit(
    hit: SearchHit,
    mut picked: Signal<Option<SearchHit>>,
    mut chosen: Signal<Option<Option<ItemId>>>,
    mut refresh: Signal<u32>,
    mut show: Signal<bool>,
) -> Element {
    let title = hit.title.clone();
    let year = hit.year;
    let existing = hit.in_library;
    rsx! {
        li {
            Button { variant: ButtonVariant::Quiet, onclick: move |_| {
                    if let Some(id) = existing {
                        chosen.set(Some(Some(id)));
                        refresh.set(refresh() + 1);
                        show.set(false);
                    } else {
                        picked.set(Some(hit.clone()));
                    }
                },
                "{title}" if let Some(year) = year { " ({year})" }
                if existing.is_some() { " · In library" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn release(title: &str, size: u64, seeders: Option<u32>, published: Option<&str>) -> ReleaseEntry {
        ReleaseEntry {
            title: title.into(),
            tracker: "RuTor".into(),
            size,
            seeders,
            leechers: None,
            grabs: None,
            published: published.map(|date| date.parse().unwrap()),
            link: title.into(),
            details: None,
        }
    }

    fn releases() -> Vec<ReleaseEntry> {
        vec![
            ReleaseEntry { grabs: Some(900), ..release("Breaking Bad S01 720p HDTV", 9, Some(5), Some("2026-09-01")) },
            release("Breaking Bad S01-05 1080p BDRip HEVC", 50, None, Some("2026-10-01")),
            ReleaseEntry {
                grabs: Some(120),
                ..release("Во все тяжкие / Breaking Bad S05 1080p WEB-DL", 30, Some(40), None)
            },
        ]
    }

    fn titles(shown: &[ReleaseEntry]) -> Vec<&str> {
        shown.iter().map(|release| release.title.as_str()).collect()
    }

    #[rstest]
    #[case::seeders_most_first_unknown_last(Column::Seeders, SortDirection::Descending, [2, 0, 1])]
    #[case::seeders_fewest_first_unknown_last(Column::Seeders, SortDirection::Ascending, [0, 2, 1])]
    #[case::downloads_most_first_uncounted_last(Column::Downloads, SortDirection::Descending, [0, 2, 1])]
    #[case::size_largest_first(Column::Size, SortDirection::Descending, [1, 2, 0])]
    #[case::age_newest_first_undated_last(Column::Age, SortDirection::Descending, [1, 0, 2])]
    #[case::title_a_to_z(Column::Title, SortDirection::Ascending, [0, 1, 2])]
    fn releases_sort_by_a_column(#[case] column: Column, #[case] direction: SortDirection, #[case] order: [usize; 3]) {
        let all = releases();

        let shown = shown(&all, "", Sort { column, direction });

        assert_eq!(titles(&shown), order.map(|index| all[index].title.as_str()));
    }

    #[rstest]
    #[case::fuzzy("bb 1080", &["Breaking Bad S01-05 1080p BDRip HEVC", "Во все тяжкие / Breaking Bad S05 1080p WEB-DL"])]
    #[case::negated("1080p !hevc", &["Во все тяжкие / Breaking Bad S05 1080p WEB-DL"])]
    #[case::cyrillic("тяжкие", &["Во все тяжкие / Breaking Bad S05 1080p WEB-DL"])]
    #[case::exactly("'web-dl", &["Во все тяжкие / Breaking Bad S05 1080p WEB-DL"])]
    #[case::starts_with("^во", &["Во все тяжкие / Breaking Bad S05 1080p WEB-DL"])]
    #[case::ends_with("hdtv$", &["Breaking Bad S01 720p HDTV"])]
    #[case::capital_matches_case("HEVC", &["Breaking Bad S01-05 1080p BDRip HEVC"])]
    #[case::capital_misses_other_case("Hevc", &[])]
    #[case::nothing("2160p", &[])]
    fn the_filter_keeps_matching_titles_in_sort_order(#[case] filter: &str, #[case] expected: &[&str]) {
        let sort = Sort { column: Column::Size, direction: SortDirection::Descending };

        assert_eq!(titles(&shown(&releases(), filter, sort)), expected);
    }

    #[rstest]
    #[case::plain_word_matches_loosely("1080p", 1)]
    #[case::quoted_word_matches_exactly("'1080p", 0)]
    fn a_quote_keeps_a_word_from_matching_loosely(#[case] filter: &str, #[case] found: usize) {
        let all = [release("Breaking Bad [S01-05] (2008-2013) BDRip 720p | LostFilm", 9, None, None)];

        assert_eq!(shown(&all, filter, Sort::default()).len(), found);
    }

    #[test]
    fn pressing_a_column_sorts_by_it_then_turns_it_around() {
        let mut sort = Sort::default();

        sort.press(Column::Title);
        assert_eq!(sort, Sort { column: Column::Title, direction: SortDirection::Ascending });
        sort.press(Column::Title);
        assert_eq!(sort, Sort { column: Column::Title, direction: SortDirection::Descending });
        sort.press(Column::Age);
        assert_eq!(sort, Sort { column: Column::Age, direction: SortDirection::Descending });
    }
}
