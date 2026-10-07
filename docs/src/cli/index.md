# CLI Overview

```
yokoku [OPTIONS] [COMMAND]
```

Without a command, `yokoku` runs the service: the web UI and the background
jobs, until it's stopped. The commands are for setup and maintenance; they can
run while the service does, against the same config and database.

| Command | |
|---|---|
| [`root`](./root.md) | Manage the folders that hold series and movies |
| [`scan`](./scan.md) | Link files in the root folders to the library |
| [`refresh`](./refresh.md) | Refresh metadata for one item or the whole library |
| [`files`](./files.md) | Show the details of library files, or read them with ffprobe |
| [`jellyfin`](./jellyfin.md) | Test the Jellyfin connection or ask it to rescan |
| [`job`](./job.md) | Run a scheduled job once, now |
| [`settings`](./settings.md) | Store settings in the database, over the config file |
| [`completions`](./completions.md) | Print the completion script for a shell |

## Global Options

These work before or after the command.

`-c`, `--config <FILE>`
:   The config file; `config/app.toml` by default. Point commands at the file
    the service uses, like `--config /etc/yokoku/app.toml` on NixOS.

`-v`, `--verbose`; `-q`, `--quiet`
:   More or less logging; repeat for more, like `-vv`.

`-V`, `--version`; `-h`, `--help`
:   The version, or help for any command.

## Running Next to the Service

Run commands as the service's user, so files they create have the right owner:

```bash
# NixOS
sudo -u yokoku yokoku --config /etc/yokoku/app.toml root list
# Docker
docker exec yokoku /app/yokoku --config /config/app.toml root list
```

A change a command makes, like a stored setting, reaches the running service
within the event poll interval, 5 seconds by default.

A command that fails prints `Error:` with the cause and exits with status 1.
