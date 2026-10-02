# Adding Series and Movies

On **Library**, click **Add**. Choose **Series** or **Movies**, type a title
and click **Search**.

Movies are searched on TMDB. Series are searched on TMDB too, or on TVDB once
a TVDB API key is set. Each result shows its poster, year and overview. One
already in your library is tagged **In library** and opens its page.

> [!TIP]
> The search is part of the address, so you can bookmark it or go back to it.

## The Add Dialog

Pick a result to add it.

**Root folder**
:   Where the item's folder goes. Only root folders for its kind are offered;
    add one under [Settings](./settings.md#root-folders) first.

**Monitor**
:   What counts toward **Wanted** and **Upcoming**. It starts at the
    **Monitor new series** setting.

    | Series | |
    |---|---|
    | All episodes | Every episode, aired or not |
    | Future episodes | Episodes that haven't aired yet, and ones without an air date |
    | Latest season | The episodes of the latest season |
    | Nothing | No episode |

    Specials (season 0) are never monitored at first. A movie is
    **Monitored** or **Not monitored**. You can change all of this later on
    the item's page.

**Folder**
:   The item's folder name, from your naming patterns. The hint under it says
    what happens:
    - *Creates …*: the folder is made when the first file is imported.
    - *… exists; the files in it are linked to this item*: Yokoku scans the
      folder after adding the item and links the files it recognizes.
    - *… belongs to another item*: choose another name.

**Add** saves the item and opens its page.

## Bringing In an Existing Library

Add each series and movie into the root folder that already holds it, and
keep the folder name the dialog shows as existing. Its files are linked
without moving or renaming them.

Files whose episode Yokoku can't tell from the name stay unlinked; the item's
page lets you sort them out. To bring the names in line with your patterns
afterwards, use [**Rename files**](./items.md#rename-files).

## When Adding Fails

| Message | What to do |
|---|---|
| Set a TMDB token … to search and add items | Set the TMDB token in [Settings](./settings.md) |
| No root folder for series / movies | Add a root folder in Settings |
| It is already in the library | It's there; search the library |
| The metadata source refused the request; check the token | The TMDB token or TVDB key is wrong |
| The metadata source could not be reached; try again | TMDB or TVDB is down or unreachable from the server |
