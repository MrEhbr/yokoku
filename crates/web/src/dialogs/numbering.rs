use dioxus::prelude::*;
use yokoku_domain::SeriesId;

use crate::{
    api::{
        failure,
        library::{detail::Numbering, manage::set_numbering},
    },
    components::{
        button::{Button, ButtonVariant},
        dialog::{DialogDescription, DialogFooter},
        label::Label,
        select::{Select, SelectOption},
    },
    dialogs::ClosableDialog,
};

const CHOICES: [(Numbering, &str, &str); 2] = [
    (
        Numbering::Standard,
        "Standard · S01E02",
        "The number is the episode within its season. With more than one season, Yokoku guesses and asks you to \
         review the import.",
    ),
    (
        Numbering::Absolute,
        "Absolute · 12",
        "The number counts episodes across all seasons, without specials. Common for anime.",
    ),
];

/// Chooses how the series' file names without a season are read while `open`; `on_change` runs
/// once the server has the choice.
#[component]
pub fn NumberingDialog(series: SeriesId, numbering: Numbering, mut open: Signal<bool>, on_change: Callback) -> Element {
    rsx! {
        ClosableDialog { title: "Episode numbering", open,
            DialogDescription { "How Yokoku reads file names that have an episode number but no season, like Show - 05." }
            if open() {
                Form {
                    series,
                    numbering,
                    on_change,
                    on_close: move |()| open.set(false),
                }
            }
        }
    }
}

#[component]
fn Form(series: SeriesId, numbering: Numbering, on_change: Callback, on_close: Callback) -> Element {
    let mut chosen = use_signal(|| Some(numbering));
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let picked = chosen().unwrap_or(numbering);
    let (_, label, explained) = CHOICES.iter().find(|(choice, ..)| *choice == picked).copied().unwrap_or(CHOICES[0]);
    let save = move |_| async move {
        saving.set(true);
        error.set(None);
        match set_numbering(series, picked).await {
            Ok(()) => {
                on_change(());
                on_close(());
            },
            Err(failed) => {
                error.set(Some(failure(&failed)));
                saving.set(false);
            },
        }
    };
    rsx! {
        div { class: "grid gap-2",
            Label { html_for: "numbering", "Numbering" }
            Select::<Numbering> {
                id: "numbering",
                value: Some(chosen.into()),
                placeholder: label,
                disabled: saving(),
                on_value_change: move |next: Option<Numbering>| {
                    if next.is_some() {
                        chosen.set(next);
                    }
                },
                for (index, (choice, text, _)) in CHOICES.into_iter().enumerate() {
                    SelectOption::<Numbering> {
                        key: "{text}",
                        index,
                        value: choice,
                        text_value: text,
                        "{text}"
                    }
                }
            }
            p { class: "text-caption text-muted", "{explained}" }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            Button { onclick: move |_| on_close(()), "Cancel" }
            Button {
                variant: ButtonVariant::Primary,
                disabled: saving() || picked == numbering,
                aria_busy: saving(),
                onclick: save,
                "Save"
            }
        }
    }
}
