# Schedules

Background jobs run on cron schedules with seconds:
`second minute hour day month weekday`. A change applies from the job's next
run.

```toml
[serve]
sync_downloads        = "*/30 * * * * *"
sync_active_downloads = "*/5 * * * * *"
execute_imports       = "*/5 * * * * *"
rescan_media_server   = "*/10 * * * * *"
sync_watched          = "0 */15 * * * *"
refresh_metadata      = "0 0 */12 * * *"
scan_library          = "0 0 5 * * *"
```

| Setting | Default | Job |
|---|---|---|
| `sync_downloads` | every 30 s | Reads torrents from Transmission and picks up new ones |
| `sync_active_downloads` | every 5 s | The same, only while a download is queued, checking or downloading |
| `execute_imports` | every 5 s | Places the files of approved imports |
| `rescan_media_server` | every 10 s | Asks Jellyfin to rescan once the library has been quiet for 30 s |
| `sync_watched` | every 15 min | Reads the Jellyfin user's played items |
| `refresh_metadata` | every 12 h | Refreshes the items that are [due](../integrations/metadata.md#refreshing) |
| `scan_library` | 05:00 daily | Looks for files added or deleted outside Yokoku in the root folders |

Schedules follow the **Time zone** setting (`clock.timezone`), or the server's own
zone when it's empty: `0 0 5 * * *` is 05:00 there.

An invalid schedule is refused with `Invalid schedule "...": <reason>`.
