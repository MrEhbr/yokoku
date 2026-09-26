# Validation

Tailwind CSS 4.3.3 compilation passed. Font files have valid TrueType signatures.

Verified on 26 September 2026 in the Codex browser:

- Light and dark visual rendering; local fonts loaded.
- Keyboard tab navigation, Escape dismissal, and focus restoration to the dialog trigger.
- Unresolved selected files block import with an explanatory error.
- Ordered assignment displays a draft, disables import until applied, then completes the simulated batch.
- Naming format changes produce matching rename previews; deselecting a row updates the action count and mixed checkbox state.
- At a 375px viewport, the reference and rename dialog have no horizontal page overflow; dialog width is 343px.
- No browser warnings or errors were reported during the checks.
- Dependency audit reported zero known vulnerabilities across the installed dependency tree.

Select rendering correction: the original system picker could receive focus without displaying its menu in the embedded preview. Shared select styles now opt into the browser's in-page picker. Visually verified the format and episode menus in dark mode, selection changes using Space/arrow/Enter, and the episode picker above the modal. A 375px viewport check reported no horizontal page overflow. Tailwind compilation passed again after this change. The earlier programmatic selection checks alone did not catch this rendering problem.

The workflow examples simulate operations on sample data. Server jobs, real filesystem operations, complete matching, and conflict resolution require application implementation. Browser checks here cover the reference, not every browser or assistive technology.

## Semantic contrast checks

| Theme | Pair | Measured | Minimum | Result |
|---|---|---|---|---|
| Light | ink / canvas | 14.85:1 | 4.5:1 | Pass |
| Light | ink / surface | 15.50:1 | 4.5:1 | Pass |
| Light | muted / canvas | 5.74:1 | 4.5:1 | Pass |
| Light | muted / surface | 5.99:1 | 4.5:1 | Pass |
| Light | success / canvas | 6.64:1 | 4.5:1 | Pass |
| Light | success / surface | 6.93:1 | 4.5:1 | Pass |
| Light | warning / canvas | 6.54:1 | 4.5:1 | Pass |
| Light | warning / surface | 6.82:1 | 4.5:1 | Pass |
| Light | danger / canvas | 6.52:1 | 4.5:1 | Pass |
| Light | danger / surface | 6.80:1 | 4.5:1 | Pass |
| Light | info / canvas | 6.54:1 | 4.5:1 | Pass |
| Light | info / surface | 6.83:1 | 4.5:1 | Pass |
| Light | success / success-soft | 5.94:1 | 4.5:1 | Pass |
| Light | warning / warning-soft | 5.83:1 | 4.5:1 | Pass |
| Light | danger / danger-soft | 5.75:1 | 4.5:1 | Pass |
| Light | info / info-soft | 5.83:1 | 4.5:1 | Pass |
| Light | accent-ink / accent | 7.50:1 | 4.5:1 | Pass |
| Light | accent-ink / accent-hover | 6.44:1 | 4.5:1 | Pass |
| Light | control / surface | 4.36:1 | 3:1 | Pass |
| Light | control / canvas | 4.18:1 | 3:1 | Pass |
| Light | muted / subtle | 5.14:1 | 4.5:1 | Pass |
| Dark | ink / canvas | 13.87:1 | 4.5:1 | Pass |
| Dark | ink / surface | 12.76:1 | 4.5:1 | Pass |
| Dark | muted / canvas | 7.60:1 | 4.5:1 | Pass |
| Dark | muted / surface | 7.00:1 | 4.5:1 | Pass |
| Dark | success / canvas | 8.70:1 | 4.5:1 | Pass |
| Dark | success / surface | 8.01:1 | 4.5:1 | Pass |
| Dark | warning / canvas | 8.53:1 | 4.5:1 | Pass |
| Dark | warning / surface | 7.85:1 | 4.5:1 | Pass |
| Dark | danger / canvas | 7.98:1 | 4.5:1 | Pass |
| Dark | danger / surface | 7.34:1 | 4.5:1 | Pass |
| Dark | info / canvas | 8.99:1 | 4.5:1 | Pass |
| Dark | info / surface | 8.27:1 | 4.5:1 | Pass |
| Dark | success / success-soft | 6.51:1 | 4.5:1 | Pass |
| Dark | warning / warning-soft | 6.72:1 | 4.5:1 | Pass |
| Dark | danger / danger-soft | 6.50:1 | 4.5:1 | Pass |
| Dark | info / info-soft | 6.80:1 | 4.5:1 | Pass |
| Dark | accent-ink / accent | 7.41:1 | 4.5:1 | Pass |
| Dark | accent-ink / accent-hover | 8.76:1 | 4.5:1 | Pass |
| Dark | control / surface | 4.58:1 | 3:1 | Pass |
| Dark | control / canvas | 4.98:1 | 3:1 | Pass |
| Dark | muted / subtle | 6.04:1 | 4.5:1 | Pass |

Decorative divider colors and disabled controls are excluded from these thresholds. This checks token pairs, not full WCAG conformance.
