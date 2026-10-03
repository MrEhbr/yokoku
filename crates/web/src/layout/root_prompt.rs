use dioxus::prelude::*;

use crate::{
    api::{
        library::Kind,
        settings::{Root, roots},
    },
    components::alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
    route::{Route, SettingsPart},
};

/// Asks for a root folder in Settings while there is none of `kind`, or none at all without one.
#[component]
pub(crate) fn RootPrompt(kind: Option<Kind>) -> Element {
    let listed = use_resource(roots);
    let Some(Ok(listed)) = &*listed.read() else { return rsx! {} };
    if !missing(listed, kind) {
        return rsx! {};
    }
    let title = match kind {
        Some(Kind::Series) => "No root folder for series",
        Some(Kind::Movie) => "No root folder for movies",
        None => "No root folders yet",
    };
    rsx! {
        Alert { variant: AlertVariant::Warning, class: "mt-6",
            AlertTitle { "{title}" }
            AlertDescription {
                "Series and movies are kept in root folders. Add one in "
                Link {
                    class: "underline",
                    to: Route::Settings { part: SettingsPart::RootFolders },
                    "Settings"
                }
                " to add items."
            }
        }
    }
}

/// No root folder holds `kind`, or with `None` no root folder exists.
fn missing(roots: &[Root], kind: Option<Kind>) -> bool {
    !roots.iter().any(|root| kind.is_none_or(|kind| root.kind == kind))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::missing;
    use crate::api::{library::Kind, settings::Root};

    fn root(kind: Kind) -> Root {
        Root { kind, path: "/media".to_owned(), name: "media".to_owned(), configured: false, items: 0 }
    }

    #[rstest]
    #[case::none_at_all(&[], None, true)]
    #[case::any_kind(&[Kind::Movie], None, false)]
    #[case::other_kind(&[Kind::Movie], Some(Kind::Series), true)]
    #[case::same_kind(&[Kind::Movie, Kind::Series], Some(Kind::Series), false)]
    fn a_root_folder_is_missing_when_none_holds_the_kind(
        #[case] kinds: &[Kind],
        #[case] kind: Option<Kind>,
        #[case] expected: bool,
    ) {
        let roots: Vec<Root> = kinds.iter().copied().map(root).collect();

        assert_eq!(missing(&roots, kind), expected);
    }
}
