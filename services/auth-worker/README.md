# BetterFy auth worker

This Cloudflare Worker is BetterFy's deployed identity, Telegram bot, session,
avatar, entitlement, and release-metadata boundary. Native desktop sign-in uses
a ten-minute device challenge with an explicit Telegram approve or deny action.
The Worker stores keyed challenge, device, code, access-token, and refresh-token
hashes rather than the corresponding bearer values.

It issues twelve-hour opaque sessions for the website. Desktop challenge
redemption or fallback-code exchange returns a fifteen-minute access session and
a single-use rotating refresh credential with a fixed thirty-day family
lifetime. Replay revokes the credential family and its access sessions; the
native app stores the refresh credential and stable public device ID in
Credential Manager or Keychain.

The primary desktop route is challenge-bound Telegram confirmation. Rust keeps
the stable device identifier, the bot requires an explicit approve or deny
action, and the approved challenge can be redeemed once. The six-digit code
remains available as a cross-device fallback.

BetterFy ID now has prepared independent username/email/password registration
with email verification. Telegram remains a quick separate sign-in. Existing
Telegram users can still link an email for code sign-in, but the two identities
are not silently merged. Password recovery through a verified email code is
available as a sign-in fallback; a dedicated password-change and account-linking
flow remains a release gate. The new ID endpoints have not been deployed or
validated through a real mail provider or Windows Credential Manager.

Challenge creation, Telegram lookup, approval, denial, and redemption use the
latest D1 primary state for the security-sensitive transition. This avoids
accepting replica lag as part of the authentication contract.

The Worker offers 3-day and 15-day Stars passes plus recurring 30-day access.
Wallet Pay is intentionally not exposed inside the bot for this digital access;
see `../../docs/PAYMENTS_ARCHITECTURE.md`.

## Secret rotation before public release

Any bot token ever pasted into a chat, screenshot, terminal recording, or issue
must be revoked in BotFather with `/revoke` before public release. The
replacement belongs only in Cloudflare secrets or a local ignored `.dev.vars`
file; it must never enter this repository or a client bundle.

## Local setup

1. Run `npm install` in this directory.
2. Copy `.dev.vars.example` to `.dev.vars` and fill it with disposable local
   values. `.dev.vars` is ignored by the repository.
3. Create a local D1 database with `npm run db:migrate:local`.
4. Run `npm test`, then `npm run dev`.

## Cloudflare deployment

1. Create the database: `npx wrangler d1 create betterfy-auth`.
2. Put its ID into `wrangler.jsonc`.
3. Add secrets individually:
   - `npx wrangler secret put TELEGRAM_BOT_TOKEN`
   - `npx wrangler secret put TELEGRAM_WEBHOOK_SECRET`
   - `npx wrangler secret put AUTH_CODE_PEPPER`
   - `npx wrangler secret put AUTH_PASSWORD_PEPPER` (a different, independent secret)
   - `npx wrangler secret put RESEND_API_KEY`
   - `npx wrangler secret put EMAIL_FROM` (a sender on a verified email domain)
   - optional: `npx wrangler secret put BETTERFY_DEVELOPER_LOGINS`, a
     comma-separated list of BetterFy ID logins that see the desktop stress
     tests. Only BetterFy ID logins match, never Telegram usernames. It is a
     secret rather than a `wrangler.jsonc` var so the list stays out of the
     repository and survives `npm run deploy`.
4. Review `BETTERFY_PLAN_3D_STARS`, `BETTERFY_PLAN_15D_STARS`, and
   `BETTERFY_PLAN_30D_STARS` in `wrangler.jsonc` before charging users.
5. Run `npm run db:migrate:remote` and `npm run deploy`.
6. Export `BETTERFY_AUTH_WORKER_URL` and `DEPLOY_ADMIN_SECRET`, then run
   `node scripts/set-webhook.mjs` once after every deploy that changes the bot.
   The Worker sets Telegram's secret webhook header, discards old pending
   updates, and refreshes the command menus and first-open descriptions in both
   languages.
7. Set desktop `VITE_BETTERFY_AUTH_URL` to the deployed HTTPS Worker origin.

The ID registration routes fail closed with HTTP 503 until mail values and the
password pepper are configured. Password sign-in fails closed without its pepper.
The email routes fail closed with HTTP 503 until both mail values are configured.
Set up the sender domain with the mail provider before enabling the UI for real
users. Migration `0007_betterfy_id_email.sql` must be applied before the email
routes are used. Do not put the mail key or sender credentials in the app or
website build. A rejected/unknown email address receives the same accepted
response as a known address to avoid account enumeration. Requests are limited
by IP and hashed email address; codes expire after ten minutes and can be used
only once. Delivery, linked-account continuity, and Windows credential-vault
behavior still require staged end-to-end testing before public release.

The bot ships five localized visual states from the Worker's static asset
binding: main menu, Premium access, sign-in confirmation, approved sign-in and
one-time code. Each state has an explicit Russian and English 1280×720 JPEG in
`../../website/public/bot`; the mapping in `src/index.mjs` is the canonical
contract. New filenames are required when a card is replaced so Telegram does
not reuse a previously cached image URL.

## Client routes

- `POST /v1/auth/device/challenges` creates a ten-minute device-bound challenge
  for the native rotating-credential client and returns an allowlisted bot link.
- `POST /v1/auth/device/challenges/poll` returns pending/denied/expired states or
  redeems one approved challenge for desktop credentials exactly once.
- `POST /v1/auth/telegram/code` consumes a six-digit code and returns an opaque
  web session or explicitly negotiated rotating desktop credentials once.
- `POST /v1/auth/email/start` requests a six-digit code for an already linked
  email. `POST /v1/auth/email/verify` consumes it and issues credentials.
- `POST /v1/auth/id/register/start` accepts username, email and password,
  stores only a salted password verifier in a pending registration, and mails a
  short-lived code. `/verify` creates the account after code confirmation.
- `POST /v1/auth/id/login` accepts username or email and password. It returns
  the same web or rotating desktop session contract as Telegram entry.
- `GET /v1/session/email` returns link status and a masked address; authenticated
  `POST /v1/session/email/start` and `/verify` verify a new email link.
- `GET /v1/session/profile` returns the Telegram-backed BetterFy profile,
  current access period, and avatar availability. Missing avatars are retried
  without blocking sign-in.
- `GET /v1/session/avatar` proxies the current Telegram avatar without exposing
  the bot token or Telegram file URL to a client.
- `POST /v1/session/logout` revokes the presented session.
- `GET /v1/session/devices` lists active sessions using only neutral client
  labels; it never stores or returns IP addresses, user agents, or device names.
- `POST /v1/session/devices/revoke` revokes one session owned by the signed-in
  BetterFy profile.
- `GET /v1/releases/latest` resolves the latest allowlisted Windows asset for an
  authenticated client. GitHub releases remain public; this route gates the
  BetterFy website flow, not direct GitHub access. A stable release is preferred;
  until signing is configured, the route falls back to the newest immutable
  Early Access prerelease. `BetterFy-Setup.exe` is preferred over the older NSIS
  name `BetterFy-Windows-x64-setup.exe` within a release.
