# Direct Torznab Feeds

Yokoku can search torrent indexers directly through their Torznab API, alongside trackers configured in [Jackett](./jackett.md). It searches only when you press **Search**, and sends only the release you choose to Transmission.

## Add a Feed

In **Settings → Indexer → Direct Torznab feeds**, choose **Add feed**. Enter a name, the full API URL supplied by the indexer (often ending in `/api`), and its API key if required. **Test** checks the feed's search capabilities. You can edit, disable or remove feeds added here. A feed set in the config file is shown as read only and can be tested in Settings.

For a config file feed, add:

```toml
[[torznab.feeds]]
id = "example"
name = "Example"
url = "https://indexer.example/api"
api_key = { file = "/run/secrets/example_api_key" }
enabled = true
```

The ID must be unique across file and Settings feeds; use ASCII letters, digits, `-` or `_`. The URL must not contain credentials or query parameters. Yokoku keeps API keys and torrent download links on the server.

## Search and Download

Add a movie or series to the library, then use **Search releases** on its page. Choose indexers to search, filter and sort results, then press **Download**. For a series, you can select a season to narrow the search and place files whose names omit the season.

Search results expire after 30 minutes. If a download says the search expired, run it again and choose the release.
