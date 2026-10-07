# yokoku completions

```
yokoku completions <SHELL>
```

Prints the completion script for `bash`, `zsh`, `fish`, `elvish` or
`powershell`. It reads no config file, so it works anywhere the binary does.
The NixOS package installs the bash, zsh and fish scripts already.

```bash
# bash
yokoku completions bash > ~/.local/share/bash-completion/completions/yokoku
# zsh, into a directory on $fpath
yokoku completions zsh > ~/.zfunc/_yokoku
# fish
yokoku completions fish > ~/.config/fish/completions/yokoku.fish
```

Generate the script again after upgrading, so new commands and options complete.
