# Changelog

BetterFy is in Early Access. Every build is an unsigned prerelease
(`v<version>-ea.<run>`) published by the `Early Access Windows installer`
workflow; the run number is the EA number. "Verified" below means a person ran
it on Windows and the evidence is recorded; everything else is "built and tested
in CI" only. The honest status table lives in [README.md](README.md).

Versions before 0.5.0 were all `0.1.x`; the version was raised to 0.5.0 in
October 2026 to reflect what the product has become (engine, builder, catalog
of audited skins, bot, site and installer), not a change in stability.

## 0.5.0 (Early Access)

Version bump together with the work below.

### Skins (wardrobe)
- Skin archives are kept whole: Panorama image wrappers are proven from their
  parsed structure in both compiler containers (209 more installed, 2 layouts
  carrying a script payload stay rejected), zero-length blank overrides install,
  and the per-file and install size bounds follow the VPK limit (500 of 512
  archives are now in the allowlist). The build review counts files that are
  not game files apart from files that are really dropped. Built and tested in
  CI only; not installed on Windows or run in Dota yet. Details:
  [docs/SKIN_ARCHIVE_AUDIT.md](docs/SKIN_ARCHIVE_AUDIT.md).

## 0.1.4 EA.18 to EA.38 (30 September to 17 October 2026)

### Engine and mods
- Generic package engine: strict package manifests, a verified content store,
  deterministic single-VPK builds, ownership-scoped backup, journal, recovery
  and rollback (EA.18 is the founder-verified baseline,
  [docs/EA18_WINDOWS_BASELINE.md](docs/EA18_WINDOWS_BASELINE.md)).
- KV3 and Panorama layout editing, blacklist patterns and Panorama styles; 24
  Minify mods installable together, with only audited scripts and layouts
  (hashes embedded in the binary). Windows result recorded in
  [docs/EA24_WINDOWS_TEST_PLAN.md](docs/EA24_WINDOWS_TEST_PLAN.md): a 24-mod
  build worked in game and restored cleanly.
- Language slots: any real Dota language folder (27 languages plus an
  experimental English folder) with the matching `-language` option; the
  language can follow what Dota uses now.
- Steam restore now removes only BetterFy's `-language` option when Steam has
  rewritten `localconfig.vdf`, and keeps the player's own changes.
- Catalog sequence floor: a signed catalog at the build floor is accepted over
  an older cached one.
- Dev mode: stress tests are offered only to accounts the auth service marks
  as developers.
- Stale installs: a way out when the game or another tool replaced BetterFy's
  file ("Let go of the old install"); nothing in the game folder is touched.

### Skins (wardrobe)
- Audited hero skins: a VPK analyzer, pinned archive hashes embedded in the app,
  two download mirrors, a build review that lists what installs and what is
  ignored. 499 of 512 hero and hero-item archives in the Dota2PornFx catalog
  are in the allowlist; whole archives install by default because that is what
  works when the VPKs are copied by hand. None has been installed by BetterFy
  on Windows or run in Dota yet. Details:
  [docs/SKIN_ARCHIVE_AUDIT.md](docs/SKIN_ARCHIVE_AUDIT.md),
  [docs/WARDROBE_AUDIT_REPORT.md](docs/WARDROBE_AUDIT_REPORT.md),
  [docs/SKIN_WINDOWS_TEST_PLAN.md](docs/SKIN_WINDOWS_TEST_PLAN.md).
- Staging no longer refuses builds above 64 MiB; the cap is 256 MiB like the
  merged VPK.

### App
- New build screen for ordinary people: journal, plain-language errors with one
  next step, custom choice controls, one-click fixes next to dependency and
  conflict messages, a demo scenario switcher in the browser preview.
- Home redesign with a coverflow showcase, Days One headings, keyboard
  shortcuts (`G` then a letter to jump between screens, `?` for the cheat
  sheet), accessibility fixes in both themes.

### Installer (BetterFy Setup)
- Redesigned with a brand panel, step-by-step progress, a finish screen with a
  launch button and clearer error states. The progress bar is an estimate:
  the install reports only when it finishes.

### Website
- Honest Early Access status, a gallery of real app screens, a safety section,
  FAQ, privacy and terms pages (not lawyer-reviewed), SEO and social tags,
  404 page, light theme.

### Telegram bot and auth service
- One menu that edits in place, commands `/menu`, `/status`, `/devices`,
  `/language`, `/support`, sign out everywhere with a Not me button, Premium
  ending reminders, first-open descriptions in two languages. Support is the
  separate support account.
- BetterFy ID (password hashing that works inside a Worker, branded email).

## Earlier

- Earlier 0.1.x builds: Windows-first shell, Dota discovery, Steam profile
  activation, Telegram identity, installer, signed updater infrastructure.
  See [docs/ROADMAP.md](docs/ROADMAP.md).
