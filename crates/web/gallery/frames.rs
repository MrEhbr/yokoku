//! Standalone documents embedded as iframes by stories that need the whole viewport.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{Child, View, attributes, component, view},
};
use yokoku_web::{
    components::{
        document_head::document_head,
        ui::{alert_dialog::*, button::*, checkbox::*, dialog::*, label::*, sheet::*, sidebar::*},
    },
    theme,
};

/// A bare document around one example.
#[component]
async fn frame(cx: &Cx, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme::current(cx).attribute())>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                document_head()
            </head>
            <body>(child)</body>
        </html>
    })
}

#[page("/frames/dialog")]
pub(crate) async fn dialog_frame() -> Result<impl View> {
    Ok(view! {
        frame(
            <p class="p-6 text-muted">"Page behind the dialog."</p>
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
                        button("Cancel")
                        button(variant: ButtonVariant::Primary, "Rename 8 files")
                    )
                )
            )
        )
    })
}

#[page("/frames/alert-dialog")]
pub(crate) async fn alert_dialog_frame() -> Result<impl View> {
    Ok(view! {
        frame(
            alert_dialog(
                open: true,
                attrs: attributes! { aria-labelledby="replace-title" },
                dialog_content(
                    dialog_header(
                        dialog_title(attrs: attributes! { id="replace-title" }, "Replace existing file?")
                        dialog_description("The existing 1.2 GB file is deleted and replaced by the new 1.4 GB file.")
                    )
                    dialog_footer(
                        button("Keep both")
                        button(variant: ButtonVariant::Danger, "Replace file")
                    )
                )
            )
        )
    })
}

#[page("/frames/sheet")]
pub(crate) async fn sheet_frame() -> Result<impl View> {
    Ok(view! {
        frame(
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
                    button("Close")
                )
            )
        )
    })
}

#[page("/frames/sidebar")]
pub(crate) async fn sidebar_frame() -> Result<impl View> {
    Ok(view! {
        frame(
            sidebar_provider(
                sidebar(
                    open: true,
                    mobile_open: false,
                    sidebar_header(<span class="px-2 font-mono text-xl tracking-tight">"yokoku"</span>)
                    sidebar_content(
                        sidebar_group(
                            sidebar_group_label("Media")
                            sidebar_group_content(
                                sidebar_menu(
                                    sidebar_menu_item(sidebar_menu_button(href: Some("#"), active: true, icon(data: iconify_icon!("lucide:library")) <span>"Library"</span>))
                                    sidebar_menu_item(sidebar_menu_button(href: Some("#"), icon(data: iconify_icon!("lucide:calendar")) <span>"Upcoming"</span>))
                                    sidebar_menu_item(
                                        sidebar_menu_button(href: Some("#"), icon(data: iconify_icon!("lucide:download")) <span>"Downloads"</span>)
                                        sidebar_menu_badge("3")
                                    )
                                )
                            )
                        )
                        sidebar_group(
                            sidebar_group_label("System")
                            sidebar_group_content(
                                sidebar_menu(
                                    sidebar_menu_item(sidebar_menu_button(href: Some("#"), icon(data: iconify_icon!("lucide:settings")) <span>"Settings"</span>))
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
        )
    })
}
