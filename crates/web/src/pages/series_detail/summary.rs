use dioxus::prelude::*;
use jiff::civil::Date;

use crate::{
    api::library::{FileStatus, detail::EpisodeRow},
    components::item_status::FileState,
    format::{date, episode as code, relative},
};

/// One highlighted episode, such as the next to air, or `none` when there is none.
#[component]
pub(super) fn EpisodeSummary(
    label: &'static str,
    none: &'static str,
    episode: Option<EpisodeRow>,
    today: Date,
) -> Element {
    rsx! {
        section { class: "flex flex-col gap-1 border border-line p-4",
            h2 { class: "text-caption font-medium text-muted", "{label}" }
            match episode {
                Some(episode) => rsx! {
                    p { class: "font-medium",
                        span { class: "yk-code mr-2", "{code(episode.season, episode.number)}" }
                        "{episode.title}"
                    }
                    if let Some(aired) = episode.air_date {
                        p { class: "text-caption text-muted", "{date(aired)} · {relative(aired, today)}" }
                    }
                    if episode.file != FileStatus::Upcoming {
                        FileState { status: episode.file, episode: true }
                    }
                },
                None => rsx! {
                    p { class: "text-muted", "{none}" }
                },
            }
        }
    }
}
