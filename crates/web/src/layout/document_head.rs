use dioxus::prelude::*;

/// `None` when `dx` has not rendered the stylesheet, as in a plain `cargo` build.
const STYLES: Option<Asset> = option_asset!("/assets/tailwind.css");
/// The bundled fonts; the kit's `@font-face` rules load them from `fonts/` next to the stylesheet.
const _FONTS: Asset = asset!("/assets/fonts", AssetOptions::folder().with_hash_suffix(false));

/// Sets `data-theme` on `<html>` before first paint: the choice saved under `key`, or the system
/// scheme, followed live, until a choice is saved. [`theme_script`] defines `key`.
const THEME: &str = r#"
  const media = matchMedia("(prefers-color-scheme: dark)");
  const saved = () => { try { return localStorage.getItem(key); } catch { return null; } };
  const apply = () => {
    document.documentElement.dataset.theme = saved() ?? (media.matches ? "dark" : "light");
  };
  apply();
  media.addEventListener("change", apply);
"#;

fn theme_script(key: &str) -> String {
    format!("(() => {{\n  const key = {key:?};{THEME}}})();")
}

/// The stylesheet, fonts, and theme every page needs; the light/dark choice is saved in localStorage
/// under `theme_key`. Render it once at the root.
#[component]
pub fn DocumentHead(theme_key: &'static str) -> Element {
    rsx! {
        document::Script { {theme_script(theme_key)} }
        if let Some(styles) = STYLES {
            document::Stylesheet { href: styles }
        }
    }
}
