use dioxus::prelude::*;
use dioxus_primitives::{
    alert_dialog::{
        self, AlertDialogActionProps, AlertDialogActionsProps, AlertDialogCancelProps, AlertDialogDescriptionProps,
        AlertDialogRootProps, AlertDialogTitleProps,
    },
    dioxus_attributes::attributes,
    merge_attributes,
};

/// A dialog that interrupts to confirm a consequential action. Escape closes it; there is no
/// click-outside dismissal. Give it an [`AlertDialogTitle`], an [`AlertDialogDescription`], and
/// [`AlertDialogActions`].
#[component]
pub fn AlertDialog(props: AlertDialogRootProps) -> Element {
    let base = attributes!(div {
        class: "group/dialog fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-overlay p-4 \
                data-[state=open]:animate-fade-in data-[state=closed]:animate-fade-out",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogRoot {
            id: props.id,
            default_open: props.default_open,
            open: props.open,
            on_open_change: props.on_open_change,
            attributes: merged,
            alert_dialog::AlertDialogContent {
                class: "relative my-auto flex w-full max-w-dialog flex-col gap-4 border border-ink \
                                                            bg-surface p-5 text-ink shadow-dialog \
                                                            group-data-[state=open]/dialog:animate-popover-in \
                                                            group-data-[state=closed]/dialog:animate-popover-out \
                                                            motion-reduce:animate-none"
                    .to_string(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn AlertDialogTitle(props: AlertDialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "text-section font-medium" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogTitle { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogDescription(props: AlertDialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "text-body text-muted" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogDescription { attributes: merged, {props.children} }
    }
}

/// The dialog's actions, aligned right; they wrap when they don't fit.
#[component]
pub fn AlertDialogActions(props: AlertDialogActionsProps) -> Element {
    let base = attributes!(div { class: "flex flex-wrap items-center justify-end gap-2 border-t border-line pt-4" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogActions { attributes: merged, {props.children} }
    }
}

/// Dismisses the dialog without confirming.
#[component]
pub fn AlertDialogCancel(props: AlertDialogCancelProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 border \
                border-control px-3 py-1.5 text-body font-medium text-ink transition-colors \
                duration-120 ease-interface hover:bg-subtle",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogCancel { on_click: props.on_click, attributes: merged, {props.children} }
    }
}

/// Confirms the action and closes the dialog. Styled as destructive.
#[component]
pub fn AlertDialogAction(props: AlertDialogActionProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 border \
                border-danger px-3 py-1.5 text-body font-medium text-danger transition-colors \
                duration-120 ease-interface hover:bg-danger-soft",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        alert_dialog::AlertDialogAction { on_click: props.on_click, attributes: merged, {props.children} }
    }
}
