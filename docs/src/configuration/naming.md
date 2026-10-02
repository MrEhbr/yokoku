# Naming

Patterns decide the names of item folders and of the files Yokoku imports.
The defaults follow Jellyfin's recommended layout:

```
The Simpsons (1989)/Season 33/The Simpsons (1989) - S33E01 - Treehouse of Horror XII.mkv
Dune (2021)/Dune (2021).mkv
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
  subtitle suffixes fit.

## Subtitles

Subtitles are named after their video, with their tags:
`<video name>[.<language>][.sdh][.forced].<extension>`, like
`The Simpsons (1989) - S33E01.en.forced.srt`.

## Changing Patterns

New patterns apply to new imports and new items. Existing files keep their
names until you use [**Rename files…**](../using/items.md#rename-files) on an
item's page. An item's folder name is set when it's added.
