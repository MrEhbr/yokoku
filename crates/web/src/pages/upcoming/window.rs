use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};
use jiff::civil::Date;

use crate::{
    api::library::calendar::{Agenda, Period},
    components::button::{Button, ButtonSize, ButtonVariant},
    format::date,
};

/// The period the page shows: coming up from today, or the week or month holding `day` or today.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) struct Window {
    pub period: Period,
    pub day: Option<Date>,
}

impl Window {
    /// Moves to the period before the one starting at `from`.
    fn previous(&mut self, from: Date) {
        self.day = from.yesterday().ok();
    }

    /// Moves to the period after the one ending at `to`.
    fn next(&mut self, to: Date) {
        self.day = to.tomorrow().ok();
    }
}

/// The shown period, with controls to move through weeks or months and switch periods.
#[component]
pub(super) fn WindowBar(window: Signal<Window>, agenda: Agenda) -> Element {
    let Agenda { from, to, today, .. } = agenda;
    let period = window().period;
    let (range, unit) = match period {
        Period::ComingUp => (format!("From today, {}", date(today)), None),
        Period::Week => (format!("{} – {}", from.strftime("%b %-d"), date(to)), Some("week")),
        Period::Month => (from.strftime("%B %Y").to_string(), Some("month")),
    };
    rsx! {
        div { class: "mt-6 flex flex-wrap items-center justify-between gap-4",
            div { class: "flex min-h-9 items-center gap-2",
                if let Some(unit) = unit {
                    Button {
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Icon,
                        aria_label: "Previous {unit}",
                        onclick: move |_| window.write().previous(from),
                        ChevronLeft {}
                    }
                    Button {
                        size: ButtonSize::Sm,
                        disabled: from <= today && today <= to,
                        onclick: move |_| window.write().day = None,
                        "Today"
                    }
                    Button {
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Icon,
                        aria_label: "Next {unit}",
                        onclick: move |_| window.write().next(to),
                        ChevronRight {}
                    }
                }
                p { class: "ml-2 font-medium first:ml-0", aria_live: "polite", "{range}" }
            }
            div { role: "group", aria_label: "Period", class: "flex gap-1",
                for option in Period::ALL {
                    Button {
                        key: "{option.label()}",
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Sm,
                        class: "aria-pressed:bg-subtle aria-pressed:text-ink",
                        aria_pressed: period == option,
                        onclick: move |_| window.write().period = option,
                        "{option.label()}"
                    }
                }
            }
        }
    }
}
