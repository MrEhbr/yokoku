# Transmission

Yokoku sends torrents to [Transmission](https://transmissionbt.com), follows
them while they download, and imports their files when they finish. Use
Transmission 3.0 or newer: labels, which Yokoku uses to recognize its
torrents, came in 3.0.

## Connect

| Setting | |
|---|---|
| `transmission.url` | The full RPC address, like `http://localhost:9091/transmission/rpc` |
| `transmission.username` | Empty when Transmission asks for none |
| `transmission.password` | A secret: a value or `{ file = "..." }` |

**Test connection** in Settings shows *Connected: Transmission 4.0.6*, or why
it failed:

| Message | Cause |
|---|---|
| download client unavailable | Transmission isn't running, the address is wrong, or it's blocked |
| download client refused the request: wrong username or password | Check the username and password |

> [!TIP]
> If Transmission runs on another machine, allow Yokoku's address in its
> `rpc-whitelist`, or turn the whitelist off and use a password.

## Paths

Yokoku doesn't choose where torrents download: Transmission's own download
folder applies. When a torrent finishes, Yokoku reads its files at the path
Transmission reports, so both must see the download folder at the same path.

With Docker, mount the download folder at the same path in both containers.
For hard links, keep downloads and the library on one file system, and in one
volume; see [Import Modes](../using/queue.md#import-modes).

## Labels and Picking Up Torrents

Every torrent Yokoku adds gets the label `yokoku`. On each sync Yokoku takes
on any torrent it doesn't know yet that has:

- the label `yokoku`;
- a label from **Pick up labels** (`downloads.pick_up_labels`);
- or a download folder under **Pick up folder** (`downloads.pick_up_folder`).

Picked-up torrents are matched to library items by their file names.

```toml
[downloads]
pick_up_labels = ["tv", "movies"]
pick_up_folder = "/srv/media/downloads/yokoku"
```

## Syncing

Yokoku asks Transmission for its torrents every 30 seconds, and every 5
while one is queued, checking or downloading. A download counts as finished
once all its wanted files are complete and it isn't being checked. Change the
timing under [Schedules](../using/settings.md#sections).

## After Seeding

With **Remove torrents after seeding** (`downloads.remove_after_seeding`) on,
a torrent is removed **with its downloaded data** once:

- its files were imported, and
- Transmission finished seeding it, by its seed ratio or idle limit.

Set those limits in Transmission. The library files stay: hard links and
copies don't depend on the download.

A torrent you remove in Transmission yourself leaves the Queue on the next
sync.
