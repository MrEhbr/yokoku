//! A component gallery: one page per component, stories in isolated canvases.
//! Run with `just web gallery`, then open http://127.0.0.1:3000.

pub(crate) mod components;
pub(crate) mod frames;
pub(crate) mod patterns;
pub(crate) mod ui;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, see_other},
        href, page,
        request::uri,
        route,
    },
    runtime::RouterBuilderRuntimeExt,
    view::{Child, View, class, component, view},
};
use yokoku_web::{
    components::document_head::document_head,
    theme::{self, ThemeChange, theme_switch},
};

/// Resolves a story page's URL.
type Link = fn(&Cx) -> String;

/// A sidebar group: its title, then (label, link) per story page.
type Group = (&'static str, &'static [(&'static str, Link)]);

const NAV: &[Group] = &[
    (
        "Yokoku",
        &[
            ("Attribution", |cx| href!(components::attribution_story).resolve(cx)),
            ("Empty state", |cx| href!(components::empty_state_story).resolve(cx)),
            ("Job progress", |cx| href!(components::job_progress_story).resolve(cx)),
            ("Media card", |cx| href!(components::media_card_story).resolve(cx)),
            ("Page header", |cx| href!(components::page_header_story).resolve(cx)),
            ("Rename row", |cx| href!(components::rename_row_story).resolve(cx)),
            ("Selection bar", |cx| href!(components::selection_bar_story).resolve(cx)),
            ("Status", |cx| href!(components::status_story).resolve(cx)),
        ],
    ),
    ("Patterns", &[("Interactivity", |cx| href!(patterns::interactivity_story).resolve(cx))]),
    (
        "Primitives",
        &[
            ("Accordion", |cx| href!(ui::accordion_story).resolve(cx)),
            ("Alert", |cx| href!(ui::alert_story).resolve(cx)),
            ("Alert dialog", |cx| href!(ui::alert_dialog_story).resolve(cx)),
            ("Avatar", |cx| href!(ui::avatar_story).resolve(cx)),
            ("Badge", |cx| href!(ui::badge_story).resolve(cx)),
            ("Breadcrumb", |cx| href!(ui::breadcrumb_story).resolve(cx)),
            ("Button", |cx| href!(ui::button_story).resolve(cx)),
            ("Card", |cx| href!(ui::card_story).resolve(cx)),
            ("Checkbox", |cx| href!(ui::checkbox_story).resolve(cx)),
            ("Dialog", |cx| href!(ui::dialog_story).resolve(cx)),
            ("Dropdown menu", |cx| href!(ui::dropdown_menu_story).resolve(cx)),
            ("Field", |cx| href!(ui::field_story).resolve(cx)),
            ("Hover card", |cx| href!(ui::hover_card_story).resolve(cx)),
            ("Input", |cx| href!(ui::input_story).resolve(cx)),
            ("Kbd", |cx| href!(ui::kbd_story).resolve(cx)),
            ("Label", |cx| href!(ui::label_story).resolve(cx)),
            ("Pagination", |cx| href!(ui::pagination_story).resolve(cx)),
            ("Progress", |cx| href!(ui::progress_story).resolve(cx)),
            ("Radio group", |cx| href!(ui::radio_group_story).resolve(cx)),
            ("Select", |cx| href!(ui::select_story).resolve(cx)),
            ("Separator", |cx| href!(ui::separator_story).resolve(cx)),
            ("Sheet", |cx| href!(ui::sheet_story).resolve(cx)),
            ("Sidebar", |cx| href!(ui::sidebar_story).resolve(cx)),
            ("Skeleton", |cx| href!(ui::skeleton_story).resolve(cx)),
            ("Spinner", |cx| href!(ui::spinner_story).resolve(cx)),
            ("Switch", |cx| href!(ui::switch_story).resolve(cx)),
            ("Table", |cx| href!(ui::table_story).resolve(cx)),
            ("Tabs", |cx| href!(ui::tabs_story).resolve(cx)),
            ("Textarea", |cx| href!(ui::textarea_story).resolve(cx)),
            ("Toggle", |cx| href!(ui::toggle_story).resolve(cx)),
            ("Tooltip", |cx| href!(ui::tooltip_story).resolve(cx)),
        ],
    ),
];

#[tokio::main]
async fn main() {
    let router: Router = Router::builder()
        .discover()
        .app_context(patterns::Demo::new())
        .assets(AssetBundle::load().unwrap())
        .cookies()
        .runtime()
        .build();
    topcoat::start(router).await.unwrap();
}

/// Stores the posted theme, then returns to the page that posted it.
#[route(POST "/theme")]
async fn set_theme(cx: &Cx, Form(change): Form<ThemeChange>) -> Result<SeeOther> {
    theme::remember(cx, change.theme);
    Ok(see_other(change.back()))
}

/// The gallery document: sidebar of stories beside the current page.
#[component]
async fn chrome(cx: &Cx, child: Child<'_>) -> Result<impl View> {
    let current = uri(cx).path();
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme::current(cx).attribute())>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Yokoku components"</title>
                topcoat::dev::script()
                topcoat::runtime::script()
                document_head()
            </head>
            <body class="md:flex">
                <aside class="border-b border-line md:sticky md:top-0 md:h-svh md:w-60 md:shrink-0 md:overflow-y-auto md:border-r md:border-b-0">
                    <div class="flex items-center justify-between gap-2 px-5 py-4">
                        <a href=(href!(overview).resolve(cx)) class="font-mono text-xl tracking-tight">"yokoku"</a>
                        theme_switch(action: href!(set_theme).resolve(cx))
                    </div>
                    <nav aria-label="Components" class="max-h-56 overflow-y-auto px-3 pb-4 md:max-h-none">
                        for (group, entries) in NAV {
                            <p class="yk-kicker px-2 pt-4 pb-1">(*group)</p>
                            for (label, link) in entries.iter() {
                                let link = link(cx);
                                <a
                                    href=(link.clone())
                                    aria-current=((current == link).then_some("page"))
                                    class=(class!(
                                        "block px-2 py-1 text-body hover:bg-subtle",
                                        "bg-subtle font-medium text-ink" if current == link,
                                        "text-muted" if current != link,
                                    ))
                                >
                                    (*label)
                                </a>
                            }
                        }
                    </nav>
                </aside>
                <main class="min-w-0 flex-1 px-5 py-8 sm:px-8">
                    <div class="mx-auto max-w-4xl">(child)</div>
                </main>
                <script>"for (const box of document.querySelectorAll('[data-indeterminate]')) box.indeterminate = true;"</script>
            </body>
        </html>
    })
}

/// One component's page: name, summary, import path, then its stories.
#[component]
async fn story_page(name: &str, path: &str, summary: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        chrome(
            <header class="border-b border-line pb-6">
                <h1 class="yk-page-title">(name)</h1>
                <p class="mt-2 max-w-prose text-muted">(summary)</p>
                <p class="mt-3"><code class="yk-code bg-subtle px-1.5 py-0.5">"use yokoku_web::components::" (path)</code></p>
            </header>
            <div class="flex flex-col gap-8 pt-8">(child)</div>
        )
    })
}

/// One labelled example, rendered in its own canvas.
#[component]
async fn story(title: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class="flex flex-col gap-2">
            <h2 class="yk-kicker">(title)</h2>
            <div class="border border-line bg-canvas p-6">(child)</div>
        </section>
    })
}

/// An example that needs the whole viewport, rendered in an iframe of `frame`.
#[component]
async fn story_frame(title: &str, frame: String, #[default(24)] height: u16) -> Result<impl View> {
    Ok(view! {
        <section class="flex flex-col gap-2">
            <h2 class="yk-kicker">(title)</h2>
            <iframe
                src=(frame)
                title=(title)
                style=(format!("height: {height}rem"))
                class="w-full border border-line bg-canvas"
            ></iframe>
        </section>
    })
}

#[page("/")]
async fn overview(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        chrome(
            <p class="yk-kicker mb-4">"Design system"</p>
            <h1 class="yk-page-title sm:text-display">"The " <mark class="yk-highlight">"Paper"</mark> " components."</h1>
            <p class="mt-4 max-w-prose text-muted">
                "Every component with its variants and states. Yokoku components compose the primitives; \
                 the primitives are topcoat-ui restyled to Paper."
            </p>
            for (group, entries) in NAV {
                <h2 class="mt-10 mb-4 font-mono text-section">(*group)</h2>
                <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
                    for (label, link) in entries.iter() {
                        <a href=(link(cx)) class="border border-line px-3 py-2 hover:border-control hover:bg-subtle">
                            (*label)
                        </a>
                    }
                </div>
            }
        )
    })
}
