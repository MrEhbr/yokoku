use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// A movie or series in the library grid: poster, linked title, and one line per
/// status.
///
/// `meta` is the year and type, such as "2015 · Series". Pass each status as its own
/// child, such as lifecycle, file availability, and next release, never merged into
/// one. Only the title links to the detail page, so checkboxes and actions placed
/// beside the card stay separately reachable. Without a `poster` the title fills a
/// neutral placeholder. Lay cards out with
/// `grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-x-5 gap-y-8`.
///
/// ```ignore
/// view! {
///     media_card(
///         title: "The Expanse",
///         href: "/series/the-expanse",
///         meta: "2015 · Series",
///         poster: "/posters/the-expanse.jpg",
///         status(tone: Tone::Muted, label: "Ended")
///         status(tone: Tone::Success, label: "All files present")
///     )
/// }
/// ```
#[component]
pub async fn media_card(
    title: &str,
    href: &str,
    meta: &str,
    #[into]
    #[default]
    poster: Option<&str>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <article class=(class!("flex min-w-0 flex-col gap-3", attrs.remove("class"))) (attrs)>
            <div class="aspect-2/3 w-full overflow-hidden border border-ink bg-subtle shadow-paper">
                match poster {
                    Some(src) => {
                        <img src=(src) alt="" loading="lazy" decoding="async" class="size-full object-cover">
                    }
                    None => {
                        <div class="flex size-full items-end p-3 font-mono text-caption text-muted" aria-hidden="true">
                            (title)
                        </div>
                    }
                }
            </div>
            <div class="grid gap-1">
                <h3 class="font-medium leading-snug">
                    <a href=(href) class="hover:underline">(title)</a>
                </h3>
                <p class="font-mono text-caption text-muted">(meta)</p>
                <div class="flex flex-col items-start gap-0.5 empty:hidden">(child)</div>
            </div>
        </article>
    })
}
