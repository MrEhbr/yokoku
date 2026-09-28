use dioxus::prelude::*;

use crate::{layout::Shell, pages::library::Library};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Library {},
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
