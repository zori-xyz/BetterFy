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

The static site contains no bot secret and never validates Telegram identity by
itself. It opens `@BeterFyBot`, exchanges the user's six-digit fallback code with
the deployed auth Worker, and receives a revocable twelve-hour opaque web
session. The session is kept in `sessionStorage`, so closing the tab clears the
local copy. The site can show the Telegram-backed profile and proxied avatar,
list privacy-safe web/desktop sessions, revoke an owned session, and require a
valid session before enabling its download action.

`VITE_BETTERFY_AUTH_URL` must be an allowlisted HTTPS Worker origin. The public
GitHub repository and its release artifacts remain directly accessible, so the
website download control is an account UX gate rather than access control over
GitHub. HTTP-only browser cookies, CSRF protection, account deletion, and a
retention policy remain release work as defined in
`../docs/IDENTITY_AND_WEB_ARCHITECTURE.md`.
