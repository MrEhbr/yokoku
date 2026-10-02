# yokoku refresh

```
yokoku refresh [KIND SOURCE]
```

Reads titles, dates, episodes and artwork from TMDB or TVDB again.

```bash
yokoku refresh                    # every item in the library
yokoku refresh series tvdb:71663  # one series
yokoku refresh movie tmdb:438631  # one movie
```

`KIND` is `series` or `movie`; `SOURCE` is the item's source and id, as shown
on its page. Give both or neither.

Without arguments, every item is refreshed, whether it's due or not; the
service refreshes only [due items](../integrations/metadata.md#refreshing) on
its own.

```console
$ yokoku refresh
Refreshed 9 items
Failed Severance (2022): metadata source unavailable
Error: 1 items could not be refreshed
```

A TMDB token is required, also for TVDB series.
