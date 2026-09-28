use dioxus::prelude::*;
use dioxus_primitives::{
    avatar::{self, AvatarState},
    dioxus_attributes::attributes,
    merge_attributes,
};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum AvatarSize {
    /// A compact avatar for dense lists.
    Sm,
    /// The standard avatar size.
    #[default]
    Md,
    /// A prominent avatar for profile headers.
    Lg,
}

impl AvatarSize {
    fn class(self) -> &'static str {
        match self {
            Self::Sm => "size-8 text-caption",
            Self::Md => "size-10 text-body",
            Self::Lg => "size-12 text-body",
        }
    }
}

/// The props for the [`Avatar`] root component.
#[derive(Props, Clone, PartialEq)]
pub struct AvatarProps {
    /// Callback when image loads successfully.
    #[props(default)]
    pub on_load: Option<EventHandler<()>>,

    /// Callback when image fails to load.
    #[props(default)]
    pub on_error: Option<EventHandler<()>>,

    /// Callback when the avatar state changes.
    #[props(default)]
    pub on_state_change: Option<EventHandler<AvatarState>>,

    #[props(default)]
    pub size: AvatarSize,

    /// Additional attributes for the avatar element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The fallback content shown while the image is loading or if it fails to load.
    pub children: Element,
}

/// A circular image with an optional fallback. Corners stay square, per Paper.
#[component]
pub fn Avatar(props: AvatarProps) -> Element {
    let size = props.size.class();
    let class = format!(
        "relative flex shrink-0 overflow-hidden data-[state=loading]:bg-subtle data-[state=empty]:bg-subtle {size}"
    );
    let base = attributes!(span { class });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        avatar::Avatar {
            on_load: props.on_load,
            on_error: props.on_error,
            on_state_change: props.on_state_change,
            attributes: merged,
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct AvatarImageProps {
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    pub src: String,

    #[props(default)]
    pub alt: String,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

#[component]
pub fn AvatarImage(props: AvatarImageProps) -> Element {
    let base = attributes!(img { class: "absolute inset-0 size-full object-cover", draggable: "false" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        avatar::AvatarImage {
            id: props.id,
            src: props.src,
            alt: props.alt,
            attributes: merged,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct AvatarFallbackProps {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    pub children: Element,
}

#[component]
pub fn AvatarFallback(props: AvatarFallbackProps) -> Element {
    let base = attributes!(span {
        class: "flex size-full items-center justify-center bg-subtle font-medium text-ink select-none",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        avatar::AvatarFallback { attributes: merged, {props.children} }
    }
}

/// The props for the [`ImageAvatar`] convenience component.
#[derive(Props, Clone, PartialEq)]
pub struct ImageAvatarProps {
    /// The image source URL.
    pub src: String,

    /// The image alt text.
    #[props(default)]
    pub alt: String,

    /// Callback when image loads successfully.
    #[props(default)]
    pub on_load: Option<EventHandler<()>>,

    /// Callback when image fails to load.
    #[props(default)]
    pub on_error: Option<EventHandler<()>>,

    /// Callback when the avatar state changes.
    #[props(default)]
    pub on_state_change: Option<EventHandler<AvatarState>>,

    #[props(default)]
    pub size: AvatarSize,

    /// Additional attributes for the avatar element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The fallback content shown while the image is loading or if it fails to load.
    pub children: Element,
}

/// An [`Avatar`] with an [`AvatarImage`]; pass the fallback (usually initials) as children.
#[component]
pub fn ImageAvatar(props: ImageAvatarProps) -> Element {
    rsx! {
        Avatar {
            on_load: props.on_load,
            on_error: props.on_error,
            on_state_change: props.on_state_change,
            size: props.size,
            attributes: props.attributes,
            AvatarImage { src: props.src, alt: props.alt }
            AvatarFallback { {props.children} }
        }
    }
}
