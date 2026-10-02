# Troubleshooting

Common problems and where to start looking.

> [!TIP]
> Most problems show in the log. Read it with `journalctl -u yokoku` on
> NixOS or `docker logs yokoku` with Docker, and run with `-v` or
> `RUST_LOG=debug` for more detail. The **Test connection** buttons in
> Settings check Transmission and Jellyfin.

## The Service Doesn't Start

**"web assets not found at …"**
:   The `public/` folder with the web UI isn't next to the binary. Keep the
    archive's `public/` beside `yokoku`, or point `DIOXUS_PUBLIC_PATH` at it.

**"Failed to load configuration"**
:   The config file or a `YOKOKU__*` variable has a value of the wrong type,
    or a schedule or naming pattern that doesn't parse. The rest of the
    message names the setting.

## The Web UI Can't Be Reached

`config/app.toml` listens on `127.0.0.1`, so only the server itself can open
it. Set `web.host = "0.0.0.0"` to listen on every interface, open the port in
the firewall (`openFirewall` on NixOS), and restart.

## Search and Add Fail

**"Set a TMDB token … to search and add items"**
:   Set the TMDB token; see [TMDB](./integrations/metadata.md#tmdb). It's
    required even with a TVDB key.

**"The metadata source refused the request; check the token"**
:   Use TMDB's **API Read Access Token**, the long one, not the short API
    key. For TVDB, check the key and, with a user-supported key, the PIN.

**"No root folder for series"**
:   Add a root folder for that kind under **Settings → Library**.

## A Download Doesn't Import

Check its state on the [Queue](./using/queue.md#download-states).

**Import failed: "… is missing"**
:   Yokoku can't find the download where Transmission says it is. Both must
    see the download folder at the same path; see
    [Paths](./integrations/transmission.md#paths). Fix the mounts, then
    **Retry**.

**Import failed: "… already exists"**
:   Another file already has the name the import would give it. Move it away,
    or **Rename files…** on the item, then **Retry**.

**Import failed: "Permission denied"**
:   The service's user can't read the download or write the library folder.
    On NixOS, run the service as your media user and group; with Docker, set
    the container's user.

**Needs review**
:   Some files couldn't be matched for certain. See
    [Review an Import](./using/queue.md#review-an-import).

**The download never shows on the Queue**
:   A torrent added straight in Transmission is only taken on with a pick-up
    label or folder; see
    [Picking Up Torrents](./integrations/transmission.md#labels-and-picking-up-torrents).

## Imports Take Twice the Space

The log says *cannot hard-link across file systems; copying*. Downloads and the
library are on different file systems, or different Docker volumes. Put them
on one, mounted as one volume; see [Import Modes](./using/queue.md#import-modes).

A file you delete in Yokoku also keeps its space while a seeding torrent still
holds its hard link.

## Files Are Matched to the Wrong Episodes

- Files named without a season, like `Show - 05.mkv`: set the series'
  [Numbering](./using/items.md#numbering), or the **Season** when adding the
  torrent.
- The source numbers episodes differently from the release: TVDB and TMDB
  orders can differ, especially for anime. Fix the matches in the review.

## Something Is Missing from Wanted or Upcoming

- The episode, its season and its series must all be monitored.
- An episode counts as aired from the day after its air date, in the
  **Time zone** setting.
- A movie is only wanted from its digital or physical release, not while it's
  in cinemas.
- Episodes and dates come from the source; refresh the item if they changed.

## Watched State Doesn't Show

- Set the Jellyfin **User**; its name must match a Jellyfin user.
- Watched sync runs every 15 minutes.
- When Jellyfin sees the library at other paths, items are matched by id, so
  Jellyfin's series need ids from the same source as Yokoku's; see
  [Watched State](./integrations/jellyfin.md#watched-state).

## Jellyfin Doesn't Pick Up New Files

- **Test connection** under **Media server**: the API key must be an
  administrator's.
- A rescan waits until the library has been quiet for 30 seconds.
- `yokoku jellyfin rescan` asks for one now.

## A Job Runs at the Wrong Time

Schedules run in UTC; see [Schedules](./configuration/schedules.md).

## File Details Say "Not read yet"

`ffprobe` wasn't found when the file came in. Install ffmpeg or set
`files.ffprobe`, then run [`yokoku files probe`](./cli/files.md#files-probe).
