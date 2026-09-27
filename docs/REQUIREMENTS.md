# Yokoku — Functional Requirements

> **Yokoku** (予告) — the "next time" preview at the end of an anime episode.

A single self-hosted app that manages a personal library of **movies and TV series** together. It is a lighter replacement for Sonarr + Radarr: it tracks what's released and what's coming, takes torrents you add manually, detects which episode each file is, and puts the files into a structure Jellyfin reads correctly.

This document describes **what** the app must do, not how it is built.

---

## 1. Scope

### In scope (MVP)
- One library for movies and series
- Release tracking: next episode, upcoming movie releases, calendar
- Manual torrent adding through the app, sent to Transmission
- Watching Transmission and importing finished downloads
- Automatic detection of which episode(s) a downloaded file is
- Renaming and folder layout compatible with Jellyfin
- File management: scan, match, delete
- History of what happened

### Out of scope
| Feature (Sonarr/Radarr) | Reason |
|---|---|
| Indexers, tracker search, RSS, automatic grabbing | Torrents are added manually only |
| Quality profiles, upgrades, custom formats, cutoffs | One file per episode/movie is enough |
| Delay profiles, preferred words, release blocklist | No automatic grabbing |
| Import lists (Trakt, IMDb lists, …) | Items are added by hand |
| Collections, minimum availability | Not needed |
| Writing NFO/artwork files | Jellyfin fetches its own metadata |
| Multiple users, login | Single user; MVP runs on a trusted network |
| Download clients other than Transmission | Later |

---

## 2. Decisions

| Topic | Decision |
|---|---|
| Grabbing | No trackers or search. The user adds a magnet link or .torrent file in the app. |
| Download client | Transmission first. |
| Anime numbering | Set per series: standard season/episode, or absolute numbering. |
| Quality | Not tracked. One file per episode or movie. |
| Users | Single user, no authentication in the MVP. |
| Metadata source | Movies: TMDB. Series: TMDB by default, TVDB as an option. See §4. |

---

## 3. Functional requirements

### FR-1 Library
- **1.1** The user can search for a movie or series by title and add it to the library, choosing its root folder (FR-8.1).
- **1.2** The library lists all items with poster, title, year, type (movie/series), status and whether files are present.
  - Series status: continuing, on break, or ended.
  - Movie status: announced, in cinemas, or released.
- **1.3** The library can be filtered by type and status, and sorted by title, date added and next release.
- **1.4** Series detail shows seasons and episodes, each with:
  - Air date
  - File status: downloaded, missing, or not yet aired
- **1.5** Movie detail shows cinema, digital and physical release dates, plus file status.
- **1.6** Metadata (titles, dates, episode lists) refreshes on a schedule and on demand.
- **1.7** The user can remove an item and choose whether to delete its files too.
- **1.8** For each series, the user can choose which numbering it uses: standard season/episode, or absolute.

### FR-2 Monitoring
- **2.1** Series, seasons, episodes and movies can each be marked monitored or not.
- **2.2** When adding a series, the user chooses what to monitor: all episodes, future episodes only, latest season, or none.
- **2.3** Only monitored items appear in the "missing" and "upcoming" views and on the calendar.

### FR-3 Download client (Transmission)
- **3.1** The user configures the Transmission connection (address, credentials) and can test it.
- **3.2** The user can add a torrent from the app with a magnet link or a .torrent file:
  - linked to a chosen movie or series, or
  - left unlinked, so the app works out what it is (FR-4).
- **3.3** Optional: pick up torrents added directly in Transmission, filtered by label or download folder.
- **3.4** Show the downloads the app knows about: name, progress, speed, ETA, state and linked item.
- **3.5** When a download finishes, run detection (FR-4) and import (FR-5).
- **3.6** The user chooses the import mode:
  - hard-link or copy, so the torrent keeps seeding, or
  - move.
- **3.7** Optional: remove the torrent from Transmission after import once seeding is finished (ratio or time reached).

### FR-4 Automatic episode detection
- **4.1** List every file in the download:
  - Keep video files.
  - Keep subtitles and attach each one to its matching video.
  - Ignore samples, NFO/text files, images and other junk.
- **4.2** Recognise common naming patterns:
  - `S01E02`, `s1e2`, `1x02`, "Season 1 Episode 2"
  - Multi-episode: `S01E01-E03`, `S01E01E02`
  - Absolute numbering, fansub style: `[Group] Title - 12 [1080p]`
  - Date-based (daily shows): `2026.09.26`
  - Specials → season 0
- **4.3** Season packs: map each file to its own episode. If file names have no season, take it from the folder name.
- **4.4** Full-series packs containing several season folders.
- **4.5** When a file name has no numbers, match by episode title, including original-language titles (e.g. Russian).
- **4.6** Match series titles tolerantly, ignoring case, punctuation and year, and accepting alternate or original titles.
- **4.7** If the torrent was added for a specific series, use that series and don't guess the title.
- **4.8** Check every match against the series' real episode list, and reject episodes that don't exist.
- **4.9** For series set to absolute numbering, convert the absolute number to season/episode for naming.
- **4.10** Give every file a confidence level:
  - **Certain**: imported automatically.
  - **Guess** or **unknown**: sent to review.
- **4.11** The review screen shows one row per file: file → detected episode → new name. The user can correct any row, then import.
- **4.12** Ask the user about conflicts instead of resolving them automatically. The user chooses to replace, skip, or keep both.
  - Two files map to the same episode.
  - The episode already has a file.
- **4.13** Movies: detect title and year, and pick the main video as the largest non-sample file. Extras are ignored.

### FR-5 Renaming and folder structure
- **5.1** Imported files are placed and named in a layout Jellyfin recognises:
  - Movies: `Movies/Title (Year)/Title (Year).ext`
  - Series: `Shows/Title (Year)/Season 01/Title (Year) - S01E01 - Episode Title.ext`
- **5.2** Multi-episode files are named as a range, e.g. `S01E01-E02`.
- **5.3** Subtitles are renamed to match their video, keeping the language tag (e.g. `….en.srt`).
- **5.4** Specials go into `Season 00`.
- **5.5** Characters that aren't allowed in file names are replaced safely.
- **5.6** Naming patterns are configurable, with Jellyfin-compatible defaults.
- **5.7** The user can rename existing library files, with a preview (old → new) before applying. Files are renamed inside their item's folder; the folder itself is never renamed (FR-8.1).
- **5.8** Episode numbers in names must follow the same metadata source and episode order that Jellyfin uses.

### FR-6 Next episode tracking
- **6.1** Each series shows its next episode: number, title, and air date/time in the user's time zone.
- **6.2** Each series shows its last aired episode and whether that episode is downloaded.
- **6.3** Monitored episodes that have aired but have no file are flagged as missing.
- **6.4** A "missing" view lists all of them, grouped by series.

### FR-7 Upcoming releases / calendar
- **7.1** A calendar (week and month views) shows upcoming episodes and movie releases for monitored items.
- **7.2** A "coming up" list shows the same items sorted by date.
- **7.3** Movie entries show which release a date refers to: cinema, digital, or physical.
- **7.4** Calendar entries are marked downloaded, missing, or upcoming.
- **7.5** Optional: an iCal feed, so the calendar can be shown in other calendar apps.

### FR-8 File management
- **8.1** The user configures root folders, each for movies or for series, with any number of each (e.g. `Anime`, `Shows`, `Movies`).
  - Every item belongs to one root folder of its kind, chosen when it is added.
  - Each item has its own folder in that root folder, named by the FR-5.1 layout when it is added (e.g. `Shows/Title (Year)`), or a name the user gives, such as an existing folder. The folder never changes afterwards, even when the item's title or year does.
  - A root folder that holds items can't be removed.
- **8.2** The app scans the folder of each library item and links the files there to that item, so an existing collection can be imported.
  - Items are only added from the app (FR-1.1); a scan never adds items.
  - Folders that belong to no library item are ignored.
- **8.3** Files in an item's folder that it can't recognise are listed for manual matching, using the same review screen as FR-4.11.
- **8.4** The user can delete a movie or episode file from the app.
- **8.5** Deleting files asks for confirmation first; deleted files are removed for good.
- **8.6** File details are shown: path, size, resolution, and audio/subtitle languages where detectable.
- **8.7** Files removed or changed outside the app are detected on the next scan.
- **8.8** Adding an item scans its folder right away (FR-8.2).

### FR-9 History and activity
- **9.1** A history log records adds, torrent additions, imports, renames and deletions, each with a date and the item it affected.
- **9.2** Failed imports show their reason and can be retried.

### FR-10 General
- **10.1** One app, one library, one settings screen for both movies and series.
- **10.2** Used from a browser on the local network. There's no login in the MVP.
- **10.3** Settings, library and history survive restarts.
- **10.4** Optional: tell Jellyfin to rescan after an import.

---

## 4. Metadata source notes

- Sonarr uses **TheTVDB**. Radarr uses **TMDB**.
- TheTVDB v4 needs a project API key plus either a negotiated licence or each end user's subscriber PIN ($12/year).
- TMDB is free for non-commercial use, and Jellyfin uses it by default for both movies and shows.
- **Hard rule:** episode numbering must match what Jellyfin uses. Otherwise a file named `S02E05` shows up as a different episode in Jellyfin.
- Plan:
  - Movies: TMDB.
  - Series: TMDB by default, TVDB optional (the user supplies a PIN).
  - The configured provider must be the same one Jellyfin uses.

---

## 5. Suggested build order

1. **Library + metadata:** add, list and detail views; refresh (FR-1, FR-2)
2. **Next episode + calendar** (FR-6, FR-7)
3. **Root folders + scan + manual match** (FR-8.1–8.3)
4. **Renaming engine + preview** (FR-5)
5. **Transmission: add torrent, watch downloads** (FR-3)
6. **Episode detection + review screen + auto import** (FR-4)
7. **History, Jellyfin rescan** (FR-9, FR-10.4)

---

## 6. Later / maybe
- Login and multiple users
- More download clients (qBittorrent, …)
- Notifications, e.g. "new episode aired" and "import done"
- Learning from manual corrections in the review screen, so the same release group is detected next time
