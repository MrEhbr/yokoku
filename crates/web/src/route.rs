use dioxus::prelude::*;

use crate::{layout::Shell, pages::home::Home};

#[derive(Routable, Clone, PartialEq)]
pub(crate) enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
}

#[component]
pub fn App() -> Element {
    rsx! { Router::<Route> {} }
}
