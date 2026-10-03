# Configuration

Yokoku reads its settings from four places. Each one goes over the ones after
it:

1. **Environment variables** `YOKOKU__<SECTION>__<KEY>`
2. **Stored settings**, changed on the [Settings](../using/settings.md) page
   or with `yokoku settings set`
3. **The config file**
4. **Defaults**

## The Config File

`--config` (or `-c`) names a TOML file, `config/app.toml` by default. A
missing file is fine: the defaults apply.
[`config/app.toml`](https://github.com/MrEhbr/yokoku/blob/main/config/app.toml)
in the repository lists every setting with a comment; copy it as a starting
point.

```toml
[metadata.tmdb]
token = { file = "/run/secrets/tmdb_token" }

[transmission]
url = "http://localhost:9091/transmission/rpc"

[jellyfin]
url = "http://localhost:8096"
api_key = { file = "/run/secrets/jellyfin_api_key" }
user = "admin"

[import]
mode = "hardlink"
```

The service reads the file when it starts, so a change to it applies after a
restart.

## Environment Variables

Every setting has a variable: `YOKOKU__`, then its section and key, upper case
and separated by double underscores.

```bash
YOKOKU__WEB__PORT=9000
YOKOKU__IMPORT__MODE=copy
YOKOKU__METADATA__TMDB__TOKEN=eyJhbGciOi...
YOKOKU__METADATA__TMDB__TOKEN__FILE=/run/secrets/tmdb_token
```

A setting set by a variable shows as **Locked** on the Settings page.

## Secrets

The TMDB token, TVDB key and PIN, Transmission password and Jellyfin API key
take a value or a file:

```toml
password = "literal-value"
password = { file = "/run/secrets/transmission_password" }
```

From the environment, add `__FILE` to the variable for the file form. Prefer
files: they keep secrets out of the config file, the process environment and,
on NixOS, the Nix store.

## Stored Settings

Settings changed on the Settings page are kept in the database and apply at
once. The CLI does the same:

```bash
yokoku settings list                 # what's stored
yokoku settings get import.mode      # the value in effect
yokoku settings set import.mode copy
yokoku settings unset import.mode    # back to the config file
```

A running service picks up a change from the CLI within the event poll
interval, 5 seconds by default.

## Read at Start

`[database]`, `[web]`, `[log]`, `[events]` and `[[roots]]` are read only when
Yokoku starts. They can't be stored; set them in the config file or environment and
restart.

| Setting | Default | |
|---|---|---|
| `web.host` | `127.0.0.1` in `config/app.toml`, `0.0.0.0` in the Docker image | Address the web UI listens on |
| `web.port` | `8080` | |
| `database.path` | `yokoku.db` | See [Logging and Database](./logging-database.md) |
| `events.poll_interval_ms` | `5000` | How often the service checks for changes made from the CLI |

## Root Folders

Root folders can also be set in the config file, next to the ones added in the
web UI or with `yokoku root add`:

```toml
[[roots]]
kind = "series"            # series or movies
path = "/media/library/Anime"

[[roots]]
kind = "movies"
path = "/media/library/AnimeMovies"
name = "Anime movies"      # shown instead of the path; the folder's name when unset
```

- They show in Settings marked **Config file**, and can't be removed there or
  with `yokoku root remove`; delete them from the file instead.
- A root folder both in the file and added earlier in the UI counts once, as
  the file's.
- The service refuses to start when a configured root folder is relative,
  overlaps another one, or conflicts with a stored one: the same path for the
  other kind, or one inside the other. Remove the stored one with
  `yokoku root remove`, or change the file.
- Items on a root folder removed from the file keep working; only new items
  can't be added to it.

## Sections

- [Naming](./naming.md): folder and file name patterns
- [Imports](./imports.md): import mode, picking up and removing torrents
- [Schedules](./schedules.md): when background jobs run
- [Logging and Database](./logging-database.md)
- [Transmission](../integrations/transmission.md),
  [Jellyfin](../integrations/jellyfin.md) and
  [TMDB and TVDB](../integrations/metadata.md)
