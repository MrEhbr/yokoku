//! One story page per component.

mod accordion;
mod alert;
mod alert_dialog;
mod avatar;
mod breadcrumb;
mod card;
mod dropdown_menu;
mod hover_card;
mod kbd;
mod pagination;
mod radio_group;
mod separator;
mod sheet;
mod sidebar;
mod skeleton;
mod spinner;
mod switch;
mod tabs;
mod textarea;
mod toggle;
mod tooltip;

use dioxus::prelude::*;
use yokoku_web::components::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    combobox::{Combobox, ComboboxEmpty, ComboboxOption},
    dialog::{Dialog, DialogDescription, DialogFooter, DialogTitle},
    disclosure::Disclosure,
    field::{Field, FieldError, FieldHint},
    input::Input,
    item_hero::ItemHero,
    label::Label,
    progress::Progress,
    select::{Select, SelectGroup, SelectGroupLabel, SelectOption},
    status::{Status, Tone},
    table::{
        SortDirection, Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow, TableSortHead,
    },
};

pub use self::{
    accordion::*, alert::*, alert_dialog::*, avatar::*, breadcrumb::*, card::*, dropdown_menu::*, hover_card::*,
    kbd::*, pagination::*, radio_group::*, separator::*, sheet::*, sidebar::*, skeleton::*, spinner::*, switch::*,
    tabs::*, textarea::*, toggle::*, tooltip::*,
};
use crate::{Story, StoryPage};

#[component]
pub fn BadgeStory() -> Element {
    rsx! {
        StoryPage {
            name: "Badge",
            path: "badge",
            summary: "Small labels for counts and states. Words carry the meaning; color only supports it.",
            Story { title: "Variants",
                div { class: "flex flex-wrap gap-2",
                    Badge { "12 files" }
                    Badge { variant: BadgeVariant::Outline, "Series" }
                    Badge { variant: BadgeVariant::Success, "Downloaded" }
                    Badge { variant: BadgeVariant::Warning, "Missing" }
                    Badge { variant: BadgeVariant::Danger, "Failed" }
                    Badge { variant: BadgeVariant::Info, "Upcoming" }
                }
            }
        }
    }
}

#[component]
pub fn ButtonStory() -> Element {
    rsx! {
        StoryPage {
            name: "Button",
            path: "button",
            summary: "One pink primary action per local context, named with its count. Destructive actions are never pink.",
            Story { title: "Variants",
                div { class: "flex flex-wrap gap-2",
                    Button { variant: ButtonVariant::Primary, "Import 12 files" }
                    Button { "Cancel" }
                    Button { variant: ButtonVariant::Quiet, "Skip" }
                    Button { variant: ButtonVariant::Danger, "Delete 3 files" }
                }
            }
            Story { title: "Sizes",
                div { class: "flex flex-wrap items-center gap-2",
                    Button { size: ButtonSize::Sm, "Small" }
                    Button { "Medium" }
                    Button { size: ButtonSize::Icon, aria_label: "More actions", "…" }
                }
            }
            Story { title: "Disabled and busy",
                div { class: "flex flex-wrap gap-2",
                    Button { variant: ButtonVariant::Primary, disabled: true, "Import 0 files" }
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: true,
                        aria_busy: "true",
                        "Importing 12 files"
                    }
                }
            }
        }
    }
}

#[component]
pub fn CheckboxStory() -> Element {
    let mut state = use_signal(|| CheckboxState::Indeterminate);
    rsx! {
        StoryPage {
            name: "Checkbox",
            path: "checkbox",
            summary: "Row and header selection. A header shows the mixed state while some rows are selected.",
            Story { title: "States",
                div { class: "flex flex-col gap-3",
                    for (label, initial) in [
                        ("Unchecked", CheckboxState::Unchecked),
                        ("Checked", CheckboxState::Checked),
                        ("Mixed", CheckboxState::Indeterminate),
                    ]
                    {
                        div { key: "{label}", class: "flex items-center gap-2",
                            Checkbox {
                                id: "checkbox-{label}",
                                default_checked: initial,
                            }
                            Label { html_for: "checkbox-{label}", "{label}" }
                        }
                    }
                    div { class: "flex items-center gap-2",
                        Checkbox {
                            id: "checkbox-disabled",
                            disabled: true,
                            default_checked: CheckboxState::Checked,
                        }
                        Label { html_for: "checkbox-disabled", "Disabled" }
                    }
                }
            }
            Story { title: "Controlled",
                div { class: "flex items-center gap-3",
                    Checkbox {
                        aria_label: "Select all files",
                        checked: Some(state()),
                        on_checked_change: move |next| state.set(next),
                    }
                    span { class: "text-caption text-muted",
                        match state() {
                            CheckboxState::Checked => "All 12 files selected",
                            CheckboxState::Indeterminate => "4 of 12 files selected",
                            CheckboxState::Unchecked => "No files selected",
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DialogStory() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        StoryPage {
            name: "Dialog",
            path: "dialog",
            summary: "Modal operations with one footer. Escape and a click outside close it; closing a preview discards it.",
            Story { title: "Rename preview",
                Button { onclick: move |_| open.set(true), "Rename 2 files…" }
                Dialog {
                    open: Some(open()),
                    on_open_change: move |next| open.set(next),
                    DialogTitle { "Rename existing files" }
                    DialogDescription { "Orbital (2024) · Season 1. Files are renamed inside the series folder." }
                    div { class: "grid gap-3",
                        for (old, new) in [
                            ("orbital.s01e01.mkv", "Orbital (2024) - S01E01 - Launch.mkv"),
                            ("orbital.s01e02.mkv", "Orbital (2024) - S01E02 - Drift.mkv"),
                        ]
                        {
                            div { key: "{old}", class: "grid gap-0.5",
                                span { class: "yk-code text-muted", "{old}" }
                                span { class: "yk-code", "→ {new}" }
                            }
                        }
                    }
                    DialogFooter {
                        Button { onclick: move |_| open.set(false), "Cancel" }
                        Button {
                            variant: ButtonVariant::Primary,
                            onclick: move |_| open.set(false),
                            "Rename 2 files"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn FieldStory() -> Element {
    rsx! {
        StoryPage {
            name: "Field",
            path: "field",
            summary: "A label, one control, and a hint or an error that says how to fix the value.",
            Story { title: "Hint",
                Field {
                    Label { html_for: "root-folder", "Root folder" }
                    Input {
                        id: "root-folder",
                        value: "/media/shows",
                        aria_describedby: "root-folder-hint",
                    }
                    FieldHint { id: "root-folder-hint", "New series are added here." }
                }
            }
            Story { title: "Invalid",
                Field {
                    Label { html_for: "address", "Transmission address" }
                    Input {
                        id: "address",
                        value: "localhost",
                        aria_invalid: "true",
                        aria_describedby: "address-error",
                    }
                    FieldError { id: "address-error", "Use host:port, such as localhost:9091." }
                }
            }
            Story { title: "Disabled",
                Field {
                    Label { html_for: "api-key", "API key" }
                    Input {
                        id: "api-key",
                        value: "Set by the environment",
                        disabled: true,
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProgressStory() -> Element {
    rsx! {
        StoryPage {
            name: "Progress",
            path: "progress",
            summary: "Known progress shows a value. Unknown progress is indeterminate, never 0%.",
            Story { title: "Known",
                Progress { value: Some(40.0), aria_label: "Import progress" }
            }
            Story { title: "Indeterminate",
                Progress { value: None, aria_label: "Scanning the library" }
            }
        }
    }
}

#[component]
pub fn SelectStory() -> Element {
    let mut episode = use_signal(|| None::<u32>);
    let titles = ["Launch", "Drift", "Burn", "Coast"];
    rsx! {
        StoryPage {
            name: "Select",
            path: "select",
            summary: "A single choice over typed values, with a placeholder until one is picked.",
            Story { title: "Episode",
                div { class: "flex max-w-sm flex-col gap-2",
                    Select::<u32> {
                        aria_label: "Episode",
                        placeholder: "Choose an episode…",
                        value: Some(episode.into()),
                        on_value_change: move |next| episode.set(next),
                        for (index, title) in titles.iter().enumerate() {
                            SelectOption::<u32> {
                                key: "{index}",
                                index,
                                value: index as u32 + 1,
                                text_value: format!("S01E{:02} · {title}", index + 1),
                                "S01E{index + 1:02} · {title}"
                            }
                        }
                    }
                    p { class: "text-caption text-muted",
                        match episode() {
                            Some(number) => format!("Episode {number} selected"),
                            None => "Nothing selected".to_owned(),
                        }
                    }
                }
            }
            Story { title: "Groups and disabled",
                div { class: "max-w-sm",
                    Select::<String> {
                        aria_label: "Root folder",
                        placeholder: "Choose a root folder…",
                        SelectGroup {
                            SelectGroupLabel { "Shows" }
                            SelectOption::<String> {
                                index: 0usize,
                                value: "/media/shows",
                                text_value: "/media/shows",
                                "/media/shows"
                            }
                            SelectOption::<String> {
                                index: 1usize,
                                value: "/media/anime",
                                text_value: "/media/anime",
                                "/media/anime"
                            }
                        }
                        SelectGroup {
                            SelectGroupLabel { "Offline" }
                            SelectOption::<String> {
                                index: 2usize,
                                value: "/mnt/archive",
                                text_value: "/mnt/archive",
                                disabled: true,
                                "/mnt/archive"
                            }
                        }
                    }
                }
            }
            Story { title: "Disabled",
                div { class: "max-w-sm",
                    Select::<String> {
                        aria_label: "Quality",
                        placeholder: "1080p",
                        disabled: true,
                    }
                }
            }
        }
    }
}

#[component]
pub fn ComboboxStory() -> Element {
    let mut episode = use_signal(|| None::<u32>);
    let titles = [
        "The Journey's End",
        "It Didn't Have to Be Magic",
        "Killing Magic",
        "The Land Where Souls Rest",
        "Phantoms of the Dead",
        "The Hero of the Village",
        "Like a Fairy Tale",
        "Frieren the Slayer",
    ];
    rsx! {
        StoryPage {
            name: "Combobox",
            path: "combobox",
            summary: "A single choice from a long list, filtered by what is typed.",
            Story { title: "Episode",
                div { class: "flex max-w-sm flex-col gap-2",
                    Label { html_for: "story-episode", "Episode" }
                    Combobox::<u32> {
                        id: "story-episode",
                        placeholder: "Search by code or title…",
                        value: Some(episode.into()),
                        on_value_change: move |next| episode.set(next),
                        ComboboxEmpty { "No episode matches" }
                        for (index, title) in titles.iter().enumerate() {
                            ComboboxOption::<u32> {
                                key: "{index}",
                                index,
                                value: index as u32 + 1,
                                text_value: format!("S01E{:02} {title}", index + 1),
                                "S01E{index + 1:02} {title}"
                            }
                        }
                    }
                    p { class: "text-caption text-muted",
                        match episode() {
                            Some(number) => format!("Episode {number} selected"),
                            None => "Nothing selected".to_owned(),
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TableStory() -> Element {
    let mut ascending = use_signal(|| true);
    let mut rows = vec![("S01E01", "Launch", "1.4 GB"), ("S01E02", "Drift", "1.3 GB"), ("S01E03", "Burn", "—")];
    if !ascending() {
        rows.reverse();
    }
    rsx! {
        StoryPage {
            name: "Table",
            path: "table",
            summary: "Dense rows with quiet rules. Select rows with checkboxes; sort buttons announce their direction.",
            Story { title: "Episodes",
                Table {
                    TableCaption { "3 episodes in Season 1" }
                    TableHeader {
                        TableRow {
                            TableHead {
                                Checkbox {
                                    aria_label: "Select all episodes",
                                    default_checked: CheckboxState::Indeterminate,
                                }
                            }
                            TableSortHead {
                                label: "Episode",
                                direction: Some(if ascending() { SortDirection::Ascending } else { SortDirection::Descending }),
                                onclick: move |_| ascending.toggle(),
                            }
                            TableHead { "Title" }
                            TableHead { "Status" }
                            TableHead { class: "text-right", "Size" }
                        }
                    }
                    TableBody {
                        for (code, title, size) in rows {
                            TableRow {
                                key: "{code}",
                                "data-selected": code == "S01E01",
                                TableCell {
                                    Checkbox {
                                        aria_label: "Select {code}",
                                        default_checked: if code == "S01E01" { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                    }
                                }
                                TableCell {
                                    span { class: "yk-code", "{code}" }
                                }
                                TableCell { "{title}" }
                                TableCell {
                                    if size == "—" {
                                        Badge { variant: BadgeVariant::Warning, "Missing" }
                                    } else {
                                        Badge { variant: BadgeVariant::Success, "Downloaded" }
                                    }
                                }
                                TableCell { class: "text-right tabular-nums", "{size}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn StatusStory() -> Element {
    rsx! {
        StoryPage {
            name: "Status",
            path: "status",
            summary: "A symbol and a label per state. Each category maps its own values; categories never share one badge.",
            Story { title: "Tones",
                div { class: "flex flex-wrap gap-4",
                    Status { tone: Tone::Success, label: "Downloaded" }
                    Status { tone: Tone::Warning, label: "Missing" }
                    Status { tone: Tone::Danger, label: "Failed" }
                    Status { tone: Tone::Info, label: "Not yet aired" }
                    Status { tone: Tone::Muted, label: "Unmonitored" }
                }
            }
        }
    }
}

#[component]
pub fn ItemHeroStory() -> Element {
    rsx! {
        StoryPage {
            name: "Item hero",
            path: "item-hero",
            summary: "A detail page's header. The logo is the title only over a backdrop; without artwork the title is text.",
            Story { title: "Without artwork",
                ItemHero { title: "Frieren: Beyond Journey's End",
                    p { class: "text-muted", "2023 · Series" }
                    Status { tone: Tone::Info, label: "On break" }
                }
            }
        }
    }
}

#[component]
pub fn DisclosureStory() -> Element {
    rsx! {
        StoryPage {
            name: "Disclosure",
            path: "disclosure",
            summary: "A section that opens under its summary. A native details element, so it works before the page hydrates.",
            Story { title: "Seasons, the latest open",
                div { class: "border-t border-line",
                    Disclosure {
                        open: true,
                        summary: rsx! {
                            span { class: "text-section font-medium", "Season 2" }
                        },
                        p { class: "text-muted", "10 episodes" }
                    }
                    Disclosure {
                        summary: rsx! {
                            span { class: "text-section font-medium", "Season 1" }
                        },
                        p { class: "text-muted", "28 episodes" }
                    }
                }
            }
        }
    }
}
