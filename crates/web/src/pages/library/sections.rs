use std::collections::BTreeMap;

use dioxus::prelude::*;

use super::{View, grid::PosterGrid, table::EntryTable};
use crate::api::library::{Entry, Kind};

/// One section per root folder; a single root shows without a heading.
#[component]
pub(super) fn RootSections(entries: Vec<Entry>, view: View) -> Element {
    let mut groups = by_root(entries);
    if groups.len() == 1 {
        let (_, entries) = groups.remove(0);
        return shown(entries, view);
    }
    rsx! {
        div { class: "flex flex-col gap-12",
            for (root, entries) in groups {
                section { key: "{root}",
                    h2 { class: "flex flex-wrap items-baseline gap-x-2 text-section font-medium",
                        "{entries[0].root_name}"
                        span { class: "text-caption font-normal text-muted", "{entries.len()}" }
                    }
                    div { class: "mt-4", {shown(entries, view)} }
                }
            }
        }
    }
}

fn shown(entries: Vec<Entry>, view: View) -> Element {
    match view {
        View::Grid => rsx! {
            PosterGrid { entries }
        },
        View::Table => rsx! {
            EntryTable { entries }
        },
    }
}

/// Series roots, then movie roots, each by path; entries keep their order.
fn by_root(entries: Vec<Entry>) -> Vec<(String, Vec<Entry>)> {
    let mut groups: BTreeMap<(bool, String), Vec<Entry>> = BTreeMap::new();
    for entry in entries {
        groups.entry((entry.status.kind() == Kind::Movie, entry.root.clone())).or_default().push(entry);
    }
    groups.into_iter().map(|((_, root), entries)| (root, entries)).collect()
}

#[cfg(test)]
mod tests {
    use yokoku_domain::{ItemId, MovieId, SeriesId};

    use super::by_root;
    use crate::api::library::{Entry, FileCount, Status};

    fn entry(title: &str, status: Status, root: &str) -> Entry {
        let id = match status {
            Status::Continuing | Status::OnBreak | Status::Ended => ItemId::Series(SeriesId::generate()),
            Status::Announced | Status::InCinemas | Status::Released => ItemId::Movie(MovieId::generate()),
        };
        Entry {
            id,
            title: title.into(),
            year: None,
            status,
            files: FileCount::default(),
            next_release: None,
            poster: None,
            root: root.into(),
            root_name: root.into(),
        }
    }

    #[test]
    fn groups_series_roots_then_movie_roots_keeping_entry_order() {
        let entries = vec![
            entry("Zeta", Status::Released, "/media/movies"),
            entry("Beta", Status::Ended, "/media/tv"),
            entry("Alpha", Status::Continuing, "/media/anime"),
            entry("Gamma", Status::Continuing, "/media/tv"),
            entry("Delta", Status::Announced, "/media/cinema"),
            entry("Alpha", Status::Released, "/media/movies"),
        ];

        let groups: Vec<(String, Vec<String>)> = by_root(entries)
            .into_iter()
            .map(|(root, entries)| (root, entries.into_iter().map(|entry| entry.title).collect()))
            .collect();

        assert_eq!(
            groups,
            [
                ("/media/anime".into(), vec!["Alpha".into()]),
                ("/media/tv".into(), vec!["Beta".into(), "Gamma".into()]),
                ("/media/cinema".into(), vec!["Delta".into()]),
                ("/media/movies".into(), vec!["Zeta".into(), "Alpha".into()]),
            ]
        );
    }
}
