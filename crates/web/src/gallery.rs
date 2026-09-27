//! Every Paper component on one page: `topcoat dev -p yokoku-web --bin gallery`.
//!
//! `?theme=light` or `?theme=dark` pins the scheme; otherwise the system preference applies.

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{Router, RouterBuilderDiscoverExt, page, request::uri},
    view::{Child, View, attributes, component, view},
};
use yokoku_web::components::{
    app_shell::{Destination, app_shell},
    document_head::document_head,
    empty_state::empty_state,
    job_progress::job_progress,
    media_card::media_card,
    page_header::page_header,
    rename_row::rename_row,
    selection_bar::selection_bar,
    status::{Tone, status},
    ui::{
        accordion::{accordion, accordion_content, accordion_item, accordion_trigger},
        alert::{AlertVariant, alert, alert_description, alert_title},
        alert_dialog::alert_dialog,
        avatar::{AvatarSize, avatar, avatar_fallback},
        badge::{BadgeVariant, badge},
        breadcrumb::{
            breadcrumb, breadcrumb_ellipsis, breadcrumb_item, breadcrumb_link, breadcrumb_list, breadcrumb_page,
            breadcrumb_separator,
        },
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        checkbox::checkbox,
        dialog::{dialog, dialog_content, dialog_description, dialog_footer, dialog_header, dialog_title},
        dropdown_menu::{
            dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label, dropdown_menu_separator,
            dropdown_menu_sub, dropdown_menu_sub_content, dropdown_menu_sub_trigger, dropdown_menu_trigger,
        },
        field::{field, field_description, field_error, field_group, field_label, field_legend, field_set},
        hover_card::{hover_card, hover_card_content},
        input::input,
        kbd::{kbd, kbd_group},
        label::label,
        pagination::{
            pagination, pagination_content, pagination_ellipsis, pagination_item, pagination_link, pagination_next,
            pagination_previous,
        },
        progress::progress,
        radio_group::{radio_group, radio_group_item},
        select::select,
        separator::{SeparatorOrientation, separator},
        sheet::{SheetSide, sheet, sheet_content},
        sidebar::{
            sidebar, sidebar_content, sidebar_group, sidebar_group_content, sidebar_group_label, sidebar_header,
            sidebar_inset, sidebar_menu, sidebar_menu_badge, sidebar_menu_button, sidebar_menu_item, sidebar_provider,
        },
        skeleton::skeleton,
        spinner::spinner,
        switch::switch,
        table::{table, table_body, table_caption, table_cell, table_head, table_header, table_row},
        tabs::{tabs, tabs_content, tabs_list, tabs_trigger},
        textarea::textarea,
        toggle::{ToggleKind, ToggleSize, toggle, toggle_group},
        tooltip::{tooltip, tooltip_content},
    },
};

#[tokio::main]
async fn main() {
    let router = Router::builder().assets(AssetBundle::load().unwrap()).discover().build();
    topcoat::start(router).await.unwrap();
}

fn theme(cx: &Cx) -> Option<&'static str> {
    let query = uri(cx).query().unwrap_or("");
    if query.contains("theme=dark") {
        Some("dark")
    } else if query.contains("theme=light") {
        Some("light")
    } else {
        None
    }
}

#[component]
async fn shell(cx: &Cx, title: &str, #[default] current: Option<Destination>, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        app_shell(
            title: title,
            current: current,
            theme: theme(cx),
            <p class="flex gap-3 pt-4 text-caption">
                "Appearance:"
                <a href="?" class="underline">"System"</a>
                <a href="?theme=light" class="underline">"Light"</a>
                <a href="?theme=dark" class="underline">"Dark"</a>
            </p>
            (child)
            <script>"for (const box of document.querySelectorAll('[data-indeterminate]')) box.indeterminate = true;"</script>
        )
    })
}

#[component]
async fn section(title: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class="border-t border-line py-8">
            <h2 class="mb-5 font-mono text-section">(title)</h2>
            <div class="flex flex-col gap-5">(child)</div>
        </section>
    })
}

#[page("/")]
async fn index() -> Result<impl View> {
    Ok(view! {
        shell(title: "Paper components",
            <div class="py-8 sm:py-12">
                <p class="yk-kicker mb-4">"Design system"</p>
                <h1 class="yk-page-title sm:text-display">"The " <mark class="yk-highlight">"Paper"</mark> " components."</h1>
                <p class="mt-4 flex flex-wrap gap-3 text-body">
                    <a class="underline" href="/dialog">"Dialog"</a>
                    <a class="underline" href="/alert-dialog">"Alert dialog"</a>
                    <a class="underline" href="/sheet">"Sheet"</a>
                    <a class="underline" href="/sidebar">"Sidebar"</a>
                    <a class="underline" href="/widgets">"Yokoku widgets"</a>
                </p>
            </div>

            section(title: "Button",
                <div class="flex flex-wrap items-center gap-2">
                    button(variant: ButtonVariant::Primary, "Import 12 files")
                    button("Cancel")
                    button(variant: ButtonVariant::Quiet, "Skip")
                    button(variant: ButtonVariant::Danger, "Delete 3 files")
                    button(variant: ButtonVariant::Primary, attrs: attributes! { disabled="" }, "Disabled")
                    button(size: ButtonSize::Sm, "Small")
                    button(
                        size: ButtonSize::Icon,
                        attrs: attributes! { aria-label="Refresh" },
                        icon(data: iconify_icon!("lucide:refresh-cw"))
                    )
                    <a href="/" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>"Link as button"</a>
                    button(
                        variant: ButtonVariant::Primary,
                        attrs: attributes! { aria-busy="true" disabled="" },
                        spinner()
                        "Importing"
                    )
                </div>
            )

            section(title: "Badge",
                <div class="flex flex-wrap items-center gap-2">
                    badge("Neutral")
                    badge(variant: BadgeVariant::Outline, "S01E03")
                    badge(variant: BadgeVariant::Success, icon(data: iconify_icon!("lucide:check")) "Downloaded")
                    badge(variant: BadgeVariant::Warning, icon(data: iconify_icon!("lucide:circle-alert")) "Missing")
                    badge(variant: BadgeVariant::Danger, icon(data: iconify_icon!("lucide:x")) "Failed")
                    badge(variant: BadgeVariant::Info, icon(data: iconify_icon!("lucide:calendar")) "Upcoming")
                </div>
            )

            section(title: "Alert",
                alert(
                    icon(data: iconify_icon!("lucide:info"))
                    alert_title("Metadata refresh scheduled")
                    alert_description("Runs nightly at 03:00.")
                )
                alert(
                    variant: AlertVariant::Info,
                    icon(data: iconify_icon!("lucide:calendar"))
                    alert_title("4 episodes air this week")
                )
                alert(
                    variant: AlertVariant::Warning,
                    icon(data: iconify_icon!("lucide:circle-alert"))
                    alert_title("3 files need review")
                    alert_description("Detection could not match them to an episode.")
                )
                alert(
                    variant: AlertVariant::Danger,
                    icon(data: iconify_icon!("lucide:triangle-alert"))
                    alert_title("Import failed")
                    alert_description("The destination disk is full. Free space and retry.")
                )
            )

            section(title: "Fields",
                <div class="grid gap-6 md:grid-cols-2">
                    field_set(
                        field_legend("Import")
                        field_group(
                            field(
                                field_label(attrs: attributes! { for="root" }, "Series root folder")
                                input(attrs: attributes! { id="root" value="/media/shows" })
                                field_description("New series are created here.")
                            )
                            field(
                                field_label(attrs: attributes! { for="search" }, "Search library")
                                input(attrs: attributes! { id="search" placeholder="Title or year" })
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
                    field_set(
                        field_legend("Numbering")
                        field_group(
                            field(
                                field_label(attrs: attributes! { for="order" }, "Episode numbering")
                                select(
                                    attrs: attributes! { id="order" },
                                    <option>"Aired order"</option>
                                    <option selected="">"DVD order"</option>
                                    <option>"Absolute numbering, for long-running anime series with a very long label"</option>
                                    <option disabled="">"Unavailable order"</option>
                                )
                            )
                            field(
                                field_label(attrs: attributes! { for="notes" }, "Notes")
                                textarea(attrs: attributes! { id="notes" placeholder="Anything to remember" })
                            )
                        )
                    )
                </div>
            )

            section(title: "Selection",
                <div class="grid gap-6 md:grid-cols-3">
                    <div class="flex flex-col gap-3">
                        <div class="flex items-center gap-2">
                            checkbox(attrs: attributes! { id="monitored" checked="" })
                            label(attrs: attributes! { for="monitored" }, "Monitored")
                        </div>
                        <div class="flex items-center gap-2">
                            checkbox(attrs: attributes! { id="include" })
                            label(attrs: attributes! { for="include" }, "Include file in this operation")
                        </div>
                        <div class="flex items-center gap-2">
                            checkbox(attrs: attributes! { id="mixed" data-indeterminate="" })
                            label(attrs: attributes! { for="mixed" }, "Some episodes monitored")
                        </div>
                        <div class="flex items-center gap-2">
                            checkbox(attrs: attributes! { id="locked" disabled="" })
                            label(attrs: attributes! { for="locked" }, "Disabled")
                        </div>
                    </div>
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
                            radio_group_item(attrs: attributes! { id="move" name="mode" value="move" })
                            label(attrs: attributes! { for="move" }, "Move")
                        </div>
                    )
                    <div class="flex flex-col gap-3">
                        <div class="flex items-center gap-2">
                            switch(attrs: attributes! { id="auto" checked="" })
                            label(attrs: attributes! { for="auto" }, "Import certain matches")
                        </div>
                        <div class="flex items-center gap-2">
                            switch(attrs: attributes! { id="subs" })
                            label(attrs: attributes! { for="subs" }, "Rename subtitles")
                        </div>
                        toggle_group(
                            toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="list" checked="" }, "List")
                            toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="week" }, "Week")
                            toggle(kind: ToggleKind::Exclusive, size: ToggleSize::Sm, attrs: attributes! { name="view" value="month" }, "Month")
                        )
                    </div>
                </div>
            )

            section(title: "Table",
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
                        table_row(
                            table_cell(checkbox(attrs: attributes! { aria-label="Select S01E03" }))
                            table_cell(<span class="yk-code">"S01E03"</span>)
                            table_cell(<span class="text-muted">"—"</span>)
                            table_cell(badge(variant: BadgeVariant::Info, "Airs Oct 4"))
                            table_cell(attrs: attributes! { class="text-right tabular-nums" }, "—")
                        )
                    )
                )
                pagination(
                    pagination_content(
                        pagination_item(pagination_previous(attrs: attributes! { href="?page=1" }))
                        pagination_item(pagination_link(attrs: attributes! { href="?page=1" }, "1"))
                        pagination_item(pagination_link(active: true, attrs: attributes! { href="?page=2" }, "2"))
                        pagination_item(pagination_ellipsis())
                        pagination_item(pagination_next(attrs: attributes! { href="?page=3" }))
                    )
                )
            )

            section(title: "Tabs and navigation",
                tabs(
                    tabs_list(
                        tabs_trigger(active: true, attrs: attributes! { href="#" }, "Library")
                        tabs_trigger(attrs: attributes! { href="#" }, "Upcoming")
                        tabs_trigger(attrs: attributes! { href="#" }, "Downloads")
                        tabs_trigger(attrs: attributes! { href="#" }, "Activity")
                    )
                    tabs_content(<p class="text-muted">"Tab panel content."</p>)
                )
                breadcrumb(
                    breadcrumb_list(
                        breadcrumb_item(breadcrumb_link(attrs: attributes! { href="#" }, "Library"))
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_ellipsis())
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_page("Season 1"))
                    )
                )
                <div class="flex flex-wrap items-start gap-6">
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
                    tooltip(
                        button(size: ButtonSize::Icon, attrs: attributes! { aria-label="Copy path" }, icon(data: iconify_icon!("lucide:copy")))
                        tooltip_content("Copy path")
                    )
                    hover_card(
                        <a href="#" class="underline">"The Expanse"</a>
                        hover_card_content(
                            <p class="font-medium">"The Expanse (2015)"</p>
                            <p class="text-caption text-muted">"Ended · 6 seasons · 62 episodes"</p>
                        )
                    )
                    kbd_group(kbd("Ctrl") kbd("K"))
                </div>
            )

            section(title: "Progress and loading",
                <div class="grid max-w-md gap-4">
                    <div class="grid gap-1.5">
                        <div class="flex justify-between text-caption"><span>"Downloading"</span><span class="tabular-nums">"62% · 4.1 MB/s · 3 min"</span></div>
                        progress(value: 62.0)
                    </div>
                    <div class="grid gap-1.5">
                        <span class="text-caption">"Scanning library"</span>
                        progress()
                    </div>
                    <div class="flex items-center gap-2 text-muted">spinner() "Loading"</div>
                    <div class="flex items-center gap-3">
                        skeleton(attrs: attributes! { class="h-24 w-16" })
                        <div class="grid flex-1 gap-2">
                            skeleton(attrs: attributes! { class="h-4 w-3/4" })
                            skeleton(attrs: attributes! { class="h-4 w-1/2" })
                        </div>
                    </div>
                </div>
            )

            section(title: "Card, accordion, avatar, separator",
                <div class="grid gap-6 md:grid-cols-2">
                    card(
                        card_header(
                            card_title("Jellyfin")
                            card_description("Connected · 10.10.3")
                        )
                        card_content(<p>"Libraries refresh after every import."</p>)
                        card_footer(button(size: ButtonSize::Sm, "Test connection") button(variant: ButtonVariant::Quiet, size: ButtonSize::Sm, "Disconnect"))
                    )
                    <div class="flex flex-col gap-5">
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
                        <div class="flex items-center gap-3">
                            avatar(size: AvatarSize::Sm, avatar_fallback("TE"))
                            avatar(avatar_fallback("SV"))
                            avatar(size: AvatarSize::Lg, avatar_fallback("YK"))
                            <div class="flex h-6 items-center gap-3 text-caption">
                                "Movies"
                                separator(orientation: SeparatorOrientation::Vertical)
                                "Series"
                            </div>
                        </div>
                        separator()
                    </div>
                </div>
            )
        )
    })
}

#[page("/dialog")]
async fn dialog_page() -> Result<impl View> {
    Ok(view! {
        shell(title: "Dialog",
            <p class="py-8">"Page behind the dialog."</p>
            dialog(
                open: true,
                attrs: attributes! { aria-labelledby="rename-title" },
                dialog_content(
                    dialog_header(
                        dialog_title(attrs: attributes! { id="rename-title" }, "Rename existing files")
                        dialog_description("8 files change. Subtitles are renamed with their video.")
                    )
                    <div class="grid gap-2">
                        <p class="yk-code text-muted">"The.Expanse.S01E01.1080p.mkv"</p>
                        <p class="yk-code">"The Expanse (2015) - S01E01 - Dulcinea.mkv"</p>
                    </div>
                    dialog_footer(
                        <a href="/" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>"Cancel"</a>
                        button(variant: ButtonVariant::Primary, "Rename 8 files")
                    )
                )
            )
        )
    })
}

#[page("/alert-dialog")]
async fn alert_dialog_page() -> Result<impl View> {
    Ok(view! {
        shell(title: "Alert dialog",
            alert_dialog(
                open: true,
                attrs: attributes! { aria-labelledby="replace-title" },
                dialog_content(
                    dialog_header(
                        dialog_title(attrs: attributes! { id="replace-title" }, "Replace existing file?")
                        dialog_description("The existing 1.2 GB file is deleted and replaced by the new 1.4 GB file.")
                    )
                    dialog_footer(
                        <a href="/" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>"Keep both"</a>
                        button(variant: ButtonVariant::Danger, "Replace file")
                    )
                )
            )
        )
    })
}

#[page("/sheet")]
async fn sheet_page() -> Result<impl View> {
    Ok(view! {
        shell(title: "Sheet",
            sheet(
                open: true,
                attrs: attributes! { aria-label="Filters" },
                sheet_content(
                    side: SheetSide::Right,
                    <h2 class="text-section font-medium">"Filters"</h2>
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="only-missing" checked="" })
                        label(attrs: attributes! { for="only-missing" }, "Only missing")
                    </div>
                    <a href="/" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>"Close"</a>
                )
            )
        )
    })
}

#[page("/sidebar")]
async fn sidebar_page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme(cx))>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Sidebar"</title>
                topcoat::dev::script()
                document_head()
            </head>
            <body>
                sidebar_provider(
                    sidebar(
                        open: true,
                        mobile_open: false,
                        sidebar_header(<a href="/" class="px-2 font-mono text-xl tracking-tight">"yokoku"</a>)
                        sidebar_content(
                            sidebar_group(
                                sidebar_group_label("Media")
                                sidebar_group_content(
                                    sidebar_menu(
                                        sidebar_menu_item(sidebar_menu_button(href: Some("/sidebar"), active: true, icon(data: iconify_icon!("lucide:library")) <span>"Library"</span>))
                                        sidebar_menu_item(sidebar_menu_button(href: Some("/sidebar"), icon(data: iconify_icon!("lucide:calendar")) <span>"Upcoming"</span>))
                                        sidebar_menu_item(
                                            sidebar_menu_button(href: Some("/sidebar"), icon(data: iconify_icon!("lucide:download")) <span>"Downloads"</span>)
                                            sidebar_menu_badge("3")
                                        )
                                        sidebar_menu_item(sidebar_menu_button(href: Some("/sidebar"), icon(data: iconify_icon!("lucide:activity")) <span>"Activity"</span>))
                                    )
                                )
                            )
                            sidebar_group(
                                sidebar_group_label("System")
                                sidebar_group_content(
                                    sidebar_menu(
                                        sidebar_menu_item(sidebar_menu_button(href: Some("/sidebar"), icon(data: iconify_icon!("lucide:settings")) <span>"Settings"</span>))
                                    )
                                )
                            )
                        )
                    )
                    sidebar_inset(
                        <div class="p-8">
                            <h1 class="yk-page-title">"Library"</h1>
                            <p class="mt-2 text-muted">"Page content next to the sidebar."</p>
                        </div>
                    )
                )
            </body>
        </html>
    })
}

const POSTER: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2 3'%3E%3Cdefs%3E%3ClinearGradient id='g' x2='0' y2='1'%3E%3Cstop stop-color='%23355e82'/%3E%3Cstop offset='1' stop-color='%23e99db4'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='2' height='3' fill='url(%23g)'/%3E%3C/svg%3E";

#[page("/widgets")]
async fn widgets_page() -> Result<impl View> {
    Ok(view! {
        shell(title: "Library", current: Some(Destination::Library),
            page_header(
                title: "Library",
                button("Rename files")
                button(variant: ButtonVariant::Primary, "Add a movie or series")
            )

            section(title: "Media cards",
                <div class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-x-5 gap-y-8">
                    media_card(
                        title: "The Expanse", href: "#", meta: "2015 · Series", poster: POSTER,
                        status(tone: Tone::Muted, label: "Ended")
                        status(tone: Tone::Success, label: "All files present")
                    )
                    media_card(
                        title: "Orbital", href: "#", meta: "2024 · Series",
                        status(tone: Tone::Info, label: "Continuing")
                        status(tone: Tone::Warning, label: "2 missing")
                        <span class="font-mono text-caption text-muted">"Next: S02E04 · Oct 4"</span>
                    )
                    media_card(
                        title: "Night Train to a Very Long Title That Wraps", href: "#", meta: "2026 · Movie",
                        status(tone: Tone::Info, label: "In cinemas")
                        <span class="font-mono text-caption text-muted">"Digital: Nov 12"</span>
                    )
                </div>
            )

            section(title: "Status taxonomy",
                <div class="flex flex-wrap gap-x-6 gap-y-3">
                    status(tone: Tone::Success, label: "Downloaded")
                    status(tone: Tone::Warning, label: "Missing")
                    status(tone: Tone::Info, label: "Not yet aired")
                    status(tone: Tone::Muted, label: "Not monitored")
                    status(tone: Tone::Success, label: "Certain")
                    status(tone: Tone::Warning, label: "Guess")
                    status(tone: Tone::Danger, label: "Import failed")
                </div>
            )

            section(title: "Rename preview",
                selection_bar(summary: "2 of 3 files selected")
                <div>
                    rename_row(
                        old: "The.Expanse.S01E01.1080p.WEB.mkv",
                        checkbox_attrs: attributes! { name="file" value="1" checked="" },
                        <span class="yk-code break-all">"The Expanse (2015) - S01E01 - Dulcinea.mkv"</span>
                    )
                    rename_row(
                        old: "The.Expanse.S01E02.1080p.WEB.mkv",
                        checkbox_attrs: attributes! { name="file" value="2" checked="" },
                        <span class="yk-code break-all">"The Expanse (2015) - S01E02 - The Big Empty.mkv"</span>
                    )
                    rename_row(
                        old: "The.Expanse.S01E03.1080p.WEB.en.srt",
                        checkbox_attrs: attributes! { name="file" value="3" },
                        <span class="yk-code break-all">"The Expanse (2015) - S01E03 - Remember the Cant.en.srt"</span>
                    )
                </div>
            )

            section(title: "Import review",
                selection_bar(
                    summary: "2 files selected",
                    <div class="grid gap-1.5">
                        label(attrs: attributes! { for="bulk-season" class="text-caption" }, "Season")
                        select(attrs: attributes! { id="bulk-season" class="w-auto" }, <option>"Mixed"</option> <option>"Season 1"</option> <option>"Season 2"</option>)
                    </div>
                    button("Detect again")
                    button("Assign episodes in order")
                )
                <div>
                    rename_row(
                        old: "orbital.ep1.mkv",
                        checkbox_attrs: attributes! { name="file" value="1" checked="" },
                        select(
                            attrs: attributes! { aria-label="New name for orbital.ep1.mkv" class="font-mono text-caption" },
                            <option selected="">"Orbital (2024) - S01E01 - Launch.mkv"</option>
                            <option>"Orbital (2024) - S01E02 - Drift.mkv"</option>
                        )
                    )
                    rename_row(
                        old: "orbital.bonus.mkv",
                        checkbox_attrs: attributes! { name="file" value="2" checked="" },
                        select(
                            attrs: attributes! { aria-label="New name for orbital.bonus.mkv" aria-invalid="true" class="font-mono text-caption" },
                            <option value="" selected="">"Choose the correct name…"</option>
                            <option>"Orbital (2024) - S00E01 - Behind the Launch.mkv"</option>
                        )
                    )
                </div>
                alert(variant: AlertVariant::Warning, alert_title("1 file needs an episode match before import."))
                <div class="flex justify-end gap-2">
                    button("Cancel")
                    button(variant: ButtonVariant::Primary, attrs: attributes! { disabled="" }, "Import 2 files")
                </div>
            )

            section(title: "Operations",
                <div class="grid max-w-xl gap-6">
                    job_progress(title: "Orbital · Season 1", detail: "64% · 4.1 MB/s · 2 min left", value: 64.0)
                    job_progress(title: "Scanning library", detail: "1,204 files checked")
                    job_progress(
                        title: "Night Train (2026)",
                        detail: "Stopped at 12%",
                        value: 12.0,
                        <div class="flex flex-wrap items-center justify-between gap-2">
                            status(tone: Tone::Danger, label: "Tracker unreachable")
                            button(size: ButtonSize::Sm, "Retry")
                        </div>
                    )
                </div>
            )

            section(title: "Empty states",
                empty_state(
                    title: "Your library is empty.",
                    description: "Add a movie or series to start tracking releases.",
                    button(variant: ButtonVariant::Primary, "Add a movie or series")
                )
                empty_state(
                    title: "No results match these filters.",
                    button("Clear filters")
                )
                empty_state(title: "No files need review.")
            )
        )
    })
}
