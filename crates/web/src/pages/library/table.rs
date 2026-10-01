use dioxus::prelude::*;

use super::fields::Files;
use crate::{
    api::library::Entry,
    components::{
        item_status::Lifecycle,
        table::{Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow},
    },
    format::{date, year},
    route::Route,
};

#[component]
pub(super) fn EntryTable(entries: Vec<Entry>) -> Element {
    let count = entries.len();
    rsx! {
        Table {
            TableCaption { "{count} items" }
            TableHeader {
                TableRow {
                    TableHead { "Title" }
                    TableHead { "Year" }
                    TableHead { "Type" }
                    TableHead { "Status" }
                    TableHead { "Files" }
                    TableHead { "Next release" }
                }
            }
            TableBody {
                for entry in entries {
                    TableRow { key: "{entry.id:?}",
                        TableCell { class: "font-medium",
                            Link {
                                class: "hover:underline",
                                to: Route::item(entry.id),
                                "{entry.title}"
                            }
                        }
                        TableCell { class: "tabular-nums", "{year(entry.year)}" }
                        TableCell { "{entry.status.kind().label()}" }
                        TableCell {
                            Lifecycle { status: entry.status }
                        }
                        TableCell {
                            Files { kind: entry.status.kind(), files: entry.files }
                        }
                        TableCell { class: "tabular-nums whitespace-nowrap",
                            {entry.next_release.map_or_else(|| "—".to_owned(), date)}
                        }
                    }
                }
            }
        }
    }
}
