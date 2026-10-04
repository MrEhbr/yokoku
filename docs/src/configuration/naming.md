# Naming

Patterns decide the names of item folders and of the files Yokoku imports.
The defaults follow Jellyfin's recommended layout:

```
Show Name (2020)/Season 01/Show Name (2020) - S01E01 - Pilot.mkv
Movie Name (2021)/Movie Name (2021).mkv
```

```toml
[naming]
series_folder = "{title}[ ({year})]"
season_folder = "Season {season}"
episode_file  = "{title}[ ({year})] - {episodes}[ - {episode_title}]"
movie_folder  = "{title}[ ({year})]"
movie_file    = "{title}[ ({year})]"
```

## Tokens

| Token | Value | Allowed in |
|---|---|---|
| `{title}` | The series or movie title | Every pattern but the season folder |
| `{year}` | Its year | Every pattern but the season folder |
| `{season}` | The season, two digits: `01`, `00` for specials | Season folder (required), episode file |
| `{episodes}` | `S01E01`, or `S01E01-E02` for a file with several episodes | Episode file (required) |
| `{episode_title}` | The episode title | Episode file |

A `[...]` group is dropped when a token in it has no value, so
`{title}[ ({year})]` gives `Title` for an item without a year instead of
`Title ()`.

A pattern that uses a token where it isn't allowed, or misses a required one,
is refused with an error naming the pattern.

## Safe Names

Names are made valid on Linux, macOS, Windows and SMB shares:

- `: ` becomes ` - `, and `:`, `/`, `\` and `|` become `-`;
- `"` becomes `'`; `<`, `>`, `?` and `*` are dropped;
- a name is cut to 255 bytes, a file name before its extension to 200 so
  subtitle and audio suffixes fit.

## Subtitles and External Audio

Subtitle files and external audio tracks (`.mka`, `.ac3`, `.dts` and the like)
that belong to a video are imported with it. A file belongs to a video when its
name starts with the video's name, or it sits in a folder named after the video;
a subtitle also belongs to the only video of a download.

In the library each is named after its video, followed by the words of the
folders it sat in and the rest of its own name:

| In the download | In the library |
|---|---|
| `Show - 01.en.forced.srt` | `Show (2016) - S04E01.en.forced.srt` |
| `RUS Sound/Studio/Show - 01.mka` | `Show (2016) - S04E01.RUS.Sound.Studio.mka` |
| `RUS Subs/Group/Show - 01.ass` | `Show (2016) - S04E01.RUS.Subs.Group.ass` |

Jellyfin reads the language and flags like `forced` from these names and
shows the other words as the track's title. Renaming an item keeps them.

## Changing Patterns

New patterns apply to new imports and new items. Existing files keep their
names until you use [**Rename files…**](../using/items.md#rename-files) on an
item's page. An item's folder name is set when it's added.
