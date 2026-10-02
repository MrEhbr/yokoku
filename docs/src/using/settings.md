# Settings

**Settings** (the gear icon) shows every setting in effect and lets you change
most of them while Yokoku runs.

## Saving

- A text field saves with **Save** or Enter; a switch or choice saves at once.
- A change is checked, kept in the database and applied at once. Schedules
  apply from their next run.
- A value saved here goes over the config file, and its hint says
  *Saved here, over the config file*. **Use the config file's value** drops
  it; so does saving an empty field.
- A value that doesn't fit shows its error under the field and isn't kept.

> [!NOTE]
> A setting set by a `YOKOKU__*` variable shows as **Locked**, and its hint
> names the variable. The variable takes precedence; unset it to edit the
> setting here.

Secrets (passwords, API keys and tokens) start empty and show as
*Set (abcd…wxyz)*, the first and last four characters, once they have a
value. Type a new value to replace one.

## Sections

**Download client**
:   Transmission's RPC address, username and password.

**Media server**
:   Jellyfin's address, an API key and the user whose played items count as
    watched. Leave the address empty to not use Jellyfin.

**Metadata**
:   The TMDB token, the TVDB API key and subscriber PIN, the **Language** of
    titles and overviews, and the **Region** whose movie release dates are
    shown. Once a TVDB key is set, series come from TVDB.

**Library**
:   Root folders, what a new series monitors at first, and the **Time zone**
    for air dates.

**Import**
:   Import mode (hard link, copy or move), removing torrents after seeding,
    and the labels and folder of Transmission torrents to pick up.

**Naming**
:   Patterns for folders and files.

**Files**
:   Where `ffprobe` is.

**Schedules**
:   When the background jobs run.

**Server**
:   The database, listen address, port, logging and event poll interval. These
    are read when Yokoku starts, so they can't be changed here: set them in the
    config file or environment and restart.

## Test a Connection

**Download client** and **Media server** have **Test connection**. It tries
the values as typed, saved or not, and shows *Connected* with the server's
version, or the error.

## Root Folders

Root folders hold the item folders, one root for series and one for movies or
more. Under **Library → Root folders**, enter the absolute path of an existing
folder, choose **Series** or **Movies**, and click **Add**. Root folders can't
overlap.

**Remove** only works on a root folder no item belongs to anymore.

**Scan now** looks through the item folders for files added or deleted
outside Yokoku, which it otherwise does once a day. It reports the files it
linked and the ones it forgot, and folders whose files it couldn't match: open
those items' pages to sort them out.
