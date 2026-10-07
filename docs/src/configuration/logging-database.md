# Logging and Database

Both are read when Yokoku starts.

## Logging

```toml
[log]
level = "info"       # trace, debug, info, warn or error
format = "console"   # console or json
output = "stderr"    # stderr, stdout or { file = "logs/yokoku.log" }
```

- `-v` and `-q` on the command line raise or lower the level, over
  `log.level`.
- `RUST_LOG`, when set, replaces the level with its own filter, like
  `RUST_LOG=debug`.
- A file output is appended to and never rotated; use logrotate or the
  console output under systemd or Docker.

## Database

```toml
[database]
path = "data/yokoku.db"
```

Yokoku keeps everything in one SQLite file: the library, downloads, history
and stored settings. Next to it are:

| Path | |
|---|---|
| `yokoku.db-wal`, `yokoku.db-shm` | SQLite's write-ahead log; part of the database |
| `yokoku.lock` | Lets one process at a time change library files |
| `artwork/` | Cached posters, backdrops and logos; downloaded again when missing |
| `ratings/` | The IMDb ratings dataset; downloaded again when missing |

A relative `path` is relative to the working directory. The default is
`yokoku.db`; `config/app.toml` uses `data/yokoku.db`, the Docker image
`/data/yokoku.db`, and the NixOS module `/var/lib/yokoku/yokoku.db`.

The schema is upgraded automatically when a new version starts.

### Backups

Stop Yokoku and copy the database file together with its `-wal` and `-shm`
files, or back up a running one with SQLite:

```bash
sqlite3 data/yokoku.db ".backup 'yokoku-backup.db'"
```

The `artwork` and `ratings` folders don't need a backup.
