# Queue and Import Review

**Queue** lists the torrents Yokoku follows in Transmission and where each
import stands. Its menu entry counts active downloads, with a warning badge
for downloads that need you: an error, a review or a failed import.

## How a Download Becomes a Library File

```
added → downloading → finished → matched → imported → Jellyfin rescan
                                      ↘ needs review
```

1. **Added.** With **Add torrent** on an item's page or on the Queue, or
   [picked up](#pick-up-torrents-added-in-transmission) from Transmission.
   Torrents Yokoku adds get the label `yokoku`.
2. **Downloading.** Yokoku checks Transmission every 30 seconds, and every 5
   while something downloads.
3. **Matched.** When the download finishes, Yokoku matches its video files to
   episodes or the movie by their names. Samples, extras and other files are
   left alone; subtitles go with their video.
4. **Imported.** Matched files are placed in the item's folder under your
   naming patterns, by the **Import mode** setting. Jellyfin is asked to
   rescan once the library has been quiet for 30 seconds.

An import goes through on its own only when every file matched for certain, no
two files claim the same episode, and no episode already has a file. Otherwise
it waits for review. A download without any video fails.

## Add a Torrent

**Add torrent** takes a magnet link or a `.torrent` file.

**For**
:   The item its files go to. Leave it at *Work it out from its files* to
    match them against the whole library by their names.

**Season**
:   For a series, the season of files named without one, like
    `Frieren - 05.mkv`. *From the file names* when the names include it.

## Download States

| State | Meaning |
|---|---|
| Queued, Checking | Waiting in Transmission, or verifying data |
| *N%* with speed and time left | Downloading |
| Paused at *N%* | Stopped in Transmission before it finished |
| Error | Transmission reports an error, shown below it |
| Waiting to import, Importing… | Finished; its files are being placed |
| Needs review | Some files need you; click **Review** |
| Import failed | The reason is shown below; fix it and click **Retry** |
| Imported · seeding | Done, and Transmission still seeds it |
| Imported | Done |

Yokoku can't pause or remove torrents; do that in Transmission. A torrent
removed there leaves the Queue.

## Review an Import

**Review** opens the files of the download, with the series, season and
episodes Yokoku made of each.

- **Checked** files are imported; unchecked ones stay where they are.
  Unmatched files start unchecked.
- A row notes what's wrong: *Not matched*, *Set the season*, *Set the
  episodes*, or *Guessed from the name* when Yokoku isn't sure.
- Click the season or episodes to change them, or **Edit** to pick another
  series or movie.
- Check several files to set them at once: **Set series…** (and *Detect with
  this series* to match them again), **Set season…**, or **Set episodes…**,
  which hands the ticked episodes to the checked files in order.

When a file's episode **already has a file**, choose:

**Replace library file**
:   The old file is deleted once the new one is in place.

**Keep both**
:   The new file gets a numbered name, like `name (2).mkv`.

**Import N files** is enabled once every checked file has a match and no
conflict. The same review opens from an item's page for files found in its
folder that Yokoku couldn't match; those are linked where they are instead of
imported.

## Import Modes

**Hard link** (default)
:   The library file and the download share the same data, so the torrent
    keeps seeding without using twice the space. Across file systems, Yokoku
    copies instead.

**Copy**
:   A separate copy. The torrent keeps seeding.

**Move**
:   The file moves into the library and the torrent can no longer seed it.

## Pick Up Torrents Added in Transmission

Torrents added straight in Transmission are taken on when they carry one of
the **Pick up labels** or download under the **Pick up folder** (Settings →
Import), as well as any labelled `yokoku`. Their files are matched against the
whole library.

## Remove Torrents After Seeding

With **Remove torrents after seeding** on, a torrent that was imported and
finished seeding, by Transmission's ratio or idle limit, is removed together
with its downloaded data. The library files stay.
