use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde_json::Value;

use crate::{
    api::{
        failure,
        settings::{Control, Section, Setting, reset_setting, save_setting},
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

/// One setting: its control, a hint, and where its value comes from. Text is saved with Save or
/// Enter, a choice or switch at once; an empty text or list goes back to the config file.
#[component]
pub(super) fn SettingField(setting: Setting) -> Element {
    let mut current = use_signal(|| setting);
    let shown = move || {
        let setting = current.read();
        text(&setting.value, &setting.field.control)
    };
    let mut draft = use_signal(shown);
    let chosen = use_memo(move || Some(draft()));
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let key = current.read().field.key.clone();
    let id = format!("setting-{}", key.replace('.', "-"));
    let (hint_id, error_id) = (format!("{id}-hint"), format!("{id}-error"));
    let locked = current.read().from_env || saving();
    let changed = draft() != shown();
    let described = if error.read().is_some() { error_id.clone() } else { hint_id.clone() };
    let save = move |value: Value| async move {
        saving.set(true);
        error.set(None);
        let key = current.read().field.key.clone();
        let saved = if value.is_null() { reset_setting(key).await } else { save_setting(key, value).await };
        match saved {
            Ok(setting) => {
                draft.set(text(&setting.value, &setting.field.control));
                current.set(setting);
            },
            Err(failed) => {
                error.set(Some(failure(&failed)));
                if matches!(current.read().field.control, Control::Choice(_)) {
                    draft.set(shown());
                }
            },
        }
        saving.set(false);
    };
    let typed = move || typed(&draft(), &current.read().field.control);
    let Unsaved(mut unsaved) = use_context();
    use_effect(move || {
        let key = current.read().field.key.clone();
        if draft() == shown() {
            unsaved.write().remove(&key);
        } else {
            unsaved.write().insert(key, typed());
        }
    });
    let setting = current();
    let variable = format!("APP__{}", setting.field.key.to_uppercase().replace('.', "__"));
    rsx! {
        Field {
            Label { html_for: "{id}", "{setting.field.label}" }
            match &setting.field.control {
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
                    let options = choice_options(choices, &draft());
                    let placeholder =
                        options
                        .iter()
                        .find(|(value, _)| *value == draft())
                        .map(|(_, text)| text.clone())
                        .unwrap_or_default();
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
                }
                Control::ReadOnly => rsx! {
                    Input {
                        id: "{id}",
                        class: "bg-subtle text-muted",
                        readonly: true,
                        value: "{draft}",
                        placeholder: "Not set",
                        aria_describedby: "{described}",
                    }
                },
                _ => rsx! {
                    div { class: "flex gap-2",
                        Input {
                            id: "{id}",
                            class: if setting.field.section == Section::Naming { "flex-1 yk-code" } else { "flex-1" },
                            r#type: if setting.field.control == Control::Secret { "password" } else { "text" },
                            autocomplete: "off",
                            value: "{draft}",
                            placeholder: match (&setting.field.control, &setting.value) {
                                (Control::Secret, Value::String(masked)) => {
                                    format!("Set ({masked}); type to replace it")
                                }
                                (Control::Secret, _) => "Not set".to_owned(),
                                (Control::Text(placeholder), _) => placeholder.clone(),
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
                "{setting.field.hint}"
                if setting.field.control == Control::ReadOnly && !setting.from_env {
                    " Set in the config file or by "
                    code { class: "yk-code", "{variable}" }
                    "."
                } else if setting.from_env {
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
fn text(value: &Value, control: &Control) -> String {
    match (control, value) {
        (Control::Secret, _) | (_, Value::Null) => String::new(),
        (Control::List, Value::Array(items)) => items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
        (_, Value::String(text)) => text.clone(),
        (_, value) => value.to_string(),
    }
}

/// The value to save for `draft`; a list is split at commas.
fn typed(draft: &str, control: &Control) -> Value {
    match control {
        Control::List => {
            Value::Array(draft.split(',').map(str::trim).filter(|item| !item.is_empty()).map(Into::into).collect())
        },
        _ => Value::String(draft.to_owned()),
    }
}

/// `choices` as values and labels, with `current` first when it is set and not among them.
fn choice_options(choices: &[(String, String)], current: &str) -> Vec<(String, String)> {
    let mut options = choices.to_vec();
    if !current.is_empty() && !options.iter().any(|(value, _)| value == current) {
        options.insert(0, (current.to_owned(), current.to_owned()));
    }
    options
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dioxus::prelude::*;
    use rstest::rstest;
    use serde_json::{Value, json};

    use super::{SettingField, Unsaved, choice_options, text, typed};
    use crate::api::settings::{Control, Field, Section, Setting};

    const LANGUAGES: &[(&str, &str)] = &[("en-US", "English, US"), ("uk-UA", "Ukrainian")];

    /// A switch for a setting that is `on`, inside the context the Settings page provides.
    #[component]
    fn SwitchSetting(on: bool) -> Element {
        use_context_provider(|| Unsaved(Signal::new(BTreeMap::new())));
        let field = Field::new(Section::Import, "downloads.pick_up", "Pick up", "", Control::Switch);
        let setting = Setting { field, value: Value::Bool(on), stored: false, from_env: false };
        rsx! {
            SettingField { setting }
        }
    }

    #[rstest]
    #[case::on(true, r#"data-state="checked""#)]
    #[case::off(false, r#"data-state="unchecked""#)]
    fn a_switch_shows_whether_the_setting_is_on(#[case] on: bool, #[case] state: &str) {
        let mut dom = VirtualDom::new_with_props(SwitchSetting, SwitchSettingProps { on });
        dom.rebuild_in_place();

        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(state), "{html}");
    }

    #[rstest]
    #[case::secret(json!("hunter2"), Control::Secret, "")]
    #[case::unset(Value::Null, Control::text(""), "")]
    #[case::list(json!(["tv", "anime"]), Control::List, "tv, anime")]
    #[case::empty_list(json!([]), Control::List, "")]
    #[case::text(json!("copy"), Control::text(""), "copy")]
    #[case::switch(json!(true), Control::Switch, "true")]
    fn a_value_is_shown_as_its_control_edits_it(
        #[case] value: Value,
        #[case] control: Control,
        #[case] expected: &str,
    ) {
        assert_eq!(text(&value, &control), expected);
    }

    #[rstest]
    #[case::list(" tv, ,anime ", Control::List, json!(["tv", "anime"]))]
    #[case::empty_list("", Control::List, json!([]))]
    #[case::text(" copy ", Control::text(""), json!(" copy "))]
    fn a_draft_is_saved_as_its_control_types_it(
        #[case] draft: &str,
        #[case] control: Control,
        #[case] expected: Value,
    ) {
        assert_eq!(typed(draft, &control), expected);
    }

    #[rstest]
    #[case::listed("uk-UA", &[("en-US", "English, US"), ("uk-UA", "Ukrainian")])]
    #[case::outside_the_list("fr-FR", &[("fr-FR", "fr-FR"), ("en-US", "English, US"), ("uk-UA", "Ukrainian")])]
    #[case::unset("", &[("en-US", "English, US"), ("uk-UA", "Ukrainian")])]
    fn a_value_outside_the_choices_is_offered_first(#[case] current: &str, #[case] expected: &[(&str, &str)]) {
        let expected: Vec<(String, String)> =
            expected.iter().map(|(value, text)| (value.to_string(), text.to_string())).collect();

        let Control::Choice(languages) = Control::choice(LANGUAGES) else { unreachable!() };

        assert_eq!(choice_options(&languages, current), expected);
    }
}
