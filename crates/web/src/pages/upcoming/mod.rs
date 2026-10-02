mod agenda;
mod window;

use dioxus::prelude::*;

use self::{
    agenda::AgendaList,
    window::{Window, WindowBar},
};
use crate::{
    api::library::calendar::{Period, agenda},
    components::{
        load_failed::LoadFailed,
        skeleton::{Loaded, Skeleton},
    },
};

/// Episodes and movie releases of monitored items, coming up from today or a week or month at a
/// time, by day.
#[component]
pub fn Upcoming() -> Element {
    let window = use_signal(Window::default);
    let agenda = use_server_future(move || {
        let Window { period, day } = window();
        agenda(Some(period), day)
    })?;

    rsx! {
        document::Title { "Upcoming · Yokoku" }
        h1 { class: "yk-page-title", "Upcoming" }
        match &*agenda.read() {
            None => rsx! {
                Skeleton { class: "mt-6 h-64 w-full" }
            },
            Some(Err(_)) => rsx! {
                LoadFailed { class: "mt-6", subject: "The calendar" }
            },
            Some(Ok(agenda)) => rsx! {
                Loaded {
                    WindowBar { window, agenda: agenda.clone() }
                    div { class: "mt-8",
                        if agenda.entries.is_empty() {
                            p { class: "text-muted",
                                match window().period {
                                    Period::ComingUp => "Nothing is scheduled for monitored items in the next year.",
                                    Period::Week => "Nothing is scheduled for monitored items this week.",
                                    Period::Month => "Nothing is scheduled for monitored items this month.",
                                }
                            }
                        } else {
                            AgendaList { entries: agenda.entries.clone(), today: agenda.today }
                        }
                    }
                }
            },
        }
    }
}
