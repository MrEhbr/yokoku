# yokoku job

```
yokoku job <JOB>
```

Runs one of the [scheduled jobs](../configuration/schedules.md) once, now, the
same way its schedule does.

| Job | |
|---|---|
| `sync-downloads` | Read torrents from Transmission and pick up new ones |
| `sync-active-downloads` | The same, only while a download is queued or downloading |
| `execute-imports` | Place the files of approved imports |
| `scan-library` | Look for files changed outside Yokoku in the root folders |
| `refresh-metadata` | Refresh the items that are [due](../integrations/metadata.md#refreshing) |
| `rescan-media-server` | Ask Jellyfin to rescan once the library has been quiet for 30 s |
| `sync-watched` | Read the Jellyfin user's played items |

```console
$ yokoku job sync-watched
Ran sync-watched
```

The job runs in the command's own process, next to the service. To refresh
every item rather than the due ones, use [`yokoku refresh`](./refresh.md); to
rescan Jellyfin without waiting, [`yokoku jellyfin rescan`](./jellyfin.md).
