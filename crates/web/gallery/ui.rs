//! Stories for the topcoat-ui primitives restyled to Paper.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{href, page},
    view::{View, attributes, view},
};
use yokoku_web::components::ui::{
    accordion::*, alert::*, avatar::*, badge::*, breadcrumb::*, button::*, card::*, checkbox::*, dropdown_menu::*,
    field::*, hover_card::*, input::*, kbd::*, label::*, pagination::*, progress::*, radio_group::*, select::*,
    separator::*, skeleton::*, spinner::*, switch::*, table::*, tabs::*, textarea::*, toggle::*, tooltip::*,
};

use crate::{story, story_frame, story_page};

#[page("/ui/accordion")]
pub(crate) async fn accordion_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Accordion", path: "ui::accordion", summary: "Disclosure sections built on <details>. Items sharing a name close each other.",
            story(title: "Exclusive items",
                accordion(
                    accordion_item(
                        attrs: attributes! { name="faq" },
                        accordion_trigger("What counts as missing?")
                        accordion_content("A monitored episode that has aired and has no file.")
                    )
                    accordion_item(
                        attrs: attributes! { name="faq" },
                        accordion_trigger("How are files renamed?")
                        accordion_content("From saved naming rules and existing matches.")
                    )
                )
            )
        )
    })
}

#[page("/ui/alert")]
pub(crate) async fn alert_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Alert", path: "ui::alert", summary: "A notice within the page: a rule and a soft fill in the status color.",
            story(title: "Neutral",
                alert(
                    icon(data: iconify_icon!("lucide:info"))
                    alert_title("Metadata refresh scheduled")
                    alert_description("Runs nightly at 03:00.")
                )
            )
            story(title: "Info, title only",
                alert(
                    variant: AlertVariant::Info,
                    icon(data: iconify_icon!("lucide:calendar"))
                    alert_title("4 episodes air this week")
                )
            )
            story(title: "Warning",
                alert(
                    variant: AlertVariant::Warning,
                    icon(data: iconify_icon!("lucide:circle-alert"))
                    alert_title("3 files need review")
                    alert_description("Detection could not match them to an episode.")
                )
            )
            story(title: "Danger",
                alert(
                    variant: AlertVariant::Danger,
                    icon(data: iconify_icon!("lucide:triangle-alert"))
                    alert_title("Import failed")
                    alert_description("The destination disk is full. Free space and retry.")
                )
            )
            story(title: "Without icon",
                alert(alert_title("Naming rules apply to new imports only."))
            )
        )
    })
}

#[page("/ui/alert-dialog")]
pub(crate) async fn alert_dialog_story(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        story_page(name: "Alert dialog", path: "ui::alert_dialog", summary: "A dialog that interrupts to confirm a consequential action.",
            story_frame(title: "Confirm replacement", frame: href!(crate::frames::alert_dialog_frame).resolve(cx))
        )
    })
}

#[page("/ui/avatar")]
pub(crate) async fn avatar_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Avatar", path: "ui::avatar", summary: "An image with a fallback shown until it loads or when it fails.",
            story(title: "Sizes with fallback",
                <div class="flex items-center gap-3">
                    avatar(size: AvatarSize::Sm, avatar_fallback("TE"))
                    avatar(avatar_fallback("SV"))
                    avatar(size: AvatarSize::Lg, avatar_fallback("YK"))
                </div>
            )
        )
    })
}

#[page("/ui/badge")]
pub(crate) async fn badge_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Badge", path: "ui::badge", summary: "A small tag for a label, count, or status. Pair a status variant with a label and an icon.",
            story(title: "Neutral and outline",
                <div class="flex flex-wrap items-center gap-2">
                    badge("Neutral")
                    badge(variant: BadgeVariant::Outline, "S01E03")
                </div>
            )
            story(title: "Status",
                <div class="flex flex-wrap items-center gap-2">
                    badge(variant: BadgeVariant::Success, icon(data: iconify_icon!("lucide:check")) "Downloaded")
                    badge(variant: BadgeVariant::Warning, icon(data: iconify_icon!("lucide:circle-alert")) "Missing")
                    badge(variant: BadgeVariant::Danger, icon(data: iconify_icon!("lucide:x")) "Failed")
                    badge(variant: BadgeVariant::Info, icon(data: iconify_icon!("lucide:calendar")) "Upcoming")
                </div>
            )
        )
    })
}

#[page("/ui/breadcrumb")]
pub(crate) async fn breadcrumb_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Breadcrumb", path: "ui::breadcrumb", summary: "The path from a top-level page to the current one.",
            story(title: "With ellipsis",
                breadcrumb(
                    breadcrumb_list(
                        breadcrumb_item(breadcrumb_link(attrs: attributes! { href="#" }, "Library"))
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_ellipsis())
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_page("Season 1"))
                    )
                )
            )
        )
    })
}

#[page("/ui/button")]
pub(crate) async fn button_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Button", path: "ui::button", summary: "One primary action per context; name the action and its count.",
            story(title: "Variants",
                <div class="flex flex-wrap items-center gap-2">
                    button(variant: ButtonVariant::Primary, "Import 12 files")
                    button("Cancel")
                    button(variant: ButtonVariant::Quiet, "Skip")
                    button(variant: ButtonVariant::Danger, "Delete 3 files")
                </div>
            )
            story(title: "Sizes",
                <div class="flex flex-wrap items-center gap-2">
                    button(size: ButtonSize::Sm, "Small")
                    button("Medium")
                    button(size: ButtonSize::Icon, attrs: attributes! { aria-label="Refresh" }, icon(data: iconify_icon!("lucide:refresh-cw")))
                </div>
            )
            story(title: "States",
                <div class="flex flex-wrap items-center gap-2">
                    button(variant: ButtonVariant::Primary, attrs: attributes! { disabled="" }, "Disabled")
                    button(variant: ButtonVariant::Primary, attrs: attributes! { aria-busy="true" disabled="" }, spinner() "Importing")
                </div>
            )
            story(title: "Link styled as a button",
                <a href="#" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>"Open settings"</a>
            )
        )
    })
}

#[page("/ui/card")]
pub(crate) async fn card_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Card", path: "ui::card", summary: "A bordered panel with header, content, and footer.",
            story(title: "Connection",
                <div class="max-w-sm">
                    card(
                        card_header(
                            card_title("Jellyfin")
                            card_description("Connected · 10.10.3")
                        )
                        card_content(<p>"Libraries refresh after every import."</p>)
                        card_footer(
                            button(size: ButtonSize::Sm, "Test connection")
                            button(variant: ButtonVariant::Quiet, size: ButtonSize::Sm, "Disconnect")
                        )
                    )
                </div>
            )
        )
    })
}

#[page("/ui/checkbox")]
pub(crate) async fn checkbox_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Checkbox", path: "ui::checkbox", summary: "A native checkbox. The indeterminate state is a DOM property set from script.",
            story(title: "States",
                <div class="flex flex-col gap-3">
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="monitored" checked="" })
                        label(attrs: attributes! { for="monitored" }, "Checked")
                    </div>
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="include" })
                        label(attrs: attributes! { for="include" }, "Unchecked")
                    </div>
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="mixed" data-indeterminate="" })
                        label(attrs: attributes! { for="mixed" }, "Indeterminate")
                    </div>
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="locked" disabled="" })
                        label(attrs: attributes! { for="locked" }, "Disabled")
                    </div>
                </div>
            )
        )
    })
}

#[page("/ui/dialog")]
pub(crate) async fn dialog_story(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        story_page(name: "Dialog", path: "ui::dialog", summary: "A modal panel over the page. Label it, keep one footer, and avoid nesting.",
            story_frame(title: "Rename preview", frame: href!(crate::frames::dialog_frame).resolve(cx))
        )
    })
}

#[page("/ui/dropdown-menu")]
pub(crate) async fn dropdown_menu_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Dropdown menu", path: "ui::dropdown_menu", summary: "A menu of actions opened from a trigger, with optional submenus.",
            story(title: "Bulk actions",
                <div class="h-72">
                    dropdown_menu(
                        dropdown_menu_trigger(
                            attrs: attributes! { class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md)) },
                            "Bulk actions"
                            icon(data: iconify_icon!("lucide:chevron-down"), attrs: attributes! { class="size-4 group-open:rotate-180" })
                        )
                        dropdown_menu_content(
                            dropdown_menu_label("3 selected")
                            dropdown_menu_item("Set series")
                            dropdown_menu_item("Set season")
                            dropdown_menu_sub(
                                dropdown_menu_sub_trigger("Detect again")
                                dropdown_menu_sub_content(
                                    dropdown_menu_item("Using aired order")
                                    dropdown_menu_item("Using absolute order")
                                )
                            )
                            dropdown_menu_separator()
                            dropdown_menu_item(attrs: attributes! { class="text-danger" }, "Remove from queue")
                        )
                    )
                </div>
            )
        )
    })
}

#[page("/ui/field")]
pub(crate) async fn field_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Field", path: "ui::field", summary: "Label, control, hint, and error, grouped into fieldsets.",
            story(title: "Fieldset",
                <div class="max-w-md">
                    field_set(
                        field_legend("Import")
                        field_group(
                            field(
                                field_label(attrs: attributes! { for="root" }, "Series root folder")
                                input(attrs: attributes! { id="root" value="/media/shows" })
                                field_description("New series are created here.")
                            )
                            field(
                                field_label(attrs: attributes! { for="format" }, "Series filename format")
                                input(attrs: attributes! {
                                    id="format" value="{Title} - S{season}E{episode}"
                                    aria-invalid="true" aria-describedby="format-error"
                                })
                                field_error(attrs: attributes! { id="format-error" }, "Add an {episode} token so files stay unique.")
                            )
                            field(
                                field_label(attrs: attributes! { for="disabled" }, "Disabled")
                                input(attrs: attributes! { id="disabled" value="Unavailable" disabled="" })
                            )
                        )
                    )
                </div>
            )
        )
    })
}

#[page("/ui/hover-card")]
pub(crate) async fn hover_card_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Hover card", path: "ui::hover_card", summary: "Supplementary details shown while hovering a trigger.",
            story(title: "Series preview (hover the link)",
                <div class="h-32">
                    hover_card(
                        <a href="#" class="underline">"The Expanse"</a>
                        hover_card_content(
                            <p class="font-medium">"The Expanse (2015)"</p>
                            <p class="text-caption text-muted">"Ended · 6 seasons · 62 episodes"</p>
                        )
                    )
                </div>
            )
        )
    })
}

#[page("/ui/input")]
pub(crate) async fn input_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Input", path: "ui::input", summary: "A text field with a control border; 16px text on narrow screens.",
            story(title: "States",
                <div class="grid max-w-md gap-3">
                    input(attrs: attributes! { aria-label="Filled" value="/media/shows" })
                    input(attrs: attributes! { aria-label="Placeholder" placeholder="Title or year" })
                    input(attrs: attributes! { aria-label="Invalid" value="{Title}" aria-invalid="true" })
                    input(attrs: attributes! { aria-label="Disabled" value="Unavailable" disabled="" })
                </div>
            )
        )
    })
}

#[page("/ui/kbd")]
pub(crate) async fn kbd_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Kbd", path: "ui::kbd", summary: "A keyboard key or shortcut.",
            story(title: "Shortcut", kbd_group(kbd("Ctrl") kbd("K")))
        )
    })
}

#[page("/ui/label")]
pub(crate) async fn label_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Label", path: "ui::label", summary: "A control's label; it dims with a disabled control.",
            story(title: "With a checkbox",
                <div class="flex items-center gap-2">
                    checkbox(attrs: attributes! { id="label-demo" })
                    label(attrs: attributes! { for="label-demo" }, "Rename subtitles with their video")
                </div>
            )
        )
    })
}

#[page("/ui/pagination")]
pub(crate) async fn pagination_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Pagination", path: "ui::pagination", summary: "Page links for long lists.",
            story(title: "Middle page",
                pagination(
                    pagination_content(
                        pagination_item(pagination_previous(attrs: attributes! { href="#" }))
                        pagination_item(pagination_link(attrs: attributes! { href="#" }, "1"))
                        pagination_item(pagination_link(active: true, attrs: attributes! { href="#" }, "2"))
                        pagination_item(pagination_ellipsis())
                        pagination_item(pagination_next(attrs: attributes! { href="#" }))
                    )
                )
            )
        )
    })
}

#[page("/ui/progress")]
pub(crate) async fn progress_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Progress", path: "ui::progress", summary: "A native progress bar. Without a value it is indeterminate, never 0%.",
            story(title: "Known", <div class="max-w-md">progress(value: 62.0, attrs: attributes! { aria-label="Download" })</div>)
            story(title: "Indeterminate", <div class="max-w-md">progress(attrs: attributes! { aria-label="Scan" })</div>)
        )
    })
}

#[page("/ui/radio-group")]
pub(crate) async fn radio_group_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Radio group", path: "ui::radio_group", summary: "One choice from a short list of options.",
            story(title: "Import mode",
                radio_group(
                    <div class="flex items-center gap-2">
                        radio_group_item(attrs: attributes! { id="hardlink" name="mode" value="hardlink" checked="" })
                        label(attrs: attributes! { for="hardlink" }, "Hard link")
                    </div>
                    <div class="flex items-center gap-2">
                        radio_group_item(attrs: attributes! { id="copy" name="mode" value="copy" })
                        label(attrs: attributes! { for="copy" }, "Copy")
                    </div>
                    <div class="flex items-center gap-2">
                        radio_group_item(attrs: attributes! { id="move" name="mode" value="move" disabled="" })
                        label(attrs: attributes! { for="move" }, "Move (disabled)")
                    </div>
                )
            )
        )
    })
}

#[page("/ui/select")]
pub(crate) async fn select_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Select", path: "ui::select", summary: "A native select with a styled picker where the browser supports it.",
            story(title: "Options, long label, disabled option",
                <div class="max-w-md">
                    select(
                        attrs: attributes! { aria-label="Episode numbering" },
                        <option>"Aired order"</option>
                        <option selected="">"DVD order"</option>
                        <option>"Absolute numbering, for long-running anime series with a very long label"</option>
                        <option disabled="">"Unavailable order"</option>
                    )
                </div>
            )
            story(title: "Invalid",
                <div class="max-w-md">
                    select(
                        attrs: attributes! { aria-label="New name" aria-invalid="true" },
                        <option value="" selected="">"Choose the correct name…"</option>
                    )
                </div>
            )
        )
    })
}

#[page("/ui/separator")]
pub(crate) async fn separator_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Separator", path: "ui::separator", summary: "A thin rule between groups of content.",
            story(title: "Horizontal", separator())
            story(title: "Vertical",
                <div class="flex h-6 items-center gap-3 text-caption">
                    "Movies"
                    separator(orientation: SeparatorOrientation::Vertical)
                    "Series"
                </div>
            )
        )
    })
}

#[page("/ui/sheet")]
pub(crate) async fn sheet_story(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        story_page(name: "Sheet", path: "ui::sheet", summary: "A panel that slides in from an edge of the viewport.",
            story_frame(title: "Filters from the right", frame: href!(crate::frames::sheet_frame).resolve(cx))
        )
    })
}

#[page("/ui/sidebar")]
pub(crate) async fn sidebar_story(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        story_page(name: "Sidebar", path: "ui::sidebar", summary: "A desktop panel that becomes a sheet below md.",
            story_frame(title: "Navigation", frame: href!(crate::frames::sidebar_frame).resolve(cx), height: 28)
        )
    })
}

#[page("/ui/skeleton")]
pub(crate) async fn skeleton_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Skeleton", path: "ui::skeleton", summary: "A placeholder with the shape of content that is loading.",
            story(title: "Media card",
                <div class="flex items-center gap-3">
                    skeleton(attrs: attributes! { class="h-24 w-16" })
                    <div class="grid flex-1 gap-2">
                        skeleton(attrs: attributes! { class="h-4 w-3/4" })
                        skeleton(attrs: attributes! { class="h-4 w-1/2" })
                    </div>
                </div>
            )
        )
    })
}

#[page("/ui/spinner")]
pub(crate) async fn spinner_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Spinner", path: "ui::spinner", summary: "An indeterminate loading indicator with an accessible label.",
            story(title: "Inline", <div class="flex items-center gap-2 text-muted">spinner() "Loading"</div>)
        )
    })
}

#[page("/ui/switch")]
pub(crate) async fn switch_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Switch", path: "ui::switch", summary: "An on/off setting that applies immediately.",
            story(title: "States",
                <div class="flex flex-col gap-3">
                    <div class="flex items-center gap-2">
                        switch(attrs: attributes! { id="auto" checked="" })
                        label(attrs: attributes! { for="auto" }, "Import certain matches")
                    </div>
                    <div class="flex items-center gap-2">
                        switch(attrs: attributes! { id="subs" })
                        label(attrs: attributes! { for="subs" }, "Rename subtitles")
                    </div>
                    <div class="flex items-center gap-2">
                        switch(attrs: attributes! { id="locked-switch" disabled="" })
                        label(attrs: attributes! { for="locked-switch" }, "Disabled")
                    </div>
                </div>
            )
        )
    })
}

#[page("/ui/table")]
pub(crate) async fn table_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Table", path: "ui::table", summary: "Dense data rows with quiet rules. Select rows with checkboxes, not row clicks.",
            story(title: "Episode files with selection",
                table(
                    table_caption("3 files in Season 1")
                    table_header(
                        table_row(
                            table_head(checkbox(attrs: attributes! { aria-label="Select all" data-indeterminate="" }))
                            table_head("Episode")
                            table_head("File")
                            table_head("Status")
                            table_head(attrs: attributes! { class="text-right" }, "Size")
                        )
                    )
                    table_body(
                        table_row(
                            attrs: attributes! { data-selected="true" },
                            table_cell(checkbox(attrs: attributes! { aria-label="Select S01E01" checked="" }))
                            table_cell(<span class="yk-code">"S01E01"</span>)
                            table_cell(<span class="yk-code">"Show (2024) - S01E01 - Pilot.mkv"</span>)
                            table_cell(badge(variant: BadgeVariant::Success, "Downloaded"))
                            table_cell(attrs: attributes! { class="text-right tabular-nums" }, "1.4 GB")
                        )
                        table_row(
                            table_cell(checkbox(attrs: attributes! { aria-label="Select S01E02" }))
                            table_cell(<span class="yk-code">"S01E02"</span>)
                            table_cell(<span class="text-muted">"—"</span>)
                            table_cell(badge(variant: BadgeVariant::Warning, "Missing"))
                            table_cell(attrs: attributes! { class="text-right tabular-nums" }, "—")
                        )
                    )
                )
            )
        )
    })
}

#[page("/ui/tabs")]
pub(crate) async fn tabs_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Tabs", path: "ui::tabs", summary: "Links that switch between panels, underlined when current.",
            story(title: "Views",
                tabs(
                    tabs_list(
                        tabs_trigger(active: true, attrs: attributes! { href="#" }, "List")
                        tabs_trigger(attrs: attributes! { href="#" }, "Week")
                        tabs_trigger(attrs: attributes! { href="#" }, "Month")
                    )
                    tabs_content(<p class="text-muted">"Tab panel content."</p>)
                )
            )
        )
    })
}

#[page("/ui/textarea")]
pub(crate) async fn textarea_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Textarea", path: "ui::textarea", summary: "A multi-line text field that grows with its content.",
            story(title: "Placeholder",
                <div class="max-w-md">textarea(attrs: attributes! { aria-label="Notes" placeholder="Anything to remember" })</div>
            )
        )
    })
}

#[page("/ui/toggle")]
pub(crate) async fn toggle_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Toggle", path: "ui::toggle", summary: "Pressable options; exclusive toggles in a group behave like radios.",
            story(title: "Exclusive group",
                toggle_group(
                    toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="list" checked="" }, "List")
                    toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="week" }, "Week")
                    toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="month" }, "Month")
                )
            )
        )
    })
}

#[page("/ui/tooltip")]
pub(crate) async fn tooltip_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Tooltip", path: "ui::tooltip", summary: "A short label shown on hover or focus.",
            story(title: "Icon button (hover it)",
                <div class="pt-10">
                    tooltip(
                        button(size: ButtonSize::Icon, attrs: attributes! { aria-label="Copy path" }, icon(data: iconify_icon!("lucide:copy")))
                        tooltip_content("Copy path")
                    )
                </div>
            )
        )
    })
}
