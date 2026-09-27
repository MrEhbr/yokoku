//! The appearance preference, stored per browser in a cookie and rendered as
//! `data-theme` on `<html>`, so the first paint already uses it.

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    router::request::uri,
    view::{View, class, component, view},
};

const COOKIE: &str = "theme";

/// The appearance a browser asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follows the operating system.
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// The `data-theme` value. `None` lets `light-dark()` follow the operating system.
    pub fn attribute(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::Light => Some("light"),
            Self::Dark => Some("dark"),
        }
    }

    fn value(self) -> &'static str {
        self.attribute().unwrap_or("system")
    }

    fn label(self) -> &'static str {
        match self {
            Self::System => "Auto",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// The theme stored in this browser.
pub fn current(cx: &Cx) -> Theme {
    match cookies(cx).get(COOKIE).as_ref().map(Cookie::value) {
        Some("light") => Theme::Light,
        Some("dark") => Theme::Dark,
        _ => Theme::System,
    }
}

/// Stores `theme` for this browser. `System` removes the cookie.
pub fn remember(cx: &Cx, theme: Theme) {
    let jar = cookies(cx);
    match theme.attribute() {
        Some(value) => jar.add(
            Cookie::build((COOKIE, value))
                .path("/")
                .http_only(true)
                .same_site(SameSite::Lax)
                .max_age(Duration::days(365))
                .build(),
        ),
        None => jar.remove(Cookie::build((COOKIE, "")).path("/").build()),
    }
}

/// The form posted by [`theme_switch`].
#[derive(Debug, Deserialize)]
pub struct ThemeChange {
    pub theme: Theme,
    back: String,
}

impl ThemeChange {
    /// The page to return to. Anything but a local path becomes `/`.
    pub fn back(&self) -> &str {
        let local = self.back.starts_with('/') && !self.back.starts_with("//") && !self.back.starts_with("/\\");
        if local { &self.back } else { "/" }
    }
}

/// Auto, Light, and Dark buttons posting a [`ThemeChange`] to `action`, which
/// calls [`remember`] and redirects back.
#[component]
pub async fn theme_switch(cx: &Cx, action: String) -> Result<impl View> {
    let current = current(cx);
    let back = uri(cx).path_and_query().map_or("/", |path| path.as_str()).to_owned();
    Ok(view! {
        <form method="post" action=(action) class="flex gap-2 text-caption">
            <input type="hidden" name="back" value=(back)>
            for theme in Theme::ALL {
                <button
                    type="submit"
                    name="theme"
                    value=(theme.value())
                    aria-pressed=(if theme == current { "true" } else { "false" })
                    class=(class!(
                        "cursor-pointer hover:text-ink",
                        "text-ink underline" if theme == current,
                        "text-muted" if theme != current,
                    ))
                >
                    (theme.label())
                </button>
            }
        </form>
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("/ui/button", "/ui/button")]
    #[case("/ui/button?x=1", "/ui/button?x=1")]
    #[case("https://example.com", "/")]
    #[case("//example.com", "/")]
    #[case("/\\example.com", "/")]
    #[case("", "/")]
    fn back_stays_local(#[case] back: &str, #[case] expected: &str) {
        let change = ThemeChange { theme: Theme::Dark, back: back.to_owned() };
        assert_eq!(change.back(), expected);
    }
}
