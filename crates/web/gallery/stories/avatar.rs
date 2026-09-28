use dioxus::prelude::*;
use yokoku_web::components::avatar::{Avatar, AvatarFallback, AvatarImage, AvatarSize, ImageAvatar};

use crate::{Story, StoryPage};

#[component]
pub fn AvatarStory() -> Element {
    rsx! {
        StoryPage {
            name: "Avatar",
            path: "avatar",
            summary: "An image identifying a person or connection, with initials shown until it loads or if it fails.",
            Story { title: "Sizes with fallback",
                div { class: "flex items-center gap-3",
                    Avatar { size: AvatarSize::Sm,
                        AvatarFallback { "TE" }
                    }
                    Avatar {
                        AvatarFallback { "SV" }
                    }
                    Avatar { size: AvatarSize::Lg,
                        AvatarFallback { "YK" }
                    }
                }
            }
            Story { title: "Image with fallback",
                div { class: "flex items-center gap-3",
                    ImageAvatar {
                        src: "https://avatars.githubusercontent.com/u/66571940?s=96&v=4",
                        alt: "",
                        "EA"
                    }
                    Avatar {
                        AvatarImage { src: "", alt: "" }
                        AvatarFallback { "??" }
                    }
                }
            }
        }
    }
}
