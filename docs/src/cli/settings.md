# yokoku settings

Stores settings in the database, over the config file, like the
[Settings](../using/settings.md) page. Keys are the setting's section and name,
like `import.mode`; see [Configuration](../configuration/index.md).

```console
$ yokoku settings set import.mode copy
Set import.mode = "copy"
$ yokoku settings get import.mode
"copy"
$ yokoku settings list
import.mode = "copy"
$ yokoku settings unset import.mode
Unset import.mode
```

When a `YOKOKU__*` variable sets the same key, `set` stores the value but warns
that the variable takes precedence.

`set`
:   Stores a value. It overrides the config file until it's unset.

`get`
:   Shows the value in effect, from wherever it comes; a secret is masked.

`list`
:   Lists the stored values.

`unset`
:   Removes a stored value, so the config file or default applies again.

Settings read at start (`database`, `web`, `log`, `events`) can't be stored.
