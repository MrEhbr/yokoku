# yokoku root

Manage the folders that hold series and movies; the same as **Settings →
Library → Root folders**.

## root add

```
yokoku root add <KIND> <PATH>
```

Adds a root folder. `KIND` is `series` or `movies`; `PATH` is an absolute path
to an existing folder that doesn't overlap another root folder.

```console
$ yokoku root add series /srv/media/series
Added series root /srv/media/series
```

## root list

```console
$ yokoku root list
movies  /srv/media/movies
series  /srv/media/series
```

## root remove

```
yokoku root remove <PATH>
```

Removes a root folder. It's refused while series or movies still belong to it;
remove them first. The folder and its files stay on disk.
