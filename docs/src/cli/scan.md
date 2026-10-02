# yokoku scan

```
yokoku scan
```

Looks through the item folders in every root folder and links files that were
added or deleted outside Yokoku; the same as **Scan now** in Settings. The
service also does it every day; see [Schedules](../configuration/schedules.md).

```console
$ yokoku scan
Linked 3 new files
Forgot 1 files missing from disk
2 folders need review; match their files on each item's page
```

- **Linked**: new files whose episode or movie was recognized.
- **Forgot**: files that are gone from disk.
- **Need review**: folders with files Yokoku couldn't match. Open the item's
  page and click **Match** on the *files weren't recognised* notice.

Only folders of items in the library are scanned. To bring in a folder of an
item you haven't added, [add the item](../using/adding.md#bringing-in-an-existing-library).
