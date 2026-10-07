use dioxus::prelude::*;

use crate::{api::library::detail::ItemRating, format::count};

/// Ratings from fewer votes are not shown.
const MIN_VOTES: u32 = 1_000;

/// One chip per rating, like `IMDb | 7.8 1.2M votes`; nothing when none is shown.
#[component]
pub fn ItemRatings(ratings: Vec<ItemRating>) -> Element {
    let shown: Vec<ItemRating> =
        ratings.into_iter().filter(|rating| rating.votes.is_none_or(|votes| votes >= MIN_VOTES)).collect();
    if shown.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "flex flex-wrap gap-2",
            for rating in shown {
                Chip { key: "{rating.source}", rating }
            }
        }
    }
}

/// How good a rating is, as a share of its scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    /// 70% and up, like IMDb 7.0.
    Good,
    /// 55% and up, like IMDb 5.5.
    Mixed,
    Poor,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Self::Good => "bg-success-soft text-success",
            Self::Mixed => "bg-warning-soft text-warning",
            Self::Poor => "bg-danger-soft text-danger",
        }
    }
}

fn tone(rating: &ItemRating) -> Tone {
    match rating.value / rating.out_of {
        share if share >= 0.7 => Tone::Good,
        share if share >= 0.55 => Tone::Mixed,
        _ => Tone::Poor,
    }
}

/// Links to the item's page at the source when known.
#[component]
fn Chip(rating: ItemRating) -> Element {
    let body = rsx! {
        span { class: "flex items-center border-r border-line bg-subtle px-2 text-caption font-semibold text-ink",
            "{rating.source}"
        }
        span { class: "flex items-baseline gap-1.5 px-2 py-1 {tone(&rating).class()}",
            span { class: "font-semibold leading-none tabular-nums sm:text-section", "{rating.value:.1}" }
            if let Some(votes) = rating.votes {
                span { class: "text-caption leading-none text-muted", "{count(votes)} votes" }
            }
        }
    };
    let class = "inline-flex w-fit items-stretch border border-control bg-surface";
    match rating.url {
        Some(url) => rsx! {
            a {
                class: "{class} hover:shadow-paper",
                href: "{url}",
                target: "_blank",
                rel: "noreferrer",
                title: "{rating.source} rating",
                {body}
            }
        },
        None => rsx! {
            span { class: "{class}", {body} }
        },
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{Tone, tone};
    use crate::api::library::detail::ItemRating;

    #[rstest]
    #[case(9.5, Tone::Good)]
    #[case(7.0, Tone::Good)]
    #[case(6.9, Tone::Mixed)]
    #[case(5.5, Tone::Mixed)]
    #[case(5.4, Tone::Poor)]
    fn tones_an_imdb_rating_by_its_share_of_ten(#[case] value: f32, #[case] expected: Tone) {
        let rating = ItemRating { source: "IMDb".into(), value, out_of: 10.0, votes: None, url: None };
        assert_eq!(tone(&rating), expected);
    }
}
