pub mod document_head;

use dioxus::prelude::*;

use self::document_head::DocumentHead;

#[component]
pub fn Shell() -> Element {
    rsx! {
        document::Title { "Yokoku" }
        DocumentHead {}
        main { class: "p-8", Outlet::<crate::route::Route> {} }
    }
}
