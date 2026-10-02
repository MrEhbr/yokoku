# Imports

```toml
[import]
mode = "hardlink"            # hardlink, copy or move

[downloads]
remove_after_seeding = false
pick_up_labels = []
# pick_up_folder = "/downloads/tv"

[add]
monitor = "all"              # all, future, latest-season or none

[files]
ffprobe = "ffprobe"
```

## `import.mode`

How finished downloads reach the library.

| Mode | Torrent keeps seeding | Extra space |
|---|---|---|
| `hardlink` (default) | Yes | None; across file systems Yokoku copies instead |
| `copy` | Yes | A full copy |
| `move` | No | None |

See [Import Modes](../using/queue.md#import-modes).

## `downloads.remove_after_seeding`

Removes an imported torrent **with its downloaded data** once Transmission
finished seeding it. Off by default. See
[After Seeding](../integrations/transmission.md#after-seeding).

## `downloads.pick_up_labels`, `downloads.pick_up_folder`

Torrents added straight in Transmission are taken on when they carry one of
these labels or download under this folder. See
[Labels and Picking Up Torrents](../integrations/transmission.md#labels-and-picking-up-torrents).

## `add.monitor`

What a new series monitors at first: `all`, `future`, `latest-season` or
`none`. A new movie is monitored unless this is `none`. The add dialog starts
at this choice. See [The Add Dialog](../using/adding.md#the-add-dialog).

## `files.ffprobe`

The `ffprobe` that reads a file's codecs, resolution, length and subtitles:
its name on the `PATH`, or a path to it. The Docker image and the Nix package
include it. Without it, file details show *Not read yet*; read them later
with [`yokoku files probe`](../cli/files.md).
