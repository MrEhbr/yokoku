use dioxus::prelude::*;

use crate::components::alert::{Alert, AlertDescription, AlertTitle, AlertVariant};

/// A page's data failing to load: "`subject` could not be loaded".
#[component]
pub fn LoadFailed(subject: &'static str, #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    rsx! {
        Alert { variant: AlertVariant::Danger, attributes,
            AlertTitle { "{subject} could not be loaded" }
            AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
        }
    }
}
