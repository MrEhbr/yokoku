# yokoku root

Manage the folders that hold series and movies; the same as **Settings →
Library → Root folders**. Root folders can also be set in the
[config file](../configuration/index.md#root-folders).

## root add

```
yokoku root add <KIND> <PATH> [--name <NAME>]
```

Adds a root folder. `KIND` is `series` or `movies`; `PATH` is an absolute path
to an existing folder that doesn't overlap another root folder. `--name` is
shown instead of the path; without it, the folder's name is.

```console
$ yokoku root add series /media/library/CartoonShows --name "Cartoon shows"
Added series root /media/library/CartoonShows as Cartoon shows
```

## root list

Lists every root folder with its name, those from the config file included.

```console
$ yokoku root list
movies  /media/library/Movies  Movies
series  /media/library/Anime  Anime  (config file)
series  /media/library/CartoonShows  Cartoon shows
```

## root remove

```
yokoku root remove <PATH>
```

Removes a root folder. It's refused while series or movies still belong to it,
and for one from the config file. The folder and its files stay on disk.
