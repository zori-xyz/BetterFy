# BetterFy auth worker

This Cloudflare Worker is the first deployable authentication boundary for BetterFy.
It receives Telegram webhooks, issues ten-minute one-time codes, stores only
keyed hashes, and exposes the code verification endpoint already used by the
desktop client.

It issues twelve-hour opaque sessions for the website. Desktop exchanges a code
for a fifteen-minute access session and a single-use rotating refresh credential
with a fixed thirty-day family lifetime. Only keyed token hashes are stored in
D1. Replay revokes the credential family and its access sessions; the native app
stores the refresh credential in Credential Manager or Keychain.

The Worker offers 3-day and 15-day Stars passes plus recurring 30-day access.
Wallet Pay is intentionally not exposed inside the bot for this digital access;
see `../../docs/PAYMENTS_ARCHITECTURE.md`.

## Before setup

The bot token previously pasted into a chat must be revoked in BotFather with
`/revoke`. Never reuse it. Create a fresh token and place it only in Cloudflare
secrets or a local ignored `.dev.vars` file.

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

- `POST /v1/auth/telegram/code` consumes a six-digit code and returns an opaque
  twelve-hour session token once.
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
