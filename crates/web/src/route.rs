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
        movie_detail::MovieDetail, releases::Releases, series_detail::SeriesDetail, settings::Settings,
        upcoming::Upcoming,
    },
};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Library {},
    #[route("/wanted")]
    Missing {},
    #[route("/releases")]
    Releases {},
    #[route("/add?:query&:kind")]
    Add { query: SearchText, kind: Kind },
    #[route("/series/:id")]
    SeriesDetail { id: SeriesId },
    #[route("/movies/:id")]
    MovieDetail { id: MovieId },
    #[route("/upcoming")]
    Upcoming {},
    #[route("/queue")]
    Downloads {},
    #[route("/history")]
    History {},
    #[route("/settings#:part")]
    Settings { part: SettingsPart },
}

impl Route {
    /// The item's detail page.
    pub(crate) fn item(id: ItemId) -> Self {
        match id {
            ItemId::Series(id) => Self::SeriesDetail { id },
            ItemId::Movie(id) => Self::MovieDetail { id },
        }
    }

    /// The main destination this page belongs to: items and Add are part of the Library.
    pub(crate) fn section(&self) -> Self {
        match self {
            Self::Library {} | Self::Add { .. } | Self::SeriesDetail { .. } | Self::MovieDetail { .. } => {
                Self::Library {}
            },
            Self::Missing {} => Self::Missing {},
            Self::Releases {} => Self::Releases {},
            Self::Upcoming {} => Self::Upcoming {},
            Self::Downloads {} => Self::Downloads {},
            Self::History {} => Self::History {},
            Self::Settings { .. } => Self::settings(),
        }
    }

    pub(crate) fn settings() -> Self {
        Self::Settings { part: SettingsPart::Top }
    }
}

/// The part of the Settings page scrolled to, as the URL's hash fragment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsPart {
    #[default]
    Top,
    RootFolders,
}

impl SettingsPart {
    /// The element id; empty for the top.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Top => "",
            Self::RootFolders => "root-folders",
        }
    }
}

impl fmt::Display for SettingsPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for SettingsPart {
    type Err = Infallible;

    /// The top for an unknown part.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(if value == Self::RootFolders.id() { Self::RootFolders } else { Self::Top })
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

    use super::{Kind, Route, SearchText, SettingsPart};

    #[rstest]
    #[case::top(SettingsPart::Top, "/settings")]
    #[case::root_folders(SettingsPart::RootFolders, "/settings#root-folders")]
    fn a_settings_part_is_the_hash_fragment(#[case] part: SettingsPart, #[case] url: &str) {
        let route = Route::Settings { part };

        let parsed: Route = url.parse().unwrap();

        assert_eq!(route.to_string(), url);
        assert!(parsed == route, "{url} parsed as {parsed}");
    }

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
