# BetterFy identity and web architecture

This document fixes the next product boundary after trusted content intake. The
goal is one BetterFy profile across desktop and web, confirmed through
`@BeterFyBot`, without placing bot secrets, long-lived tokens, or privileged
engine operations in React or the public website.

## Product sequence

1. Define the account, device, entitlement, and public-profile contracts.
2. Build the authentication service and database migrations behind a local test
   environment.
3. Connect `@BeterFyBot` to that service and implement single-use device codes.
4. Store the desktop session in the operating-system credential vault and expose
   only a neutral signed-in profile to React.
5. Add profile editing and sync with explicit offline/conflict states.
6. Publish the public website for product information, downloads, documentation,
   release checksums, privacy terms, and the same account boundary.

The site and bot do not unlock Dota writes. Engine deployment keeps its own
confirmation, backup, journal, and recovery gates.

## Ownership boundaries

| Component | Owns | Must never own |
| --- | --- | --- |
| Desktop React | Display name, avatar URL, sync state, device-code phase | Bot token, refresh token, database key, arbitrary auth URL |
| Desktop Rust | HTTPS API origin allowlist, credential-vault access, device identity, token refresh | Telegram bot secret, user password, raw Steam profile data in IPC |
| Auth API | BetterFy user ID, device sessions, code challenges, entitlements, audit events | Dota paths, Steam files, build archives |
| Telegram bot worker | Bot update handling and a narrow call to issue/approve challenges | Database administration credentials in client code |
| Public website | Landing, documentation, downloads, release evidence, profile UI through the API | Desktop engine commands, local filesystem access, updater signing key |

Production secrets live in the deployment environment. They are never committed,
bundled in the desktop application, written to logs, or included in diagnostics.

## Device-code flow

1. The desktop asks the API for a random device challenge and receives a short
   challenge ID, expiry, polling interval, and bot deep link.
2. The user opens `@BeterFyBot`. The bot shows the device and application context
   and requires an explicit confirmation.
3. The server stores only a keyed hash of any human-entered code. Codes are
   single-use, expire after ten minutes, have strict attempt and issue limits, and
   are bound to the challenge and device public identifier.
4. The desktop polls at the server-provided interval. `pending`, `approved`,
   `denied`, `expired`, and `rate_limited` remain distinct states.
5. Approval can be redeemed exactly once for a short-lived access token and a
   rotating refresh token. Replay invalidates the token family and records a
   privacy-minimal security event.
6. Rust stores the refresh token in Windows Credential Manager (and Keychain for
   macOS development). React receives neither token.

The six-digit manual code is a fallback for crossing devices. The deep link is
the primary path. Telegram identity is never treated as proof of a Steam account.

## Profile contract

The first profile version contains:

- opaque BetterFy user ID;
- display name and optional approved avatar;
- interface language and theme preference;
- created/updated version for optimistic concurrency;
- device list with user-revocable sessions;
- subscription/entitlement facts returned by the server;
- saved preset references by immutable content identity, never arbitrary paths.

Profile writes use an idempotency key and expected version. A stale write returns
`profile_conflict`; the client never silently overwrites a newer server record.
Offline edits remain local until the user chooses which version wins.

## Public website boundary

The first site ships as a separate deployable application with:

- landing and product explanation in Russian and English;
- verified Windows download and release checksum/signature links;
- changelog, status, documentation, privacy, and terms;
- Telegram sign-in and a minimal profile/device-session page;
- no catalog install button until signed catalog delivery exists.

The desktop API origin and website origin are separate allowlisted clients.
The current static GitHub Pages slice keeps its opaque twelve-hour bearer token
in `sessionStorage`, so closing the tab clears it; the desktop keeps the token in
memory only. The production target remains secure HTTP-only same-site cookies
for browsers and an OS credential vault with rotation for desktop. CORS is
deny-by-default. Security
headers, rate limits, structured redacted logs, database backups, and token-key
rotation are release requirements rather than later cleanup.

## Acceptance evidence

- unit tests for code expiry, single use, denial, replay, throttling, and token rotation;
- integration tests with a fake Telegram update and disposable database;
- desktop tests proving secrets never cross IPC or diagnostics;
- profile conflict/offline/revocation tests;
- website accessibility, RU/EN, theme, CSP, and download-integrity checks;
- a documented account deletion and retention path before public launch.

## Implemented first slice

`services/auth-worker` now provides the narrow manual-code slice used by the
current desktop UI:

- Telegram webhook requests require Telegram's secret header before their body
  is parsed;
- `/start`, `/code`, `/help`, `/privacy`, language switching, and inline code
  issue actions are available in Russian and English;
- six-digit codes expire after ten minutes, are limited per Telegram profile,
  are stored only as HMAC-SHA-256 values, and are consumed atomically once;
- verification attempts are rate-limited using a keyed hash of the requester
  address, and CORS allows only explicit desktop/site origins;
- founder-approved Telegram cards are served from the public site, while every
  bot and signing secret remains in the Worker deployment environment.
- 3-day and 15-day one-time Stars passes plus recurring 30-day access share the
  same pre-checkout validation, idempotent payment event, refund, cancellation,
  and Premium-entitlement boundary;
- a twelve-hour opaque session joins the website and current desktop flow;
  profile photos are referenced by Telegram `file_id` and proxied server-side,
  so the bot token never appears in client code or image URLs;
- the website download control requires a valid BetterFy session. The Windows
  artifact itself remains public on GitHub Releases, so this is an account UX
  gate rather than access control over the public repository.

This is deliberately not presented as complete account infrastructure.
Challenge-bound deep links, refresh-token rotation, device management, secure
browser cookies, and OS credential-vault storage remain required before the
identity layer is declared production-complete.
