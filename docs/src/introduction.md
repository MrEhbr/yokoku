# Introduction

**Yokoku** is a self-hosted manager for a movie and TV library. It tracks when
episodes and movies come out, imports the torrents you download with
Transmission, and keeps the files named and organized the way Jellyfin
expects them.

## How It Works

```
TMDB / TVDB → Library → Transmission → Import → Library folders → Jellyfin
```

1. **Library**: you add series and movies by searching TMDB. Yokoku keeps
   their episodes, air dates and release dates up to date. Series come from
   TVDB instead once a TVDB key is set.
2. **Downloads**: you add a magnet link or a `.torrent` file to an item, or
   add a torrent in Transmission with a label or folder Yokoku watches.
   Yokoku follows it in Transmission.
3. **Import**: when a download finishes, Yokoku matches its files to episodes
   or the movie, then hardlinks, copies or moves them into the item's folder
   under your naming patterns. Files it can't match wait for your review.
4. **Jellyfin**: after the library changes, Yokoku asks Jellyfin to rescan. It
   also reads which files the Jellyfin user has watched.

> [!NOTE]
> Yokoku does not search indexers or trackers. You find the torrent; Yokoku
> takes it from there.

## Features

- **Wanted and Upcoming**: monitored episodes and movies that came out
  without a file, and a calendar of what comes out next
- **Monitoring**: all episodes, future ones, the latest season or nothing,
  set when you add a series, then toggled per series, season or episode
- **Imports**: hardlink, copy or move, with a review step for files that
  don't match
- **Naming**: patterns for folders and files; existing files can be renamed
  from an item's page
- **Existing libraries**: an item added over a folder that already exists
  takes the files in it, and a daily scan picks up files changed on disk
- **Watched state**: filter the library by what you've watched in Jellyfin,
  and delete watched files to free space
- **One binary**: the web UI, the scheduled jobs and a CLI for setup and
  maintenance, with an SQLite database

## Where to Go Next

- [Installation](./getting-started/installation.md): Docker, a release
  archive or NixOS
- [First Run](./getting-started/first-run.md): from an empty install to the
  first item in the library
