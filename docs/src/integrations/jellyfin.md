# Jellyfin

With [Jellyfin](https://jellyfin.org) connected, Yokoku asks it to rescan
after the library changes, and reads which files a Jellyfin user has watched.
Both are optional.

## Connect

| Setting | |
|---|---|
| `jellyfin.url` | Its address, like `http://localhost:8096`; empty to not use Jellyfin |
| `jellyfin.api_key` | An API key, from Jellyfin's *Dashboard → API Keys* |
| `jellyfin.user` | The user whose played items count as watched; empty to not sync watched files |

**Test connection** in Settings, or `yokoku jellyfin test`, shows the
version, or why it failed:

| Message | Cause |
|---|---|
| media server unavailable | Jellyfin isn't running, or the address is wrong |
| the API key is missing, wrong or not an administrator's | Create a new key under *API Keys* |
| no Jellyfin user is named "…" | Check `jellyfin.user`; case doesn't matter |

## Rescans

Importing, renaming or deleting files asks Jellyfin to rescan its libraries.
Yokoku waits until nothing has changed for 30 seconds, so an import of a
whole season gives one rescan. A rescan Jellyfin misses, because it's down,
is retried every 10 seconds, also after Yokoku restarts.

To rescan now: `yokoku jellyfin rescan`.

Jellyfin rescans all its libraries, not just the changed folders.

## Watched State

Every 15 minutes Yokoku reads the played episodes and movies of the
`jellyfin.user`, and marks the matching library files as watched. It's a
mirror: marking something unplayed in Jellyfin clears it in Yokoku too, and
Yokoku never writes to Jellyfin.

A Jellyfin item matches a library file when:

1. its path is the same as the file's, or
2. its TMDB or TVDB id matches the item's, with the same season and episode.

A file holding several episodes counts as watched once all of them are.

> [!IMPORTANT]
> If Jellyfin sees your library at other paths, as is common with Docker,
> matching relies on ids: Jellyfin's series must carry the id of the source
> Yokoku uses. Series come from TVDB once a TVDB key is set; if Jellyfin's
> series only have TMDB ids, keep Yokoku on TMDB, or the other way around.

Watched state shows on the [Library](../using/library.md) filter and badges,
and on item pages, where hovering shows when it was last played. On
**Remove…**, **Watched** chooses every watched file to free space.

## Library Setup

Point Jellyfin's TV library at your series root folder and its movie library
at the movies root folder. Yokoku's default naming patterns,
`Title (Year)/Season 01/Title (Year) - S01E01 - Episode.mkv`, are names
Jellyfin recognizes.
