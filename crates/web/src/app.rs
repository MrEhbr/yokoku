mod activity;
mod downloads;
mod settings;
mod upcoming;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    router::{
        Router, RouterBuilderDiscoverExt, Slot, StatusCode, error::NotFoundError, href, layout, module_router,
        not_found, page,
    },
    view::{View, class, component, error_boundary, view},
};

use crate::{
    components::{
        attribution::attribution, document_head::document_head, empty_state::empty_state, page_header::page_header,
    },
    theme,
};

not_found!();

/// The web application. `assets` is the bundle built for the running binary.
pub fn router(assets: AssetBundle) -> Router {
    module_router!().discover().assets(assets).cookies().build()
}

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let library = href!(page);
    let upcoming = href!(upcoming::page);
    let downloads = href!(downloads::page);
    let activity = href!(activity::page);
    let settings = href!(settings::page);

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme::current(cx).attribute())>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Yokoku"</title>
                topcoat::dev::script()
                document_head()
            </head>
            <body class="flex min-h-svh flex-col">
                <a href="#main" class="sr-only focus:not-sr-only focus:block focus:p-3">"Skip to content"</a>
                <header class="border-b border-ink">
                    <div class="mx-auto flex max-w-content flex-wrap items-center gap-x-8 gap-y-2 px-5 pt-3 sm:px-8">
                        <a href=(library.resolve(cx)) class="py-2 font-mono text-xl tracking-tight">
                            "yokoku " <span class="text-caption text-muted">"予告"</span>
                        </a>
                        <nav aria-label="Main" class="-mb-px flex flex-wrap">
                            nav_link(href: library.resolve(cx), current: library.is_current(cx), "Library")
                            nav_link(href: upcoming.resolve(cx), current: upcoming.is_current(cx), "Upcoming")
                            nav_link(href: downloads.resolve(cx), current: downloads.is_current(cx), "Downloads")
                            nav_link(href: activity.resolve(cx), current: activity.is_current(cx), "Activity")
                            nav_link(href: settings.resolve(cx), current: settings.is_current(cx), "Settings")
                        </nav>
                    </div>
                </header>
                <main id="main" class="mx-auto w-full max-w-content flex-1 px-5 pb-12 sm:px-8">
                    error_boundary(
                        fallback: |error| {
                            if error.downcast_ref::<NotFoundError>().is_none() {
                                return Err(error);
                            }
                            Ok(view! {
                                (StatusCode::NOT_FOUND)
                                page_header(title: "Not found")
                                empty_state(title: "This page does not exist.")
                            })
                        },
                        (slot)
                    )
                </main>
                attribution()
            </body>
        </html>
    })
}

#[component]
async fn nav_link(href: String, current: bool, child: topcoat::view::Child<'_>) -> Result<impl View> {
    Ok(view! {
        <a
            href=(href)
            aria-current=(current.then_some("page"))
            class=(class!(
                "flex min-h-11 shrink-0 items-center border-b-2 px-3 text-body \
                 transition-colors hover:bg-subtle hover:text-ink",
                "border-ink text-ink" if current,
                "border-transparent text-muted" if !current,
            ))
        >
            (child)
        </a>
    })
}

#[page]
async fn page() -> Result<impl View> {
    Ok(view! {
        page_header(title: "Library")
        empty_state(
            title: "Your library is empty.",
            description: "Add a movie or series to start tracking releases."
        )
    })
}
