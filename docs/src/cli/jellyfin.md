# yokoku jellyfin

## jellyfin test

Checks that Jellyfin can be reached with the API key.

```console
$ yokoku jellyfin test
Connected to Jellyfin 10.10.7
```

## jellyfin rescan

Asks Jellyfin to rescan its libraries now, instead of after the next library
change.

```console
$ yokoku jellyfin rescan
Jellyfin is rescanning
```

Both fail with *No Jellyfin configured* while `jellyfin.url` is empty. For
other errors, see [Jellyfin](../integrations/jellyfin.md#connect).
