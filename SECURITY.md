# Security Policy

## Reporting a Vulnerability

Please report security issues privately through
[GitHub Security Advisories](https://github.com/MrEhbr/yokoku/security/advisories/new).
**Do not open a public issue.**

Include:

- what the issue is and its impact;
- steps to reproduce, ideally a minimal proof of concept;
- the affected version and how Yokoku is installed (Docker, archive or NixOS).

You'll get an answer within a week. Reporters are credited in the advisory unless they'd
rather not be.

## Scope

In scope: the `yokoku` binary and web UI, the Docker image and the release archives.

Out of scope:

- vulnerabilities in dependencies; report those upstream;
- exposing the web UI to the internet: Yokoku has no login, so keep it on a private network
  or behind an authenticating reverse proxy;
- findings that need existing access to the host running Yokoku.

## Supported Versions

Only the latest release gets security fixes.
