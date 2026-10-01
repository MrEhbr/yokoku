# Jellyfin and Transmission for development and the integration tests: `nix run .#services` (or
# `just services`) from the repository root; state under data/services.
{ inputs, ... }:
{
  imports = [ inputs.process-compose-flake.flakeModule ];

  perSystem =
    { pkgs, ... }:
    let
      dir = "data/services";
      apiKey = "yokoku-dev-key";
      ports = {
        jellyfin = 18096;
        transmission = 19091;
      };

      # Jellyfin without its setup wizard: the first start seeds the database, an API key and
      # libraries over yokoku's library roots and the integration tests' media folders.
      jellyfin-dev = pkgs.writeShellApplication {
        name = "jellyfin-dev";
        runtimeInputs = with pkgs; [
          jellyfin
          sqlite
          coreutils
        ];
        text = ''
          state=${dir}/jellyfin
          tests=$state/test-media
          run() {
            jellyfin --datadir "$state" --configdir "$state/config" --cachedir "$state/cache" --logdir "$state/log" "$@"
          }

          if [ ! -f "$state/data/jellyfin.db" ]; then
            mkdir -p "$state/config" "$state/root/default/Shows" "$state/root/default/Movies" \
              data/library/series data/library/movies "$tests/tv" "$tests/movies"
            cat > "$state/config/system.xml" <<EOF
          <?xml version="1.0" encoding="utf-8"?>
          <ServerConfiguration xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
            <IsStartupWizardCompleted>true</IsStartupWizardCompleted>
          </ServerConfiguration>
          EOF
            cat > "$state/config/network.xml" <<EOF
          <?xml version="1.0" encoding="utf-8"?>
          <NetworkConfiguration xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
            <InternalHttpPort>${toString ports.jellyfin}</InternalHttpPort>
            <EnableHttps>false</EnableHttps>
            <LocalNetworkAddresses>
              <string>127.0.0.1</string>
            </LocalNetworkAddresses>
          </NetworkConfiguration>
          EOF
            printf "%s" "$(realpath data/library/series)" > "$state/root/default/Shows/library.mblink"
            printf "%s" "$(realpath "$tests/tv")" > "$state/root/default/Shows/tests.mblink"
            printf "%s" "$(realpath data/library/movies)" > "$state/root/default/Movies/library.mblink"
            printf "%s" "$(realpath "$tests/movies")" > "$state/root/default/Movies/tests.mblink"
            touch "$state/root/default/Shows/tvshows.collection" "$state/root/default/Movies/movies.collection"

            run --mode SeedSystem
            sqlite3 "$state/data/jellyfin.db" "INSERT INTO ApiKeys (DateCreated, DateLastActivity, Name, AccessToken)
              VALUES (datetime('now'), datetime('now'), 'yokoku', '${apiKey}');"
          fi

          run
        '';
      };

      # Once Jellyfin answers: the administrator `dev` without a password, no users left by the
      # integration tests, then a library scan.
      jellyfin-setup = pkgs.writeShellApplication {
        name = "jellyfin-setup";
        runtimeInputs = with pkgs; [
          curl
          jq
        ];
        text = ''
          api() {
            curl -fsS -H 'Authorization: MediaBrowser Token="${apiKey}"' -H 'Content-Type: application/json' \
              "$@"
          }
          url=http://127.0.0.1:${toString ports.jellyfin}
          for _ in $(seq 60); do api "$url/Users" > /dev/null 2>&1 && break; sleep 1; done

          if ! api "$url/Users" | jq -e 'any(.[]; .Name == "dev")' > /dev/null; then
            user=$(api -X POST "$url/Users/New" -d '{"Name": "dev"}')
            id=$(jq -r .Id <<< "$user")
            jq '.Policy | .IsAdministrator = true' <<< "$user" | api -X POST "$url/Users/$id/Policy" -d @-
          fi
          api "$url/Users" | jq -r '.[] | select(.Name | startswith("test-")) | .Id' | while read -r id; do
            api -X DELETE "$url/Users/$id"
          done
          api -X POST "$url/Library/Refresh"
        '';
      };

      transmission-dev = pkgs.writeShellApplication {
        name = "transmission-dev";
        runtimeInputs = with pkgs; [
          transmission_4
          coreutils
        ];
        text = ''
          mkdir -p ${dir}/transmission/downloads
          state=$(realpath ${dir}/transmission)
          exec transmission-daemon -f -g "$state" -w "$state/downloads" -p ${toString ports.transmission} \
            -T -M -O -Y -P 51419 --log-level=error
        '';
      };
    in
    {
      process-compose.services = {
        cli = {
          preHook = "mkdir -p ${dir}";
          options = {
            use-uds = true;
            unix-socket = "${dir}/process-compose.sock";
          };
        };
        settings.processes = {
          transmission = {
            command = "${transmission-dev}/bin/transmission-dev";
            availability.restart = "on_failure";
            readiness_probe.exec.command = "${pkgs.transmission_4}/bin/transmission-remote 127.0.0.1:${toString ports.transmission} -l";
          };
          jellyfin = {
            command = "${jellyfin-dev}/bin/jellyfin-dev";
            availability.restart = "on_failure";
            readiness_probe = {
              http_get = {
                host = "127.0.0.1";
                port = ports.jellyfin;
                path = "/health";
              };
              initial_delay_seconds = 5;
              period_seconds = 2;
              failure_threshold = 60;
            };
          };
          jellyfin-setup = {
            command = "${jellyfin-setup}/bin/jellyfin-setup";
            depends_on.jellyfin.condition = "process_healthy";
            availability.restart = "no";
          };
        };
      };

      # The integration tests' tools and the services' addresses, for the default shell's inputsFrom.
      devShells.services = pkgs.mkShellNoCC {
        packages = [ pkgs.transmission_4 ];
        shellHook = ''
          export YOKOKU_TEST_SERVICES_DIR="$PWD/${dir}"
          export YOKOKU_TEST_JELLYFIN_URL=http://127.0.0.1:${toString ports.jellyfin}
          export YOKOKU_TEST_JELLYFIN_API_KEY=${apiKey}
          export YOKOKU_TEST_TRANSMISSION_URL=http://127.0.0.1:${toString ports.transmission}/transmission/rpc
        '';
      };
    };
}
