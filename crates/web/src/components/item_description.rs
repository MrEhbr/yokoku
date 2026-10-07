use dioxus::prelude::*;

use crate::{
    api::library::detail::{Description, ItemRating},
    format::{count, runtime},
};

/// Ratings from fewer votes are not shown.
const MIN_VOTES: u32 = 1_000;

/// Genres, runtime, ratings and overview, each only when present.
/// A series' runtime is `per_episode`.
#[component]
pub fn ItemDescription(
    description: Description,
    #[props(default)] ratings: Vec<ItemRating>,
    #[props(default)] per_episode: bool,
) -> Element {
    let length = description.runtime.map(|minutes| {
        let length = runtime(u64::from(minutes));
        if per_episode { format!("{length} per episode") } else { length }
    });
    let facts: Vec<String> = description.genres.iter().cloned().chain(length).collect();
    let ratings: Vec<ItemRating> =
        ratings.into_iter().filter(|rating| rating.votes.is_none_or(|votes| votes >= MIN_VOTES)).collect();
    rsx! {
        if !facts.is_empty() || !ratings.is_empty() {
            p { class: "text-caption text-muted",
                {facts.join(" · ")}
                for (index, rating) in ratings.into_iter().enumerate() {
                    if index > 0 || !facts.is_empty() {
                        " · "
                    }
                    Rating { rating }
                }
            }
        }
        if !description.overview.is_empty() {
            p { class: "max-w-prose whitespace-pre-line", "{description.overview}" }
        }
    }
}

/// `IMDb 7.8 (1.2M votes)`, linked to the item's page at the source when known.
#[component]
fn Rating(rating: ItemRating) -> Element {
    let votes = rating.votes.map(|votes| format!(" ({} votes)", count(votes))).unwrap_or_default();
    let text = format!("{} {:.1}{votes}", rating.source, rating.value);
    match rating.url {
        Some(url) => rsx! {
            a {
                class: "underline-offset-4 hover:text-ink hover:underline",
                href: "{url}",
                target: "_blank",
                rel: "noreferrer",
                "{text}"
            }
        },
        None => rsx! { "{text}" },
    }
}
