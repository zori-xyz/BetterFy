# EA.45 catalog categories on Windows: observation

The founder installed the non-hero categories added in `v0.5.0-ea.45` and checked
them in Dota on Windows. This records what he reported and what the photographs
show, and keeps what was not checked apart.

## Artifact

- Release `BetterFy v0.5.0 · Early Access 45`, tag `v0.5.0-ea.45`
- `BetterFy-Setup.exe` SHA-256 `50ae7202a8c673be173f530d9890cc1cb7a254ae515f4082670da46c60d6d25b`
  (the published sidecar matched the downloaded bytes)
- Machine and day: the founder's Windows machine, 2026-10-19. Two evidence
  reports were saved, at 07:29:43 UTC (installed) and 07:45:57 UTC (after the
  restore).

## Reported and seen

The founder's verdict: everything works; sounds change, the map changes,
everything changes, checked in game. Eleven photographs of the screen, not part
of the repository, show:

- a match with an underwater terrain on the map, with reef and coral models,
  health bars and creeps visible;
- a river and trees restyled in purple and green tones;
- creep and courier models replaced, in the Arsenal and in the match;
- purple item icons in the guide and shop;
- replaced rank-medal icons on the profile page.

## Evidence reports

Operation 13 in both reports: language `russian`, `vpkName` `pak65_dir.vpk`, bundle
plan `sha256:82090316fe948eb181354f61b6306c1247322a716cc5d71b2445dad3c08c858e`,
pak installed SHA-256 `fc691510c174387f31df2a6c0154c8ef4e2777ea976a0dbb9f36b7d3a9a3d369`.
It carried 15 packages: mega-kill Diretide, courier Billy Bounceback, creeps
Nemestice (radiant and dire), shaders Aghanim Purple, emblem of the Diretide
(blue), Oil River, Lowpoly Trees, item effects TI6 Bottle, Purple item icons,
Normis rank icons, hero sounds Sven Already Red, creep deny Razor Deny, high
five Crownfall, and the TI7 Reef's Edge map (`maps/dota.vpk`, installed SHA-256
`03706dc6a079fc56b1b89a970ee9c10b40f7c19197a2eb7ce00ad0e720de67af`).

- First report: phase `committed`, active, backup verified for the pak and the
  map, no error code.
- Second report: the same operation is `rolled_back` with `rollbackVerified`
  true for the pak and for the map, and not active. The earlier three-skin
  build (operation 12) shows `rolled_back` with `rollbackVerified` true as well.
- So the first install of a pak plus a map through BetterFy, and its restore,
  are on record, with both files restored.

## Not recorded

- The sounds were reported by ear only; no recording or log was kept.
- Re-installing after the restore and comparing hashes (the same plan should give
  the same pak hash `fc691510...`), the heaviest packs (the 92 MB music pack, the
  207 MiB hero skin), memory and time.
- Other items of each category; the categories that are not enabled remain
  untested by design.
- The Dota build number, and whether Minify's own file was beside BetterFy's.
- An old tuning-build record (operation 11) and an old Steam record (sequence 9)
  still read `committed` in the report; they are records of earlier sessions.
