# Security policy

BetterFy is an Early Access project maintained by one person. It includes a
privileged desktop engine that writes to Steam and Dota 2 files, a standalone
installer, a Cloudflare Worker for identity and entitlements, and a static
website. A vulnerability in any of these can affect a player's machine or
account, so please report it privately.

## Reporting a vulnerability

Do not open a public issue, pull request, or discussion for a suspected
vulnerability.

GitHub private vulnerability reporting is not enabled for this repository.
Contact the maintainer, [@zori-xyz](https://github.com/zori-xyz), through
GitHub and ask for a private channel before sharing details.

A useful report includes:

- the affected area (desktop engine, BetterFy Setup, auth Worker, website, or
  release/update path);
- the exact release tag or commit;
- the steps that reproduce the problem and what an attacker could achieve;
- whether you believe it is already being exploited.

## What not to send

Never include, in any channel:

- bot tokens, webhook secrets, signing keys, or updater keys;
- access or refresh credentials, challenge links, six-digit sign-in codes, or
  copied Credential Manager values;
- Steam IDs, account names, Telegram identifiers, or filesystem paths that
  contain personal names.

If a secret has already been exposed, say so without repeating its value.

## Scope and versions

Reports are assessed against `main` and the most recent Early Access
pre-release on the [Releases](https://github.com/zori-xyz/BetterFy/releases)
page. Builds from older tags and unsigned CI artifacts are internal test
builds.

BetterFy makes no claim of VAC safety, ban immunity, or universal
compatibility. Reports about those topics are welcome as compatibility
information, but they are not handled as security vulnerabilities.
