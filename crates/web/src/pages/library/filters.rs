use dioxus::prelude::*;

use crate::{
    api::library::{Kind, Sort, Status, WatchState},
    components::select::{Select, SelectOption},
};

/// What the list shows and in which order.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) struct Filters {
    pub kind: Option<Kind>,
    pub status: Option<Status>,
    pub watched: Option<WatchState>,
    pub sort: Sort,
}

impl Filters {
    /// Sets the type; a status of the other type is cleared.
    fn set_kind(&mut self, kind: Option<Kind>) {
        self.kind = kind;
        if self.status.is_some_and(|status| kind.is_some_and(|kind| status.kind() != kind)) {
            self.status = None;
        }
    }

    pub(super) fn narrows(self) -> bool {
        self.kind.is_some() || self.status.is_some() || self.watched.is_some()
    }

    /// Clears type, status and watched, keeping the sort.
    pub(super) fn clear(&mut self) {
        *self = Self { sort: self.sort, ..Self::default() };
    }
}

/// Type, status, watched and sort selects over the page's [`Filters`].
#[component]
pub(super) fn FilterBar(filters: Signal<Filters>) -> Element {
    let kind_value = use_memo(move || Some(filters().kind));
    let status_value = use_memo(move || Some(filters().status));
    let watched_value = use_memo(move || Some(filters().watched));
    let sort_value = use_memo(move || Some(filters().sort));
    let Filters { kind, status, watched, sort } = filters();
    let statuses = Status::ALL.into_iter().filter(move |status| kind.is_none_or(|kind| status.kind() == kind));

    rsx! {
        div { class: "mt-6 grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-4 lg:max-w-5xl",
            Field { label: "Type",
                Select::<Option<Kind>> {
                    aria_label: "Type",
                    placeholder: kind.map_or("All types", Kind::label),
                    value: Some(kind_value.into()),
                    on_value_change: move |next: Option<Option<Kind>>| filters.write().set_kind(next.flatten()),
                    SelectOption::<Option<Kind>> { index: 0usize, value: None, text_value: "All types", "All types" }
                    for (index, option) in Kind::ALL.into_iter().enumerate() {
                        SelectOption::<Option<Kind>> {
                            key: "{index}",
                            index: index + 1,
                            value: Some(option),
                            text_value: option.label(),
                            "{option.label()}"
                        }
                    }
                }
            }
            Field { label: "Status",
                Select::<Option<Status>> {
                    aria_label: "Status",
                    placeholder: status.map_or("All statuses", Status::label),
                    value: Some(status_value.into()),
                    on_value_change: move |next: Option<Option<Status>>| filters.write().status = next.flatten(),
                    SelectOption::<Option<Status>> {
                        index: 0usize,
                        value: None,
                        text_value: "All statuses",
                        "All statuses"
                    }
                    for (index, option) in statuses.enumerate() {
                        SelectOption::<Option<Status>> {
                            key: "{option.label()}",
                            index: index + 1,
                            value: Some(option),
                            text_value: option.label(),
                            "{option.label()}"
                        }
                    }
                }
            }
            Field { label: "Watched",
                Select::<Option<WatchState>> {
                    aria_label: "Watched",
                    placeholder: watched.map_or("Any", WatchState::label),
                    value: Some(watched_value.into()),
                    on_value_change: move |next: Option<Option<WatchState>>| filters.write().watched = next.flatten(),
                    SelectOption::<Option<WatchState>> { index: 0usize, value: None, text_value: "Any", "Any" }
                    for (index, option) in WatchState::ALL.into_iter().enumerate() {
                        SelectOption::<Option<WatchState>> {
                            key: "{index}",
                            index: index + 1,
                            value: Some(option),
                            text_value: option.label(),
                            "{option.label()}"
                        }
                    }
                }
            }
            Field { label: "Sort by",
                Select::<Sort> {
                    aria_label: "Sort by",
                    placeholder: sort.label(),
                    value: Some(sort_value.into()),
                    on_value_change: move |next: Option<Sort>| filters.write().sort = next.unwrap_or_default(),
                    for (index, option) in Sort::ALL.into_iter().enumerate() {
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
        }
    }
}

#[component]
fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "yk-field",
            span { class: "yk-label", aria_hidden: "true", "{label}" }
            {children}
        }
    }
}
