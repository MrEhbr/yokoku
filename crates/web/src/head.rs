use topcoat::{
    Result,
    font::{Font, fontsource::fontsource_font},
    tailwind,
    view::{View, component, view},
};

/// DM Sans, the reading and control face.
pub const SANS: Font = fontsource_font!(DM_SANS, weight: [400, 500], style: Normal, host: Asset);

/// IBM Plex Mono, for headings, numbers, dates and filenames.
pub const MONO: Font = fontsource_font!(IBM_PLEX_MONO, weight: [400, 500], style: Normal, host: Asset);

/// The Paper fonts and stylesheet, for a page's `<head>`.
///
/// The router must serve assets for the self-hosted fonts.
#[component]
pub async fn head() -> Result<impl View> {
    Ok(view! {
        topcoat::font::link(font: SANS)
        topcoat::font::link(font: MONO)
        <link rel="stylesheet" href=(tailwind::stylesheet!())>
    })
}
