# yokoku-web

The web UI. Topcoat 0.9 is the fullstack framework: pages render on the server and call use cases directly. There is no JSON API and no separate frontend build.

Spec: `docs/design-system/DESIGN-SYSTEM.md` (Paper). Topcoat reference: the pinned 0.9.0 sources, `~/.cargo/registry/src/*/topcoat-*-0.9.0/docs/*.md`. Quick index: https://github.com/tokio-rs/topcoat/blob/main/llms.txt (tracks `main`, which may be ahead of 0.9.0).

## Layout

```
src/lib.rs              re-exports app::router(assets); `yokoku serve` will mount it
src/app.rs              module_router!() root: #[layout] (document, nav, error boundary, attribution), Library #[page], not_found!()
src/app/<page>.rs       one module per URL segment; path params via path_param! in their own module
src/components/         Yokoku components (media_card, rename_row, selection_bar, ...)
src/components/ui/      topcoat-ui primitives, tracked by components.toml
gallery/                dev-only bin: one page per component (stories), iframe frames for overlays; own router, not the app
styles.css              Tailwind input: Paper tokens, @source, base layer
build.rs                stages lucide icons, renders Tailwind
```

- The module path is the URL: `app::series::series_id` → `/series/{series_id}`. A `_name` module is a group and adds no segment. Handlers never take absolute paths.
- Link with `href!(module::page)`, never string paths. `href!` panics if the handler isn't registered in the serving router.
- Build with `module_router!().discover()`. `module_router!` collects module-derived handlers; `.discover()` adds fonts, shards, procedures, and absolute-path handlers from every linked crate.
- Only pathless layers wrap routing misses. `not_found!()` in `app.rs` renders misses through the layouts.
- Nav state: `href.is_current(cx)` → `aria-current="page"`.

## Dependencies and data

- `web` is a driving adapter. It depends on feature modules (`library`, `media`, `downloads`) and calls their use cases, like the CLI does. It never touches `db`, sqlx, or other adapters.
- Services arrive with the first data-backed page: register one `Services` value with `.app_context(...)` in `router(...)`, and read it through small helpers (`fn library(cx: &Cx) -> &dyn …`). `app_context::<T>` panics if `T` wasn't registered, and the match is on the exact type.
- `#[memoize] async fn x(cx: &Cx)` caches per request, so the layout and the page share one query.
- Cross-cutting checks are plain functions called by each handler (functions, not middlewares). `#[layer]` is only for whole-request wrapping.

## Components

- Signature: `#[component] pub async fn name(<props>, #[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View>`. Use `#[into]` for optional strings and labels. Take `cx: &Cx` only when needed; callers never pass it.
- Components are presentational: plain strings, numbers, and local enums (`Tone`). Pages map domain enums to props. The status taxonomy is separate per category; never merge categories into one badge.
- `class!` concatenates; it neither merges nor dedupes Tailwind classes. Never append a conflicting utility (e.g. `text-caption` over `text-body`); drop it from the base. An `attrs` `class` replaces the element's class unless the component merges it through `class!(BASE, attrs.remove("class"))`.
- Colors only through Paper tokens (`bg-canvas`, `text-ink`, `border-control`, `text-warning`, ...). Pink (`accent`) means the primary action only. Focus styling is global; don't add `outline-none` or rings.
- Keep class names statically visible in `.rs` files under `src/` or `gallery/` (the `@source` globs). Tailwind scans only this crate, honors `.gitignore` (the root `bin/` rule hides any `src/bin/`), and misses dynamically built names.
- `components/ui/*` is vendored upstream code restyled to Paper. `topcoat ui add --overwrite` discards the restyle; `ui list` can't see local edits.

## View gotchas

- Text is string literals: `<p>"Hi, " (name)</p>`. Output is escaped; raw HTML only via `Unescaped::new_unchecked` with trusted markup.
- `disabled="false"` still disables. Pass a `bool` or `Option` to toggle an attribute. `aria-*` values are strings.
- `for` loops that contain components, signals, or live regions need `#[key(item.id)]`.
- Components render concurrently in unspecified order; no side effects in render.
- `(StatusCode::X)` in node position sets the status. After streaming starts (`suspense`, `live!`), status and cookies can't change.

## Mutations

- Forms: a real `<form method="post">` + `#[route(POST "./action")]` taking `Form<T>`, then `Ok(see_other(href!(page).resolve(cx)))` (Post/Redirect/Get). Works without JS.
- Validation errors: a `#[page(POST …)]` that re-renders with `(StatusCode::UNPROCESSABLE_ENTITY)`, or `rewrite` to the GET page with the errors in request context.
- A same-origin policy rejects cross-origin writes (403). There are no CSRF tokens.
- Mutations use stable IDs (file, episode), never row positions. Validate again on the server.

## Browser runtime (experimental)

- `signal(cx, || v)` + `$(...)` for client-only state (dialogs, toggles). The `$()` vocabulary is small: no iterators, no `Vec::contains`/`push`.
- `$()` captures are sent to the browser; never capture secrets. Unsuffixed integers are `usize`, and overflow panics.
- No built-in browser storage. `raw!("js ${binding}", rust_fallback)` reaches JS such as `localStorage` from an event handler, but the server can't read it at render time, so the first paint uses the server value. Prefer, in order: cookies read on the server (theme), query params (view mode, sort, filters), then `localStorage` for client-only memory.
- `#[shard]`: server re-render on argument change (filters, search, an import-review row). A tracked `.get()` in a page body re-renders the whole page; keep tracked reads inside shards.
- `#[procedure]`: typed RPC from an event handler. Return `Ok(Result<T, String>)` when the page must handle the failure.
- Shard and procedure paths change between builds unless set explicitly. Their args are user input; authorize and validate inside.
- Live job progress: `live!` + `connected(cx)`, looping on a broadcast/watch receiver from app context. The body restarts on reconnect, so start jobs in a POST or procedure, never in the live body.
- Don't use the htmx/datastar/alpine integrations; one DOM-morphing system only.
- Large lists: plain markup rows, paginate through a shard, no per-row signals or `live!`. Select-all can't iterate N row signals in `$()`; keep selection server-side or count-based.

## Assets, build, run

- Fonts (`fontsource_font!(…, host: Asset)`), icons, and the stylesheet are assets. They're served from an `assets/` directory next to the binary; there's no embedding in 0.9. Rendering an asset missing from the bundle panics.
- Bundle against the binary that runs: `topcoat asset bundle -p <pkg> --bin <bin> [--release]`. A bundle from another binary or profile has mismatched IDs.
- An asset ID hashes crate, source file, and path. `tailwind::stylesheet!()`'s path is the absolute `OUT_DIR`, which contains the target triple, so a bundle from a plain build doesn't serve a `--target` build ("failed to resolve asset"). goreleaser always passes `--target`. Don't use `stylesheet!()`: render Tailwind to a fixed, gitignored `src/tailwind.css` (`.output("src/tailwind.css")`) and link it with `asset!("../tailwind.css")` from `components/document_head.rs`, as done here. One host bundle then serves every target (the pattern in denis's releases).
- Release: a goreleaser `before` hook runs `topcoat asset bundle -p yokoku --release -o target/yokoku-assets`; archives and the Dockerfile ship it as `assets/` next to the binary. CI installs `topcoat-cli` at the `topcoat-asset` version in `Cargo.lock`.
- The build downloads Tailwind (4.3.2), lucide, and fonts.
- Gallery: `just web gallery` (`topcoat dev -p yokoku-web --bin gallery`) at http://127.0.0.1:3000. Add a story page in `gallery/ui.rs` or `gallery/components.rs` and its entry in `NAV` (`gallery/main.rs`) with every new component. Its theme switch uses the app's cookie.
- `topcoat dev` starts the binary with no arguments and sets `HOST`/`PORT`/`TOPCOAT_DEV_URL`. `dev::script()` renders nothing outside it.
- Serving inside `yokoku serve`: `topcoat::serve_until(listener, router, token.cancelled_owned())`. Never `topcoat::start`, which installs its own signal handling.
- Topcoat CLI must match the crate: `cargo install topcoat-cli --version 0.9.0 --locked`.
- Format view macros with `just web fmt` (`topcoat fmt src` in the crate); never run it without paths (it ignores `.gitignore` and walks `target/`). It has no check mode.

## Tests

- No test client: `router.handle(http::Request)` then `topcoat::router::to_bytes(body)`, with fake services through the same `router(...)` constructor.
- A component alone: `view.single().await?.render(cx)` renders non-live HTML.
- POST tests: send same-origin headers or none, or the origin policy returns 403.

## Cookies and sessions

- Per-browser preferences are plain cookies, so the server renders them on first paint. Routers that read cookies need `.cookies()`.
- Theme: `theme::current(cx).attribute()` sets `data-theme` on `<html>`. `theme_switch(action)` posts a `ThemeChange` form to a route owned by each router; the route calls `theme::remember(cx, change.theme)` and returns `see_other(change.back())`. `back()` only accepts local paths.
- Topcoat sessions use `__Host-`/`Secure` cookies and don't work over plain-HTTP LAN.
