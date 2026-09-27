use topcoat::{
    Result,
    view::{Child, View, class, component, view},
};

use crate::head::head;

/// A primary destination in the top navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Destination {
    Library,
    Upcoming,
    Downloads,
    Activity,
    Settings,
}

impl Destination {
    const ALL: [Self; 5] = [Self::Library, Self::Upcoming, Self::Downloads, Self::Activity, Self::Settings];

    fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Upcoming => "Upcoming",
            Self::Downloads => "Downloads",
            Self::Activity => "Activity",
            Self::Settings => "Settings",
        }
    }

    fn href(self) -> &'static str {
        match self {
            Self::Library => "/",
            Self::Upcoming => "/upcoming",
            Self::Downloads => "/downloads",
            Self::Activity => "/activity",
            Self::Settings => "/settings",
        }
    }
}

/// The whole document: `<head>`, the top navigation, the page, and the metadata
/// attribution footer.
///
/// `current` marks the active destination. `theme` is the resolved appearance setting,
/// `"light"` or `"dark"`; `None` follows the system.
///
/// ```ignore
/// view! {
///     app_shell(title: "Library", current: Some(Destination::Library), page_header(title: "Library"))
/// }
/// ```
#[component]
pub async fn app_shell(
    title: &str,
    #[default] current: Option<Destination>,
    #[default] theme: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme)>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(title) " · Yokoku"</title>
                topcoat::dev::script()
                head()
            </head>
            <body class="flex min-h-svh flex-col">
                <a href="#main" class="sr-only focus:not-sr-only focus:block focus:p-3">"Skip to content"</a>
                <header class="border-b border-ink">
                    <div class="mx-auto flex max-w-content flex-wrap items-center gap-x-8 gap-y-2 px-5 pt-3 sm:px-8">
                        <a href="/" class="py-2 font-mono text-xl tracking-tight">
                            "yokoku " <span class="text-caption text-muted">"予告"</span>
                        </a>
                        <nav aria-label="Main" class="-mb-px flex flex-wrap">
                            for destination in Destination::ALL {
                                <a
                                    href=(destination.href())
                                    aria-current=((current == Some(destination)).then_some("page"))
                                    class=(class!(
                                        "flex min-h-11 shrink-0 items-center border-b-2 px-3 text-body \
                                         transition-colors hover:bg-subtle hover:text-ink",
                                        "border-ink text-ink" if current == Some(destination),
                                        "border-transparent text-muted" if current != Some(destination),
                                    ))
                                >
                                    (destination.label())
                                </a>
                            }
                        </nav>
                    </div>
                </header>
                <main id="main" class="mx-auto w-full max-w-content flex-1 px-5 pb-12 sm:px-8">(child)</main>
                <footer class="border-t border-line">
                    <div class="mx-auto flex max-w-content flex-col gap-1 px-5 py-5 text-caption text-muted sm:px-8">
                        <p>
                            "This product uses TMDB and the TMDB APIs but is not endorsed, certified, \
                             or otherwise approved by TMDB."
                        </p>
                        <p>
                            "Metadata provided by "
                            <a href="https://thetvdb.com" class="underline">"TheTVDB"</a>
                            "."
                        </p>
                    </div>
                </footer>
            </body>
        </html>
    })
}
