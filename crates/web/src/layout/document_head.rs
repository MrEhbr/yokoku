use dioxus::prelude::*;

/// `None` when `dx` has not rendered the stylesheet, as in a plain `cargo` build.
const STYLES: Option<Asset> = option_asset!("/assets/tailwind.css");
/// The bundled fonts; the kit's `@font-face` rules load them from `fonts/` next to the stylesheet.
const _FONTS: Asset = asset!("/assets/fonts", AssetOptions::folder().with_hash_suffix(false));

/// Sets `data-theme` on `<html>` before first paint: the saved light/dark choice, or the system
/// scheme (followed live) until the first toggle. `ykToggleTheme()` saves and returns the opposite.
const THEME: &str = r#"(() => {
  const key = "yokoku-theme";
  const media = matchMedia("(prefers-color-scheme: dark)");
  const saved = () => { try { return localStorage.getItem(key); } catch { return null; } };
  const apply = () => {
    document.documentElement.dataset.theme = saved() ?? (media.matches ? "dark" : "light");
  };
  apply();
  media.addEventListener("change", apply);
  window.ykToggleTheme = () => {
    const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    try { localStorage.setItem(key, next); } catch {}
    apply();
    return next;
  };
})();"#;

/// The stylesheet, fonts, and theme every page needs. Render it once at the root.
#[component]
pub fn DocumentHead() -> Element {
    rsx! {
        document::Script { {THEME} }
        if let Some(styles) = STYLES {
            document::Stylesheet { href: styles }
        }
    }
}
