use dioxus::prelude::*;

use crate::api::system::version;

#[component]
pub fn Home() -> Element {
    let version = use_server_future(version)?;
    let version = match &*version.read() {
        Some(Ok(version)) => version.clone(),
        Some(Err(error)) => format!("error: {error}"),
        None => String::new(),
    };
    rsx! {
        h1 { class: "yk-page-title", "Yokoku" }
        p { class: "mt-2 text-muted", "Server version {version}" }
    }
}
