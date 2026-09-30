use dioxus::prelude::*;
use yokoku_domain::SeriesId;

use crate::{
    api::{
        failure,
        library::{detail::Numbering, manage::set_numbering},
    },
    components::{
        label::Label,
        select::{Select, SelectOption},
    },
};

const CHOICES: [(Numbering, &str); 2] =
    [(Numbering::Standard, "Standard · S01E02"), (Numbering::Absolute, "Absolute · 12")];

/// How episode numbers in the series' file names are read (FR-1.8); a choice is saved at once and
/// `on_change` called once the server has it.
#[component]
pub(super) fn NumberingSelect(series: SeriesId, numbering: Numbering, on_change: Callback) -> Element {
    let mut chosen = use_signal(|| Some(numbering));
    let mut saving = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    use_effect(use_reactive!(|numbering| chosen.set(Some(numbering))));
    let label = |numbering| CHOICES.iter().find(|(choice, _)| *choice == numbering).map_or("", |(_, label)| label);
    rsx! {
        span { class: "inline-flex flex-wrap items-center gap-2",
            Label { html_for: "numbering", "Numbering" }
            div { class: "w-48",
                Select::<Numbering> {
                    id: "numbering",
                    value: Some(chosen.into()),
                    placeholder: label(numbering),
                    disabled: saving(),
                    on_value_change: move |next: Option<Numbering>| {
                        let Some(next) = next.filter(|next| Some(*next) != chosen()) else { return };
                        chosen.set(Some(next));
                        spawn(async move {
                            saving.set(true);
                            failed.set(None);
                            match set_numbering(series, next).await {
                                Ok(()) => on_change(()),
                                Err(error) => {
                                    chosen.set(Some(numbering));
                                    failed.set(Some(failure(&error)));
                                }
                            }
                            saving.set(false);
                        });
                    },
                    for (index, (choice, text)) in CHOICES.into_iter().enumerate() {
                        SelectOption::<Numbering> {
                            key: "{text}",
                            index,
                            value: choice,
                            text_value: text,
                            "{text}"
                        }
                    }
                }
            }
            if let Some(error) = failed() {
                span { role: "alert", class: "text-caption text-danger", "{error}" }
            }
        }
    }
}
