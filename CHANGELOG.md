# Changelog

BetterFy is in Early Access. Every build is an unsigned prerelease
(`v<version>-ea.<run>`) published by the `Early Access Windows installer`
workflow; the run number is the EA number. "Verified" below means a person ran
it on Windows and the evidence is recorded; everything else is "built and tested
in CI" only. The honest status table lives in [README.md](README.md).

Versions before 0.5.0 were all `0.1.x`; the version was raised to 0.5.0 in
October 2026 to reflect what the product has become (engine, builder, catalog
of audited skins, bot, site and installer), not a change in stability.

## 0.6.3 (Early Access)

Built and tested on a Mac and in CI, not run on Windows.

- New home screen: an emote hero, the whole wardrobe shuffled in the carousel,
  Telegram bot scenes and a reminder for an unfinished build; the profile is a
  player card; every screen polished (pull request #47).
- Tester program: an in-app journal of clicks, failures, notes and window
  snapshots, sent to the founder's Telegram chat (pull request #46).
- The website is rebuilt on the app's own styles and components, with
  BetterFy ID registration; the installer downloads straight from GitHub
  Releases without an account.
- Dependencies: `rustls` 0.23.45, `source-map-js` 1.2.2 and, in the auth
  worker's local tooling, `sharp` 0.35.5. `glib` 0.18 stays: it is pulled in
  only by Tauri's Linux GTK stack and is not built for Windows.
- The studio bundle is split so the catalog data and third-party libraries
  load as their own chunks.

## 0.6.0 (Early Access)

Version bump for the complete catalog below.

### Complete catalog (pull request #45; built and tested on a Mac and in CI, not run on Windows)

- Screens offer only what installs: the Windows-pilot and preview-only badges,
  checksums, file paths, audit counts and "not checked yet" notes are gone; the
  build review keeps size, what an item changes, the category tip, the
  "Change only this hero" switch and how to restore. Test plan:
  [docs/CATALOG_COMPLETE_WINDOWS_TEST_PLAN.md](docs/CATALOG_COMPLETE_WINDOWS_TEST_PLAN.md).
- Wardrobe: 1,028 of 1,044 audited archives install (was 1,011). Brackets and
  apostrophes in archive names, a non-ASCII name dropping only that file, the
  Workshop addon template treated as a leftover, a stale offset on an empty VPK
  entry, the merged build limit raised to 512 MiB, and the project's second
  Hugging Face dataset trusted for its root files.
- The game's own loose files: a journaled `game_files` transaction for
  `resource/cursor`, `panorama/fonts` and `itembuilds`, linked to the build's
  deployment, with every touched file backed up and the game's originals put
  back on restore. Installs the 34 cursor packs and 10 font packs of the catalog
  (font packs as their own guide says: the game's fonts out, the pack's in; all
  carry Cyrillic) and Minify's OpenDotaGuides item builds (one pinned release).
- All 30 game tuning mods install. New: User Styles (17 snippets, each with a
  switch), Custom Fonts (the player's .ttf/.otf, renamed for each of the 29
  Radiance/RadianceM/Reaver files with a proper font rebuild), Custom
  Backgrounds (one of 28 event loading screens or the player's WebM), Custom
  Hero Grids (three pinned grids added beside the player's own in every Steam
  account, restored without touching the player's edits) and OpenDotaGuides.
- The catalog's six packs are ready-made sets: adding one adds its items.
- Package catalog re-signed as sequence 8 and deployed on 2026-10-19 together
  with EA.47. Older apps reject it (User Styles reads Minify's `_styling.css`)
  and silently keep the catalog they have.

Windows observation of the catalog categories (EA.45): the founder reported sounds, the map, river, trees, creeps, couriers, item icons and rank icons all working in game; see [docs/EA45_CATALOG_WINDOWS_OBSERVATION.md](docs/EA45_CATALOG_WINDOWS_OBSERVATION.md) for what was and was not recorded.

First Windows observation of skins: three catalog skins installed together by EA.41 and seen applied on the Dota hero pages, with BetterFy using `pak65_dir.vpk` and the "Let go of the old install" action used once; see [docs/EA41_SKIN_WINDOWS_OBSERVATION.md](docs/EA41_SKIN_WINDOWS_OBSERVATION.md). Not yet checked in a match, with restore, or for the other skins.

## 0.5.0 (Early Access)

Version bump together with the work below.

### Skins (wardrobe)
- Menu backgrounds, HUD skins and versus screens are installable through the
  audited-archive pipeline: 62 backgrounds, 3 of 4 HUD skins and 7 versus screens
  (1,011 archives in all, 31 categories). A layout, style or video installs only
  when the Panorama proofs of `panorama_decor` establish it, the build review
  says what was proven, what was dropped with the exact reason, and how to undo,
  and it states that the container of a video is checked but no frame is
  decoded. One HUD skin (`huds-black-hud`) is blocked by two unsafe file names.
  Built and tested in CI only; not installed on Windows or run in Dota yet (test
  section 6 of
  [docs/SKIN_WINDOWS_TEST_PLAN.md](docs/SKIN_WINDOWS_TEST_PLAN.md)).
- Skin archives are kept whole: Panorama image wrappers are proven from their
  parsed structure in both compiler containers (209 more installed, 2 layouts
  carrying a script payload stay rejected), zero-length blank overrides install,
  and the per-file and install size bounds follow the VPK limit (500 of 512
  archives are now in the allowlist). The build review counts files that are
  not game files apart from files that are really dropped. Built and tested in
  CI only; not installed on Windows or run in Dota yet. Details:
  [docs/SKIN_ARCHIVE_AUDIT.md](docs/SKIN_ARCHIVE_AUDIT.md).

### Map terrains
- The 12 standard terrains of the catalog (ZIPs holding a whole-map
  `maps/dota.vpk`, 34 to 53 MB) are installable. The map is a second file owned by
  the same reviewed, journaled, recoverable transaction as the `pakNN_dir.vpk`: it
  goes to `game/dota_<language>/maps/dota.vpk`, can be installed alone or beside a
  hero skin, is backed up and restored exactly, is never written over a file
  BetterFy did not write (`map_target_foreign`, checked before Steam and Dota are
  closed), and creates and removes the `maps` folder only when it made it and it
  is empty. Records of a build without a map are unchanged on disk. One map per
  build. A pinned allowlist (`src-tauri/maps/`) and a whole-map analyzer back it;
  the build review says what a map does and the upstream caveats (shows only in
  matches, not in Demo; turn off the Dota+ terrain; a whole-file replacement may
  go stale after a Dota patch, an unverified risk). Built and tested in CI and on a
  Mac against the real archives; not installed on Windows or run in Dota yet.
  Details: [docs/ENGINE_ARCHITECTURE.md](docs/ENGINE_ARCHITECTURE.md),
  [docs/MAP_AUDIT_REPORT.md](docs/MAP_AUDIT_REPORT.md),
  [docs/MAP_WINDOWS_TEST_PLAN.md](docs/MAP_WINDOWS_TEST_PLAN.md).

### Catalog categories (wardrobe)
- Every plain resource category of the catalog is now audited and installable through the
  same verified pipeline as hero skins: 439 more archives in 26 categories (couriers, creeps,
  item and TI effects, mega-kill, music and other sounds, shaders, river, trees, wards, ranks
  and item icons, emblems, and the rest), 939 in all. Content with no hero installs whole, with no
  opt-in mode; sounds, panorama textures and materials have their own type policies; raw
  `pakNN_dir.vpk` items are pinned by their own hash. Cards show the Windows-pilot mark and
  "not run in Dota yet"; the build review shows files, size and class counts per item with the
  category's help (and the upstream tips for emblems and shaders). Panorama layouts, loose
  cursors and fonts, tools and the two categories whose files do not exist upstream stay
  not enabled, with the reason in [docs/CATALOG_CATEGORIES.md](docs/CATALOG_CATEGORIES.md).
  Built and tested on a Mac only: not installed on Windows and not run in Dota.
### Panorama decor (proofs only)
- New proofs for main-menu backgrounds, HUD skins and versus screens: a static
  layout proof, a style sheet proof and a WebM container check, built to refuse
  anything they do not recognise with the exact reason. Run over the 73 real
  catalog archives of these categories they accept every layout, style and
  video; event handlers pass only as 79 exact bindings read in those archives
  (to be reviewed by the founder before use). **Not wired in: no category is
  installable yet, and nothing was installed on Windows or run in Dota.** Also
  fixes a panic in the KV3 reader on a corrupt count. Details:
  [docs/PANORAMA_DECOR_AUDIT.md](docs/PANORAMA_DECOR_AUDIT.md).

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
