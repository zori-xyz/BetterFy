# Wardrobe catalog audit report

Written by the local-only test `wardrobe_audit::generate_wardrobe_allowlist` (`src-tauri/src/wardrobe_audit.rs`); do not edit by hand. It is the audit trail for `src-tauri/wardrobe/manifests*.json`. The run is a Mac run over archives downloaded to a local cache: nothing here was run on Windows or in Dota, and no archive bytes are part of the repository.

## Inputs

| Input | Value |
| --- | --- |
| Catalog | `src/webCatalog.json`, snapshot 2026-07-26 |
| Items audited | 971 in 28 enabled categories |
| Valve file list | SteamDatabase/GameTracking-Dota2 `game/dota/pak01_dir.txt`, 388116 entries, SHA-256 `6d98c763afe09ba0de5caf0fc965b6b21f26eeb2954ff1e7744996ba056db1b7` |
| Hero inference (hero skins only) | folders of Valve-present paths: `models/heroes/<h>`, `models/items/<h>`, `particles/units/heroes/hero_<h>`, `particles/econ/items/<h>`, `materials/models/(heroes\|items)/<h>`; every name with at least 5% of the top name's paths is kept (at most 8) |
| Other categories | no hero: every accepted file installs; "overrides a Valve file" and "new file" are computed against the same list and pinned (information, not a policy) |
| Mirrors | candidate URLs checked by HEAD request: kept only when they answer 200 with the pinned size |
| Embedded allowlist | 4 files, 2.16 MB in all (`manifests.json` 0.89 MB, `manifests-effects.json` 0.56 MB, `manifests-world.json` 0.46 MB, `manifests-audio-ui.json` 0.24 MB); interface copy `index.json` 0.33 MB |

Emitted 939 of 971 items.
Skipped 32 of 971 items.

## Categories

| Category | Scope | Types | Items | Emitted | Skipped | Install MiB | Overrides a Valve file | New files |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `heroes` | hero | compiled resources | 460 | 449 | 11 | 5621.5 | 2845 | 33984 |
| `hero-items` | hero | compiled resources | 52 | 51 | 1 | 38.7 | 42 | 30 |
| `ancient` | none | compiled resources | 2 | 2 | 0 | 10.6 | 11 | 12 |
| `announcers` | none | sounds | 7 | 7 | 0 | 0.1 | 7 | 0 |
| `couriers` | none | compiled resources | 25 | 25 | 0 | 49.5 | 184 | 0 |
| `creep-deny` | none | compiled resources | 12 | 12 | 0 | 0.3 | 13 | 3 |
| `creeps` | none | compiled resources | 21 | 21 | 0 | 70.8 | 154 | 104 |
| `emblems` | none | compiled resources | 16 | 16 | 0 | 0.4 | 19 | 90 |
| `hero-sounds` | none | sounds | 15 | 15 | 0 | 31.4 | 26 | 0 |
| `herofx` | none | compiled resources | 5 | 5 | 0 | 0.6 | 53 | 7 |
| `high-five` | none | compiled resources | 2 | 2 | 0 | 0.2 | 16 | 0 |
| `item-effects` | none | compiled resources | 143 | 143 | 0 | 13.7 | 846 | 381 |
| `item-icons` | none | panorama textures | 8 | 8 | 0 | 23.4 | 1533 | 318 |
| `mega-kill` | none | sounds | 34 | 34 | 0 | 15.6 | 171 | 83 |
| `music` | none | sounds | 14 | 14 | 0 | 879.0 | 577 | 80 |
| `optimization` | none | compiled resources | 4 | 2 | 2 | 3.4 | 123 | 2 |
| `pings` | none | compiled resources | 3 | 3 | 0 | 0.1 | 42 | 0 |
| `ranged-attack` | none | compiled resources | 9 | 9 | 0 | 5.5 | 1368 | 1431 |
| `ranks` | none | panorama textures | 8 | 8 | 0 | 13.0 | 132 | 0 |
| `river` | none | compiled resources | 20 | 20 | 0 | 3.0 | 364 | 7 |
| `roshan` | none | compiled resources | 10 | 10 | 0 | 3.9 | 10 | 0 |
| `shaders` | none | materials and textures | 32 | 32 | 0 | 12.2 | 37 | 78 |
| `sounds` | none | sounds | 9 | 8 | 1 | 50.4 | 43 | 0 |
| `terrains` | none | compiled resources | 20 | 4 | 16 | 119.9 | 3598 | 327 |
| `ti-bp-effects` | none | compiled resources | 16 | 16 | 0 | 5.4 | 1273 | 309 |
| `towers` | none | compiled resources | 1 | 1 | 0 | 0.2 | 1 | 0 |
| `trees` | none | compiled resources | 18 | 17 | 1 | 104.7 | 1475 | 261 |
| `wards` | none | compiled resources | 5 | 5 | 0 | 1.3 | 5 | 0 |

## Categories that are not enabled

| Category | Items | Why |
| --- | ---: | --- |
| `backgrounds` | 62 | Panorama layouts, styles and video (vxml_c, vcss_c, webm): the layout proof covers plain image wrappers only; a separate proof is in progress |
| `huds` | 4 | Panorama layouts and styles (vcss_c with textures): not covered by the layout proof |
| `versus-screens` | 7 | Panorama layouts (vxml_c): not covered by the layout proof |
| `cursors` | 34 | loose cursor files in a ZIP (no VPK): no verified install target |
| `fonts` | 10 | loose font files in a ZIP (no VPK): no verified install target |
| `tools` | 11 | programs and scripts, not game content: BetterFy installs none |
| `guides` | 5 | reading material, not game content |
| `news` | 2 | links, not game content |
| `sites` | 8 | links, not game content |
| `other` | 9 | scripts and text (scripts/*.txt, unproven layouts): not installed |
| `pedestal` | 4 | the files do not exist on either mirror (HTTP 404 on Hugging Face, absent from the GitHub tree) |
| `tormentor` | 2 | the files do not exist on either mirror (HTTP 404 on Hugging Face, absent from the GitHub tree) |
| `packs` | 6 | collections of other items, with no file of their own |
| `terrains` (ZIP items) | 12 | each ZIP holds `maps/dota.vpk` and a guide, not a `pakNN_dir.vpk`: a whole-map replacement, installed through the separate map allowlist (`src-tauri/maps`), not this one |

## Items that are not in the allowlist

| Reason | Items |
| --- | ---: |
| VPK container unreadable | 1 |
| ZIP does not hold exactly one pakNN_dir.vpk | 12 |
| analyzer blocked the install (skin_archive_hostile) | 11 |
| archive over the 256 MiB limit | 2 |
| no archive in the cache | 1 |
| the catalog's file is an external link, not a mirrored file | 5 |

### VPK container unreadable (1)

- `heroes-black-witch-doctor`: vpk_invalid

### ZIP does not hold exactly one pakNN_dir.vpk (12)

- `terrains-crownfall`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-dota-autumn`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-dota-spring`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-dota-summer`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-dota-winter`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-the-kings-new-journey`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti10-sanctum-of-the-divine`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti5-desert-terrain`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti6-immortal-gardens`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti7-reefs-edge`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti8-emerald-abyss`: 2 entries: maps/dota.vpk, Guide.txt
- `terrains-ti9-overgrown-empire`: 2 entries: maps/dota.vpk, Guide.txt

### analyzer blocked the install (skin_archive_hostile) (11)

- `hero-items-disruptor-tractor`: PathUnsafe x32
- `heroes-ember-spirit-mai-shiranui`: PathUnsafe x4
- `heroes-enigma-blackhole-chan`: PathUnsafe x1, Script x1
- `heroes-lina-crystal-empress`: PathUnsafe x1
- `heroes-mirana-suigintou`: PathUnsafe x9
- `heroes-outworld-destroyer-tatsumaki`: PathUnsafe x3
- `terrains-darkpoly`: PathUnsafe x1
- `terrains-darkpoly-trees`: PathUnsafe x1
- `terrains-lowpoly`: PathUnsafe x1
- `terrains-lowpoly-trees`: PathUnsafe x1
- `trees-cartoon-trees`: PathUnsafe x1

### archive over the 256 MiB limit (2)

- `heroes-legion-commander-chen`: the VPK inside the ZIP is over the unpacked-size bound of the archive inspector
- `heroes-nude-drow-ranger-v2`: the VPK inside the ZIP is over the unpacked-size bound of the archive inspector

### no archive in the cache (1)

- `heroes-crimson-anti-mage`: never fetched: the survey could not read its ZIP directory or no mirror had the file

### the catalog's file is an external link, not a mirrored file (5)

- `heroes-phantom-assassin-yor-forger`: https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Phantom%20Assassin%20Yor%20Forger.zip
- `heroes-tinker-alexander-prime`: https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Tinker%20Alexander%20Prime.zip
- `optimization-commands`: https://github.com/h6rd/Dota2PornFxWeb/tree/main/assets/files/optimization/Commands.md
- `optimization-dota2-minify`: https://egezenn.github.io/dota2-minify/
- `sounds-warcraft-iii-allstars-voices`: https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Warcraft%20III%20AllStars%20Voices.zip

## Emitted items in numbers

Total install bytes of all emitted items: 7078.9 MiB (101618 files) from 4234.9 MiB of downloads; hero skins with the shared files left out (opt-in): 5339.2 MiB of 5660.3 MiB.
Biggest install: `heroes-nude-marci`, 207.3 MiB.
Install size per item (MiB): median 0.7, p90 22.5, p99 76.0.
Items that override no Valve file: 273; that override more than 100: 26; Valve files overridden in total: 14968.
Hero-name ambiguity: 56 items carry more than one internal hero name (a second name with at least 5% of the top name's Valve-present paths).
Paths folded to lower case: 34 files in 3 items.
Classes over all emitted items: 49143 hero-scoped files, 37507 files the game does not ship (installed), 14968 files that override a Valve file (installed).
Items with two verified mirrors: 938; with one: 1.
Files left out of the installs: 350 dropped (game files or unprovable ones BetterFy does not install), 585 ignored (not game files).

Files ignored because they are not game files (uncompiled sources, readmes and tool leftovers; the game loads none of them from a VPK, so leaving them out changes nothing in game):

| Reason | Files |
| --- | ---: |
| not_game_file | 46 |
| uncompiled_source | 539 |

Files dropped for real (not installed, listed per item in the plan):

| Reason | Files |
| --- | ---: |
| panorama_scripted | 2 |
| type_not_allowed | 348 |

Files dropped for real, by reason and file type:

| Reason | Type | Files | MiB | Items |
| --- | --- | ---: | ---: | ---: |
| panorama_scripted | `vxml_c` | 2 | 0.0 | 2 |
| type_not_allowed | `dds` | 1 | 0.0 | 1 |
| type_not_allowed | `png` | 231 | 19.5 | 21 |
| type_not_allowed | `psf` | 15 | 0.0 | 4 |
| type_not_allowed | `txt` | 4 | 15.6 | 4 |
| type_not_allowed | `usm` | 17 | 11.6 | 17 |
| type_not_allowed | `vcss_c` | 11 | 0.2 | 5 |
| type_not_allowed | `vmat_` | 2 | 0.0 | 1 |
| type_not_allowed | `webm` | 67 | 81.2 | 64 |

Dropped for a reason other than an unsupported type, listed by item:

- `heroes-pudge-clown-toy`: `models/heroes/76561199168196947.vxml_c` (1608 bytes): panorama_scripted
- `heroes-void-spirit-keqing`: `models/heroes/76561199380779061.vxml_c` (1608 bytes): panorama_scripted

Folders where items override Valve files most often (first three path segments):

| Folder | Files |
| --- | ---: |
| `materials/default` | 1682 |
| `panorama/images/items` | 1552 |
| `particles/econ/events` | 1361 |
| `models/props_tree` | 1119 |
| `particles/units/heroes` | 905 |
| `materials/overlays` | 728 |
| `particles/econ/items` | 596 |
| `materials/models/cubemaps` | 585 |
| `sounds/music/valve_dota_001` | 558 |
| `models/props_nature` | 432 |
| `models/props_debris` | 368 |
| `materials/water` | 290 |
| `models/props_structures` | 286 |
| `materials/models/props_debris` | 280 |
| `particles/items2_fx` | 269 |
| `particles/items_fx` | 240 |
| `materials/models/props_nature` | 234 |
| `sounds/vo/announcer_killing_spree` | 146 |
| `materials/nature` | 144 |
| `materials/models/props_structures` | 136 |
| `models/props_bones` | 135 |
| `panorama/images/rank_tier_icons` | 132 |
| `models/props_rock` | 124 |
| `models/props_gameplay` | 120 |
| `particles/generic_gameplay` | 111 |

## Per category

### `heroes`

- Emitted 449 of 460 (11 skipped); source: zip; downloads 2800.0 MiB.
- Install: 85452 files, 5621.5 MiB in all; per item median 5.4 MiB, largest `heroes-nude-marci` 207.3 MiB.
- Overrides a Valve file: 2845 files; new files (the game does not ship them): 33984; dropped: 350; ignored: 561.
- skipped `heroes-black-witch-doctor`: VPK container unreadable (vpk_invalid)
- skipped `heroes-crimson-anti-mage`: no archive in the cache (never fetched: the survey could not read its ZIP directory or no mirror had the file)
- skipped `heroes-ember-spirit-mai-shiranui`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x4)
- skipped `heroes-enigma-blackhole-chan`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1, Script x1)
- skipped `heroes-legion-commander-chen`: archive over the 256 MiB limit (the VPK inside the ZIP is over the unpacked-size bound of the archive inspector)
- skipped `heroes-lina-crystal-empress`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)
- skipped `heroes-mirana-suigintou`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x9)
- skipped `heroes-nude-drow-ranger-v2`: archive over the 256 MiB limit (the VPK inside the ZIP is over the unpacked-size bound of the archive inspector)
- skipped `heroes-outworld-destroyer-tatsumaki`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x3)
- skipped `heroes-phantom-assassin-yor-forger`: the catalog's file is an external link, not a mirrored file (https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Phantom%20Assassin%20Yor%20Forger.zip)
- skipped `heroes-tinker-alexander-prime`: the catalog's file is an external link, not a mirrored file (https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Tinker%20Alexander%20Prime.zip)

### `hero-items`

- Emitted 51 of 52 (1 skipped); source: zip; downloads 23.5 MiB.
- Install: 592 files, 38.7 MiB in all; per item median 0.2 MiB, largest `hero-items-tinker-cripple` 16.1 MiB.
- Overrides a Valve file: 42 files; new files (the game does not ship them): 30; dropped: 0; ignored: 12.
- skipped `hero-items-disruptor-tractor`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x32)

### `ancient`

- Emitted 2 of 2 (0 skipped); source: zip; downloads 9.2 MiB.
- Install: 23 files, 10.6 MiB in all; per item median 4.0 MiB, largest `ancient-ancient-dragon-king` 6.6 MiB.
- Overrides a Valve file: 11 files; new files (the game does not ship them): 12; dropped: 0; ignored: 0.

### `announcers`

- Emitted 7 of 7 (0 skipped); source: vpk; downloads 0.1 MiB.
- Install: 7 files, 0.1 MiB in all; per item median 0.0 MiB, largest `announcers-lina` 0.1 MiB.
- Overrides a Valve file: 7 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `couriers`

- Emitted 25 of 25 (0 skipped); source: vpk; downloads 49.5 MiB.
- Install: 184 files, 49.5 MiB in all; per item median 1.8 MiB, largest `couriers-baby-roshan-2018` 4.0 MiB.
- Overrides a Valve file: 184 files; new files (the game does not ship them): 0; dropped: 0; ignored: 10.

### `creep-deny`

- Emitted 12 of 12 (0 skipped); source: vpk, zip; downloads 0.3 MiB.
- Install: 16 files, 0.3 MiB in all; per item median 0.0 MiB, largest `creep-deny-serega-pirat-deny` 0.3 MiB.
- Overrides a Valve file: 13 files; new files (the game does not ship them): 3; dropped: 0; ignored: 0.

### `creeps`

- Emitted 21 of 21 (0 skipped); source: vpk; downloads 70.8 MiB.
- Install: 258 files, 70.8 MiB in all; per item median 2.4 MiB, largest `creeps-nemestice-radiant-creeps` 18.5 MiB.
- Overrides a Valve file: 154 files; new files (the game does not ship them): 104; dropped: 0; ignored: 0.

### `emblems`

- Emitted 16 of 16 (0 skipped); source: vpk; downloads 0.4 MiB.
- Install: 109 files, 0.4 MiB in all; per item median 0.0 MiB, largest `emblems-emblem-lsq-black-star` 0.1 MiB.
- Overrides a Valve file: 19 files; new files (the game does not ship them): 90; dropped: 0; ignored: 0.

### `hero-sounds`

- Emitted 15 of 15 (0 skipped); source: zip; downloads 31.0 MiB.
- Install: 26 files, 31.4 MiB in all; per item median 0.2 MiB, largest `hero-sounds-largo-serega-pirat-v2` 17.4 MiB.
- Overrides a Valve file: 26 files; new files (the game does not ship them): 0; dropped: 0; ignored: 1.

### `herofx`

- Emitted 5 of 5 (0 skipped); source: vpk, zip; downloads 0.6 MiB.
- Install: 60 files, 0.6 MiB in all; per item median 0.1 MiB, largest `herofx-tinker-peashooter-turrets` 0.3 MiB.
- Overrides a Valve file: 53 files; new files (the game does not ship them): 7; dropped: 0; ignored: 0.

### `high-five`

- Emitted 2 of 2 (0 skipped); source: vpk; downloads 0.2 MiB.
- Install: 16 files, 0.2 MiB in all; per item median 0.1 MiB, largest `high-five-high-five-aghanim-puppet` 0.1 MiB.
- Overrides a Valve file: 16 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `item-effects`

- Emitted 143 of 143 (0 skipped); source: vpk, zip; downloads 9.0 MiB.
- Install: 1227 files, 13.7 MiB in all; per item median 0.0 MiB, largest `item-effects-earthshaker-gold-blink-dagger` 3.5 MiB.
- Overrides a Valve file: 846 files; new files (the game does not ship them): 381; dropped: 0; ignored: 0.

### `item-icons`

- Emitted 8 of 8 (0 skipped); source: vpk; downloads 23.5 MiB.
- Install: 1851 files, 23.4 MiB in all; per item median 1.7 MiB, largest `item-icons-white-items` 7.2 MiB.
- Overrides a Valve file: 1533 files; new files (the game does not ship them): 318; dropped: 0; ignored: 0.

### `mega-kill`

- Emitted 34 of 34 (0 skipped); source: vpk; downloads 15.6 MiB.
- Install: 254 files, 15.6 MiB in all; per item median 0.0 MiB, largest `mega-kill-diretide` 5.4 MiB.
- Overrides a Valve file: 171 files; new files (the game does not ship them): 83; dropped: 0; ignored: 0.

### `music`

- Emitted 14 of 14 (0 skipped); source: vpk; downloads 879.0 MiB.
- Install: 657 files, 879.0 MiB in all; per item median 61.9 MiB, largest `music-deadmau5-dieback` 87.8 MiB.
- Overrides a Valve file: 577 files; new files (the game does not ship them): 80; dropped: 0; ignored: 0.

### `optimization`

- Emitted 2 of 4 (2 skipped); source: vpk; downloads 3.4 MiB.
- Install: 125 files, 3.4 MiB in all; per item median 0.0 MiB, largest `optimization-default-wards` 3.3 MiB.
- Overrides a Valve file: 123 files; new files (the game does not ship them): 2; dropped: 0; ignored: 0.
- skipped `optimization-commands`: the catalog's file is an external link, not a mirrored file (https://github.com/h6rd/Dota2PornFxWeb/tree/main/assets/files/optimization/Commands.md)
- skipped `optimization-dota2-minify`: the catalog's file is an external link, not a mirrored file (https://egezenn.github.io/dota2-minify/)

### `pings`

- Emitted 3 of 3 (0 skipped); source: vpk; downloads 0.1 MiB.
- Install: 42 files, 0.1 MiB in all; per item median 0.0 MiB, largest `pings-skeleton-purple-ping` 0.0 MiB.
- Overrides a Valve file: 42 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `ranged-attack`

- Emitted 9 of 9 (0 skipped); source: vpk; downloads 5.7 MiB.
- Install: 2799 files, 5.5 MiB in all; per item median 0.6 MiB, largest `ranged-attack-ti12-ranged-attack` 0.6 MiB.
- Overrides a Valve file: 1368 files; new files (the game does not ship them): 1431; dropped: 0; ignored: 0.

### `ranks`

- Emitted 8 of 8 (0 skipped); source: vpk; downloads 13.0 MiB.
- Install: 132 files, 13.0 MiB in all; per item median 0.9 MiB, largest `ranks-azur-lane-medals` 5.0 MiB.
- Overrides a Valve file: 132 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `river`

- Emitted 20 of 20 (0 skipped); source: vpk; downloads 3.1 MiB.
- Install: 371 files, 3.0 MiB in all; per item median 0.1 MiB, largest `river-black-river` 0.9 MiB.
- Overrides a Valve file: 364 files; new files (the game does not ship them): 7; dropped: 0; ignored: 0.

### `roshan`

- Emitted 10 of 10 (0 skipped); source: vpk; downloads 3.9 MiB.
- Install: 10 files, 3.9 MiB in all; per item median 0.0 MiB, largest `roshan-greevil-grief-roshan` 1.2 MiB.
- Overrides a Valve file: 10 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `shaders`

- Emitted 32 of 32 (0 skipped); source: vpk; downloads 12.3 MiB.
- Install: 115 files, 12.2 MiB in all; per item median 0.0 MiB, largest `shaders-aghanim-yellow-lite` 1.0 MiB.
- Overrides a Valve file: 37 files; new files (the game does not ship them): 78; dropped: 0; ignored: 0.

### `sounds`

- Emitted 8 of 9 (1 skipped); source: zip; downloads 48.9 MiB.
- Install: 43 files, 50.4 MiB in all; per item median 2.1 MiB, largest `sounds-sora-main-menu` 14.8 MiB.
- Overrides a Valve file: 43 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.
- skipped `sounds-warcraft-iii-allstars-voices`: the catalog's file is an external link, not a mirrored file (https://huggingface.co/datasets/hrdq/d2pfx/resolve/main/Warcraft%20III%20AllStars%20Voices.zip)

### `terrains`

- Emitted 4 of 20 (16 skipped); source: vpk; downloads 120.1 MiB.
- Install: 3925 files, 119.9 MiB in all; per item median 26.8 MiB, largest `terrains-simple-dark-terrain-minify` 33.5 MiB.
- Overrides a Valve file: 3598 files; new files (the game does not ship them): 327; dropped: 0; ignored: 0.
- skipped `terrains-crownfall`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-darkpoly`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)
- skipped `terrains-darkpoly-trees`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)
- skipped `terrains-dota-autumn`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-dota-spring`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-dota-summer`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-dota-winter`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-lowpoly`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)
- skipped `terrains-lowpoly-trees`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)
- skipped `terrains-the-kings-new-journey`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti10-sanctum-of-the-divine`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti5-desert-terrain`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti6-immortal-gardens`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti7-reefs-edge`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti8-emerald-abyss`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)
- skipped `terrains-ti9-overgrown-empire`: ZIP does not hold exactly one pakNN_dir.vpk (2 entries: maps/dota.vpk, Guide.txt)

### `ti-bp-effects`

- Emitted 16 of 16 (0 skipped); source: vpk; downloads 5.5 MiB.
- Install: 1582 files, 5.4 MiB in all; per item median 0.2 MiB, largest `ti-bp-effects-purple-quarteros-curios` 1.0 MiB.
- Overrides a Valve file: 1273 files; new files (the game does not ship them): 309; dropped: 0; ignored: 1.

### `towers`

- Emitted 1 of 1 (0 skipped); source: vpk; downloads 0.2 MiB.
- Install: 1 files, 0.2 MiB in all; per item median 0.2 MiB, largest `towers-holiday-radiant-tower` 0.2 MiB.
- Overrides a Valve file: 1 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

### `trees`

- Emitted 17 of 18 (1 skipped); source: vpk; downloads 104.7 MiB.
- Install: 1736 files, 104.7 MiB in all; per item median 3.2 MiB, largest `trees-metal-barrel-trees` 44.1 MiB.
- Overrides a Valve file: 1475 files; new files (the game does not ship them): 261; dropped: 0; ignored: 0.
- skipped `trees-cartoon-trees`: analyzer blocked the install (skin_archive_hostile) (PathUnsafe x1)

### `wards`

- Emitted 5 of 5 (0 skipped); source: vpk; downloads 1.3 MiB.
- Install: 5 files, 1.3 MiB in all; per item median 0.2 MiB, largest `wards-curious-snaptrap` 0.4 MiB.
- Overrides a Valve file: 5 files; new files (the game does not ship them): 0; dropped: 0; ignored: 0.

## Emitted items

Heroes are the inferred internal names (hero skins only). Files are what goes into the build; overrides is how many of the files override a Valve file (for a hero skin, the opt-in mode leaves them out); dropped is files really left out and ignored is files that are not game files (see above).

| Catalog ID | Heroes | Files | MiB | Overrides | Dropped | Ignored | Folded |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `heroes-abaddon-bloody-fallen-king` | abaddon | 188 | 17.3 | 10 | 0 | 0 | 0 |
| `heroes-abaddon-fallen-king` | abaddon | 414 | 21.1 | 26 | 0 | 14 | 0 |
| `heroes-abaddon-sayaka-miki` | abaddon | 285 | 7.7 | 0 | 22 | 1 | 0 |
| `heroes-alchemist-darkbrew-enforcer` | alchemist | 13 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-alien-nyx-assassin` | nyx_assassin, nerubian_assassin | 186 | 7.7 | 15 | 0 | 1 | 0 |
| `ancient-ancient-dragon-king` |  | 14 | 6.6 | 2 | 0 | 0 | 0 |
| `heroes-ancient-apparition-yoshino` | ancient_apparition | 105 | 4.0 | 0 | 10 | 0 | 0 |
| `ancient-frostivus-ancient` |  | 9 | 4.0 | 9 | 0 | 0 | 0 |
| `heroes-ancient-sand-naga-siren` | siren | 214 | 4.8 | 16 | 0 | 0 | 0 |
| `heroes-ancients-lina` | lina | 567 | 25.0 | 15 | 0 | 14 | 0 |
| `announcers-dark-willow` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `announcers-darkest-dungeon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `announcers-kunkka-tidehunter` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `announcers-lina` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `announcers-meepo` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `announcers-tusk` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `announcers-voice-of-the-international` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `heroes-anti-mage-blue-illidan` | antimage, nightstalker | 435 | 22.9 | 19 | 0 | 0 | 0 |
| `heroes-anti-mage-creep` | antimage | 120 | 1.1 | 12 | 0 | 0 | 0 |
| `heroes-anti-mage-illidan` | antimage | 666 | 28.2 | 0 | 5 | 0 | 0 |
| `heroes-anti-mage-illidan-v2` | antimage | 318 | 26.0 | 21 | 0 | 0 | 0 |
| `heroes-anti-mage-ronin-warrior` | antimage | 138 | 11.2 | 0 | 1 | 0 | 0 |
| `heroes-anti-mage-shadow-slayer` | antimage | 192 | 13.6 | 24 | 0 | 16 | 0 |
| `heroes-anti-mage-turstarkuri-pilgrim` | antimage | 197 | 17.8 | 2 | 0 | 0 | 0 |
| `heroes-anubis-phantom-lancer` | phantom_lancer | 12 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-aquamarine-naga-siren` | siren, naga | 103 | 10.0 | 9 | 0 | 0 | 0 |
| `heroes-arc-warden-ancient-gaoler` | arc_warden | 5 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-arc-warden-black-hole` | arc_warden | 338 | 8.2 | 15 | 0 | 1 | 0 |
| `heroes-arc-warden-sakura-winter` | arc_warden | 351 | 7.0 | 31 | 0 | 1 | 0 |
| `heroes-arc-warden-yin-yang` | arc_warden | 329 | 4.4 | 16 | 0 | 0 | 0 |
| `heroes-ashes-arc-warden` | arc_warden | 369 | 11.9 | 27 | 0 | 0 | 0 |
| `heroes-axe-gawr-gura` | axe | 352 | 90.7 | 0 | 1 | 0 | 0 |
| `heroes-axe-heracles` | axe | 321 | 17.2 | 12 | 1 | 0 | 0 |
| `heroes-axe-immortal-bootblack` | axe | 59 | 3.6 | 2 | 0 | 0 | 0 |
| `heroes-axe-immortal-bootblack-brawler` | axe | 64 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-axe-kratos` | axe | 134 | 14.3 | 24 | 0 | 0 | 0 |
| `heroes-axe-unleashed` | axe | 54 | 3.3 | 0 | 0 | 0 | 0 |
| `heroes-bane-komeiji-koishi` | bane | 138 | 19.3 | 0 | 10 | 0 | 0 |
| `heroes-bare-brewmaster` | brewmaster | 2 | 0.0 | 0 | 0 | 0 | 0 |
| `heroes-bare-hoodwink` | hoodwink | 33 | 11.0 | 9 | 0 | 1 | 0 |
| `heroes-bare-kez` | kez | 1 | 0.0 | 0 | 0 | 0 | 0 |
| `heroes-bare-natures-prophet` | furion | 11 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-bare-pangolier` | pangolier | 3 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-batrider-night-snotty` | batrider | 26 | 1.2 | 0 | 0 | 0 | 0 |
| `heroes-beastmaster-bone-armor` | beastmaster | 9 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-beastmaster-yuuki-yuuna` | beastmaster | 705 | 26.5 | 0 | 1 | 0 | 0 |
| `heroes-black-phoenix` | phoenix | 262 | 9.9 | 0 | 0 | 0 | 0 |
| `heroes-bloodelf-silencer` | silencer | 225 | 18.5 | 6 | 0 | 0 | 0 |
| `heroes-bloodseeker-harvest-hound` | blood_seeker | 7 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-bloodseeker-strygwyr` | blood_seeker | 42 | 2.0 | 4 | 8 | 0 | 4 |
| `heroes-bloody-ancient-apparition` | ancient_apparition | 264 | 10.5 | 27 | 0 | 14 | 0 |
| `heroes-bloody-dazzle` | dazzle | 23 | 1.9 | 0 | 2 | 0 | 0 |
| `heroes-bloody-death-prophet` | death_prophet | 289 | 17.7 | 25 | 0 | 16 | 0 |
| `heroes-bloody-enigma` | enigma | 909 | 53.8 | 25 | 0 | 14 | 0 |
| `heroes-bloody-gore-spectre` | spectre | 1126 | 61.0 | 20 | 0 | 4 | 0 |
| `heroes-bloody-kez` | kez | 649 | 22.9 | 14 | 0 | 30 | 0 |
| `heroes-blue-dragon-puck` | puck | 248 | 8.3 | 26 | 0 | 0 | 0 |
| `heroes-blue-muerta` | muerta | 223 | 169.7 | 15 | 0 | 0 | 0 |
| `heroes-blue-soul-huskar` | huskar | 647 | 57.0 | 21 | 0 | 18 | 0 |
| `heroes-bounty-hunter-hei` | bounty_hunter | 268 | 6.3 | 0 | 1 | 0 | 0 |
| `heroes-bounty-hunter-m1a2-abrams` | bounty_hunter | 391 | 47.9 | 78 | 1 | 0 | 0 |
| `heroes-bounty-hunter-old-man` | bounty_hunter | 10 | 0.3 | 2 | 0 | 0 | 0 |
| `heroes-bounty-hunter-old-verminator` | bounty_hunter | 9 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-brewmaster-jousting-panda` | brewmaster | 82 | 3.8 | 12 | 0 | 0 | 0 |
| `heroes-brewmaster-military-saint` | brewmaster | 112 | 6.3 | 28 | 0 | 0 | 0 |
| `heroes-bristleback-crimson-ring` | bristleback | 5 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-epoch-of-the-abysm` | broodmother | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-golden-orbweaver` | broodmother | 5 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-redan` | broodmother | 294 | 15.9 | 23 | 0 | 0 | 0 |
| `heroes-broodmother-silken-queen` | broodmother | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-undermount-candles` | broodmother | 10 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-undermount-gloom` | broodmother | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-broodmother-white-widow` | broodmother | 6 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-bunshinjutsu-meepo` | meepo | 203 | 3.3 | 12 | 2 | 11 | 0 |
| `heroes-cardboard-shadow-fiend` | shadow_fiend | 37 | 45.6 | 3 | 0 | 0 | 0 |
| `heroes-centaur-chaos-chosen` | centaur | 7 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-centaur-cunning-counterfeit` | centaur | 7 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-centaur-emerald-marauder` | centaur | 7 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-chaos-invoker` | invoker, invoker_kid | 341 | 14.9 | 1 | 0 | 0 | 0 |
| `heroes-chaos-io` | wisp | 88 | 3.0 | 0 | 0 | 0 | 0 |
| `heroes-chaos-knight-flandre-scarlet` | chaos_knight | 240 | 9.0 | 0 | 9 | 0 | 0 |
| `heroes-chen-infernal-psychic` | chen | 6 | 0.7 | 1 | 0 | 0 | 0 |
| `heroes-chen-lelouch` | chen | 320 | 12.7 | 0 | 1 | 0 | 0 |
| `heroes-chen-seed-of-strife` | chen | 15 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-clinkz-silver-metall-pink` | clinkz | 597 | 12.2 | 22 | 0 | 30 | 0 |
| `heroes-clockwerk-apex-automated` | rattletrap | 6 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-clockwerk-seadog-stash` | rattletrap | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-alchemist` | alchemist | 13 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-cartographer` | marci | 4 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-earthshaker` | earthshaker | 12 | 1.2 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-enigma` | enigma | 12 | 2.5 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-faceless-void` | faceless_void | 6 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-immortal-tinker` | tinker | 13 | 1.6 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-spirit-breaker` | spirit_breaker | 10 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-tinker` | tinker | 9 | 1.8 | 0 | 0 | 0 | 0 |
| `heroes-cosmic-zeus` | zeus | 5 | 1.1 | 0 | 0 | 0 | 0 |
| `couriers-aghanim-baby-roshan-dire` |  | 8 | 3.4 | 8 | 0 | 0 | 0 |
| `couriers-aghanim-baby-roshan-radiant` |  | 8 | 3.4 | 8 | 0 | 0 | 0 |
| `couriers-baby-roshan` |  | 4 | 1.1 | 4 | 0 | 0 | 0 |
| `couriers-baby-roshan-2017` |  | 9 | 4.0 | 9 | 0 | 1 | 0 |
| `couriers-baby-roshan-2018` |  | 9 | 4.0 | 9 | 0 | 0 | 0 |
| `couriers-baby-roshan-crownfall` |  | 4 | 1.8 | 4 | 0 | 0 | 0 |
| `couriers-billy-bounceback` |  | 4 | 0.6 | 4 | 0 | 0 | 0 |
| `couriers-dark-moon-baby-roshan` |  | 9 | 1.6 | 9 | 0 | 0 | 0 |
| `couriers-desert-sands-baby-roshan` |  | 9 | 1.9 | 9 | 0 | 1 | 0 |
| `couriers-gingerbread-baby-roshan` |  | 8 | 1.9 | 8 | 0 | 0 | 0 |
| `couriers-golden-doomling` |  | 9 | 1.2 | 9 | 0 | 1 | 0 |
| `couriers-golden-flopjaw-the-boxhound` |  | 9 | 1.1 | 9 | 0 | 1 | 0 |
| `couriers-golden-huntling` |  | 9 | 1.4 | 9 | 0 | 1 | 0 |
| `couriers-golden-seekling` |  | 10 | 1.7 | 10 | 0 | 1 | 0 |
| `couriers-golden-venoling` |  | 9 | 1.1 | 9 | 0 | 1 | 0 |
| `couriers-honey-heist-baby-roshan` |  | 4 | 0.7 | 4 | 0 | 0 | 0 |
| `couriers-ice-baby-roshan` |  | 5 | 1.1 | 5 | 0 | 0 | 0 |
| `couriers-jade-baby-roshan` |  | 9 | 1.9 | 9 | 0 | 0 | 0 |
| `couriers-jadehoof` |  | 8 | 1.5 | 8 | 0 | 1 | 0 |
| `couriers-nian-courier` |  | 8 | 3.5 | 8 | 0 | 1 | 0 |
| `couriers-onibi` |  | 8 | 2.7 | 8 | 0 | 0 | 0 |
| `couriers-palico-odogaron` |  | 4 | 3.0 | 4 | 0 | 0 | 0 |
| `couriers-pudgling` |  | 8 | 1.9 | 8 | 0 | 0 | 0 |
| `couriers-strongback-the-swift` |  | 8 | 2.0 | 8 | 0 | 1 | 0 |
| `couriers-waldi-the-faithful` |  | 4 | 1.1 | 4 | 0 | 0 | 0 |
| `creep-deny-deny-fall-2021` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-deny-fall-2022` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-deny-spring` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-drow-ranger-deny` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-fuck-deny` |  | 2 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-huh-deny` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-lasthit-golden-shards` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-queen-of-pain-deny` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-razor-deny` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-serega-pirat-deny` |  | 3 | 0.3 | 2 | 0 | 0 | 0 |
| `creep-deny-soul-deny` |  | 2 | 0.0 | 1 | 0 | 0 | 0 |
| `creep-deny-spectre-deny` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `creeps-crownfall-dire-creeps` |  | 6 | 4.4 | 6 | 0 | 0 | 0 |
| `creeps-crownfall-dire-siege` |  | 2 | 0.8 | 2 | 0 | 0 | 0 |
| `creeps-crownfall-radiant-creeps` |  | 6 | 6.4 | 6 | 0 | 0 | 0 |
| `creeps-crownfall-radiant-siege` |  | 3 | 0.7 | 3 | 0 | 0 | 0 |
| `creeps-dark-carnival-dire-creeps` |  | 6 | 5.2 | 6 | 0 | 0 | 0 |
| `creeps-dark-carnival-dire-siege` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `creeps-dark-carnival-radiant-creeps` |  | 6 | 6.2 | 6 | 0 | 0 | 0 |
| `creeps-dark-carnival-radiant-siege` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `creeps-diretide-dire-creeps` |  | 6 | 1.4 | 6 | 0 | 0 | 0 |
| `creeps-diretide-radiant-creeps` |  | 6 | 1.8 | 6 | 0 | 0 | 0 |
| `creeps-nemestice-dire-creeps` |  | 32 | 5.0 | 32 | 0 | 0 | 0 |
| `creeps-nemestice-radiant-creeps` |  | 34 | 18.5 | 34 | 0 | 0 | 0 |
| `creeps-pine-dire-creeps` |  | 26 | 0.8 | 8 | 0 | 0 | 0 |
| `creeps-pine-radiant-creeps` |  | 26 | 0.8 | 8 | 0 | 0 | 0 |
| `creeps-reptilian-dire-creeps` |  | 6 | 3.3 | 6 | 0 | 0 | 0 |
| `creeps-reptilian-radiant-creeps` |  | 6 | 5.0 | 6 | 0 | 0 | 0 |
| `creeps-ussr-radiant-creeps` |  | 71 | 4.3 | 3 | 0 | 0 | 0 |
| `creeps-woodland-dire-creeps` |  | 6 | 2.5 | 6 | 0 | 0 | 0 |
| `creeps-woodland-dire-siege` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `creeps-woodland-radiant-creeps` |  | 6 | 2.4 | 6 | 0 | 0 | 0 |
| `creeps-woodland-radiant-siege` |  | 1 | 0.5 | 1 | 0 | 0 | 0 |
| `heroes-crimson-phantom-lancer` | phantom_lancer | 267 | 9.7 | 1 | 0 | 0 | 0 |
| `heroes-crystal-bristleback` | bristleback | 30 | 4.2 | 0 | 0 | 0 | 0 |
| `heroes-crystal-maiden-cirno` | crystal_maiden | 226 | 20.0 | 1 | 14 | 0 | 0 |
| `heroes-crystal-maiden-tamamo-no-mae` | crystal_maiden | 165 | 33.8 | 5 | 0 | 0 | 0 |
| `heroes-cursed-queen-of-pain` | queen_of_pain, queenofpain | 487 | 25.8 | 0 | 0 | 0 | 0 |
| `heroes-cursed-shadow-fiend` | nevermore, shadow_fiend | 339 | 5.1 | 13 | 0 | 0 | 0 |
| `heroes-curseding-anti-mage` | antimage | 185 | 9.3 | 22 | 0 | 4 | 0 |
| `heroes-custom-largo` | largo, bard | 163 | 92.0 | 26 | 0 | 0 | 0 |
| `heroes-custom-slark` | slark | 79 | 0.7 | 0 | 1 | 0 | 0 |
| `heroes-dark-ancient-slark` | slark, invoker | 672 | 9.7 | 20 | 0 | 0 | 0 |
| `heroes-dark-reaper-invoker` | invoker, invoker_kid | 691 | 8.7 | 20 | 0 | 0 | 0 |
| `heroes-dark-souls-razor` | razor | 127 | 18.1 | 6 | 0 | 0 | 0 |
| `heroes-dark-willow-saccharine-saboteur` | dark_willow | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-darkness-crimson-naga-siren` | siren, naga | 333 | 19.0 | 26 | 0 | 0 | 0 |
| `heroes-darkness-naga-siren` | siren | 182 | 6.3 | 19 | 0 | 0 | 0 |
| `heroes-darkness-weaver` | weaver | 452 | 22.5 | 23 | 0 | 89 | 0 |
| `heroes-dawnbreaker-death-knight` | dawnbreaker | 407 | 59.8 | 26 | 0 | 16 | 0 |
| `heroes-dawnbreaker-moonbreaker` | dawnbreaker | 324 | 25.4 | 27 | 0 | 0 | 0 |
| `heroes-dawnbreaker-starlorn-adjudicator` | dawnbreaker | 7 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-dazzle-darkclaw` | dazzle | 23 | 2.3 | 0 | 2 | 0 | 0 |
| `heroes-dazzle-lifeless-devotion` | dazzle | 231 | 14.8 | 5 | 1 | 0 | 0 |
| `heroes-dazzle-needlepoint-necromance` | dazzle | 5 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-death-prophet-bb` | death_prophet | 408 | 5.9 | 0 | 1 | 0 | 0 |
| `heroes-death-prophet-unicorn` | death_prophet | 547 | 35.3 | 0 | 1 | 0 | 0 |
| `heroes-death-prophet-yuyuko` | death_prophet | 232 | 19.5 | 0 | 14 | 1 | 29 |
| `heroes-defiree-disruptor` | disruptor | 259 | 8.3 | 17 | 0 | 0 | 0 |
| `heroes-devil-legion-commander` | legion_commander | 286 | 29.4 | 3 | 0 | 0 | 0 |
| `heroes-devil-skywrath-mage` | skywrath_mage | 85 | 8.4 | 8 | 0 | 0 | 0 |
| `heroes-disruptor-tandem-storm` | disruptor | 22 | 1.1 | 1 | 0 | 0 | 0 |
| `heroes-doom-fallen-khorne` | doom, doom_bringer | 16 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-doom-jeanne-alter` | doom, doom_bringer | 293 | 2.9 | 0 | 1 | 0 | 0 |
| `heroes-dragon-knight-burning-scale` | dragon_knight | 19 | 2.8 | 0 | 0 | 0 | 0 |
| `heroes-dragon-knight-silverwurm-sacrifice` | dragon_knight | 7 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-drow-ranger-arcana` | drow | 99 | 71.4 | 2 | 0 | 0 | 0 |
| `heroes-drow-ranger-diretide` | drow | 378 | 15.5 | 15 | 0 | 0 | 0 |
| `heroes-drow-ranger-stranger-arcana` | drow | 142 | 76.0 | 16 | 0 | 0 | 0 |
| `heroes-earth-spirit-slitehering-stone` | earth_spirit | 12 | 1.5 | 0 | 0 | 0 | 0 |
| `heroes-earthshaker-deep-magma` | earthshaker | 140 | 24.5 | 4 | 0 | 0 | 0 |
| `heroes-earthshaker-purple-arcana` | earthshaker | 355 | 32.9 | 24 | 0 | 0 | 0 |
| `heroes-earthshaker-red-arcana` | earthshaker | 371 | 27.2 | 14 | 0 | 0 | 0 |
| `heroes-ember-spirit-cinder-sensei` | ember_spirit | 18 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-ember-spirit-green-samurai` | ember_spirit | 233 | 13.3 | 21 | 0 | 0 | 0 |
| `heroes-ember-spirit-master-of-searing-path` | ember_spirit | 17 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-ember-spirit-shana` | ember_spirit | 594 | 45.6 | 0 | 2 | 0 | 0 |
| `emblems-aghanims-emblem` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-bard-frog-emblem` |  | 13 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-bard-frog-emblem-v2` |  | 13 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-battle-pass-2022-emblem` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-crimson-chains-emblem` |  | 8 | 0.0 | 2 | 0 | 0 | 0 |
| `emblems-emblem-darkness` |  | 31 | 0.1 | 1 | 0 | 0 | 0 |
| `emblems-emblem-lsq-black-star` |  | 33 | 0.1 | 3 | 0 | 0 | 0 |
| `emblems-emblem-of-divinity` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-emblem-of-the-crystal-echelon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-emblem-of-the-diretide-blue` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-emblem-of-the-diretide-green` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-emblem-of-the-diretide-red` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-emblem-of-the-diretide-yellow` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-nemestice-emblem` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-overgrown-emblem` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `emblems-sunken-emblem` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `heroes-emerald-dragon-knight` | dragon_knight | 78 | 6.3 | 14 | 0 | 0 | 0 |
| `heroes-emerald-shadow-fiend` | shadow_fiend, nevermore | 613 | 91.7 | 2 | 0 | 0 | 0 |
| `heroes-enchantress-darkwood-witness` | enchantress | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-enchantress-shiro` | enchantress | 206 | 6.6 | 0 | 1 | 0 | 0 |
| `heroes-enigma-angel-of-the-abyss` | enigma | 6 | 4.0 | 0 | 0 | 0 | 0 |
| `heroes-faceless-void-arcana` | faceless_void | 54 | 10.1 | 0 | 2 | 0 | 0 |
| `heroes-faceless-void-esdeath` | faceless_void | 287 | 4.5 | 0 | 1 | 0 | 0 |
| `heroes-faceless-void-rita` | faceless_void | 679 | 49.8 | 13 | 1 | 0 | 0 |
| `heroes-flameling-morphling` | morphling | 959 | 55.4 | 32 | 0 | 14 | 0 |
| `heroes-fog-ember-spirit` | ember_spirit | 212 | 12.2 | 13 | 0 | 0 | 0 |
| `heroes-frostivus-alchemist` | alchemist | 14 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-galactic-io` | wisp | 139 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-ghost-shadow-fiend` | shadow_fiend, nevermore | 712 | 24.3 | 25 | 0 | 14 | 0 |
| `heroes-ghost-void-spirit` | void_spirit | 1021 | 59.0 | 35 | 0 | 14 | 0 |
| `heroes-ghosty-slark` | slark | 477 | 9.0 | 24 | 0 | 15 | 0 |
| `heroes-goffren-broodmother` | broodmother | 4 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-gopo-pudge` | pudge | 57 | 10.5 | 16 | 0 | 0 | 0 |
| `heroes-gopo-tidehunter` | tidehunter | 71 | 20.2 | 12 | 0 | 1 | 0 |
| `heroes-gothic-lifestealer` | life_stealer | 81 | 6.6 | 12 | 0 | 0 | 0 |
| `heroes-green-earth-spirit` | earth_spirit | 241 | 8.1 | 11 | 0 | 0 | 0 |
| `heroes-grimstroke-arong` | grimstroke | 317 | 19.0 | 2 | 1 | 0 | 0 |
| `heroes-grimstroke-gloom-shroud` | grimstroke | 469 | 11.3 | 10 | 0 | 71 | 0 |
| `heroes-gyrocopter-kongou` | gyrocopter, gyro | 262 | 7.4 | 0 | 15 | 0 | 0 |
| `heroes-halloween-treant-protector` | treant_protector | 7 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-harrows-harvest-ringmaster` | ringmaster | 4 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-hell-muerta` | muerta | 400 | 59.3 | 0 | 1 | 0 | 0 |
| `hero-items-alchemist-radiance-blades` | alchemist | 6 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-beastmaster-primal-paean` | beastmaster | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-beastmaster-primal-peacemaker` | beastmaster | 3 | 0.3 | 0 | 0 | 0 | 0 |
| `hero-items-bloodseeker-thirst-of-eztzhok` | bloodseeker | 9 | 0.1 | 2 | 0 | 0 | 0 |
| `hero-items-bounty-hunter-hunters-hoard` | bounty_hunter | 8 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-bounty-hunter-tines-of-tybara` | bounty_hunter | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-cavalcade` | centaur | 5 | 0.4 | 0 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-chieftain` | centaur | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-menace` | centaur | 5 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-crystal-maiden-ice-blossom` | crystal_maiden | 3 | 0.3 | 0 | 0 | 0 | 0 |
| `hero-items-crystal-maiden-yulsarias-glacier` | crystal_maiden | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `hero-items-crystal-maiden-yulsarias-mantle` | crystalmaiden | 6 | 0.3 | 3 | 0 | 0 | 0 |
| `hero-items-dark-seer-forlorn-precipice` | dark_seer | 7 | 0.9 | 0 | 0 | 0 | 0 |
| `hero-items-dazzle-gates-of-nothl` | dazzle | 5 | 0.2 | 0 | 0 | 0 | 0 |
| `hero-items-disruptor-faceless-rex` | disruptor | 10 | 1.5 | 0 | 0 | 0 | 0 |
| `hero-items-earthshaker-carrot` | earthshaker | 16 | 0.7 | 8 | 0 | 0 | 0 |
| `hero-items-faceless-void-battlefury` | faceless_void | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `hero-items-invoker-magus-accord` | invoker | 9 | 0.2 | 1 | 0 | 0 | 0 |
| `hero-items-invoker-magus-apex` | invoker | 10 | 0.2 | 0 | 0 | 0 | 0 |
| `hero-items-kez-guts-sword` | kez | 9 | 1.5 | 3 | 0 | 12 | 0 |
| `hero-items-kez-zangetsu-sword` | kez | 189 | 2.1 | 0 | 0 | 0 | 0 |
| `hero-items-kunkka-shadow-blade` | kunkka | 4 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-kunkka-whale-blade` | kunkka | 5 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-lion-fin-kings-charm` | lion | 6 | 0.4 | 1 | 0 | 0 | 0 |
| `hero-items-lion-hell-spar-anathema` | lion | 8 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-lion-mulctant-pall` | lion | 3 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-lone-druid-cauldron-of-xahryx` | lone_druid | 3 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-lone-druid-claws-of-nuranu` | lone_druid | 4 | 0.3 | 1 | 0 | 0 | 0 |
| `hero-items-luna-moonfall` | luna | 6 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-luna-twilight-schism` | luna | 7 | 0.2 | 0 | 0 | 0 | 0 |
| `hero-items-magnus-eyes-of-ardenok` | magnataur | 4 | 0.3 | 0 | 0 | 0 | 0 |
| `hero-items-magnus-shock-of-the-anvil` | magnataur | 6 | 0.3 | 0 | 0 | 0 | 0 |
| `hero-items-mars-progenitors-bane` | mars | 6 | 0.4 | 0 | 0 | 0 | 0 |
| `hero-items-monkey-king-staff-of-gun-yu` | monkey_king | 7 | 3.4 | 0 | 0 | 0 | 0 |
| `hero-items-phantom-assassin-avowance` | phantom_assassin | 5 | 0.2 | 0 | 0 | 0 | 0 |
| `hero-items-phantom-assassin-codicil` | phantom_assassin | 4 | 0.2 | 0 | 0 | 0 | 0 |
| `hero-items-phantom-lancer-concord-dominion` | phantom_lancer | 3 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-pudge-hood-of-defiance` | pudge | 1 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-shadow-fiend-arms-of-desolation` | nevermore | 6 | 0.0 | 3 | 0 | 0 | 0 |
| `hero-items-shadow-fiend-eternal-harvest` | shadow_fiend | 1 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-sniper-dragon-lore` | sniper | 71 | 2.9 | 8 | 0 | 0 | 0 |
| `hero-items-spirit-breaker-savage-mettle` | spirit_breaker | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-storm-spirit-corridan-maestro` | storm_spirit | 1 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-storm-spirit-lightning-orchid` | stormspirit | 6 | 0.1 | 3 | 0 | 0 | 0 |
| `hero-items-storm-spirit-stormborn-mandate` | stormspirit | 4 | 0.1 | 2 | 0 | 0 | 0 |
| `hero-items-sven-guts-sword` | sven | 57 | 2.5 | 4 | 0 | 0 | 0 |
| `hero-items-tinker-cripple` | tinker | 21 | 16.1 | 0 | 0 | 0 | 0 |
| `hero-items-tiny-banana` | tiny | 7 | 0.1 | 0 | 0 | 0 | 0 |
| `hero-items-undying-hallows-within` | undying | 7 | 0.8 | 0 | 0 | 0 | 0 |
| `hero-items-ursa-swift-claw` | ursa | 3 | 0.0 | 0 | 0 | 0 | 0 |
| `hero-items-wraith-king-winterblight` | skeletonking | 4 | 0.0 | 1 | 0 | 0 | 0 |
| `hero-sounds-axe-rip-your-head` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `hero-sounds-enigma-black-hole-kate` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `hero-sounds-kunkka-sailor-taunt` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `hero-sounds-largo-serega-pirat` |  | 6 | 11.3 | 6 | 0 | 0 | 0 |
| `hero-sounds-largo-serega-pirat-v2` |  | 6 | 17.4 | 6 | 0 | 0 | 0 |
| `hero-sounds-lion-taunt` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `hero-sounds-naga-siren-serega-pirat` |  | 2 | 0.3 | 2 | 0 | 0 | 0 |
| `hero-sounds-nature-prophet-taunt` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `hero-sounds-shadow-shaman-taunt` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `hero-sounds-spirit-breaker-beer` |  | 1 | 0.5 | 1 | 0 | 0 | 0 |
| `hero-sounds-sven-already-red` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `hero-sounds-undying-taunt` |  | 1 | 0.3 | 1 | 0 | 1 | 0 |
| `hero-sounds-ursa-already-red` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `hero-sounds-witch-doctor-death-ward` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `hero-sounds-witch-doctor-true-maledict` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `herofx-enigma-world-chasm-artifact` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `herofx-huskar-white-screen-effect` |  | 5 | 0.0 | 5 | 0 | 0 | 0 |
| `herofx-lifestealer-golden-effects` |  | 16 | 0.1 | 16 | 0 | 0 | 0 |
| `herofx-sniper-ardalan-interdictor` |  | 9 | 0.2 | 9 | 0 | 0 | 0 |
| `herofx-tinker-peashooter-turrets` |  | 27 | 0.3 | 20 | 0 | 0 | 0 |
| `high-five-high-five-aghanim-puppet` |  | 8 | 0.1 | 8 | 0 | 0 | 0 |
| `high-five-high-five-crownfall` |  | 8 | 0.1 | 8 | 0 | 0 | 0 |
| `heroes-hollow-kez` | kez | 689 | 29.4 | 8 | 0 | 30 | 0 |
| `heroes-homelander-invoker` | invoker | 79 | 15.8 | 6 | 0 | 0 | 0 |
| `heroes-hoodwink-bilge-rat-buccaneer` | hoodwink | 5 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-hoodwink-tomokan-footsoldier` | hoodwink | 4 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-huskar-cthugha` | huskar | 254 | 9.5 | 0 | 1 | 0 | 0 |
| `heroes-huskar-flashpoint-proselyte` | huskar | 50 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-ice-chaos-knight` | chaos_knight | 198 | 10.3 | 13 | 0 | 0 | 0 |
| `heroes-ice-phoenix` | phoenix | 333 | 5.9 | 10 | 0 | 0 | 0 |
| `heroes-ice-venomancer` | venomancer | 342 | 7.7 | 13 | 0 | 0 | 0 |
| `heroes-ifrit-monkey-king` | monkey_king | 343 | 9.4 | 13 | 0 | 0 | 0 |
| `heroes-immortal-ancient-apparition` | ancient_apparition | 15 | 0.9 | 0 | 0 | 0 | 0 |
| `heroes-immortal-anti-mage` | antimage | 38 | 2.1 | 1 | 0 | 0 | 0 |
| `heroes-immortal-beastmaster` | beastmaster | 9 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-immortal-hoodwink` | hoodwink | 8 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-immortal-jakiro` | jakiro | 13 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-immortal-lich` | lich | 16 | 4.3 | 0 | 0 | 0 | 0 |
| `heroes-immortal-lina` | lina | 167 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-immortal-medusa` | medusa | 11 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-immortal-necrophos` | necrolyte | 20 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-immortal-phantom-lancer` | phantom_lancer | 16 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-immortal-primal-beast` | primal_beast | 12 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-immortal-puck` | puck | 10 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-immortal-queen-of-pain` | queenofpain | 52 | 2.4 | 13 | 0 | 2 | 0 |
| `heroes-immortal-slardar` | slardar | 13 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-immortal-storm-spirit` | storm_spirit, stormspirit | 20 | 1.8 | 0 | 0 | 0 | 0 |
| `heroes-immortal-templar-assassin` | templar_assassin, lanaya | 22 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-invoker-bloody-wizard` | invoker | 526 | 13.4 | 15 | 0 | 0 | 0 |
| `heroes-invoker-dark-artistry` | invoker | 16 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-invoker-dark-artistry-v2` | invoker | 19 | 0.4 | 1 | 0 | 0 | 0 |
| `heroes-invoker-dark-artistry-v3` | invoker | 55 | 1.3 | 1 | 0 | 0 | 0 |
| `heroes-invoker-dark-divine` | invoker | 931 | 19.6 | 12 | 0 | 0 | 0 |
| `heroes-invoker-depths-covenant` | invoker | 11 | 1.4 | 0 | 0 | 0 | 0 |
| `heroes-invoker-fate-meridian` | invoker | 8 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-invoker-maryday` | invoker | 35 | 0.7 | 2 | 0 | 0 | 0 |
| `heroes-invoker-patchouli` | invoker | 794 | 23.4 | 17 | 2 | 0 | 0 |
| `heroes-io-histoire` | wisp, io | 196 | 11.6 | 0 | 1 | 0 | 0 |
| `heroes-io-madame-scrio` | wisp | 5 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-io-pink-arcana` | wisp | 209 | 11.6 | 20 | 1 | 0 | 0 |
| `heroes-io-purple` | wisp | 97 | 0.4 | 0 | 0 | 0 | 0 |
| `item-effects-aegis-hearts` |  | 14 | 0.1 | 11 | 0 | 0 | 0 |
| `item-effects-blue-dagon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `item-effects-bp-2022-aghanim-aura-i` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-aghanim-aura-ii` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-blink-dagger` |  | 8 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-bp-2022-bottle` |  | 3 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-bp-2022-eul` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-force-staff` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-fountain-regen-i` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-fountain-regen-ii` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-fountain-regen-iii` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-lvl-up` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-mekanism` |  | 5 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-bp-2022-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-bp-2022-phase-boots` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-bp-2022-radiance` |  | 8 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-bp-2022-shiva` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-bp-2022-teleport-i` |  | 8 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-bp-2022-teleport-ii` |  | 8 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-bp-2022-teleport-iii` |  | 8 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-christmas-iron-branch` |  | 21 | 2.4 | 13 | 0 | 0 | 0 |
| `item-effects-custom-runes` |  | 261 | 1.6 | 53 | 0 | 0 | 0 |
| `item-effects-dark-bounty-rune` |  | 26 | 0.4 | 21 | 0 | 0 | 0 |
| `item-effects-darkness-blink-dagger` |  | 24 | 0.1 | 6 | 0 | 0 | 0 |
| `item-effects-darkness-bottle` |  | 15 | 0.1 | 4 | 0 | 0 | 0 |
| `item-effects-darkness-euls-scepter` |  | 13 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-darkness-lvl-up` |  | 24 | 0.2 | 4 | 0 | 0 | 0 |
| `item-effects-darkness-mekanism` |  | 7 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-darkness-mjollnir` |  | 18 | 0.1 | 2 | 0 | 0 | 0 |
| `item-effects-darkness-phase-boots` |  | 10 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-darkness-radiance` |  | 10 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-darkness-shiva` |  | 12 | 0.0 | 1 | 0 | 0 | 0 |
| `item-effects-devotions-of-dragonus-atos` |  | 36 | 0.1 | 36 | 0 | 0 | 0 |
| `item-effects-devotions-of-dragonus-atos-ii` |  | 36 | 0.1 | 36 | 0 | 0 | 0 |
| `item-effects-devotions-of-dragonus-phylactery` |  | 13 | 0.1 | 13 | 0 | 0 | 0 |
| `item-effects-devotions-of-dragonus-phylactery-ii` |  | 13 | 0.0 | 13 | 0 | 0 | 0 |
| `item-effects-earthshaker-blink-dagger` |  | 39 | 2.6 | 37 | 0 | 0 | 0 |
| `item-effects-earthshaker-gold-blink-dagger` |  | 41 | 3.5 | 38 | 0 | 0 | 0 |
| `item-effects-gg-iron-branch` |  | 2 | 0.0 | 1 | 0 | 0 | 0 |
| `item-effects-green-dagon` |  | 14 | 0.0 | 14 | 0 | 0 | 0 |
| `item-effects-phantom-adven-blademail-red` |  | 8 | 0.0 | 8 | 0 | 0 | 0 |
| `item-effects-phantom-adven-blademail-white` |  | 8 | 0.0 | 8 | 0 | 0 | 0 |
| `item-effects-purple-dagon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `item-effects-purple-hex` |  | 15 | 0.2 | 3 | 0 | 0 | 0 |
| `item-effects-red-bottle` |  | 19 | 0.1 | 7 | 0 | 0 | 0 |
| `item-effects-rpg-lvl-up` |  | 22 | 0.3 | 20 | 0 | 0 | 0 |
| `item-effects-snow-dagon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `item-effects-ti10-aghanim-aura` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti10-blink-dagger-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti10-bottle` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti10-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-lvl-up` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-maelstrom-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti10-mekanism` |  | 4 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-ti10-phase-boots` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti10-radiance` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti10-shiva` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti10-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti10-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-blink-dagger-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-bottle` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti6-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-lvl-up` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-maelstrom-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti6-mekanism` |  | 4 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-ti6-phase-boots` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti6-radiance` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti6-teleport-iii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-blink-dagger-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-bottle` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti7-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-lvl-up` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-maelstrom-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti7-mekanism` |  | 4 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-ti7-phase-boots` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti7-radiance` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-shiva` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti7-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti7-teleport-iii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-blink-dagger-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-bottle` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti8-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-lvl-up` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-maelstrom-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti8-mekanism` |  | 4 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-ti8-phase-boots` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti8-radiance` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-shiva` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti8-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti8-teleport-iii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-blink-dagger-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-bottle` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti9-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-lvl-up` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-maelstrom-mjollnir` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti9-mekanism` |  | 4 | 0.0 | 4 | 0 | 0 | 0 |
| `item-effects-ti9-phase-boots` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-ti9-radiance` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-shiva` |  | 2 | 0.0 | 2 | 0 | 0 | 0 |
| `item-effects-ti9-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-ti9-teleport-iii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-white-dagon` |  | 14 | 0.0 | 14 | 0 | 0 | 0 |
| `item-effects-white-ethereal` |  | 30 | 0.1 | 30 | 0 | 0 | 0 |
| `item-effects-winter-2016-blink-dagger` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-winter-2016-euls-scepter` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-winter-2016-force-staff` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-winter-2016-fountain-regen` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-winter-2016-fountain-regen-ii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-winter-2016-fountain-regen-iii` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `item-effects-winter-2016-teleport` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-winter-2016-teleport-ii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-winter-2016-teleport-iii` |  | 6 | 0.0 | 6 | 0 | 0 | 0 |
| `item-effects-winter-hex` |  | 2 | 0.3 | 2 | 0 | 0 | 0 |
| `item-icons-allstars-items` |  | 171 | 4.6 | 164 | 0 | 0 | 0 |
| `item-icons-arcanas-items` |  | 19 | 0.2 | 19 | 0 | 0 | 0 |
| `item-icons-beer-bottle` |  | 15 | 0.2 | 15 | 0 | 0 | 0 |
| `item-icons-dark-items` |  | 228 | 2.4 | 225 | 0 | 0 | 0 |
| `item-icons-meme-media-items` |  | 17 | 0.2 | 17 | 0 | 0 | 0 |
| `item-icons-nyash-items` |  | 169 | 1.7 | 168 | 0 | 0 | 0 |
| `item-icons-purple-icons` |  | 630 | 7.0 | 323 | 0 | 0 | 0 |
| `item-icons-white-items` |  | 602 | 7.2 | 602 | 0 | 0 | 0 |
| `heroes-jakiro-himera` | jakiro | 144 | 8.3 | 0 | 0 | 0 | 0 |
| `heroes-jakiro-kiyohime` | jakiro | 275 | 11.4 | 0 | 1 | 0 | 0 |
| `heroes-joker-ringmaster` | ringmaster, obsidian_destroyer | 277 | 48.1 | 24 | 0 | 0 | 0 |
| `heroes-juggernaut-ancient-exile` | juggernaut | 6 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-juggernaut-arcana-purple` | juggernaut | 565 | 11.8 | 7 | 0 | 0 | 0 |
| `heroes-juggernaut-ice-and-fire` | juggernaut | 591 | 74.9 | 0 | 0 | 0 | 0 |
| `heroes-juggernaut-okita-souji` | juggernaut | 635 | 12.1 | 5 | 1 | 0 | 0 |
| `heroes-juggernaut-samurai-of-wind` | juggernaut | 98 | 1.9 | 8 | 1 | 0 | 0 |
| `heroes-kai-meepo` | meepo | 487 | 24.8 | 16 | 0 | 0 | 0 |
| `heroes-keeper-of-the-light-renegade` | keeper_of_the_light | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-keeper-of-the-light-tamamo` | keeper_of_the_light | 474 | 32.0 | 21 | 1 | 0 | 0 |
| `heroes-kez-bersexer` | kez | 563 | 42.5 | 14 | 0 | 0 | 0 |
| `heroes-kez-captain-anchorbeak` | kez | 55 | 6.3 | 0 | 0 | 0 | 0 |
| `heroes-kez-nanami-yachiyo-kai` | kez | 374 | 7.3 | 0 | 1 | 0 | 0 |
| `heroes-kez-peacock` | kez | 35 | 5.6 | 0 | 0 | 0 | 0 |
| `heroes-kez-tensura` | kez | 1945 | 38.3 | 13 | 1 | 0 | 0 |
| `heroes-legion-commander-crimson-black` | legion_commander | 175 | 7.0 | 11 | 0 | 0 | 0 |
| `heroes-legion-commander-holy-paladin` | legion_commander, legion | 450 | 19.8 | 13 | 0 | 0 | 0 |
| `heroes-legion-commander-nero` | legion_commander | 170 | 4.9 | 0 | 10 | 0 | 0 |
| `heroes-legion-commander-persona` | legion_commander | 547 | 109.9 | 0 | 1 | 0 | 0 |
| `heroes-lich-miyuki-shiba` | lich | 239 | 4.7 | 0 | 8 | 0 | 0 |
| `heroes-lich-shokuhou-misaki` | lich | 753 | 33.0 | 3 | 1 | 0 | 0 |
| `heroes-lifestealer-cong-yu` | lifestealer, life_stealer | 782 | 25.2 | 0 | 1 | 0 | 0 |
| `heroes-lifestealer-diretide-shimmer` | life_stealer | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `heroes-lifestealer-golden-maw` | life_stealer | 47 | 5.4 | 1 | 0 | 0 | 0 |
| `heroes-lina-aerial-aphrodite-flame` | lina | 235 | 26.8 | 10 | 0 | 0 | 0 |
| `heroes-lion-cannonroar-confessor` | lion | 53 | 4.2 | 14 | 0 | 0 | 0 |
| `heroes-lion-immortal-chimera` | lion | 40 | 1.6 | 0 | 0 | 0 | 0 |
| `heroes-lion-witcher` | lion | 290 | 17.2 | 25 | 0 | 0 | 0 |
| `heroes-lollipop-juggernaut` | juggernaut | 58 | 1.4 | 8 | 0 | 0 | 0 |
| `heroes-luna-bloodmoon-stalker` | luna | 34 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-luna-moon-stalker` | luna | 238 | 17.5 | 2 | 0 | 0 | 0 |
| `heroes-luna-winged-sentinel` | luna | 14 | 1.8 | 0 | 0 | 0 | 0 |
| `heroes-lycan-blood-moon` | lycan | 8 | 1.5 | 0 | 0 | 0 | 0 |
| `heroes-lycan-red-wolf-clan` | lycan | 6 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-magnus-kirito` | magnataur | 237 | 6.5 | 0 | 1 | 0 | 0 |
| `heroes-magnus-magnataur` | magnataur | 152 | 13.3 | 15 | 0 | 0 | 0 |
| `heroes-magnus-ornate-cruelty` | magnataur | 5 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-magnus-vortexbull` | magnataur | 279 | 14.8 | 1 | 1 | 0 | 0 |
| `heroes-magnus-white-tiger` | magnataur | 165 | 4.1 | 12 | 0 | 0 | 0 |
| `heroes-maid-marci` | marci | 44 | 40.8 | 0 | 0 | 0 | 0 |
| `heroes-marci-atri` | marci | 127 | 58.9 | 0 | 1 | 0 | 0 |
| `heroes-marci-bloomjewel-brawler` | marci | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-marci-faithful-fortune` | marci | 4 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-marci-laurels-of-the-downstar` | marci | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-marci-school-of-the-solar-fang` | marci | 4 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-mars-diretide-shimmer` | mars | 106 | 1.3 | 0 | 0 | 0 | 0 |
| `heroes-medusa-gilgamesh` | medusa | 340 | 7.0 | 0 | 1 | 0 | 0 |
| `heroes-medusa-gorgon` | medusa | 332 | 37.3 | 10 | 0 | 0 | 0 |
| `heroes-meepo-diretide` | meepo | 12 | 1.6 | 0 | 0 | 0 | 0 |
| `heroes-meepo-lotus-bear` | meepo | 6 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-meepo-mocking-bird` | meepo | 6 | 0.7 | 0 | 0 | 0 | 0 |
| `mega-kill-axe` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-bristleback` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-chitoge-kirisaki` |  | 14 | 0.6 | 14 | 0 | 0 | 0 |
| `mega-kill-clockwerk` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-crystal-maiden` |  | 14 | 0.2 | 14 | 0 | 0 | 0 |
| `mega-kill-dark-willow` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-darkest-dungeon` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-defense-grid` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-deus-ex` |  | 39 | 0.5 | 15 | 0 | 0 | 0 |
| `mega-kill-diretide` |  | 15 | 5.4 | 1 | 0 | 0 | 0 |
| `mega-kill-fallout-4` |  | 14 | 0.2 | 14 | 0 | 0 | 0 |
| `mega-kill-gabe-newell` |  | 6 | 0.3 | 1 | 0 | 0 | 0 |
| `mega-kill-glados` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-hestia` |  | 14 | 0.5 | 14 | 0 | 0 | 0 |
| `mega-kill-iroha-isshiki` |  | 14 | 0.6 | 14 | 0 | 0 | 0 |
| `mega-kill-juggernaut` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-kunkka-tidehunter` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-lina` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-meepo` |  | 19 | 3.3 | 1 | 0 | 0 | 0 |
| `mega-kill-monkey-king` |  | 14 | 0.5 | 14 | 0 | 0 | 0 |
| `mega-kill-natures-prophet` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-nedotrax-mega-kill` |  | 26 | 1.7 | 15 | 0 | 0 | 0 |
| `mega-kill-pirate-cap` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-pyrion-flax` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-rick-morty` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-ru-glados` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-siega-mega-kill` |  | 14 | 0.5 | 14 | 0 | 0 | 0 |
| `mega-kill-stanley-parable` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-storm-spirit` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-stray228-mega-kill` |  | 6 | 0.4 | 6 | 0 | 0 | 0 |
| `mega-kill-techies` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-trine` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `mega-kill-umbrella-mega-kill` |  | 25 | 0.9 | 14 | 0 | 0 | 0 |
| `mega-kill-voice-of-the-international` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `heroes-monkey-king-arcana` | monkey_king | 6 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-monkey-king-simian-sovereign` | monkey_king | 4 | 2.4 | 0 | 0 | 0 | 0 |
| `heroes-monkey-king-water-demon` | monkey_king | 242 | 8.8 | 5 | 0 | 0 | 0 |
| `heroes-morphling-abyss-revenge` | morphling | 13 | 3.1 | 2 | 0 | 0 | 0 |
| `heroes-morphling-automaton` | morphling | 29 | 19.2 | 0 | 1 | 0 | 0 |
| `heroes-morphling-darktrench-purple` | morphling | 245 | 11.4 | 12 | 0 | 0 | 0 |
| `heroes-morphling-darktrench-tears` | morphling | 13 | 2.2 | 0 | 0 | 0 | 0 |
| `heroes-morphling-fluid-frenzy` | morphling | 45 | 9.9 | 0 | 0 | 0 | 0 |
| `heroes-morphling-megalodon` | morphling | 176 | 16.1 | 14 | 0 | 0 | 0 |
| `heroes-muerta-blackwing-bandolera` | muerta | 5 | 1.6 | 0 | 0 | 0 | 0 |
| `heroes-muerta-soul-warden` | muerta | 5 | 1.3 | 0 | 0 | 0 | 0 |
| `music-deadmau5-dieback` |  | 45 | 87.8 | 42 | 0 | 0 | 0 |
| `music-dire-shred` |  | 40 | 57.5 | 39 | 0 | 0 | 0 |
| `music-elemental-fury` |  | 45 | 57.4 | 42 | 0 | 0 | 0 |
| `music-heroes-within` |  | 47 | 64.3 | 42 | 0 | 0 | 0 |
| `music-humanity-last-breath` |  | 45 | 48.6 | 42 | 0 | 0 | 0 |
| `music-jj-lin-timekeeper` |  | 45 | 61.9 | 42 | 0 | 0 | 0 |
| `music-northern-winds` |  | 47 | 80.6 | 42 | 0 | 0 | 0 |
| `music-the-international-10` |  | 34 | 34.1 | 34 | 0 | 0 | 0 |
| `music-the-international-2014` |  | 54 | 68.2 | 42 | 0 | 0 | 0 |
| `music-the-international-2015` |  | 54 | 68.9 | 42 | 0 | 0 | 0 |
| `music-the-international-2016` |  | 54 | 81.5 | 42 | 0 | 0 | 0 |
| `music-the-international-2017` |  | 54 | 73.9 | 42 | 0 | 0 | 0 |
| `music-the-international-2019` |  | 48 | 42.5 | 42 | 0 | 0 | 0 |
| `music-thefatrat-warrior` |  | 45 | 51.8 | 42 | 0 | 0 | 0 |
| `heroes-naga-siren-allure-of-the-deep` | siren | 7 | 1.4 | 0 | 0 | 0 | 0 |
| `heroes-naga-siren-izayoi-miku` | siren | 287 | 12.6 | 10 | 1 | 0 | 0 |
| `heroes-natures-prophet-allfather` | furion | 8 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-natures-prophet-saya` | furion | 494 | 31.4 | 0 | 1 | 0 | 0 |
| `heroes-natures-prophet-spectral-sage` | furion | 193 | 13.8 | 31 | 0 | 0 | 0 |
| `heroes-necrophos-eucliwood-halsace` | necrolyte | 805 | 18.5 | 0 | 1 | 0 | 0 |
| `heroes-necrophos-serpent-spite` | necrolyte | 5 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-night-stalker-dark-oath` | nightstalker, night_stalker | 18 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-night-stalker-feasts-of-forever` | nightstalker | 16 | 4.8 | 0 | 0 | 0 | 0 |
| `heroes-night-stalker-frost-walker` | nightstalker | 96 | 6.6 | 17 | 0 | 0 | 0 |
| `heroes-night-stalker-relict` | nightstalker | 118 | 51.6 | 7 | 0 | 0 | 0 |
| `heroes-night-stalker-remilia` | nightstalker | 174 | 13.6 | 4 | 12 | 0 | 0 |
| `heroes-night-ursa` | ursa | 144 | 7.1 | 16 | 0 | 0 | 0 |
| `heroes-nightmare-chaos-knight` | chaos_knight | 480 | 26.7 | 23 | 0 | 14 | 0 |
| `heroes-nuclear-clockwerk` | rattletrap, clockwerk | 110 | 4.3 | 14 | 0 | 0 | 0 |
| `heroes-nude-drow-ranger` | drow | 64 | 7.2 | 6 | 0 | 0 | 0 |
| `heroes-nude-marci` | marci | 292 | 207.3 | 0 | 1 | 0 | 0 |
| `heroes-nude-naga-siren` | siren | 15 | 1.0 | 7 | 0 | 0 | 0 |
| `heroes-nude-queen-of-pain` | queenofpain, queen_of_pain | 160 | 27.3 | 0 | 0 | 0 | 0 |
| `heroes-ogre-magi-amakusa-shiro` | ogre_magi | 269 | 4.0 | 0 | 1 | 0 | 0 |
| `heroes-ogre-magi-custom-arcana` | ogre_magi | 108 | 11.4 | 16 | 1 | 0 | 0 |
| `heroes-ogre-magi-freeboot-fortunes` | ogre_magi | 6 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-ogre-magi-pyrexae-polymorph` | ogre_magi | 6 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-old-shadow-fiend` | shadow_fiend | 82 | 1.8 | 13 | 1 | 0 | 0 |
| `heroes-old-storm-spirit` | storm_spirit, stormspirit | 120 | 1.7 | 0 | 11 | 0 | 0 |
| `heroes-omniknight-akashi` | omniknight | 289 | 44.7 | 10 | 1 | 0 | 0 |
| `heroes-omniknight-blinding-bastion` | omniknight | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `optimization-default-wards` |  | 113 | 3.3 | 113 | 0 | 0 | 0 |
| `optimization-tinker-lite-machines` |  | 12 | 0.0 | 10 | 0 | 0 | 0 |
| `heroes-oracle-immortal-silence` | oracle | 18 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-oracle-irisviel-von-einzbern` | oracle | 730 | 124.9 | 8 | 1 | 0 | 0 |
| `heroes-outworld-destroyer-dark-metter` | obsidian_destroyer | 229 | 8.1 | 23 | 0 | 0 | 0 |
| `heroes-outworld-destroyer-yukari` | obsidian_destroyer, outworld_devourer | 364 | 34.9 | 1 | 18 | 0 | 0 |
| `heroes-pangolier-gentleman` | pangolier, antimage | 200 | 11.9 | 13 | 0 | 0 | 0 |
| `heroes-phantom-assassin-arcana` | phantom_assassin | 426 | 5.9 | 0 | 6 | 0 | 0 |
| `heroes-phantom-assassin-persona` | phantom_assassin | 113 | 46.0 | 0 | 0 | 0 | 0 |
| `heroes-phantom-assassin-rory` | phantom_assassin | 469 | 5.9 | 0 | 14 | 0 | 0 |
| `heroes-phantom-assassin-skull-arcana` | phantom_assassin | 319 | 16.2 | 17 | 0 | 0 | 0 |
| `heroes-phantom-lancer-crimson-v2` | phantom_lancer | 268 | 9.7 | 1 | 0 | 0 | 0 |
| `heroes-phantom-lancer-dark-carnival` | phantom_lancer | 5 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-phantom-lancer-humble-drifter` | phantom_lancer | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `heroes-phantom-lancer-immortal-avilliva` | phantom_lancer | 10 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-phantom-lancer-immortal-felidae` | phantom_lancer | 21 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-phantom-lancer-purple-avilliva` | phantom_lancer | 9 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-phantom-lancer-sun-warrior` | phantom_lancer | 7 | 0.1 | 0 | 0 | 0 | 0 |
| `heroes-phite-phantom-assassin` | phantom_assassin | 287 | 9.1 | 23 | 0 | 0 | 0 |
| `pings-skeleton-black-ping` |  | 15 | 0.0 | 15 | 0 | 0 | 0 |
| `pings-skeleton-blue-ping` |  | 12 | 0.0 | 12 | 0 | 0 | 0 |
| `pings-skeleton-purple-ping` |  | 15 | 0.0 | 15 | 0 | 0 | 0 |
| `heroes-pink-nyx-assassin` | nyx_assassin, nerubian_assassin | 418 | 8.1 | 29 | 0 | 0 | 0 |
| `heroes-pink-shadow-fiend` | shadow_fiend, nevermore | 719 | 36.6 | 16 | 0 | 1 | 0 |
| `heroes-pinkie-sven` | sven | 239 | 6.4 | 18 | 0 | 14 | 0 |
| `heroes-primal-beast-dark-behemoth` | primal_beast | 12 | 9.6 | 0 | 0 | 0 | 0 |
| `heroes-primal-beast-prehistoric-predator` | primal_beast | 39 | 16.2 | 0 | 0 | 0 | 0 |
| `heroes-primal-beast-snowbeast` | primal_beast | 33 | 13.2 | 14 | 0 | 0 | 0 |
| `heroes-pudge-arcana-elephant` | pudge | 290 | 38.2 | 13 | 0 | 0 | 0 |
| `heroes-pudge-candy-cat` | pudge | 7 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-pudge-clown-toy` | pudge | 416 | 11.6 | 9 | 1 | 0 | 0 |
| `heroes-pudge-cm` | pudge | 17 | 0.2 | 6 | 0 | 0 | 0 |
| `heroes-pudge-iron-hog` | pudge | 7 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-pudge-mindless-slaughter` | pudge | 7 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-pudge-rotzo-the-clown` | pudge | 11 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-pudge-toy-butcher` | pudge | 173 | 51.7 | 0 | 0 | 0 | 0 |
| `heroes-pugna-nightwither-revenant` | pugna | 468 | 8.8 | 3 | 0 | 0 | 0 |
| `heroes-purple-kez` | kez | 531 | 30.1 | 18 | 0 | 15 | 0 |
| `heroes-purple-legion-commander` | legion, legion_commander | 322 | 11.5 | 24 | 0 | 0 | 0 |
| `heroes-purple-queen-of-pain` | queen_of_pain, queenofpain | 692 | 37.5 | 21 | 0 | 0 | 0 |
| `heroes-purple-witch-broodmother` | broodmother | 170 | 9.7 | 24 | 0 | 0 | 0 |
| `heroes-queen-of-pain-rose` | queen_of_pain, queenofpain | 1112 | 72.4 | 26 | 0 | 14 | 0 |
| `ranged-attack-aghanim-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-blue-diretide-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-green-diretide-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-nemestice-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-red-diretide-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-ti10-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-ti12-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-ti9-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranged-attack-yellow-diretide-ranged-attack` |  | 311 | 0.6 | 152 | 0 | 0 | 0 |
| `ranks-azur-lane-medals` |  | 20 | 5.0 | 20 | 0 | 0 | 0 |
| `ranks-bear-medals` |  | 13 | 0.8 | 13 | 0 | 0 | 0 |
| `ranks-flaming-medals` |  | 13 | 0.9 | 13 | 0 | 0 | 0 |
| `ranks-imperial-medals` |  | 13 | 2.3 | 13 | 0 | 0 | 0 |
| `ranks-normis-rank-icons` |  | 20 | 0.6 | 20 | 0 | 0 | 0 |
| `ranks-slates-of-the-damned` |  | 20 | 1.3 | 20 | 0 | 0 | 0 |
| `ranks-umbrella-medals` |  | 13 | 1.4 | 13 | 0 | 0 | 0 |
| `ranks-void-medals` |  | 20 | 0.7 | 20 | 0 | 0 | 0 |
| `heroes-razor-rainbow-statics` | razor | 150 | 3.4 | 14 | 0 | 0 | 0 |
| `heroes-razor-rizelencer` | razor | 96 | 1.2 | 12 | 0 | 0 | 0 |
| `heroes-razor-silver-rrevenant` | razor | 8 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-red-abaddon` | abaddon | 148 | 3.6 | 21 | 0 | 0 | 0 |
| `heroes-red-monkey-king` | monkey_king | 445 | 14.3 | 0 | 1 | 0 | 0 |
| `heroes-riki-hunger-howling-wilds` | riki, rikimaru | 46 | 0.6 | 0 | 0 | 0 | 0 |
| `river-black-river` |  | 29 | 0.9 | 22 | 0 | 0 | 0 |
| `river-blood-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-blue-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-boiling-blood-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-boiling-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-chrome-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-cola-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-dry-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-electric-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-frozen-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-gold-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-green-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-oil-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-orange-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-pink-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-potion-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-purple-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-red-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-slime-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `river-yellow-river` |  | 18 | 0.1 | 18 | 0 | 0 | 0 |
| `roshan-aghanims-roshan` |  | 1 | 0.9 | 1 | 0 | 0 | 0 |
| `roshan-cosmic-roshan` |  | 1 | 0.9 | 1 | 0 | 0 | 0 |
| `roshan-desert-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `roshan-gardens-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `roshan-golden-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `roshan-greevil-grief-roshan` |  | 1 | 1.2 | 1 | 0 | 0 | 0 |
| `roshan-journey-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `roshan-reef-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `roshan-santa-hat-roshan` |  | 1 | 0.8 | 1 | 0 | 0 | 0 |
| `roshan-winter-roshan` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `heroes-rubick-ancient-magus` | rubick | 20 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-rubick-dark-mage` | rubick | 180 | 2.4 | 4 | 0 | 0 | 0 |
| `heroes-rubick-plagueroad-apothacary` | rubick | 53 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-russian-ursa` | ursa | 58 | 8.7 | 7 | 0 | 0 | 0 |
| `heroes-sakura-pink-invoker` | invoker | 740 | 25.7 | 29 | 0 | 4 | 0 |
| `heroes-sand-king-black-sands` | sand_king | 8 | 0.5 | 2 | 0 | 0 | 0 |
| `heroes-scarlet-keeper-of-the-light` | keeper_of_the_light | 996 | 55.2 | 27 | 0 | 14 | 0 |
| `heroes-sf-broodmother` | broodmother | 83 | 0.4 | 0 | 0 | 0 | 0 |
| `shaders-aghanim-green` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-green-lite` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-labyrinth` |  | 1 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-peach` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-purple` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-purple-lite` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-red` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-red-lite` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-white` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-white-lite` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-aghanim-yellow-lite` |  | 3 | 1.0 | 1 | 0 | 0 | 0 |
| `shaders-ancient` |  | 4 | 0.6 | 1 | 0 | 0 | 0 |
| `shaders-cartoon-shader` |  | 4 | 0.0 | 3 | 0 | 0 | 0 |
| `shaders-diretide-outline-` |  | 3 | 0.0 | 3 | 0 | 0 | 0 |
| `shaders-haze` |  | 6 | 0.8 | 1 | 0 | 0 | 0 |
| `shaders-purple-ancient` |  | 4 | 0.6 | 1 | 0 | 0 | 0 |
| `shaders-purple-outline` |  | 2 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-simple-minify-shader` |  | 3 | 0.0 | 2 | 0 | 0 | 0 |
| `shaders-wave-green` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-aqua` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-blue` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-green` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-peach` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-pink` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-purple` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-red` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-white` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-lite-yellow` |  | 5 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-peach` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-purple` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-red` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `shaders-wave-white` |  | 3 | 0.0 | 1 | 0 | 0 | 0 |
| `heroes-shadow-fiend-1000-7` | shadow_fiend, nevermore | 202 | 2.7 | 0 | 0 | 2 | 0 |
| `heroes-shadow-fiend-arcana` | nevermore, shadow_fiend | 30 | 5.8 | 0 | 0 | 0 | 0 |
| `heroes-shadow-fiend-black` | shadow_fiend, nevermore | 366 | 18.3 | 13 | 0 | 1 | 0 |
| `heroes-shadow-fiend-desolation` | shadow_fiend | 3 | 0.1 | 0 | 0 | 0 | 0 |
| `heroes-shadow-fiend-eternal-harvest` | shadow_fiend | 3 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-shadow-fiend-king-of-vipers` | shadow_fiend | 3 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-shadow-fiend-souls-tyrant` | shadow_fiend | 3 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-shadow-fiend-sundered-souls` | shadow_fiend, nevermore | 7 | 0.5 | 0 | 0 | 1 | 0 |
| `heroes-shadow-fiend-trenbolone` | nevermore, shadow_fiend | 33 | 4.1 | 2 | 0 | 0 | 0 |
| `heroes-shadow-fiend-white` | shadow_fiend, nevermore | 423 | 8.7 | 9 | 0 | 0 | 0 |
| `heroes-shadow-shaman-immortal-crimson` | shadowshaman, shadow_shaman | 21 | 1.0 | 2 | 0 | 0 | 0 |
| `heroes-shadow-shaman-purple` | shadowshaman | 32 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-shadow-timbersaw` | shredder | 122 | 2.8 | 5 | 0 | 0 | 0 |
| `heroes-shaolin-master-oracle` | oracle | 209 | 9.6 | 24 | 0 | 0 | 0 |
| `heroes-sherman-crystal-maiden` | crystal_maiden, crystalmaiden | 344 | 17.9 | 15 | 0 | 0 | 0 |
| `heroes-silencer-whispered-wing` | silencer | 6 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-skywrath-mage-arcana` | skywrath_mage | 90 | 44.5 | 0 | 0 | 0 | 0 |
| `heroes-skywrath-mage-thanatos` | skywrath_mage | 232 | 19.1 | 17 | 0 | 0 | 0 |
| `heroes-slardar-naga-guardian` | slardar | 281 | 18.7 | 0 | 0 | 0 | 0 |
| `heroes-slark-barracuda-re` | slark | 447 | 21.1 | 0 | 0 | 0 | 0 |
| `heroes-slark-shadow-in-the-deep` | slark | 9 | 0.3 | 0 | 0 | 0 | 0 |
| `heroes-snapfire-desert-ardalan` | snapfire | 6 | 0.9 | 0 | 0 | 0 | 0 |
| `heroes-snapfire-whippersnapper` | snapfire | 5 | 0.8 | 0 | 0 | 0 | 0 |
| `heroes-sniper-ardalan-interdictor` | sniper | 277 | 9.0 | 0 | 0 | 0 | 0 |
| `heroes-sniper-howling-wolf` | sniper | 15 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-sniper-odogaron-armor` | sniper | 11 | 2.1 | 0 | 0 | 0 | 0 |
| `heroes-snowing-marci` | marci | 341 | 7.3 | 4 | 0 | 0 | 0 |
| `sounds-dark-carnival-dora` |  | 3 | 1.1 | 3 | 0 | 0 | 0 |
| `sounds-dark-carnival-serega-pirat` |  | 3 | 2.1 | 3 | 0 | 0 | 0 |
| `sounds-dark-carnival-team-spirit` |  | 3 | 0.7 | 3 | 0 | 0 | 0 |
| `sounds-rezero-respawn-sound` |  | 16 | 7.9 | 16 | 0 | 0 | 0 |
| `sounds-rezero-subaru-phone-horn` |  | 2 | 1.6 | 2 | 0 | 0 | 0 |
| `sounds-sora-main-menu` |  | 7 | 14.8 | 7 | 0 | 0 | 0 |
| `sounds-stairs-guy-main-menu` |  | 7 | 12.5 | 7 | 0 | 0 | 0 |
| `sounds-ussr-and-germany-horn` |  | 2 | 9.6 | 2 | 0 | 0 | 0 |
| `heroes-spectre-arcana` | spectre | 37 | 5.5 | 0 | 0 | 0 | 0 |
| `heroes-spectre-phantom-advent` | spectre | 73 | 10.5 | 17 | 0 | 0 | 0 |
| `heroes-spectre-shadowveil` | spectre | 21 | 0.9 | 0 | 0 | 0 | 0 |
| `heroes-spectre-toxin` | spectre | 762 | 44.2 | 28 | 0 | 14 | 0 |
| `heroes-spirit-breaker-crystal-colossus` | spirit_breaker | 9 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-spirit-breaker-elemental-imperator` | spirit_breaker | 9 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-spirit-breaker-ghost-monster` | spirit_breaker | 180 | 10.0 | 13 | 1 | 1 | 0 |
| `heroes-steel-sand-king` | sandking, sand_king | 103 | 2.7 | 16 | 0 | 0 | 0 |
| `heroes-storm-spirit-beast-of-thunder` | storm_spirit | 5 | 0.4 | 2 | 0 | 0 | 0 |
| `heroes-storm-spirit-corridan-maestro` | storm_spirit | 5 | 0.3 | 2 | 0 | 0 | 0 |
| `heroes-sven-berseker` | sven | 196 | 6.5 | 9 | 0 | 0 | 0 |
| `heroes-sven-blue-angel` | sven | 315 | 27.1 | 16 | 0 | 0 | 0 |
| `heroes-sven-indomitable-legacy` | sven | 22 | 0.5 | 0 | 0 | 0 | 0 |
| `heroes-sven-yatogami-tohka` | winter_wyvern, sven | 563 | 37.2 | 0 | 21 | 0 | 0 |
| `heroes-sven-yatogami-tohka-v2` | sven | 321 | 5.6 | 0 | 20 | 0 | 0 |
| `heroes-techies-alien` | techies | 42 | 5.9 | 0 | 0 | 0 | 0 |
| `heroes-techies-arab` | techies | 17 | 5.0 | 0 | 0 | 0 | 0 |
| `heroes-techies-sodbustin-saboteurs` | techies | 5 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-techies-war-rig-pachyderm` | techies | 9 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-templar-assassin-playboy` | templar_assassin, lanaya | 180 | 13.3 | 4 | 0 | 1 | 0 |
| `heroes-templar-assassin-royal-squire` | templar_assassin, lanaya | 28 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-templar-assassin-whisper-gates` | lanaya | 29 | 4.4 | 0 | 0 | 0 | 0 |
| `terrains-bedrock` |  | 968 | 26.8 | 892 | 0 | 0 | 0 |
| `terrains-dark-terrain-minify` |  | 981 | 32.8 | 896 | 0 | 0 | 0 |
| `terrains-mossy-cobblestone` |  | 966 | 26.8 | 890 | 0 | 0 | 0 |
| `terrains-simple-dark-terrain-minify` |  | 1010 | 33.5 | 920 | 0 | 0 | 0 |
| `heroes-terrorblade-666-fractal-horns` | terrorblade | 84 | 1.0 | 5 | 0 | 0 | 0 |
| `heroes-terrorblade-crimson-fractal-horns` | terrorblade | 183 | 1.8 | 0 | 0 | 0 | 0 |
| `heroes-terrorblade-green-fractal-horns` | terrorblade | 185 | 3.7 | 1 | 0 | 0 | 0 |
| `heroes-terrorblade-immortal-arcana` | terrorblade | 19 | 1.2 | 0 | 0 | 1 | 0 |
| `heroes-terrorblade-unblinking-horror` | terrorblade | 179 | 20.7 | 23 | 0 | 0 | 0 |
| `heroes-terrorblade-white-fractal-horns` | terrorblade | 212 | 1.7 | 0 | 0 | 0 | 0 |
| `heroes-terrorblade-wrong-arcana` | terrorblade | 483 | 34.7 | 8 | 0 | 0 | 0 |
| `ti-bp-effects-aghanims-labyrinth-battle-pass` |  | 77 | 0.2 | 64 | 0 | 0 | 0 |
| `ti-bp-effects-battle-pass-2022` |  | 86 | 0.2 | 72 | 0 | 0 | 0 |
| `ti-bp-effects-darkness-pack` |  | 181 | 0.8 | 56 | 0 | 0 | 0 |
| `ti-bp-effects-nemestice-2021-battle-pass` |  | 78 | 0.2 | 65 | 0 | 0 | 0 |
| `ti-bp-effects-purple-quarteros-curios` |  | 240 | 1.0 | 225 | 0 | 1 | 0 |
| `ti-bp-effects-quarteros-curios` |  | 84 | 0.4 | 69 | 0 | 0 | 0 |
| `ti-bp-effects-remove-seasonal-effects` |  | 54 | 0.1 | 54 | 0 | 0 | 0 |
| `ti-bp-effects-spring-quarteros-curios` |  | 197 | 0.9 | 184 | 0 | 0 | 0 |
| `ti-bp-effects-summer-quarteros-curios` |  | 39 | 0.2 | 26 | 0 | 0 | 0 |
| `ti-bp-effects-ti-2010-battle-pass` |  | 86 | 0.2 | 72 | 0 | 0 | 0 |
| `ti-bp-effects-ti-2016-battle-pass` |  | 80 | 0.2 | 68 | 0 | 0 | 0 |
| `ti-bp-effects-ti-2017-battle-pass` |  | 88 | 0.2 | 73 | 0 | 0 | 0 |
| `ti-bp-effects-ti-2018-battle-pass` |  | 87 | 0.2 | 74 | 0 | 0 | 0 |
| `ti-bp-effects-ti-2019-battle-pass` |  | 85 | 0.2 | 72 | 0 | 0 | 0 |
| `ti-bp-effects-winter-2016-battle-pass` |  | 68 | 0.1 | 60 | 0 | 0 | 0 |
| `ti-bp-effects-winter-quarteros-curios` |  | 52 | 0.2 | 39 | 0 | 0 | 0 |
| `heroes-tidehunter-insatiable-seas` | tidehunter | 33 | 8.8 | 0 | 0 | 0 | 0 |
| `heroes-tidehunter-squid-girl` | tidehunter | 322 | 11.2 | 5 | 1 | 0 | 0 |
| `heroes-timbersaw-clearcut-cavalier` | shredder | 134 | 2.9 | 0 | 0 | 0 | 0 |
| `heroes-tinker-deathsman` | tinker | 30 | 4.2 | 0 | 0 | 0 | 0 |
| `heroes-tinker-deep-hornet` | tinker | 18 | 0.9 | 3 | 0 | 0 | 0 |
| `heroes-tinker-fortified-fabricator` | tinker | 6 | 0.7 | 0 | 0 | 0 | 0 |
| `heroes-tinker-sakuya` | tinker | 138 | 15.4 | 0 | 13 | 4 | 0 |
| `heroes-tinker-submerged-hazard` | tinker | 31 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-tiny-majesty-of-the-colossus` | tiny | 32 | 3.1 | 1 | 0 | 0 | 0 |
| `heroes-tiny-stone-giant-stronghold` | tiny | 6 | 3.3 | 1 | 0 | 0 | 0 |
| `towers-holiday-radiant-tower` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `heroes-toxicpunk-timbersaw` | shredder, timbersaw | 155 | 4.3 | 16 | 0 | 0 | 0 |
| `heroes-treant-protector-frost-hideaway` | treant_protector | 4 | 0.4 | 0 | 0 | 0 | 0 |
| `trees-catbuns-trees` |  | 21 | 11.0 | 15 | 0 | 0 | 0 |
| `trees-cone-trees` |  | 18 | 0.5 | 15 | 0 | 0 | 0 |
| `trees-crystals-trees` |  | 273 | 1.4 | 257 | 0 | 0 | 0 |
| `trees-gravestones-trees` |  | 23 | 8.4 | 16 | 0 | 0 | 0 |
| `trees-logs-trees` |  | 21 | 5.4 | 16 | 0 | 0 | 0 |
| `trees-lowpoly-barrel-trees` |  | 18 | 1.0 | 15 | 0 | 0 | 0 |
| `trees-lowpoly-shrub-trees` |  | 34 | 0.2 | 29 | 0 | 0 | 0 |
| `trees-lowpoly-skull-trees` |  | 19 | 4.3 | 16 | 0 | 0 | 0 |
| `trees-lowpoly-stumps-trees` |  | 19 | 0.9 | 16 | 0 | 0 | 0 |
| `trees-lowpoly-trees` |  | 70 | 9.3 | 15 | 0 | 0 | 0 |
| `trees-lowpoly-trees-v1` |  | 25 | 0.7 | 16 | 0 | 0 | 0 |
| `trees-metal-barrel-trees` |  | 269 | 44.1 | 242 | 0 | 0 | 0 |
| `trees-pine-cone-trees` |  | 18 | 3.2 | 15 | 0 | 0 | 0 |
| `trees-pumpkin-trees` |  | 336 | 6.0 | 258 | 0 | 0 | 0 |
| `trees-small-trees-minify` |  | 21 | 0.6 | 16 | 0 | 0 | 0 |
| `trees-stone-trees` |  | 273 | 3.1 | 257 | 0 | 0 | 0 |
| `trees-wooden-trees` |  | 278 | 4.4 | 261 | 0 | 0 | 0 |
| `heroes-troll-warlord-reimu` | troll_warlord | 178 | 20.6 | 0 | 11 | 0 | 0 |
| `heroes-tusk-immortal` | tuskarr | 9 | 0.3 | 3 | 0 | 0 | 0 |
| `heroes-tusk-king-kong` | tusk, tuskarr | 204 | 6.8 | 14 | 0 | 0 | 1 |
| `heroes-tusk-odobenus-footman` | tusk, tuskarr | 145 | 13.2 | 33 | 0 | 0 | 0 |
| `heroes-tusk-odobenus-frost-mage` | tusk, tuskarr | 200 | 17.8 | 36 | 0 | 0 | 0 |
| `heroes-underlord-immortal-ravenous` | underlord, abyssal_underlord | 21 | 0.8 | 8 | 0 | 0 | 0 |
| `heroes-undying-archange` | undying | 223 | 13.5 | 13 | 0 | 0 | 0 |
| `heroes-undying-grim-reformation` | undying | 6 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-ursa-alpine-stalker` | ursa | 4 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-ursa-crystal-drift` | ursa | 4 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-ursa-feast-of-the-damned` | ursa | 4 | 0.2 | 0 | 0 | 0 | 0 |
| `heroes-ursa-immortal-hallowed-hunt` | ursa | 10 | 1.0 | 0 | 0 | 0 | 0 |
| `heroes-ursa-roshan-hunter` | ursa | 4 | 0.1 | 0 | 0 | 0 | 0 |
| `heroes-ursa-white-and-red` | ursa | 284 | 7.3 | 11 | 0 | 1 | 0 |
| `heroes-vengeful-spirit-seraph-scion` | vengeful | 25 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-venomancer-peashooter` | venomancer | 40 | 8.3 | 0 | 0 | 0 | 0 |
| `heroes-venomancer-poisonous-assassin` | venomancer | 5 | 0.6 | 0 | 0 | 0 | 0 |
| `heroes-viper-butch-dog` | viper | 209 | 20.5 | 27 | 0 | 0 | 0 |
| `heroes-viper-elephant-hawk-moth` | viper | 28 | 3.4 | 12 | 0 | 0 | 0 |
| `heroes-viper-stuffed-menace` | viper | 18 | 3.5 | 0 | 0 | 0 | 0 |
| `heroes-visage-azure-phantom` | visage | 199 | 9.9 | 0 | 1 | 1 | 0 |
| `heroes-visage-grimfeather-orpse` | visage | 115 | 2.2 | 0 | 0 | 1 | 0 |
| `heroes-void-morphling` | morphling | 810 | 25.1 | 19 | 0 | 0 | 0 |
| `heroes-void-spirit-hidden-vector` | void_spirit | 83 | 1.5 | 0 | 0 | 0 | 0 |
| `heroes-void-spirit-keqing` | void_spirit | 851 | 27.8 | 1 | 2 | 0 | 0 |
| `heroes-warden-phantom-assassin` | phantom_assassin | 534 | 17.3 | 6 | 8 | 0 | 0 |
| `wards-curious-snaptrap` |  | 1 | 0.4 | 1 | 0 | 0 | 0 |
| `wards-floes-tower` |  | 1 | 0.3 | 1 | 0 | 0 | 0 |
| `wards-monty` |  | 1 | 0.1 | 1 | 0 | 0 | 0 |
| `wards-portal` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `wards-trailgazer` |  | 1 | 0.2 | 1 | 0 | 0 | 0 |
| `heroes-warlock-puppet-bear` | warlock | 16 | 1.1 | 0 | 0 | 0 | 0 |
| `heroes-weaver-kurumi` | weaver | 328 | 12.5 | 3 | 10 | 0 | 0 |
| `heroes-white-natures-prophet` | furion | 278 | 3.4 | 6 | 0 | 0 | 0 |
| `heroes-white-shadow-shaman` | shadowshaman | 137 | 2.4 | 14 | 0 | 0 | 0 |
| `heroes-windranger-autumn-arcana` | windrunner | 162 | 31.4 | 24 | 0 | 0 | 0 |
| `heroes-winter-ember-spirit` | ember_spirit | 711 | 29.9 | 26 | 0 | 0 | 0 |
| `heroes-winter-lycan` | lycan | 287 | 25.3 | 29 | 0 | 0 | 0 |
| `heroes-wraith-king-arcana` | wraith_king, skeletonking | 336 | 12.6 | 6 | 2 | 0 | 0 |
| `heroes-wraith-king-grim-destiny` | wraith_king | 6 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-wraith-king-itsuka-kotori` | wraith_king, skeletonking | 449 | 7.0 | 5 | 16 | 0 | 0 |
| `heroes-wraith-king-pizdec` | wraith_king, skeletonking | 32 | 1.4 | 3 | 0 | 0 | 0 |
| `heroes-wraith-king-skeleton` | skeletonking, wraith_king | 251 | 3.0 | 6 | 0 | 0 | 0 |
| `heroes-wraith-king-unbroken-arcana` | wraith_king, skeletonking | 40 | 9.3 | 5 | 0 | 0 | 0 |
| `heroes-wraith-king-veil-tyrant` | wraith_king | 6 | 0.4 | 0 | 0 | 0 | 0 |
| `heroes-young-invoker` | invoker | 182 | 13.5 | 0 | 0 | 0 | 0 |
| `heroes-young-invoker-dark-artistry` | invoker | 191 | 16.2 | 3 | 0 | 0 | 0 |
| `heroes-zeus-arcana` | zeus, zuus | 19 | 5.6 | 0 | 0 | 0 | 0 |
| `heroes-zuljin-troll-warlord` | troll_warlord | 769 | 47.1 | 7 | 1 | 0 | 0 |
