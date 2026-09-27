use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::components::{empty_state::empty_state, page_header::page_header};

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        page_header(title: "Settings")
        empty_state(title: "Not built yet.")
    })
}
