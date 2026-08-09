# BetterFy identity and web architecture

This document fixes the next product boundary after trusted content intake. The
goal is one BetterFy profile across desktop and web, confirmed through
`@BeterFyBot`, without placing bot secrets, long-lived tokens, or privileged
engine operations in React or the public website.

## Delivery sequence

1. Define the account, device, entitlement, and public-profile contracts.
2. Deploy the Worker, D1 migrations, Telegram webhook, six-digit fallback, and
   revocable browser sessions.
3. Add challenge-bound native Telegram approve/deny, one-time redemption, and
   rotating desktop credentials stored behind Rust.
4. Publish the public website with code sign-in, profile/avatar display,
   privacy-safe session controls, and an authenticated download action.
5. Add secure browser cookies, account deletion/retention, profile editing, and
   explicit sync conflict states.
6. Complete human-driven Windows evidence for vault persistence, Telegram
   approve/deny, restart, expiry, revocation, and refresh replay.

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

1. Rust asks the API for a random device challenge and receives an opaque
   challenge token, expiry, polling interval, and allowlisted bot deep link.
2. The user opens `@BeterFyBot`. The bot shows the device and application context
   and requires an explicit confirmation.
3. The server stores only keyed hashes of the challenge token and persistent
   device public identifier. Challenges are single-use, expire after ten minutes,
   and have strict creation limits. The device identifier stays in Rust and the
   operating-system vault; React receives only the deep link and public timing.
4. The desktop polls at the server-provided interval. React receives distinct
   `pending`, `confirmed`, `denied`, and `expired` states; challenge creation
   reports rate limiting separately.
5. Approval can be redeemed exactly once for a short-lived access token and a
   rotating refresh token. Replaying the challenge cannot mint a second family.
   Replaying a used refresh credential invalidates its family and records a
   privacy-minimal security event.
6. Rust stores the refresh token in Windows Credential Manager (and Keychain for
   macOS development). React receives neither token.

The six-digit manual code is a fallback for crossing devices. The deep link is
the primary path. Telegram identity is never treated as proof of a Steam account.

## Profile contract

The implemented public auth profile contains:

- opaque BetterFy user ID;
- Telegram display name and optional username;
- an avatar-availability flag plus authenticated image proxy;
- `early-access` or `premium` access state;
- active plan, expiry, and recurring state when applicable;
- opaque current-session ID and privacy-safe revocable session summaries.

Theme, profile editing, saved-preset synchronization, optimistic versioning,
offline conflict resolution, and arbitrary remote profile fields are not part
of the current server contract. When added, writes must use an idempotency key
and expected version; the client must never silently overwrite a newer record.

## Public website boundary

The implemented static site is a separate deployable application with:

- landing and product explanation in Russian and English;
- a GitHub release resolver constrained to this repository;
- six-digit Telegram fallback sign-in;
- a minimal profile/avatar and device-session page;
- an account-gated download control with an explicit public-GitHub boundary;
- no desktop engine commands or catalog installation capability.

Release checksums/signatures, full changelog/status documentation, privacy and
terms pages, and secure cookie sessions remain required before public launch.

The desktop API origin and website origin are separate allowlisted clients.
The current static GitHub Pages slice keeps its opaque twelve-hour bearer token
in `sessionStorage`, so closing the tab clears it; the desktop keeps the token in
memory only. The production target remains secure HTTP-only same-site cookies
for browsers and an OS credential vault with rotation for desktop. CORS is
deny-by-default. Security
headers, rate limits, structured redacted logs, database backups, and token-key
rotation are release requirements rather than later cleanup.

## Evidence and remaining gates

Automated evidence currently covers code normalization and freshness, challenge
parsing and terminal states, malformed route denial, webhook secret checks,
origin allowlisting, refresh expiry/replay/revocation, avatar byte validation,
privacy-safe client labels, Rust credential DTO boundaries, frontend builds,
and native Windows compilation. A production-safe smoke test verifies challenge
creation and immediate pending polling without printing credentials.

Still required: disposable-D1 end-to-end webhook tests, a human-driven Windows
approve/deny/expiry/vault/restart pass, browser accessibility and CSP evidence,
profile conflict tests once editable profiles exist, payment sandbox evidence,
and a documented account deletion and retention path.

## Implemented identity boundary

`services/auth-worker`, the public website, and the native Rust bridge now
provide the following Early Access boundary:

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
- a twelve-hour opaque session authenticates the current website flow;
  profile photos are referenced by Telegram `file_id` and proxied server-side,
  so the bot token never appears in client code or image URLs;
- active twelve-hour sessions now carry opaque public IDs and privacy-safe
  `web`, `desktop`, or `unknown` labels. The website and desktop profile can
  list and revoke sessions owned by the current BetterFy user without storing
  IP addresses, user agents, or operating-system device names;
- the website download control requires a valid BetterFy session. The Windows
  artifact itself remains public on GitHub Releases, so this is an account UX
  gate rather than access control over the public repository.

This is deliberately not presented as complete account infrastructure.
Desktop challenge redemption and manual-code fallback issue a fifteen-minute
access session and a single-use refresh credential with a fixed thirty-day
family lifetime. Rust
stores the refresh credential in Windows Credential Manager or macOS Keychain,
keeps the access token in process memory, proxies authenticated profile calls,
and returns neither token to React. Every successful refresh revokes the prior
access session. Reuse, expiry, or revocation of a refresh credential invalidates
the full family and all related access sessions.

Challenge-bound deep links are now implemented as the primary native sign-in:
the bot requires an explicit approve/deny callback, polling distinguishes
pending, denied, expired, and redeemed states, and redemption can issue only one
rotating desktop credential family. Security-sensitive challenge reads begin on
the latest D1 primary state rather than depending on replica timing. The
six-digit code remains the website and cross-device fallback. Secure browser
cookies, account deletion/retention, and end-to-end Windows vault and Telegram
interaction testing remain required before the identity layer is declared
production-complete.
