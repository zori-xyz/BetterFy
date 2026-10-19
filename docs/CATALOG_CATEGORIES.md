# Catalog categories

Status: **built and tested on a Mac and in CI only. Nothing in this document
has been run in Dota through BetterFy except the three hero skins of EA.41**
([EA41_SKIN_WINDOWS_OBSERVATION.md](EA41_SKIN_WINDOWS_OBSERVATION.md)). Every
other category below is installable through the verified pipeline, marked
"Windows pilot" in the catalog, and carries the honest note "not run in Dota"
until a person records otherwise (round three of
[SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md)).

The catalog is the Dota2PornFx snapshot of 2026-07-26 (`src/webCatalog.json`,
1,135 items). 939 archives of 28 categories are in the embedded allowlist; the
item-by-item audit is [WARDROBE_AUDIT_REPORT.md](WARDROBE_AUDIT_REPORT.md), the
rules are in [SKIN_ARCHIVE_AUDIT.md](SKIN_ARCHIVE_AUDIT.md) and the design in
"Catalog categories" of [ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md).

## How a category is installed

Every enabled category goes through the same pipeline as a hero skin: a pinned
archive, an analyzer that must reproduce the audit on every build, one merged
VPK, backup, journal and rollback. What differs is declared by the category
(`skin_archive::CATEGORIES`, a table in the binary that a manifest cannot
loosen):

| | Hero skins (`heroes`, `hero-items`) | Every other enabled category |
| --- | --- | --- |
| Scope kind | `hero`: files are the hero's own, shared or author-owned | `none`: no hero; every accepted file installs |
| Valve overrides | counted, listed, and left out by the opt-in "Keep the game's shared files untouched" | counted and listed as information only; there is no opt-in (overriding game files is what a courier or a sound pack is for) |
| Source | a ZIP holding one `pakNN_dir.vpk` | a ZIP as above (659 of the 939 archives are ZIPs, all of the heroes) or a raw `pakNN_dir.vpk` served as the item itself (280 items), pinned by its own size and SHA-256 |
| Types | compiled resources and proven image wrappers | per category, below |

Type policies. A policy only narrows the compiled types; scripts, executables,
traversal and unproven layouts are rejected under every one of them, and a
policy never admits a `vxml_c`, `vcss_c` or `webm`:

| Policy | Accepted | Folders | Categories |
| --- | --- | --- | --- |
| compiled resources | `vpcf_c vsnap_c vtex_c vmat_c vmdl_c vmesh_c vanim_c vsndevts_c vsnd_c vmorf_c vagrp_c vphys_c vseq_c`, plus image wrappers the layout reader proves | any | all not named below |
| sounds | `vsnd_c`, `vsndevts_c` | `sounds/`, `soundevents/` | `mega-kill`, `music`, `sounds`, `hero-sounds`, `announcers` |
| panorama textures | `vtex_c` | `panorama/images/` | `ranks`, `item-icons` |
| materials | `vmat_c`, `vtex_c` | any | `shaders` |

A file of an accepted type outside its category's folders is dropped as
`outside_category` (none occurs in the current catalog). A `.txt` inside
`models/`, `materials/`, `particles/` or `sounds/` that is not a `portraits.txt`
is a note someone left in the folder (Valve's list holds no such file), so it
is counted as "not a game file, ignored" instead of "dropped": ten couriers
carry one text note of that kind, whose file name has an `=` in it, and an odd
file name on such a leftover no longer blocks the archive. A file of a type the game would load whose
name has characters the VPK writer does not accept stays an unsafe path and
blocks the archive (5 items, below).

## Enabled categories

Counts are from the generated audit. "Archives" is how many of the category's
catalog items are in the allowlist; "ZIP / VPK" is how many of those are
downloaded as a ZIP or as a raw VPK; "Install" is the files and MiB that go into
the build in total; "Overrides" is how many of those files replace a file Valve
ships (informational).

| Category | Items | Archives | ZIP / VPK | Install files | MiB | Largest (MiB) | Overrides | Mechanism and what it changes | Status |
| --- | ---: | ---: | --- | ---: | ---: | --- | ---: | --- | --- |
| `heroes` | 460 | 449 | 449 / 0 | 85,452 | 5,621.5 | `heroes-nude-marci` (207.3) | 2,845 | hero scope, resources | 3 skins seen in Dota (EA.41); rest not run |
| `hero-items` | 52 | 51 | 51 / 0 | 592 | 38.7 | `hero-items-tinker-cripple` (16.1) | 42 | hero scope, resources | not run in Dota |
| `ancient` | 2 | 2 | 2 / 0 | 23 | 10.6 | `ancient-ancient-dragon-king` (6.6) | 11 | resources; building models and effects | not run in Dota |
| `announcers` | 7 | 7 | 0 / 7 | 7 | 0.1 | `announcers-lina` (0.1) | 7 | sounds; sound-event definitions only, no audio | not run in Dota |
| `couriers` | 25 | 25 | 0 / 25 | 184 | 49.5 | `couriers-baby-roshan-2018` (4.0) | 184 | resources; courier model, materials, effects | not run in Dota |
| `creep-deny` | 12 | 12 | 4 / 8 | 16 | 0.3 | `creep-deny-serega-pirat-deny` (0.3) | 13 | resources; the effect shown on a deny (one pack has a sound) | not run in Dota |
| `creeps` | 21 | 21 | 0 / 21 | 258 | 70.8 | `creeps-nemestice-radiant-creeps` (18.5) | 154 | resources; creep models, animations, textures | not run in Dota |
| `emblems` | 16 | 16 | 0 / 16 | 109 | 0.4 | `emblems-emblem-lsq-black-star` (0.1) | 19 | resources; emblem particle effects | not run in Dota |
| `hero-sounds` | 15 | 15 | 15 / 0 | 26 | 31.4 | `hero-sounds-largo-serega-pirat-v2` (17.4) | 26 | sounds; weapon and attack sounds | not run in Dota |
| `herofx` | 5 | 5 | 1 / 4 | 60 | 0.6 | `herofx-tinker-peashooter-turrets` (0.3) | 53 | resources; hero effects, a few icons and sounds | not run in Dota |
| `high-five` | 2 | 2 | 0 / 2 | 16 | 0.2 | `high-five-high-five-aghanim-puppet` (0.1) | 16 | resources | not run in Dota |
| `item-effects` | 143 | 143 | 129 / 14 | 1,227 | 13.7 | `item-effects-earthshaker-gold-blink-dagger` (3.5) | 846 | resources; item particles, materials, textures | not run in Dota |
| `item-icons` | 8 | 8 | 0 / 8 | 1,851 | 23.4 | `item-icons-white-items` (7.2) | 1,533 | panorama textures; item icons only | not run in Dota |
| `mega-kill` | 34 | 34 | 0 / 34 | 254 | 15.6 | `mega-kill-diretide` (5.4) | 171 | sounds; kill-streak voice lines and events | not run in Dota |
| `music` | 14 | 14 | 0 / 14 | 657 | 879.0 | `music-deadmau5-dieback` (87.8) | 577 | sounds; music tracks | not run in Dota |
| `optimization` | 4 | 2 | 0 / 2 | 125 | 3.4 | `optimization-default-wards` (3.3) | 123 | resources; models and effects | not run in Dota |
| `pings` | 3 | 3 | 0 / 3 | 42 | 0.1 | `pings-skeleton-purple-ping` (0.0) | 42 | resources; ping effects | not run in Dota |
| `ranged-attack` | 9 | 9 | 0 / 9 | 2,799 | 5.5 | `ranged-attack-ti12-ranged-attack` (0.6) | 1,368 | resources; projectile effects | not run in Dota |
| `ranks` | 8 | 8 | 0 / 8 | 132 | 13.0 | `ranks-azur-lane-medals` (5.0) | 132 | panorama textures; rank icons only | not run in Dota |
| `river` | 20 | 20 | 0 / 20 | 371 | 3.0 | `river-black-river` (0.9) | 364 | resources; water materials | not run in Dota |
| `roshan` | 10 | 10 | 0 / 10 | 10 | 3.9 | `roshan-greevil-grief-roshan` (1.2) | 10 | resources; model and materials | not run in Dota |
| `shaders` | 32 | 32 | 0 / 32 | 115 | 12.2 | `shaders-aghanim-green-lite` (1.0) | 37 | materials; materials and textures | not run in Dota |
| `sounds` | 9 | 8 | 8 / 0 | 43 | 50.4 | `sounds-sora-main-menu` (14.8) | 43 | sounds; music, voices, ambience | not run in Dota |
| `terrains` (raw VPK) | 20 | 4 | 0 / 4 | 3,925 | 119.9 | `terrains-simple-dark-terrain-minify` (33.5) | 3,598 | resources; terrain materials, models, textures (not the map file) | not run in Dota |
| `ti-bp-effects` | 16 | 16 | 0 / 16 | 1,582 | 5.4 | `ti-bp-effects-purple-quarteros-curios` (1.0) | 1,273 | resources; effects of TI and Battle Pass sets | not run in Dota |
| `towers` | 1 | 1 | 0 / 1 | 1 | 0.2 | `towers-holiday-radiant-tower` (0.2) | 1 | resources; tower model | not run in Dota |
| `trees` | 18 | 17 | 0 / 17 | 1,736 | 104.7 | `trees-metal-barrel-trees` (44.1) | 1,475 | resources; tree models, materials, textures | not run in Dota |
| `wards` | 5 | 5 | 0 / 5 | 5 | 1.3 | `wards-curious-snaptrap` (0.4) | 5 | resources; ward models | not run in Dota |

The "what it changes" column is read from the folders the audited archives write
to (for example `models/courier`, `particles/ui_mouseactions`,
`sounds/music`); it is a description of the files, not an observation of the
game. `optimization` has 4 catalog items: two are archives, the other two are
links (a guide and a notes file) and are not content. Overlapping packs inside
one category write the same paths: two mega-kill packs (`mega-kill-nedotrax-mega-kill`
and `mega-kill-deus-ex`) share 23 announcer files with different bytes, two creep
skins (`creeps-woodland-radiant-creeps` and `creeps-diretide-radiant-creeps`) 6,
while `creeps-pine-radiant-creeps` and `creeps-pine-dire-creeps` share 24 names
with identical bytes, which is a duplicate and not a conflict. The build reports
every path whose bytes differ and asks for the person's confirmation, exactly as
for two hero skins (see "Plan, conflicts and size" in ENGINE_ARCHITECTURE).

## Upstream help text

Only what the upstream project's troubleshooting page says
(<https://d2pfxwiki.pages.dev/en/troubleshoot.html>); shown in the build review
and the card, and nothing beyond it:

- **Emblems.** If an emblem does not show, open the console before the match and
  run `r_draw_selected_ring 0`, then `r_draw_selected_ring 1`, and make sure
  Minify's Misc Optimization mod is not on.
- **Shaders.** If a shader looks checkered, turn off the World Lighting graphics
  setting.
- **File names.** Mod file names must be `pak01_dir.vpk` to `pak99_dir.vpk`.
  BetterFy picks its file name itself (65 down to 50, see "File names and
  coexistence with Minify" in ENGINE_ARCHITECTURE), so there is nothing to
  rename.

## Not enabled, and why

These stay preview-only in the app: no half support, no install button (the one
exception in the table, the terrain ZIPs, is installable through the map
allowlist, not this one). Each reason is a fact about the files or about work
that is not done.

| Category | Items | Files | Why not |
| --- | ---: | --- | --- |
| `backgrounds` | 62 | raw VPKs of `vxml_c`, `vcss_c`, `webm`, `txt` | Panorama layouts, styles and video. The layout proof accepts plain image wrappers only; a separate Panorama proof is being built elsewhere (see `panorama_layout.rs`) |
| `huds` | 4 | raw VPKs of `vcss_c` and `vtex_c` | Panorama styles; the same proof |
| `versus-screens` | 7 | raw VPKs of `vxml_c` | Panorama layouts; the same proof |
| `terrains` (ZIP items) | 12 | a ZIP of `maps/dota.vpk` and `Guide.txt` | Not in this allowlist: each replaces the map file itself and is installed as a map through its own allowlist (`src-tauri/maps`, see [MAP_AUDIT_REPORT.md](MAP_AUDIT_REPORT.md) and "Map terrains" in ENGINE_ARCHITECTURE). Of the 8 raw-VPK terrains, 4 are enabled here and 4 are blocked (below) |
| `cursors` | 34 | a ZIP of a folder of loose cursor files | No VPK and no verified install target |
| `fonts` | 10 | a ZIP of a folder of loose font files | No VPK and no verified install target |
| `tools` | 11 | programs and scripts | Not game content; BetterFy installs no program or script |
| `guides`, `news`, `sites` | 5, 2, 8 | links | Not game content |
| `other` | 9 | `scripts/*.txt` and a few resources | Scripts and unproven layouts |
| `pedestal` | 4 | none | The files do not exist: the Hugging Face mirror answers 404 for each and the GitHub tree does not list them |
| `tormentor` | 2 | none | The same |
| `packs` | 6 | none | Collections of other items with no file of their own |

## Items of enabled categories that are not installable

32 of 971 audited items (generated list in the audit report):

- 12 terrain ZIPs: not in this allowlist, installed as maps (above).
- 11 blocked by the analyzer: 6 hero items and skins (unsafe paths and, for one,
  a script, as before) and 5 terrain and tree packs (`terrains-darkpoly`,
  `terrains-darkpoly-trees`, `terrains-lowpoly`, `terrains-lowpoly-trees`,
  `trees-cartoon-trees`): each carries one texture whose name has brackets,
  `... (carve.photos)_png_ece241c0.vtex_c`. Valve's own file names use only
  `a-z 0-9 _ - . + /` and space, so the VPK writer takes exactly those; relaxing
  it for brackets is a possible follow-up and is not done here.
- 5 items whose catalog "file" is a link rather than a mirrored file
  (`heroes-phantom-assassin-yor-forger`, `heroes-tinker-alexander-prime`, two
  guide links under `optimization`, and one `sounds` item), 2 hero
  skins over the 256 MiB bound, 1 unreadable VPK and 1 skin never fetched.

## What has and has not been run

- Run on a Mac: the audit of 971 items from 965 cached archives (509 fetched
  earlier, 456 for this change); the analyzer and the manifests' validation in unit tests;
  real plans from the cached archives for the smallest and the largest item of
  every category, a nine-item mix, packs that conflict, and selections over the
  256 MiB bound (numbers in the pull request and the test plan).
- Not run: Windows CI for this change (see the pull request), a download through
  BetterFy's own transport of any of the new archives (they were fetched with
  `curl`; the transport was exercised against fakes and, for a raw VPK, a fake
  with the new spec), an install on Windows, and anything in Dota. Peak memory
  and time for the 879 MiB of music across 14 items, or a 92 MiB single pack, on
  a real machine are unmeasured.
