use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde_json::Value;

use crate::{
    api::{
        failure,
        settings::{Setting, reset_setting, save_setting},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        combobox::{Combobox, ComboboxEmpty, ComboboxOption},
        field::{Field, FieldError, FieldHint},
        input::Input,
        label::Label,
        select::{Select, SelectOption},
        switch::Switch,
    },
};

/// Choices beyond this many are searched rather than scrolled.
const LONG_CHOICE: usize = 12;

/// Values typed but not saved yet, by key, as `save_setting` takes them.
#[derive(Clone, Copy)]
pub(super) struct Unsaved(pub(super) Signal<BTreeMap<String, Value>>);

/// How a setting is edited.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Control {
    Text(&'static str),
    /// Shown masked; typing replaces it.
    Secret,
    /// JSON string values with their labels; a value in effect outside them is offered too.
    Choice(&'static [(&'static str, &'static str)]),
    Switch,
    /// Strings, typed comma-separated.
    List,
}

/// One setting: its control, a hint, and where its value comes from. Text is saved with Save or
/// Enter, a choice or switch at once; an empty text or list goes back to the config file.
#[component]
pub(super) fn SettingField(setting: Setting, label: &'static str, hint: &'static str, control: Control) -> Element {
    let mut current = use_signal(|| setting);
    let mut draft = use_signal(|| text(&current.read().value, control));
    let chosen = use_memo(move || Some(draft()));
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let key = current.read().key.clone();
    let id = format!("setting-{}", key.replace('.', "-"));
    let (hint_id, error_id) = (format!("{id}-hint"), format!("{id}-error"));
    let locked = current.read().from_env || saving();
    let changed = draft() != text(&current.read().value, control);
    let described = if error.read().is_some() { error_id.clone() } else { hint_id.clone() };
    let save = move |value: Value| async move {
        saving.set(true);
        error.set(None);
        let key = current.read().key.clone();
        let saved = if value.is_null() { reset_setting(key).await } else { save_setting(key, value).await };
        match saved {
            Ok(setting) => {
                draft.set(if control == Control::Secret { String::new() } else { text(&setting.value, control) });
                current.set(setting);
            },
            Err(failed) => {
                error.set(Some(failure(&failed)));
                if matches!(control, Control::Choice(_)) {
                    draft.set(text(&current.read().value, control));
                }
            },
        }
        saving.set(false);
    };
    let typed = move || match control {
        Control::List => Value::Array(
            draft().split(',').map(str::trim).filter(|item| !item.is_empty()).map(|item| item.into()).collect(),
        ),
        _ => Value::String(draft()),
    };
    let Unsaved(mut unsaved) = use_context();
    use_effect(move || {
        let key = current.read().key.clone();
        if draft() == text(&current.read().value, control) {
            unsaved.write().remove(&key);
        } else {
            unsaved.write().insert(key, typed());
        }
    });
    let setting = current();
    let variable = format!("APP__{}", setting.key.to_uppercase().replace('.', "__"));
    rsx! {
        Field {
            Label { html_for: "{id}", "{label}" }
            match control {
                Control::Switch => rsx! {
                    Switch {
                        id: "{id}",
                        checked: Some(setting.value == Value::Bool(true)),
                        disabled: locked,
                        aria_invalid: error.read().is_some(),
                        aria_describedby: "{described}",
                        on_checked_change: move |on: bool| {
                            spawn(save(Value::Bool(on)));
                        },
                    }
                },
                Control::Choice(choices) => {
                    let mut options: Vec<(String, String)> =
                        choices.iter().map(|(value, text)| (value.to_string(), text.to_string())).collect();
                    if !draft().is_empty() && !options.iter().any(|(value, _)| *value == draft()) {
                        options.insert(0, (draft(), draft()));
                    }
                    let placeholder =
                        options.iter().find(|(value, _)| *value == draft()).map(|(_, text)| text.clone()).unwrap_or_default();
                    let pick = move |next: Option<String>| {
                        let Some(next) = next.filter(|next| *next != draft()) else { return };
                        draft.set(next.clone());
                        spawn(save(Value::String(next)));
                    };
                    if options.len() > LONG_CHOICE {
                        rsx! {
                            Combobox::<String> {
                                id: "{id}",
                                value: Some(chosen.into()),
                                disabled: locked,
                                aria_describedby: "{described}",
                                on_value_change: pick,
                                ComboboxEmpty { "Nothing matches" }
                                for (index, (value, text)) in options.into_iter().enumerate() {
                                    ComboboxOption::<String> {
                                        key: "{value}",
                                        index,
                                        value,
                                        text_value: text.clone(),
                                        "{text}"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            Select::<String> {
                                id: "{id}",
                                value: Some(chosen.into()),
                                placeholder,
                                disabled: locked,
                                aria_invalid: error.read().is_some(),
                                aria_describedby: "{described}",
                                on_value_change: pick,
                                for (index, (value, text)) in options.into_iter().enumerate() {
                                    SelectOption::<String> {
                                        key: "{value}",
                                        index,
                                        value,
                                        text_value: text.clone(),
                                        "{text}"
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {
                    div { class: "flex gap-2",
                        Input {
                            id: "{id}",
                            class: if setting.key.starts_with("naming.") { "flex-1 yk-code" } else { "flex-1" },
                            r#type: if control == Control::Secret { "password" } else { "text" },
                            autocomplete: "off",
                            value: "{draft}",
                            placeholder: match (control, &setting.value) {
                                (Control::Secret, Value::String(masked)) => format!("Set ({masked}); type to replace it"),
                                (Control::Secret, _) => "Not set".to_owned(),
                                (Control::Text(placeholder), _) => placeholder.to_owned(),
                                _ => String::new(),
                            },
                            disabled: locked,
                            aria_invalid: error.read().is_some(),
                            aria_describedby: "{described}",
                            oninput: move |event: FormEvent| draft.set(event.value()),
                            onkeydown: move |event: KeyboardEvent| {
                                if event.key() == Key::Enter && changed && !locked {
                                    spawn(save(typed()));
                                }
                            },
                        }
                        Button {
                            disabled: locked || !changed,
                            aria_busy: saving(),
                            onclick: move |_| save(typed()),
                            "Save"
                        }
                    }
                },
            }
            if let Some(message) = error() {
                FieldError { id: "{error_id}", "{message}" }
            }
            FieldHint { id: "{hint_id}",
                "{hint}"
                if setting.from_env {
                    " Set by "
                    code { class: "yk-code", "{variable}" }
                    ", which takes precedence."
                } else if setting.stored {
                    " Saved here, over the config file. "
                    Button {
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Sm,
                        class: "-my-1 underline",
                        disabled: saving(),
                        onclick: move |_| save(Value::Null),
                        "Use the config file's value"
                    }
                }
            }
        }
    }
}

/// The value as the control edits it; a secret starts empty.
fn text(value: &Value, control: Control) -> String {
    match (control, value) {
        (Control::Secret, _) | (_, Value::Null) => String::new(),
        (Control::List, Value::Array(items)) => items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
        (_, Value::String(text)) => text.clone(),
        (_, value) => value.to_string(),
    }
}
