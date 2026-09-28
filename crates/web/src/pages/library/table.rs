use dioxus::prelude::*;

use super::fields::{Files, Lifecycle};
use crate::{
    api::library::Entry,
    components::table::{Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow},
    format::{date, year},
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
                        TableCell { class: "font-medium", "{entry.title}" }
                        TableCell { class: "tabular-nums", "{year(entry.year)}" }
                        TableCell { "{entry.status.kind().label()}" }
                        TableCell {
                            Lifecycle { status: entry.status }
                        }
                        TableCell {
                            Files { present: entry.has_files }
                        }
                        TableCell { class: "tabular-nums whitespace-nowrap",
                            {entry.next_release.map(date).unwrap_or_else(|| "—".to_owned())}
                        }
                    }
                }
            }
        }
    }
}
