# Prowlarr

Connect [Prowlarr](https://github.com/Prowlarr/Prowlarr) to search the torrent indexers you configured there from a movie or series page. Select Prowlarr under **Settings → Indexer → Search with**. Searches start only when you press **Search**. You choose which result to download, and Yokoku adds it to Transmission for that library item.

## Connect

| Setting | Value |
|---|---|
| `indexer.provider` | `prowlarr` |
| `prowlarr.url` | Prowlarr's address, for example `http://localhost:9696` |
| `prowlarr.api_key` | The API key in Prowlarr's **Settings → General** |

Enter these under **Settings → Indexer**, then use **Test Prowlarr**. Add and configure trackers in Prowlarr. Yokoku lists only enabled torrent indexers that support search; Usenet indexers are skipped.

For a series, choose a season before searching if you know it. Yokoku passes that season to indexers that support season search. When an indexer supports only text search, Yokoku adds `S01` to the query instead. Search results identify their tracker and whether it came from Prowlarr or Jackett.

Keep Prowlarr's **Redirect** setting off for a torrent indexer if its download returns an external URL. Yokoku accepts a torrent file or magnet link from Prowlarr, then sends it to Transmission. A non-magnet redirect shows an error with this setting as the suggested fix.

If you previously configured direct Torznab feeds in Yokoku, add those trackers to Prowlarr or Jackett. The old feed settings are no longer used.
