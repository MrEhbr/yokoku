use dioxus::prelude::*;

const DEFAULT_TIMEOUT_MS: i32 = 2000;

/// A function that copies text to the clipboard, and whether it just did; that turns false again
/// after `timeout_ms`. Copying does nothing outside a secure context, where browsers offer no
/// clipboard, such as plain HTTP away from localhost.
pub fn use_copy_clipboard(timeout_ms: Option<i32>) -> (impl Fn(&str) + Clone, ReadSignal<bool>) {
    let copied = use_signal(|| false);
    let _timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);

    let copy_fn = move |_text: &str| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;

            let Some(window) = web_sys::window() else { return };
            if !window.is_secure_context() {
                return;
            }
            let _ = window.navigator().clipboard().write_text(_text);

            *copied.write_unchecked() = true;

            let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
                *copied.write_unchecked() = false;
            });
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), _timeout);
        }
    };

    (copy_fn, copied.into())
}

/// Whether the CSS media `query` matches, kept current as it changes.
#[must_use]
pub fn use_media_query(query: &str) -> ReadSignal<bool> {
    let is_match = use_signal(|| false);
    let _query = query.to_string();

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;

            let Some(window) = web_sys::window() else { return };
            let Ok(Some(mql)) = window.match_media(&_query) else {
                return;
            };

            *is_match.write_unchecked() = mql.matches();

            let is_match_clone = is_match;
            let mql_clone = mql.clone();
            let closure = wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(
                move |_: wasm_bindgen::JsValue| {
                    *is_match_clone.write_unchecked() = mql_clone.matches();
                },
            );
            let _ = mql.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    });

    is_match.into()
}
