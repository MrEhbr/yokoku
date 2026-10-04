use dioxus::prelude::*;
use yokoku_domain::ItemId;

use crate::{api::downloads::free_space, format::space, layout::LiveDownloads};

/// The space left in the download folder, and unless `download_only`, in the root folder of
/// `item`, or on every disk holding a root folder without one; read again each time the
/// downloads change.
#[component]
pub fn FreeSpace(item: Option<ItemId>, #[props(default)] download_only: bool) -> Element {
    let LiveDownloads(latest) = use_context();
    let read = use_resource(use_reactive!(|item| {
        latest.read();
        free_space(item)
    }));
    let Some(Ok(left)) = &*read.read() else { return rsx! {} };
    let download = match left.download {
        Some(download) => format!("Download folder: {}", space(download)),
        None => "Download folder: unknown, Transmission not reachable".to_owned(),
    };
    let library =
        left.library.iter().filter(|_| !download_only).map(|(names, disk)| format!("{names}: {}", space(*disk)));
    let parts: Vec<String> = std::iter::once(download).chain(library).collect();
    rsx! {
        p { class: "text-caption text-muted", {parts.join(" · ")} }
    }
}
