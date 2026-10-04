# TMDB and TVDB

Titles, episodes, air dates, release dates and artwork come from
[TMDB](https://www.themoviedb.org), and for series optionally from
[TheTVDB](https://thetvdb.com).

## TMDB

A TMDB token is required: without one, Yokoku can't search, add or refresh
anything, even with a TVDB key.

1. Sign in to TMDB and open
   [Settings → API](https://www.themoviedb.org/settings/api).
2. Request an API key, then copy the **API Read Access Token**: the long one,
   not the short API key.
3. Set it as **TMDB token** in Settings, or `metadata.tmdb.token`.

Movies always come from TMDB, and series too unless a TVDB key is set.

## TVDB

With `metadata.tvdb.api_key` set, series are searched and added from TVDB.
Get a key from your [TVDB dashboard](https://thetvdb.com/dashboard/account/apikey).
A user-supported key also needs your subscriber PIN in `metadata.tvdb.pin`.

Episodes come in TVDB's aired order.

> [!IMPORTANT]
> The source decides the episode numbers, and Jellyfin matches watched state
> by id when paths differ. Use the source your Jellyfin libraries use; see
> [Watched State](./jellyfin.md#watched-state).

A series stays with the source it was added from. Setting a TVDB key later
doesn't move existing TMDB series, and removing it breaks refreshes of TVDB
series. The source and id show on the item's page, like `tvdb:12345`.

## Language and Region

**Language** (`metadata.language`, like `en-US`)
:   The language of titles, overviews and logos. Where a translation is
    missing, TMDB returns what it has; TVDB falls back to the original.

**Region** (`metadata.region`, like `US`)
:   Whose movie release dates are shown. Of TMDB's release types, limited and
    theatrical count as **Cinema**, digital as **Digital** and physical as
    **Physical**; the earliest date of each wins.

## Refreshing

Every 12 hours Yokoku refreshes the items that are due:

- **A series** is due after 30 days, or when an aired episode still has no
  title. Otherwise, a series refreshed in the last 6 hours isn't due; after
  that, it is unless it ended and no episode aired in the last 30 days.
- **A movie** is due after 180 days. Otherwise, a movie refreshed in the last
  12 hours isn't due; after that, it is unless it's released and its
  physical release is more than 30 days past.

**Refresh** on an item's page, or `yokoku refresh`, refreshes right away.

## Artwork

Posters, backdrops and logos are downloaded the first time they're shown and
kept in an `artwork` folder next to the database. When a refresh brings new
artwork, the old file is replaced; removing an item deletes its artwork.

## Proxies and Mirrors

`metadata.tmdb.url` and `metadata.tvdb.url` change the API addresses. Images
are always loaded from `image.tmdb.org` and `artworks.thetvdb.com`.

## Errors

| Message | Cause |
|---|---|
| The metadata source refused the request; check the token | The TMDB token, TVDB key or PIN is wrong |
| The metadata source could not be reached; try again | No connection, or the source is down; Yokoku already retried |
| The metadata source no longer has it | The item was deleted at the source |
