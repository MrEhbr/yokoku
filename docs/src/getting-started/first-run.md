# First Run

This page takes a fresh install to its first imported download. Open the web
UI at `http://<server>:8080`.

## 1. Connect the Services

Open **Settings** (the gear icon). A change is saved with **Save** or Enter,
applies at once and is kept in the database, over the config file.

- **Metadata → TMDB token**: the API read access token, unless you already
  set it in the config file or environment.
- **Download client → Transmission address**: its RPC address, like
  `http://localhost:9091/transmission/rpc`, with a username and password if
  Transmission asks for them. Click **Test connection**.
- **Media server** (optional): the **Jellyfin address**, an **API key** from
  Jellyfin's dashboard under *API keys*, and the **User** whose played items
  count as watched. Click **Test connection**.

> [!NOTE]
> A setting set by a `YOKOKU__*` variable shows as **Locked**: the variable
> takes precedence, so unset it to edit the setting on this page.

## 2. Add Root Folders

Root folders hold your item folders: one for series and one for movies, like
`/srv/media/series` and `/srv/media/movies`. Under **Settings → Library →
Root folders**, enter the absolute path of an existing folder, choose
**Series** or **Movies**, and click **Add**.

## 3. Add a Series or Movie

On **Library**, click **Add**, choose **Series** or **Movies**, and search by
title. Pick a result to open the add dialog:

- **Root folder**: where the item's folder goes.
- **Monitor**: what counts toward **Wanted** and **Upcoming**. A series takes
  *All episodes*, *Future episodes*, *Latest season* or *Nothing*; a movie is
  *Monitored* or *Not monitored*.
- **Folder**: the item's folder, named by your naming patterns.

> [!TIP]
> Already have the files? When the folder exists, the dialog says so and the
> files in it are linked to the item. Add each item of an existing library
> this way.

**Add** opens the item's page with its seasons and episodes, or its release
dates.

## 4. Download It

On the item's page, click **Add torrent** and paste a magnet link or choose a
`.torrent` file. The torrent goes to Transmission and the **Queue** opens.

Yokoku checks Transmission every 30 seconds, or every 5 while something
downloads. When the download finishes, its files are imported into the item's
folder and Jellyfin is asked to rescan. A file Yokoku can't match to an
episode waits in the Queue for your review.

## Next

- Torrents added straight in Transmission can be picked up too, by label or
  download folder.
- Check **Wanted** for monitored episodes and movies that came out without a
  file, and **Upcoming** for what comes out next.
