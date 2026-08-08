# BetterFy website

The public website is an independently built static surface inside the BetterFy
repository. It shares the canonical wordmark and fonts with the desktop app but
does not import desktop routes, Tauri commands, game assets, or privileged code.

## Local work

```bash
npm run site:dev
npm run site:build
npm run site:check
```

`site:check` expects the local dev server at
`http://127.0.0.1:4174/BetterFy/` and checks RU/EN at desktop and mobile widths.

## Deployment

`.github/workflows/pages.yml` builds `dist-site/` and publishes it through the
official GitHub Pages artifact flow. The expected public URL is:

`https://zori-xyz.github.io/BetterFy/`

The release card reads the public GitHub Releases API and accepts download links
only from this repository's release path. If a Windows installer is unavailable,
the action falls back to the official Releases page.

## Telegram account boundary

The static site never validates Telegram identity and contains no bot secret.
Until the auth service exists, the account action opens `@BetterFyBot` and
explicitly remains in Early Access state.

Once the auth service is deployed, set `VITE_BETTERFY_AUTH_URL` to its HTTPS web
login entry point during the site build. Telegram verification, browser session
cookies, CSRF protection, token rotation, and account storage remain server-side
as defined in `docs/IDENTITY_AND_WEB_ARCHITECTURE.md`.
