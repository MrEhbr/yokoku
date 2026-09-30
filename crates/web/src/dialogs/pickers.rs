use dioxus::prelude::*;
use yokoku_domain::{ItemId, ItemName, SeriesId};

use crate::{
    api::{
        failure,
        library::{Kind, detail, library},
    },
    components::{
        combobox::{Combobox, ComboboxEmpty, ComboboxOption},
        label::Label,
        select::{Select, SelectOption},
        skeleton::Skeleton,
    },
};

/// A search over the library's series.
#[component]
pub(crate) fn SeriesPicker(id: &'static str, series: Signal<Option<SeriesId>>) -> Element {
    let items = use_resource(|| library(Some(Kind::Series), None, None));
    let choice = use_memo(move || Some(series()));
    rsx! {
        div { class: "grid gap-1.5",
            Label { html_for: id, "Series" }
            match &*items.read() {
                None => rsx! {
                    Skeleton { class: "h-9 w-full" }
                },
                Some(Err(failed)) => rsx! {
                    p { role: "alert", class: "text-caption text-danger", {failure(failed)} }
                },
                Some(Ok(entries)) => rsx! {
                    Combobox::<Option<SeriesId>> {
                        id,
                        value: Some(choice.into()),
                        placeholder: "Search the library…",
                        on_value_change: move |next: Option<Option<SeriesId>>| series.set(next.flatten()),
                        ComboboxEmpty { "No series matches" }
                        for (index, entry) in entries.iter().enumerate() {
                            if let ItemId::Series(found) = entry.id {
                                ComboboxOption::<Option<SeriesId>> {
                                    key: "{found}",
                                    index,
                                    value: Some(found),
                                    text_value: ItemName::new(&entry.title, entry.year).to_string(),
                                    "{ItemName::new(&entry.title, entry.year)}"
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// The seasons of `series`, specials last, with `none` as a first choice when given; cleared
/// when mounted for another series.
#[component]
pub(crate) fn SeasonPicker(
    id: &'static str,
    series: SeriesId,
    season: Signal<Option<u16>>,
    none: Option<&'static str>,
    #[props(default)] aria_describedby: Option<&'static str>,
) -> Element {
    let seasons = use_resource(move || async move {
        let detail = detail::series(series).await.ok().flatten();
        let mut numbers: Vec<u16> =
            detail.map(|detail| detail.seasons.iter().map(|season| season.number).collect()).unwrap_or_default();
        numbers.sort_by_key(|&number| (number == 0, number));
        numbers
    });
    use_effect(move || season.set(None));
    let choice = use_memo(move || Some(season()));
    let label = move |number: Option<u16>| match number {
        None => none.unwrap_or("Choose…").to_owned(),
        Some(0) => "Specials".to_owned(),
        Some(number) => format!("Season {number}"),
    };
    let choices: Vec<Option<u16>> = match &*seasons.read() {
        Some(numbers) => none.map(|_| None).into_iter().chain(numbers.iter().copied().map(Some)).collect(),
        None => Vec::new(),
    };
    rsx! {
        div { class: "grid gap-1.5",
            Label { html_for: id, "Season" }
            if seasons.read().is_none() {
                Skeleton { class: "h-9 w-full" }
            } else {
                Select::<Option<u16>> {
                    id,
                    aria_describedby,
                    value: Some(choice.into()),
                    placeholder: label(season()),
                    on_value_change: move |next: Option<Option<u16>>| season.set(next.flatten()),
                    for (index, number) in choices.into_iter().enumerate() {
                        SelectOption::<Option<u16>> {
                            key: "{number:?}",
                            index,
                            value: number,
                            text_value: label(number),
                            {label(number)}
                        }
                    }
                }
            }
        }
    }
}
