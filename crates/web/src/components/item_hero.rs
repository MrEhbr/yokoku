use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, Copy};

use crate::{
    components::button::{Button, ButtonSize, ButtonVariant},
    hooks::use_copy_clipboard,
};

/// A detail page's header: the backdrop, 3:1 but no taller than 40% of the window and never
/// cropped past 5:1, with the logo over it, then the poster beside the title, a button that
/// copies it, and `children`. The logo shows only over a backdrop. Each image is an optional
/// URL; its frame shows while it loads.
#[component]
pub fn ItemHero(
    title: String,
    poster: Option<String>,
    backdrop: Option<String>,
    logo: Option<String>,
    children: Element,
) -> Element {
    let logo = logo.filter(|_| backdrop.is_some());
    let (copy, copied) = use_copy_clipboard(None);
    rsx! {
        header { class: "@container",
            if let Some(backdrop) = &backdrop {
                div { class: "relative h-[min(33.3cqw,max(40dvh,20cqw))] w-full overflow-hidden border border-ink bg-subtle shadow-paper",
                    img {
                        class: "size-full object-cover object-[center_30%]",
                        src: "{backdrop}",
                        alt: "",
                        decoding: "async",
                    }
                    if let Some(logo) = &logo {
                        div { class: "absolute inset-0 flex items-end bg-linear-to-t from-black/70 via-black/20 to-transparent p-4 sm:p-6",
                            img {
                                class: "max-h-12 max-w-[60%] object-contain object-left-bottom sm:max-h-24",
                                src: "{logo}",
                                alt: "",
                                decoding: "async",
                            }
                        }
                    }
                }
            }
            div { class: if backdrop.is_some() { "mt-6 flex gap-6" } else { "flex gap-6" },
                if let Some(poster) = &poster {
                    div { class: "hidden aspect-[2/3] w-40 shrink-0 self-start border border-ink bg-subtle shadow-paper sm:block",
                        img {
                            class: "size-full object-cover",
                            src: "{poster}",
                            alt: "",
                            decoding: "async",
                        }
                    }
                }
                div { class: "flex min-w-0 flex-1 flex-col gap-3",
                    div { class: "flex items-start gap-2",
                        h1 { class: "yk-page-title min-w-0 break-words", "{title}" }
                        Button {
                            variant: ButtonVariant::Quiet,
                            size: ButtonSize::Icon,
                            aria_label: "Copy title",
                            title: if copied() { "Copied" } else { "Copy title" },
                            onclick: {
                                let title = title.clone();
                                move |_| copy(&title)
                            },
                            if copied() {
                                Check { size: "1rem" }
                            } else {
                                Copy { size: "1rem" }
                            }
                        }
                    }
                    {children}
                }
            }
        }
    }
}
