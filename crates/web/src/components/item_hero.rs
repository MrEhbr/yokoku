use dioxus::prelude::*;

/// A detail page's header: the backdrop with the logo as the title, then the poster beside
/// `children`. The logo shows only over a backdrop, as logos are drawn for dark images; without
/// one the title is text. Each image is an optional URL; its frame shows while it loads.
#[component]
pub fn ItemHero(
    title: String,
    poster: Option<String>,
    backdrop: Option<String>,
    logo: Option<String>,
    children: Element,
) -> Element {
    let logo = logo.filter(|_| backdrop.is_some());
    rsx! {
        header {
            if let Some(backdrop) = &backdrop {
                div { class: "relative aspect-[3/1] w-full overflow-hidden border border-ink bg-subtle shadow-paper",
                    img {
                        class: "size-full object-cover object-top",
                        src: "{backdrop}",
                        alt: "",
                        decoding: "async",
                    }
                    if let Some(logo) = &logo {
                        div { class: "absolute inset-0 flex items-end bg-linear-to-t from-black/70 via-black/20 to-transparent p-4 sm:p-6",
                            h1 {
                                img {
                                    class: "max-h-12 max-w-[60%] object-contain object-left-bottom sm:max-h-24",
                                    src: "{logo}",
                                    alt: "{title}",
                                    decoding: "async",
                                }
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
                    if logo.is_none() {
                        h1 { class: "yk-page-title break-words", "{title}" }
                    }
                    {children}
                }
            }
        }
    }
}
