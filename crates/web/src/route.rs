use std::{
    convert::Infallible,
    fmt::{self, Write},
    str::FromStr,
};

use dioxus::prelude::*;
use yokoku_domain::{ItemId, MovieId, SeriesId};

use crate::{
    api::library::Kind,
    layout::Shell,
    pages::{
        add::Add, downloads::Downloads, history::History, library::Library, missing::Missing,
        movie_detail::MovieDetail, series_detail::SeriesDetail, upcoming::Upcoming,
    },
};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Library {},
    #[route("/missing")]
    Missing {},
    #[route("/add?:query&:kind")]
    Add { query: SearchText, kind: Kind },
    #[route("/series/:id")]
    SeriesDetail { id: SeriesId },
    #[route("/movies/:id")]
    MovieDetail { id: MovieId },
    #[route("/upcoming")]
    Upcoming {},
    #[route("/downloads")]
    Downloads {},
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

    /// The main destination this page belongs to: items, Missing and Add are part of the Library.
    pub(crate) fn section(&self) -> Self {
        match self {
            Self::Library {}
            | Self::Missing {}
            | Self::Add { .. }
            | Self::SeriesDetail { .. }
            | Self::MovieDetail { .. } => Self::Library {},
            Self::Upcoming {} => Self::Upcoming {},
            Self::Downloads {} => Self::Downloads {},
            Self::History {} => Self::History {},
        }
    }
}

/// Text searched for, as a route query argument. The router percent-decodes the whole query
/// before splitting it at `&`, and cuts it at `#` first, so `%`, `&` and `#` are encoded twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchText(pub String);

impl fmt::Display for SearchText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '%' => f.write_str("%2525")?,
                '&' => f.write_str("%2526")?,
                '#' => f.write_str("%2523")?,
                c => f.write_char(c)?,
            }
        }
        Ok(())
    }
}

impl FromStr for SearchText {
    type Err = Infallible;

    /// Takes the text as the router leaves it, encoded once; `%25` goes last so it cannot form
    /// another escape.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.replace("%26", "&").replace("%23", "#").replace("%25", "%")))
    }
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{Kind, Route, SearchText};

    #[rstest]
    #[case::words("dune part two")]
    #[case::ampersand("Fast & Furious")]
    #[case::hash("#Alive")]
    #[case::percent("100% Wolf")]
    #[case::escape_like("%26 %23 %25")]
    fn search_text_survives_the_route_url(#[case] text: &str) {
        let route = Route::Add { query: SearchText(text.into()), kind: Kind::Movie };

        let parsed: Route = route.to_string().parse().unwrap();

        assert!(parsed == route, "{route} parsed as {parsed}");
    }
}
