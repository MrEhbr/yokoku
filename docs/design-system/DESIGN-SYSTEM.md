# Yokoku — Paper design system

Version 1.0 · 26 September 2026 · Approved direction: Paper

This is the canonical visual and interaction specification for Yokoku. It includes a compiled, offline component reference and Tailwind CSS sources. The reference uses fictional media and simulated operations; it is not the application or an import engine.

## 1. Character

Warm, precise, and quiet. Borrow the reference site's monospaced headings, pink highlight, sharp outlines, and offset shadows. Keep operational screens compact and literal.

- Content leads. Posters identify media; text explains availability and actions.
- Pink indicates the primary action or an occasional heading highlight. It never means failure, missing, or selected for deletion.
- Use monospaced type for headings, episode numbers, dates, counts, and filenames. Use sans-serif for reading and controls.
- Square corners and 1px rules are the default. Put hard shadows on posters and dialogs, not every row or button.
- Use one primary action in each local context. Name the action and affected count: “Import 12 files”, “Rename 8 files”.
- The app manages releases and files. Avoid playback controls, watched progress, ratings, and recommendation carousels unless the product scope adds them.

## 2. Single source of truth

| File | Responsibility |
|---|---|
| `tokens.css` | Canonical colors, fonts, type scale, dimensions, and Tailwind theme mappings |
| `components.css` | Shared component recipes and interaction states |
| `tailwind.css` | Reference build entry and explicit specimen source scanning |
| `index.html` | Inspectable component and workflow examples |
| `theme.js` | System/light/dark preference for the standalone reference |
| `specimen.js` | Reference-only interactions and sample data |
| `design-system.css` | Generated output; never edit directly |

Do not copy hex values into app components. Use semantic utilities such as `bg-canvas`, `text-ink`, `border-control`, and `text-warning`. Use the same naming policy to generate both import destinations and rename previews; the UI must never implement a second naming engine.

## 3. Color

Values live in `tokens.css`. Each semantic token resolves in light and dark mode through `light-dark()`.

| Role | Light | Dark | Use |
|---|---|---|---|
| Canvas | `#fbfaf7` | `#20211f` | Page background |
| Surface | `#ffffff` | `#262824` | Dialogs and editable fields |
| Subtle | `#eeeee7` | `#30332c` | Selected rows and quiet fills |
| Ink | `#222521` | `#eeeee6` | Primary text, focus, strong borders |
| Muted | `#62645f` | `#b0b3aa` | Secondary text |
| Line | `#d8d9d1` | `#464941` | Decorative dividers only |
| Control | `#777a73` | `#8a9184` | Input and button boundaries |
| Accent | `#e99db4` | `#dfa0b4` | Primary action; pair with accent-ink |
| Accent ink | `#282027` | `#282027` | Text on pink |
| Success | `#376344` | `#a1c8a2` | File present, completed operation |
| Warning | `#805018` | `#e1b576` | Missing, needs review |
| Danger | `#a1353b` | `#f0a1a7` | Failure, destructive action, conflict |
| Info | `#355e82` | `#a3c5e5` | Upcoming, informational state |

Success/warning/danger/info each have a matching soft background. Use the corresponding foreground on that background. Pair color with a label and symbol. Status text stays legible without color.

`line` is deliberately quiet and is not sufficient as the only boundary of an interactive control. Use `control` there. Disabled controls are intentionally subdued but need nearby explanatory text when the reason is not obvious.

## 4. Typography

Fonts are bundled locally with their OFL licenses. No external font requests are necessary.

| Utility | Typeface | Size / line height | Use |
|---|---|---|---|
| `font-display text-display` | IBM Plex Mono 400 | 48 / 52.8 | Design reference or exceptional welcome screen only |
| `yk-page-title` | IBM Plex Mono 400 | 32 / 38.4 | Library, Upcoming, Downloads |
| `text-section font-medium` | DM Sans 500 | 18 / 25.2 | Dialog and section headings |
| `text-body` | DM Sans 400 | 14 / 21 | Main interface text and controls |
| `text-caption` | DM Sans 400/500 | 12 / 18 | Field labels, helper text, status |
| `yk-code` | IBM Plex Mono 400 | 12 / 18 | Filenames, paths, episode numbers |

Use 16px editable text on narrow/touch layouts. Essential UI text never drops below 12px. Filenames wrap or reveal their full value on focus/tap; never truncate the only visible episode identifier.

Functional headings use direct language: “Review import”, “Rename existing files”, “Resolve conflict”. Editorial headings belong in the library, not operational dialogs.

## 5. Geometry and layout

- Spacing unit: 4px. Preferred increments: 4, 8, 12, 16, 24, 32, 48.
- Control height: 36px desktop, 44px on coarse pointers. Icon buttons have an accessible label and at least the same target size.
- Corners: 0px. Control outline: 1px. Keyboard focus: 2px ink outline with 3px offset.
- Poster shadow: 3px 3px 0 ink. Dialog shadow: 4px 4px 0 ink at 35% opacity.
- Page gutter: 20px small screens; 32px from `sm`. Comfortable reading/content width: 80rem maximum.
- Dialog width: content-sized up to 46rem; 16px minimum outside margin. Height limited to viewport minus 32px, with a scrollable body for long content.
- Use dense data rows rather than a card for every record. Thin horizontal rules define the rhythm.
- Native Tailwind breakpoints remain unchanged. Below 640px, old/new filename pairs stack into two lines; actions wrap and essential labels remain visible.
- A production application may use a larger bounded dialog for a large import queue. Add pagination or virtualization; never squeeze the text to display more rows.

## 6. Component contracts

### Buttons

`yk-button` is the base. Add `yk-button-primary`, `yk-button-quiet`, or `yk-button-danger`. Native `disabled` is required for unavailable actions. Loading keeps a stable label/width, uses `aria-busy`, and prevents duplicate submission. Do not use pink for destructive confirmation.

```html
<button type="button" class="yk-button yk-button-primary">Import 12 files</button>
<button type="button" class="yk-button">Cancel</button>
```

### Inputs and selection

Use `yk-field`, `yk-label`, `yk-input`/`yk-select`, and `yk-hint`. Every control needs a programmatic label. Invalid controls use `aria-invalid="true"` and `aria-describedby` pointing to their error. Errors explain a correction, not only “invalid”.

Selects use the Dioxus Components select: a button trigger with `aria-haspopup="listbox"` and a `role="listbox"` picker with typeahead, arrow-key navigation, a selected checkmark, and bounded scrolling. The picker is positioned under its trigger, so it stays inside a scrolling dialog. Escape closes only the picker, never the dialog around it. Values are typed; the select never stores display strings as values.

Checkboxes use the Dioxus Components checkbox: `role="checkbox"` with `aria-checked` of `true`, `false`, or `mixed`. Header selection uses the mixed state. Monitoring and operation selection are different controls with different labels; selecting files must not change monitoring.

For long episode lists, an accessible searchable combobox may replace the native select while preserving the same field tokens and keyboard contract. Do not build an inaccessible custom dropdown for visual consistency.

### Media cards

Poster ratio: 2:3. Render title, year/type, lifecycle status, file availability, and next release as separate fields. Use a neutral title placeholder when artwork is missing. The title is an explicit link to detail; keep checkboxes/actions independently accessible. Lazy-load below-fold artwork and reserve its dimensions.

### Tables and batch toolbars

Use `yk-table` with a caption, column headers, and numeric alignment where appropriate. Status and filenames remain readable at compact density. Sort controls announce ascending/descending state. Row selection uses checkboxes, not clicking anywhere on a row.

The bulk bar appears after selection. It shows a count and actions valid for that selection. Mixed series/seasons display “Mixed”; they never silently display the first row's value as the batch value.

### Status taxonomy

Keep these independent. Do not encode all of them into one overloaded badge.

| Category | Values |
|---|---|
| Episode file availability | Downloaded, missing, not yet aired |
| Series lifecycle | Continuing, on break, ended |
| Movie lifecycle | Announced, in cinemas, released |
| Monitoring | Monitored, unmonitored, mixed at parent level |
| Detection confidence | Certain, guess, unknown |
| Operation | Queued, running, completed, failed, awaiting review |

“Missing” means a monitored episode has aired and has no file. An unmonitored episode without a file is not included in Missing. A movie's release type is explicit: cinema, digital, physical.

### Progress, feedback, and empty states

Use `yk-progress` for known progress. Show numerical progress, speed, ETA, and textual state where available. Unknown progress must be indeterminate, not 0%. Failed operations show the reason and a retry action. Only announce significant updates through a live region.

Empty library: explain “Add a movie or series” and provide that action. Empty filter: offer Clear filters. Empty review: “No files need review.” Loading, empty, offline, and failed are separate states. A missing metadata provider connection is not an empty library.

### Dialogs

Use the Dioxus Components dialog. Label it with its title, keep focus inside, restore focus on close, and support Escape and a click outside. Closing a preview discards uncommitted changes; closing progress never claims to cancel a running job. Avoid nested dialogs: replace the dialog body for a conflict or ordered-assignment step.

Small operations use compact headings and one footer. Keep the agreed old → new layout. Do not put a hero heading or large decorative illustration into file dialogs.

## 7. Import review

Entry points: download needing review, manual import, unmatched files from a scan. Show the originating batch and number requiring attention.

1. Auto-detect files using the configured series numbering and metadata order. Certain conflict-free matches can import automatically, as specified in the requirements; the review UI handles exceptions and explicitly requested manual previews.
2. Present a checkbox, old filename, arrow, and proposed new-name dropdown per file. The dropdown selects the episode identity; the backend naming policy generates the displayed filename. It does not store arbitrary filename strings as episode matches.
3. Bulk tools: Set series, Set season, Detect again, Assign episodes in order. They apply only to selected rows. Context changes invalidate/recompute affected matches and retain visible uncertainty.
4. Ordered assignment requires one series/season, an explicit starting episode, a visible file order, and a preview confirmation. Show every proposed mapping before applying; do not equate alphabetical order with a certain match.
5. “Import N files” is the only commit. Missing matches block the selected batch, or the user explicitly deselects them. No silent skipping.
6. Expose conflicts: replace, skip, or keep both. Explain existing/new file size and destination; require a distinct confirmation for replacement. Keep both needs a unique generated path and correct file-to-episode relationships.
7. Show hard-link/copy/move mode with its effect on originals. Filename review does not silently change the configured import mode.

Multi-episode files need an episode-range selection, not a one-episode-only field. Preserve subtitles, language suffixes, and special-season handling. Absolute input numbering is a per-series matching preference; output season/episode numbering must match Jellyfin's provider/order.

The included demo covers selection, bulk context, redetection, ordered preview, validation, and a simulated commit. Full conflict resolution, subtitle handling, multi-episode selection, and backend execution are specified here but are not implemented in the demo.

## 8. Rename existing files

Entry points: episode/file selection, season, entire series, or library selection across multiple series/movies.

1. Scope is explicit: selected files, season, series, or selected library items.
2. Generate destinations from existing metadata matches and saved naming rules. Do not re-detect episodes as part of a normal rename.
3. Show changed files only, selected by default; summarize unchanged and unavailable files separately.
4. Present checkboxes and old → new names. Destinations are read-only in this flow. “Correct match” belongs to a separate matching workflow.
5. Keep a preview even across multiple series, grouped by series/season. This is Yokoku's deliberate simplification over Sonarr's separate bulk confirmation flow.
6. Apply “Rename N files”. Rename matching subtitles with their video. Report each failure and keep retryable rows.
7. Changing naming settings affects future imports. Offer “Preview changes to existing files” before applying it retroactively.

Default output: `Shows/Title (Year)/Season 01/Title (Year) - S01E01 - Episode Title.ext`; movies: `Movies/Title (Year)/Title (Year).ext`. Specials use `Season 00`; multi-episode naming uses `S01E01-E03`. Keep root-folder moves explicit and distinct from filename changes.

## 9. Navigation and screen composition

Primary destinations: Library, Upcoming, Downloads, Activity, Settings. Review is a badge/action within Downloads and unmatched-file management; it does not require another large permanent screen. Missing is a Library filter/grouped view. Upcoming offers list/week/month views of monitored items.

Library: poster or compact list, type/status filters, title/date-added/next-release sorting. Series detail: next/last episode, monitoring, seasons, episode/file rows. Movie detail: release dates by type, monitoring, file details. Downloads: compact rows, selection, progress, linked media, review/retry. Settings: one screen grouped by connections, roots/import, naming, metadata/numbering, history, appearance.

## 10. Accessibility and scale

- Keyboard-complete controls, visible focus, meaningful labels, semantic headings/tables, and reduced-motion support.
- Target WCAG AA contrast; bundled validation checks semantic text pairs and control boundaries. This is not a full accessibility certification.
- Selection across paginated results is explicit: first “Select this page”, then “Select all N matching files”. Preserve the query snapshot and exclusions. Display the true scope before commit.
- Use stable file/episode IDs, not visual row positions, for mutations. Validate selections and destinations again on the server.
- Long operations are jobs: queued/running/result, progress counts, partial failure list, safe retry. Never show completion before server confirmation; prevent repeated commits while submitting.
- Search/filter/sort and large lists should paginate or virtualize without losing keyboard position or selection semantics. Large season packs cannot render thousands of dropdown options in every row at once.
- Keep monitoring, detection confidence, operation state, and naming rules canonical in the application domain. CSS classes only present them.

## 11. Implementation

The kit uses Tailwind CSS 4.3.3, pinned in `package.json` and `package-lock.json`. The official Tailwind compiler is a development dependency.

The app uses Dioxus 0.7 (fullstack). Interactive primitives come from [Dioxus Components](https://github.com/DioxusLabs/components): `dx components add <name> --module-path src/components/ui --rev <commit>` copies a component into the app as a folder. Its `component.rs` moves to `src/components/ui/<name>.rs`, the folder and CSS module are deleted, and the component is restyled to Paper with Tailwind classes. Behavior and accessibility come from `dioxus-primitives`. Primitives without an upstream component (table, field, alert, …) are written in the same style.

Open `index.html` directly for the precompiled offline reference. To rebuild:

```sh
npm ci
npm run build
```

For an existing Tailwind v4 app, copy `tokens.css`, `components.css`, and `fonts/` into its styling directory. Add these imports to its global entry:

```css
@import "tailwindcss";
@import "./tokens.css";
@import "./components.css";
```

Let the app's normal Tailwind source discovery scan its components. The kit's `tailwind.css` uses `source(none)` and explicit specimen files only; do not copy that scanning restriction into the app unchanged. Keep utility names statically discoverable; use explicit status/variant maps instead of dynamically constructing class strings.

The component classes are CSS recipes. Framework components should own semantics and behavior while reusing those recipes; do not copy a second palette or wrap every native control in an extra integration layer. `specimen.js` is illustrative and must not become the production file-management codepath.

Load `theme.js` before the stylesheet in the standalone reference to avoid a theme flash. In the app, an inline script in the document head sets `data-theme="light|dark"` on `<html>` before first paint: the system scheme until the user toggles, then the saved light or dark choice (per browser). The theme switch is one light/dark toggle. Preserve native control color-scheme. App pages using the custom `dark:` variant require that resolved attribute.

## 12. References

- Tailwind theme variables: https://tailwindcss.com/docs/theme
- Tailwind dark mode: https://tailwindcss.com/docs/dark-mode
- Tailwind custom styles: https://tailwindcss.com/docs/adding-custom-styles
- Visual inspiration: https://denis.bobaba.xyz/
- Sonarr v4.0.20.3014: https://github.com/Sonarr/Sonarr/tree/v4.0.20.3014/frontend/src/Organize
- Sonarr bulk import: https://github.com/Sonarr/Sonarr/blob/v4.0.20.3014/frontend/src/InteractiveImport/Interactive/InteractiveImportModalContent.tsx
- Radarr v6.4.4.10685: https://github.com/Radarr/Radarr/tree/v6.4.4.10685/frontend/src/Organize
- Font upstream: https://github.com/google/fonts/tree/main/ofl/dmsans and https://github.com/google/fonts/tree/main/ofl/ibmplexmono
