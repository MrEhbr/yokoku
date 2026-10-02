# yokoku files

## files show

```
yokoku files show <KIND> <SOURCE>
```

Lists the files of a series or movie with their size, length, video, audio and
subtitles. `KIND` is `series` or `movie`; `SOURCE` is the item's source and id,
as shown on its page.

```console
$ yokoku files show series tvdb:71663
/srv/media/series/The Simpsons (1989)/Season 33/The Simpsons (1989) - S33E01 - The Star of the Backstage.mkv
  0.9 GB, 0h 20m, 1920x1080 h264
  Audio      rus eac3 5.1, eng eac3 5.1
  Subtitles  rus, eng in the file
```

## files probe

```
yokoku files probe
```

Reads the details of every library file not read yet with `ffprobe`. Imports
and scans read new files on their own; run this after installing ffprobe or
fixing `files.ffprobe`.

```console
$ yokoku files probe
Probed 12 files
```

When ffprobe can't be found, it stops with *ffprobe is not installed; set
[files] ffprobe to its path*. See [`files.ffprobe`](../configuration/imports.md#filesffprobe).
