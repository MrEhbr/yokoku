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
        movie_detail::MovieDetail, series_detail::SeriesDetail, settings::Settings, upcoming::Upcoming,
    },
};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Library {},
    #[route("/wanted")]
    Missing {},
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
    #[redirect("/settings", || Route::Settings { page: SettingsPage::default() })]
    #[route("/settings/:page")]
    Settings { page: SettingsPage },
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
            Self::Upcoming {} => Self::Upcoming {},
            Self::Downloads {} => Self::Downloads {},
            Self::History {} => Self::History {},
            Self::Settings { .. } => Self::settings(),
        }
    }

    pub(crate) fn settings() -> Self {
        Self::Settings { page: SettingsPage::Connections }
    }
}

/// A page of Settings, as the URL's last segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsPage {
    #[default]
    Connections,
    Metadata,
    Library,
    Import,
    Naming,
    Schedules,
    Server,
}

impl SettingsPage {
    /// In sidebar order.
    pub(crate) const ALL: [Self; 7] =
        [Self::Connections, Self::Metadata, Self::Library, Self::Import, Self::Naming, Self::Schedules, Self::Server];

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Connections => "Connections",
            Self::Metadata => "Metadata",
            Self::Library => "Library",
            Self::Import => "Import",
            Self::Naming => "Naming",
            Self::Schedules => "Schedules",
            Self::Server => "Server",
        }
    }
}

impl fmt::Display for SettingsPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.title().to_lowercase())
    }
}

impl FromStr for SettingsPage {
    type Err = Infallible;

    /// The first page for an unknown one.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::ALL.into_iter().find(|page| page.to_string() == value).unwrap_or_default())
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

    use super::{Kind, Route, SearchText, SettingsPage};

    #[test]
    fn each_settings_page_survives_the_route_url() {
        for page in SettingsPage::ALL {
            let route = Route::Settings { page };

            let parsed: Route = route.to_string().parse().unwrap();

            assert!(parsed == route, "{route} parsed as {parsed}");
        }
    }

    #[rstest]
    #[case::bare("/settings")]
    #[case::unknown_page("/settings/root-folders")]
    fn settings_without_a_known_page_opens_the_first(#[case] url: &str) {
        let parsed: Route = url.parse().unwrap();

        assert!(parsed == Route::settings(), "{url} parsed as {parsed}");
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
