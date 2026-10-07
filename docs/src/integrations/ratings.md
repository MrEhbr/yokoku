# IMDb Ratings

Series and movie pages show the IMDb rating next to the genres and runtime,
like *IMDb 7.8 (1.2M votes)*, linked to the title's IMDb page. Ratings come
from IMDb's [daily dataset](https://developer.imdb.com/non-commercial-datasets/),
which IMDb provides for personal and non-commercial use. No account or key is
needed.

## Which Items Have One

An item needs an IMDb id, which it gets from TMDB or TVDB on a
[refresh](./metadata.md#refreshing). An item the source knows no IMDb id for,
like a very new or regional title, shows no rating.

A rating from fewer than 1,000 votes isn't shown: it says little yet.

## Refreshing

Ratings are refreshed daily at 06:00 (`serve.refresh_ratings`), and for a new
item right after it's added. To refresh them now, run
[`yokoku job refresh-ratings`](../cli/job.md).

The dataset is about 9 MB. It's kept in a `ratings` folder next to the
database and downloaded again only once IMDb publishes a new one. When IMDb
can't be reached, the ratings already shown stay.

## Proxies and Mirrors

`ratings.imdb_url` changes the dataset's address, from
`https://datasets.imdbws.com/title.ratings.tsv.gz`.
