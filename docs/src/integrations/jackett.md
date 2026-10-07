# Jackett

With [Jackett](https://github.com/Jackett/Jackett) connected, **Search
releases** on a series or movie page searches every tracker added in Jackett,
like RuTracker, and downloads the release you pick. It's optional: without
it, you can search [direct Torznab feeds](./torznab.md) or add torrents yourself with **Add torrent**.

Yokoku never picks a release on its own; it only searches when you ask.

## Connect

| Setting | |
|---|---|
| `jackett.url` | Its address, like `http://localhost:9117`; empty to not search |
| `jackett.api_key` | The API key, shown at the top of Jackett's dashboard |

Add your trackers in Jackett's own page, with their logins. Yokoku searches
them all through Jackett's aggregate Torznab feed and never sees the
trackers' passwords.

**Test connection** in Settings shows *Jackett*, or why it failed:

| Message | Cause |
|---|---|
| indexer unavailable | Jackett isn't running, or the address is wrong |
| indexer refused the request: Invalid API Key | Copy the key from Jackett's dashboard again |

## Search

**Search releases** starts with the item's title; change it to search for
something else, like the original title. For a series, **Season** narrows it
to one season. Every category is searched, since trackers often file
releases under the wrong one.

With more than one tracker in Jackett, **Trackers** picks where to search,
every tracker at first. A search waits for its slowest tracker, like an
anime tracker that takes seconds to answer; leave it out to get the others'
results sooner. The picked trackers are searched at once, and one that fails
leaves the others' results.

Results come most seeded first, with their size, seeders, leechers, age and
tracker. **Downloads**, how often a release was downloaded, shows when a
tracker in the results counts it; Rutor doesn't. Click a column heading to
sort by it, again to reverse; on a phone,
**Sort by** picks the order. A title links to the release's page on the
tracker.

The filter box narrows the results by title as you type, without searching
again. **How to filter** under it lists the syntax:

| Filter | Keeps |
|---|---|
| `bb 1080` | Titles with these letters in this order, anything between them |
| `'web-dl` | Titles containing exactly `web-dl` |
| `^во` | Titles starting with `во` |
| `hdtv$` | Titles ending with `hdtv` |
| `!hevc` | Titles without `hevc` |

Words separated by spaces must all match, like `'1080p !hevc 'lostfilm`. A
plain word matches loosely, so `1080p` also finds *2008 … 720p*; start it
with `'` to match it exactly. Lowercase words ignore case; a word with a
capital letter matches case.

**Download** sends the release to Transmission for this item, as
**Add torrent** does, and opens the **Queue**. For a series, the season
picked for the search also places files named without one.

| Message | Cause |
|---|---|
| Set the Jackett address in Settings to search | `jackett.url` is empty |
| no tracker added in Jackett supports TV searches | Add a tracker in Jackett, or one with TV or movie search |
| the tracker sent no torrent | The tracker answered with a page instead, often a login; check the tracker in Jackett |

## RuTracker

In Jackett, add *RuTracker* with your account. Its options make results
easier to read:

- **Strip Russian letters** leaves the original title, like
  *The Walking Dead* from *Ходячие мертвецы / The Walking Dead*
- **Move all tags to end of release title** puts the year and quality after
  the title

Search with the original title: RuTracker titles carry it next to the
Russian one.
