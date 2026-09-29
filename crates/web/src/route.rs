use dioxus::prelude::*;
use yokoku_domain::{ItemId, MovieId, SeriesId};

use crate::{
    layout::Shell,
    pages::{
        history::History, library::Library, missing::Missing, movie_detail::MovieDetail, series_detail::SeriesDetail,
        upcoming::Upcoming,
    },
};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Library {},
    #[route("/missing")]
    Missing {},
    #[route("/series/:id")]
    SeriesDetail { id: SeriesId },
    #[route("/movies/:id")]
    MovieDetail { id: MovieId },
    #[route("/upcoming")]
    Upcoming {},
    #[route("/history")]
    History {},
}

impl Route {
    /// The item's detail page.
    pub(crate) fn item(id: ItemId) -> Self {
        match id {
            ItemId::Series(id) => Self::SeriesDetail { id },
            ItemId::Movie(id) => Self::MovieDetail { id },
        }
    }

    /// The main destination this page belongs to: items and Missing are part of the Library.
    pub(crate) fn section(&self) -> Self {
        match self {
            Self::Library {} | Self::Missing {} | Self::SeriesDetail { .. } | Self::MovieDetail { .. } => {
                Self::Library {}
            },
            Self::Upcoming {} => Self::Upcoming {},
            Self::History {} => Self::History {},
        }
    }
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
