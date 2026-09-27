use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// Classes for the checkbox input that forms the switch track.
const SWITCH: StaticClass = class!(
    "peer h-5 w-9 shrink-0 appearance-none border border-control \
     bg-subtle transition-colors checked:border-ink checked:bg-ink \
     disabled:pointer-events-none",
);

/// Classes that position the thumb at either end of the track according to the checked
/// state.
const THUMB: StaticClass = class!(
    "pointer-events-none absolute top-1/2 left-[3px] size-3.5 -translate-y-1/2 \
     bg-control transition-transform peer-checked:translate-x-4 peer-checked:bg-canvas \
     motion-reduce:transition-none",
);

/// An on/off control for a setting.
///
/// Uses a native checkbox with the `switch` role. Pass `checked` in `attrs` for the
/// initial state. Classes apply to the wrapper, while other attributes and event
/// handlers go on the `<input>`.
///
/// ```ignore
/// view! {
///     <div class="flex items-center gap-2">
///         switch(attrs: attributes! { id="airplane-mode" checked="" })
///         label(attrs: attributes! { for="airplane-mode" }, "Airplane mode")
///     </div>
/// }
/// ```
#[component]
pub async fn switch(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The thumb cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling overlaid on the track,
    // slid to the far end by the input's `peer` state while checked.
    Ok(view! {
        <span
            class=(class!(
                "peer relative inline-flex shrink-0 has-[:disabled]:opacity-45",
                attrs.remove("class"),
            ))
        >
            <input type="checkbox" role="switch" class=(SWITCH) (attrs)>
            <span class=(THUMB)></span>
        </span>
    })
}
