# Series and Movie Pages

Each series and movie has a page with its details, files and history. The
actions sit under the title: **Monitored**, **Numbering** (series only),
**Add torrent**, **Search releases**, **Refresh**, **Rename files…** and
**Remove…**. On a phone, the last three are in the **⋯** menu.

## Series

The page shows the next episode and the last one aired, then every season,
newest first and specials last. A season's header sums it up, like
*8 of 10 downloaded, 3 watched, 2 missing*.

Each episode has its air date and file status:

| Status | Meaning |
|---|---|
| Downloaded | It has a file |
| Missing | It aired, is monitored and has no file |
| No file | It aired without a file, but isn't monitored |
| Not yet aired | Its air date hasn't come, or it has none |

A downloaded file is also **Watched** or **Unwatched** for the Jellyfin user,
and shows its resolution and size. Click an episode's title for its overview
and the file's details: path, length, video, audio and subtitles.

## Movie

The page lists the **Cinema**, **Digital** and **Physical** release dates, or
*Not announced*, with how far away each is. The movie counts as released, and
its file as missing, from its digital or physical release; a cinema release
alone doesn't count. Under **File** are the file's details.

## Monitoring

Only monitored episodes and movies count toward **Wanted** and **Upcoming**.

- **Monitored** under the title switches the whole series or movie.
- The bookmark next to a season switches the season and every episode in it.
- The bookmark next to an episode switches only that episode.

An episode counts only when it, its season and its series are all monitored.
Turning the series off keeps the seasons' and episodes' own choices for when
you turn it back on.

New episodes found on a refresh follow their season. A new season follows the
series, except specials, which start unmonitored.

## Add a Torrent

**Add torrent** takes a magnet link or a `.torrent` file. Its files are
imported into this item, and the **Queue** opens.

For a series, **Season** helps with files named without one, like
`Show - 05.mkv`: pick the season they belong to. Leave it at *From the
file names* when the names include the season.

## Search Releases

**Search releases** searches the trackers added in Jackett for this item and
downloads the release you pick, the way **Add torrent** does. See
[Jackett](../integrations/jackett.md).

## Numbering

**Numbering** tells Yokoku how to read file names that have an episode number
but no season, like `Show - 05`:

**Standard · S01E02**
:   The number is the episode within its season. With more than one season,
    Yokoku guesses and asks you to review the import.

**Absolute · 12**
:   The number counts episodes across all seasons, without specials. Common
    for anime.

Names with a season, like `S02E05`, are read the same either way.

## Unrecognized Files

When the item's folder holds files Yokoku couldn't match to an episode, a
notice says how many. **Match** opens a review where you choose the episode
of each file.

## Refresh

**Refresh** reads the titles, dates, episodes and artwork from TMDB or TVDB
again. Yokoku also does this in the background for items that are due.
Episodes keep their monitoring and files. When the source renumbers episodes
that have files, the files follow; a file whose episode was split goes to
review.

## Rename Files

**Rename files…** moves files to the names your naming patterns give them,
inside the item's folder. It shows every change before doing it, and you can
leave files out. Subtitles and audio tracks named after a video move with it.

A file stays as it is when another file would get the same name. Nothing is
ever overwritten. Folders left empty are removed, and Jellyfin is asked to
rescan.

## Remove or Delete Files

**Remove…** deletes files, removes the item from the library, or both.

1. Choose files: one by one, a season at a time, or with **All**,
   **Watched** and **None**. The total size shows at the bottom.
2. **Stop monitoring what's deleted** is on at first, so deleted episodes
   don't come back to **Wanted**.
3. **Also remove … from the library** stops tracking the item. Files you
   don't choose stay on disk.

Deleted files are gone for good, together with their subtitles, audio tracks and the
folders left empty.

> [!NOTE]
> A file hard-linked to a seeding torrent keeps taking space until the
> torrent is removed from Transmission.

> [!TIP]
> To free space, open a series, click **Remove…**, then **Watched**.

## History

At the bottom is the item's [history](./history.md): when it was added,
downloaded, imported, renamed and more.
