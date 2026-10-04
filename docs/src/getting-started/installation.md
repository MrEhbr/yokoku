# Installation

Yokoku is one binary that serves the web UI on port 8080. Pick the method that
fits your server.

## Before You Start

- **A TMDB API read access token.** Yokoku can't search or add anything
  without one. Create it under
  [API settings](https://www.themoviedb.org/settings/api) on TMDB.
- **Transmission**, to download torrents.
- **Jellyfin** (optional), for rescans after imports and watched state.

> [!IMPORTANT]
> Yokoku reads finished downloads at the path Transmission reports for them,
> so it must see the download folder at the **same path** as Transmission.
> It reads the download folder and writes to the library folders.

## Docker

Multi-arch images (`linux/amd64`, `linux/arm64`) are published to GHCR:

```bash
docker run -d --name yokoku \
  -p 8080:8080 \
  -v yokoku-data:/data \
  -v /srv/media:/srv/media \
  -e YOKOKU__METADATA__TMDB__TOKEN=<token> \
  -e YOKOKU__TRANSMISSION__URL=http://transmission:9091/transmission/rpc \
  ghcr.io/mrehbr/yokoku:latest
```

- `/data` holds the database and artwork.
- `/srv/media` stands for the folder holding both your downloads and your
  library. Mount it at the path Transmission uses for it.

> [!TIP]
> Mount downloads and the library as **one** volume. A hardlink can't cross
> two mounts, so Yokoku falls back to copying every imported file, which
> doubles the space it takes.

The image reads `/config/app.toml`, which listens on every interface and
keeps the database in `/data`. Set anything else with
`YOKOKU__<SECTION>__<KEY>` variables, or mount your own file over
`/config/app.toml`.

## Release Archive

Static Linux binaries for x86_64 and arm64 are on the
[releases](https://github.com/MrEhbr/yokoku/releases) page:

```bash
mkdir -p /opt/yokoku
curl -sSL https://github.com/MrEhbr/yokoku/releases/latest/download/yokoku_Linux_x86_64.tar.gz \
  | tar -xz -C /opt/yokoku
/opt/yokoku/yokoku --version
```

The archive holds the `yokoku` binary and the `public/` folder with the web
UI, which must stay next to it. Copy
[`config/app.toml`](https://github.com/MrEhbr/yokoku/blob/main/config/app.toml)
next to them, set the TMDB token, and start it:

```bash
/opt/yokoku/yokoku --config /opt/yokoku/app.toml
```

Install [ffmpeg](https://ffmpeg.org) as well: Yokoku runs `ffprobe` to read
the codecs, resolution and length of library files, and `ffmpeg` to merge
external audio and subtitles into videos.

> [!NOTE]
> `config/app.toml` listens on `127.0.0.1` and keeps the database in
> `data/yokoku.db`, relative to the working directory. Change `web.host` and
> `database.path` to suit your server.

## NixOS

[nur-packages](https://github.com/MrEhbr/nur-packages) has a Yokoku package and
a `services.yokoku` module:

```nix
# flake.nix
inputs.nur-packages.url = "github:MrEhbr/nur-packages";

# configuration.nix
imports = [ inputs.nur-packages.nixosModules.yokoku ];

services.yokoku = {
  enable = true;
  openFirewall = true;
  settings = {
    web.host = "0.0.0.0";
    metadata.tmdb.token.file = "/run/secrets/tmdb-token";
    transmission.url = "http://127.0.0.1:9091/transmission/rpc";
    roots = [
      { kind = "series"; path = "/media/library/Shows"; }
      { kind = "movies"; path = "/media/library/Movies"; }
    ];
  };
};
```

`settings` takes any setting from
[`config/app.toml`](https://github.com/MrEhbr/yokoku/blob/main/config/app.toml)
and is written to `/etc/yokoku/app.toml`. The database and artwork go to
`/var/lib/yokoku`.

| Option | Default | |
|---|---|---|
| `user`, `group` | `yokoku` | Must read the download folder and write to the library folders; set them to your media user to share files with Transmission and Jellyfin |
| `dataDir` | `/var/lib/yokoku` | The database and artwork |
| `settings.web.host` | `127.0.0.1` | Address the web UI listens on |
| `settings.web.port` | `8080` | Port the web UI listens on |
| `openFirewall` | `false` | Opens the web port |

> [!TIP]
> Secrets take `{ file = "..."; }`, so they never reach the Nix store. Point
> them at agenix or sops-nix secrets.

## Next

[First Run](./first-run.md) takes you from here to the first item in the
library.
