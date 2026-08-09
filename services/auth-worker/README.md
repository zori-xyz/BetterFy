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
4. Review `BETTERFY_PLAN_3D_STARS`, `BETTERFY_PLAN_15D_STARS`, and
   `BETTERFY_PLAN_30D_STARS` in `wrangler.jsonc` before charging users.
5. Run `npm run db:migrate:remote` and `npm run deploy`.
6. Export the three values required by `scripts/set-webhook.mjs`, then run that
   script once. It configures Telegram's secret webhook header and discards old
   pending updates.
7. Set desktop `VITE_BETTERFY_AUTH_URL` to the deployed HTTPS Worker origin.

The public bot cards are served from `/bot` on the BetterFy GitHub Pages site.

## Client routes

- `POST /v1/auth/device/challenges` creates a ten-minute device-bound challenge
  for the native rotating-credential client and returns an allowlisted bot link.
- `POST /v1/auth/device/challenges/poll` returns pending/denied/expired states or
  redeems one approved challenge for desktop credentials exactly once.
- `POST /v1/auth/telegram/code` consumes a six-digit code and returns an opaque
  web session or explicitly negotiated rotating desktop credentials once.
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
  BetterFy website flow, not direct GitHub access.
