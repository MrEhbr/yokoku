use topcoat::{
    Result,
    view::{View, component, view},
};

/// The metadata providers' attribution, required on every page that shows metadata.
#[component]
pub async fn attribution() -> Result<impl View> {
    Ok(view! {
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
    })
}
