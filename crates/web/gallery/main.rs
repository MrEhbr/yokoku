//! The component gallery: one page per component, stories in isolated canvases.
//! Run with `dx serve --bin gallery`, then open http://127.0.0.1:8080.

mod patterns;
mod stories;

use dioxus::prelude::*;
use yokoku_web::{components::theme_switch::ThemeSwitch, layout::document_head::DocumentHead};

use crate::{
    patterns::{import::ManualImport, live::LiveProgress},
    stories::*,
};

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Overview {},
    #[route("/ui/accordion")]
    AccordionStory {},
    #[route("/ui/alert")]
    AlertStory {},
    #[route("/ui/alert-dialog")]
    AlertDialogStory {},
    #[route("/ui/avatar")]
    AvatarStory {},
    #[route("/ui/badge")]
    BadgeStory {},
    #[route("/ui/breadcrumb")]
    BreadcrumbStory {},
    #[route("/ui/button")]
    ButtonStory {},
    #[route("/ui/card")]
    CardStory {},
    #[route("/ui/checkbox")]
    CheckboxStory {},
    #[route("/ui/dialog")]
    DialogStory {},
    #[route("/ui/dropdown-menu")]
    DropdownMenuStory {},
    #[route("/ui/field")]
    FieldStory {},
    #[route("/ui/hover-card")]
    HoverCardStory {},
    #[route("/ui/kbd")]
    KbdStory {},
    #[route("/ui/pagination")]
    PaginationStory {},
    #[route("/ui/progress")]
    ProgressStory {},
    #[route("/ui/radio-group")]
    RadioGroupStory {},
    #[route("/ui/select")]
    SelectStory {},
    #[route("/ui/separator")]
    SeparatorStory {},
    #[route("/ui/sheet")]
    SheetStory {},
    #[route("/ui/sidebar")]
    SidebarStory {},
    #[route("/ui/skeleton")]
    SkeletonStory {},
    #[route("/ui/spinner")]
    SpinnerStory {},
    #[route("/ui/switch")]
    SwitchStory {},
    #[route("/ui/table")]
    TableStory {},
    #[route("/ui/tabs")]
    TabsStory {},
    #[route("/ui/textarea")]
    TextareaStory {},
    #[route("/ui/toggle")]
    ToggleStory {},
    #[route("/ui/tooltip")]
    TooltipStory {},
    #[route("/status")]
    StatusStory {},
    #[route("/patterns/manual-import")]
    ManualImport {},
    #[route("/patterns/live-progress")]
    LiveProgress {},
}

/// Sidebar groups: title, then (label, page) per story.
fn nav() -> [(&'static str, Vec<(&'static str, Route)>); 3] {
    [
        ("Yokoku", vec![("Status", Route::StatusStory {})]),
        ("Patterns", vec![("Manual import", Route::ManualImport {}), ("Live progress", Route::LiveProgress {})]),
        (
            "Primitives",
            vec![
                ("Accordion", Route::AccordionStory {}),
                ("Alert", Route::AlertStory {}),
                ("Alert dialog", Route::AlertDialogStory {}),
                ("Avatar", Route::AvatarStory {}),
                ("Badge", Route::BadgeStory {}),
                ("Breadcrumb", Route::BreadcrumbStory {}),
                ("Button", Route::ButtonStory {}),
                ("Card", Route::CardStory {}),
                ("Checkbox", Route::CheckboxStory {}),
                ("Dialog", Route::DialogStory {}),
                ("Dropdown menu", Route::DropdownMenuStory {}),
                ("Field", Route::FieldStory {}),
                ("Hover card", Route::HoverCardStory {}),
                ("Kbd", Route::KbdStory {}),
                ("Pagination", Route::PaginationStory {}),
                ("Progress", Route::ProgressStory {}),
                ("Radio group", Route::RadioGroupStory {}),
                ("Select", Route::SelectStory {}),
                ("Separator", Route::SeparatorStory {}),
                ("Sheet", Route::SheetStory {}),
                ("Sidebar", Route::SidebarStory {}),
                ("Skeleton", Route::SkeletonStory {}),
                ("Spinner", Route::SpinnerStory {}),
                ("Switch", Route::SwitchStory {}),
                ("Table", Route::TableStory {}),
                ("Tabs", Route::TabsStory {}),
                ("Textarea", Route::TextareaStory {}),
                ("Toggle", Route::ToggleStory {}),
                ("Tooltip", Route::TooltipStory {}),
            ],
        ),
    ]
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Title { "Yokoku components" }
        DocumentHead {}
        Router::<Route> {}
    }
}

#[component]
fn Shell() -> Element {
    rsx! {
        div { class: "md:flex",
            aside { class: "border-b border-line md:sticky md:top-0 md:h-svh md:w-60 md:shrink-0 md:overflow-y-auto md:border-r md:border-b-0",
                div { class: "flex items-center justify-between gap-2 px-5 py-4",
                    Link {
                        to: Route::Overview {},
                        class: "font-mono text-xl tracking-tight",
                        "yokoku"
                    }
                    ThemeSwitch {}
                }
                nav {
                    aria_label: "Components",
                    class: "max-h-56 overflow-y-auto px-3 pb-4 md:max-h-none",
                    for (group, entries) in nav() {
                        p { key: "{group}", class: "yk-kicker px-2 pt-4 pb-1", "{group}" }
                        for (label, route) in entries {
                            Link {
                                key: "{label}",
                                to: route,
                                class: "block px-2 py-1.5 hover:bg-subtle",
                                active_class: "bg-subtle font-medium",
                                "{label}"
                            }
                        }
                    }
                }
            }
            main { class: "min-w-0 flex-1 px-5 py-8 sm:px-8",
                div { class: "mx-auto max-w-4xl", Outlet::<Route> {} }
            }
        }
    }
}

#[component]
fn Overview() -> Element {
    rsx! {
        p { class: "yk-kicker mb-4", "Design system" }
        h1 { class: "yk-page-title sm:text-display",
            "The "
            mark { class: "yk-highlight", "Paper" }
            " components."
        }
        p { class: "mt-4 max-w-prose text-muted",
            "Every component with its variants and states. Primitives are Dioxus Components restyled to Paper; \
             Yokoku components compose them."
        }
        for (group, entries) in nav() {
            h2 { key: "{group}", class: "mt-10 mb-4 font-mono text-section", "{group}" }
            div { class: "grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4",
                for (label, route) in entries {
                    Link {
                        key: "{label}",
                        to: route,
                        class: "border border-line px-3 py-2 hover:border-control hover:bg-subtle",
                        "{label}"
                    }
                }
            }
        }
    }
}

/// A story page: the component's name, what it's for, and its import path, then its stories.
#[component]
pub(crate) fn StoryPage(name: String, path: String, summary: String, children: Element) -> Element {
    rsx! {
        header { class: "border-b border-line pb-6",
            h1 { class: "yk-page-title", "{name}" }
            p { class: "mt-2 max-w-prose text-muted", "{summary}" }
            p { class: "mt-3",
                code { class: "yk-code bg-subtle px-1.5 py-0.5", "use yokoku_web::components::{path}" }
            }
        }
        div { class: "flex flex-col gap-8 pt-8", {children} }
    }
}

/// One story: a titled canvas.
#[component]
pub(crate) fn Story(title: String, children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-2",
            h2 { class: "yk-kicker", "{title}" }
            div { class: "border border-line bg-canvas p-6", {children} }
        }
    }
}
