# Library

**Library** is the home page: every series and movie you've added, as posters
or a table. **Add** opens the [search](./adding.md).

## Filters and Sorting

**Type**
:   Series or movies.

**Status**
:   Where an item is in its life. A series is **Continuing** while an episode
    is still to air, **On break** when none is, and **Ended** once its source
    says it ended or was canceled. A movie is **Announced**, **In cinemas**
    from its cinema release, then **Released** from its digital or physical
    release.

**Watched**
:   From the Jellyfin user's played files: **Watched** when every file has been
    played, **In progress** when some have, **Unwatched** when none have.
    *Unwatched* also matches items without files. Needs the Jellyfin
    [**User**](./settings.md#sections) setting.

**Sort by**
:   **Title**, **Date added** (newest first) or **Next release** (soonest
    first, items without one last).

Filters and the view reset when you leave the page.

## What a Card Shows

- The poster, title, year and type.
- The status.
- The files: for a series, downloaded out of wanted episodes, like
  *12/24 episodes*; for a movie, **Downloaded**, **Missing** or **No file**.
  The count only covers episodes that are monitored and aired, or that have a
  file, so a series monitoring future episodes doesn't count its old ones.
  Anything missing shows in warning color.
- **Next**: the next episode air date or movie release.

The table has the same information in columns.

## Root Folder Sections

With more than one root folder, the library is split into a section per root
folder, series roots first, each with its item count.
