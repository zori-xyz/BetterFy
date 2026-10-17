# Wardrobe catalog audit report

Written by the local-only test `wardrobe_audit::generate_wardrobe_allowlist` (`src-tauri/src/wardrobe_audit.rs`); do not edit by hand. It is the audit trail for `src-tauri/wardrobe/manifests.json`. The run is a Mac run over archives downloaded to a local cache: nothing here was run on Windows or in Dota, and no archive bytes are part of the repository.

## Inputs

| Input | Value |
| --- | --- |
| Catalog | `src/webCatalog.json`, snapshot 2026-07-26 |
| Items audited | 512 (`heroes` 460, `hero-items` 52) |
| Valve file list | SteamDatabase/GameTracking-Dota2 `game/dota/pak01_dir.txt`, 388116 entries, SHA-256 `6d98c763afe09ba0de5caf0fc965b6b21f26eeb2954ff1e7744996ba056db1b7` |
| Hero inference | folders of Valve-present paths: `models/heroes/<h>`, `models/items/<h>`, `particles/units/heroes/hero_<h>`, `particles/econ/items/<h>`, `materials/models/(heroes\|items)/<h>`; every name with at least 5% of the top name's paths is kept (at most 8) |
| Mirrors | candidate URLs checked by HEAD request: kept only when they answer 200 with the pinned ZIP size |

Emitted 499 of 512 items.
Skipped 13 of 512 items.

## Items that are not in the allowlist

| Reason | Items |
| --- | ---: |
| VPK container unreadable | 1 |
| analyzer blocked the install (skin_archive_hostile) | 6 |
| analyzer blocked the install (skin_install_too_large) | 1 |
| archive over the 256 MiB limit | 2 |
| no archive in the cache | 3 |

### VPK container unreadable (1)

- `heroes-black-witch-doctor`: vpk_invalid

### analyzer blocked the install (skin_archive_hostile) (6)

- `hero-items-disruptor-tractor`: PathUnsafe x32
- `heroes-ember-spirit-mai-shiranui`: PathUnsafe x4
- `heroes-enigma-blackhole-chan`: PathUnsafe x1, Script x1
- `heroes-lina-crystal-empress`: PathUnsafe x1
- `heroes-mirana-suigintou`: PathUnsafe x9
- `heroes-outworld-destroyer-tatsumaki`: PathUnsafe x3

### analyzer blocked the install (skin_install_too_large) (1)

- `heroes-blue-muerta`: skin_install_too_large

### archive over the 256 MiB limit (2)

- `heroes-legion-commander-chen`: the VPK inside the ZIP is over the unpacked-size bound of the archive inspector
- `heroes-nude-drow-ranger-v2`: the VPK inside the ZIP is over the unpacked-size bound of the archive inspector

### no archive in the cache (3)

- `heroes-crimson-anti-mage`: never fetched: the survey could not read its ZIP directory (no end-of-central-directory record)
- `heroes-phantom-assassin-yor-forger`: never fetched: the survey could not read its ZIP directory (no end-of-central-directory record)
- `heroes-tinker-alexander-prime`: never fetched: the survey could not read its ZIP directory (no end-of-central-directory record)

## Emitted items in numbers

Total install bytes of all emitted items in the default mode: 5283.6 MiB (85593 files); with the shared files left out (opt-in): 4985.5 MiB.
Biggest install: `heroes-oracle-irisviel-von-einzbern`, 124.9 MiB.
Install size per item (MiB): median 3.5, p90 29.4, p99 76.0.
Items that override no shared Valve file: 274; that override more than 100: 0; shared files overridden in total: 2862.
Hero-name ambiguity: 56 items carry more than one internal hero name (a second name with at least 5% of the top name's Valve-present paths).
Paths folded to lower case: 34 files in 3 items.
Classes over all emitted items: 48757 hero-scoped files, 33974 author-owned files (files the game does not ship, installed).
Items with two verified mirrors: 499; with one: 0.

Dropped files by reason (not installed, listed per item in the plan):

| Reason | Files |
| --- | ---: |
| empty_resource | 11 |
| oversized | 10 |
| panorama_unproven | 209 |
| type_not_allowed | 152 |
| uncompiled_source | 769 |

Folders where skins override shared Valve files most often (first three path segments):

| Folder | Files |
| --- | ---: |
| `materials/default` | 1640 |
| `materials/models/cubemaps` | 579 |
| `models/creeps/ice_biome` | 70 |
| `panorama/images/spellicons` | 65 |
| `particles/econ/taunts` | 64 |
| `particles/items2_fx` | 60 |
| `particles/error` | 38 |
| `materials/models/heroes` | 32 |
| `sounds/weapons/hero` | 32 |
| `particles/units/heroes` | 30 |
| `sounds/physics/footsteps` | 26 |
| `panorama/images/heroes` | 24 |
| `particles/econ/courier` | 23 |
| `materials/models/particle` | 22 |
| `particles/status_fx` | 21 |
| `particles/items_fx` | 19 |
| `models/items/faceless_void` | 13 |
| `materials/particle` | 9 |
| `particles/econ/events` | 9 |
| `soundevents/voscripts` | 9 |
| `particles/econ/items` | 6 |
| `particles/generic_gameplay` | 6 |
| `particles/models/heroes` | 6 |
| `materials/models/items` | 5 |
| `materials/particle/antimage_aghanins` | 3 |

## Emitted items

Heroes are the inferred internal names. Files are what goes into the build; shared is how many of the files override a shared Valve file (the opt-in mode leaves them out); dropped is files rejected by type.

| Catalog ID | Heroes | Files | MiB | Shared | Dropped | Folded |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `heroes-abaddon-bloody-fallen-king` | abaddon | 188 | 17.3 | 10 | 0 | 0 |
| `heroes-abaddon-fallen-king` | abaddon | 414 | 21.1 | 26 | 14 | 0 |
| `heroes-abaddon-sayaka-miki` | abaddon | 285 | 7.7 | 0 | 23 | 0 |
| `heroes-alchemist-darkbrew-enforcer` | alchemist | 13 | 0.3 | 0 | 0 | 0 |
| `heroes-alien-nyx-assassin` | nyx_assassin, nerubian_assassin | 186 | 7.7 | 15 | 1 | 0 |
| `heroes-ancient-apparition-yoshino` | ancient_apparition | 105 | 4.0 | 0 | 10 | 0 |
| `heroes-ancient-sand-naga-siren` | siren | 214 | 4.8 | 16 | 0 | 0 |
| `heroes-ancients-lina` | lina | 567 | 25.0 | 15 | 14 | 0 |
| `heroes-anti-mage-blue-illidan` | antimage, nightstalker | 435 | 22.9 | 19 | 0 | 0 |
| `heroes-anti-mage-creep` | antimage | 120 | 1.1 | 12 | 0 | 0 |
| `heroes-anti-mage-illidan` | antimage | 663 | 28.2 | 0 | 8 | 0 |
| `heroes-anti-mage-illidan-v2` | antimage | 318 | 26.0 | 21 | 0 | 0 |
| `heroes-anti-mage-ronin-warrior` | antimage | 138 | 11.2 | 0 | 1 | 0 |
| `heroes-anti-mage-shadow-slayer` | antimage | 192 | 13.6 | 24 | 16 | 0 |
| `heroes-anti-mage-turstarkuri-pilgrim` | antimage | 197 | 17.8 | 2 | 0 | 0 |
| `heroes-anubis-phantom-lancer` | phantom_lancer | 12 | 0.5 | 0 | 0 | 0 |
| `heroes-aquamarine-naga-siren` | siren, naga | 103 | 10.0 | 9 | 0 | 0 |
| `heroes-arc-warden-ancient-gaoler` | arc_warden | 5 | 0.2 | 0 | 0 | 0 |
| `heroes-arc-warden-black-hole` | arc_warden | 338 | 8.2 | 15 | 1 | 0 |
| `heroes-arc-warden-sakura-winter` | arc_warden | 351 | 7.0 | 31 | 1 | 0 |
| `heroes-arc-warden-yin-yang` | arc_warden | 329 | 4.4 | 16 | 0 | 0 |
| `heroes-ashes-arc-warden` | arc_warden | 369 | 11.9 | 27 | 0 | 0 |
| `heroes-axe-gawr-gura` | axe | 343 | 54.7 | 0 | 10 | 0 |
| `heroes-axe-heracles` | axe | 313 | 17.2 | 12 | 9 | 0 |
| `heroes-axe-immortal-bootblack` | axe | 58 | 3.6 | 2 | 1 | 0 |
| `heroes-axe-immortal-bootblack-brawler` | axe | 64 | 0.7 | 0 | 0 | 0 |
| `heroes-axe-kratos` | axe | 134 | 14.3 | 24 | 0 | 0 |
| `heroes-axe-unleashed` | axe | 53 | 3.3 | 0 | 1 | 0 |
| `heroes-bane-komeiji-koishi` | bane | 138 | 19.3 | 0 | 10 | 0 |
| `heroes-bare-brewmaster` | brewmaster | 2 | 0.0 | 0 | 0 | 0 |
| `heroes-bare-hoodwink` | hoodwink | 24 | 11.0 | 0 | 10 | 0 |
| `heroes-bare-kez` | kez | 1 | 0.0 | 0 | 0 | 0 |
| `heroes-bare-natures-prophet` | furion | 11 | 0.4 | 0 | 0 | 0 |
| `heroes-bare-pangolier` | pangolier | 3 | 0.2 | 0 | 0 | 0 |
| `heroes-batrider-night-snotty` | batrider | 26 | 1.2 | 0 | 0 | 0 |
| `heroes-beastmaster-bone-armor` | beastmaster | 9 | 0.7 | 0 | 0 | 0 |
| `heroes-beastmaster-yuuki-yuuna` | beastmaster | 705 | 26.5 | 0 | 1 | 0 |
| `heroes-black-phoenix` | phoenix | 262 | 9.9 | 0 | 0 | 0 |
| `heroes-bloodelf-silencer` | silencer | 225 | 18.5 | 6 | 0 | 0 |
| `heroes-bloodseeker-harvest-hound` | blood_seeker | 7 | 0.4 | 0 | 0 | 0 |
| `heroes-bloodseeker-strygwyr` | blood_seeker | 42 | 2.0 | 4 | 8 | 4 |
| `heroes-bloody-ancient-apparition` | ancient_apparition | 264 | 10.5 | 27 | 14 | 0 |
| `heroes-bloody-dazzle` | dazzle | 23 | 1.9 | 0 | 2 | 0 |
| `heroes-bloody-death-prophet` | death_prophet | 289 | 17.7 | 25 | 16 | 0 |
| `heroes-bloody-enigma` | enigma | 909 | 53.8 | 25 | 14 | 0 |
| `heroes-bloody-gore-spectre` | spectre | 1126 | 61.0 | 20 | 4 | 0 |
| `heroes-bloody-kez` | kez | 649 | 22.9 | 14 | 30 | 0 |
| `heroes-blue-dragon-puck` | puck | 248 | 8.3 | 26 | 0 | 0 |
| `heroes-blue-soul-huskar` | huskar | 647 | 57.0 | 21 | 18 | 0 |
| `heroes-bounty-hunter-hei` | bounty_hunter | 268 | 6.3 | 0 | 1 | 0 |
| `heroes-bounty-hunter-m1a2-abrams` | bounty_hunter | 391 | 47.9 | 78 | 1 | 0 |
| `heroes-bounty-hunter-old-man` | bounty_hunter | 10 | 0.3 | 2 | 0 | 0 |
| `heroes-bounty-hunter-old-verminator` | bounty_hunter | 9 | 0.4 | 0 | 0 | 0 |
| `heroes-brewmaster-jousting-panda` | brewmaster | 82 | 3.8 | 12 | 0 | 0 |
| `heroes-brewmaster-military-saint` | brewmaster | 112 | 6.3 | 28 | 0 | 0 |
| `heroes-bristleback-crimson-ring` | bristleback | 5 | 0.3 | 0 | 0 | 0 |
| `heroes-broodmother-epoch-of-the-abysm` | broodmother | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-broodmother-golden-orbweaver` | broodmother | 5 | 0.3 | 0 | 0 | 0 |
| `heroes-broodmother-redan` | broodmother | 294 | 15.9 | 23 | 0 | 0 |
| `heroes-broodmother-silken-queen` | broodmother | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-broodmother-undermount-candles` | broodmother | 10 | 0.8 | 0 | 0 | 0 |
| `heroes-broodmother-undermount-gloom` | broodmother | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-broodmother-white-widow` | broodmother | 6 | 0.8 | 0 | 0 | 0 |
| `heroes-bunshinjutsu-meepo` | meepo | 203 | 3.3 | 12 | 13 | 0 |
| `heroes-cardboard-shadow-fiend` | shadow_fiend | 37 | 45.6 | 3 | 0 | 0 |
| `heroes-centaur-chaos-chosen` | centaur | 7 | 0.2 | 0 | 0 | 0 |
| `heroes-centaur-cunning-counterfeit` | centaur | 7 | 1.0 | 0 | 0 | 0 |
| `heroes-centaur-emerald-marauder` | centaur | 7 | 0.4 | 0 | 0 | 0 |
| `heroes-chaos-invoker` | invoker, invoker_kid | 341 | 14.9 | 1 | 0 | 0 |
| `heroes-chaos-io` | wisp | 88 | 3.0 | 0 | 0 | 0 |
| `heroes-chaos-knight-flandre-scarlet` | chaos_knight | 240 | 9.0 | 0 | 9 | 0 |
| `heroes-chen-infernal-psychic` | chen | 6 | 0.7 | 1 | 0 | 0 |
| `heroes-chen-lelouch` | chen | 320 | 12.7 | 0 | 1 | 0 |
| `heroes-chen-seed-of-strife` | chen | 15 | 0.7 | 0 | 0 | 0 |
| `heroes-clinkz-silver-metall-pink` | clinkz | 597 | 12.2 | 22 | 30 | 0 |
| `heroes-clockwerk-apex-automated` | rattletrap | 6 | 0.5 | 0 | 0 | 0 |
| `heroes-clockwerk-seadog-stash` | rattletrap | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-cosmic-alchemist` | alchemist | 13 | 0.7 | 0 | 0 | 0 |
| `heroes-cosmic-cartographer` | marci | 4 | 1.1 | 0 | 0 | 0 |
| `heroes-cosmic-earthshaker` | earthshaker | 12 | 1.2 | 0 | 0 | 0 |
| `heroes-cosmic-enigma` | enigma | 12 | 2.5 | 0 | 0 | 0 |
| `heroes-cosmic-faceless-void` | faceless_void | 6 | 0.7 | 0 | 0 | 0 |
| `heroes-cosmic-immortal-tinker` | tinker | 13 | 1.6 | 0 | 0 | 0 |
| `heroes-cosmic-spirit-breaker` | spirit_breaker | 10 | 0.5 | 0 | 0 | 0 |
| `heroes-cosmic-tinker` | tinker | 9 | 1.8 | 0 | 0 | 0 |
| `heroes-cosmic-zeus` | zeus | 5 | 1.1 | 0 | 0 | 0 |
| `heroes-crimson-phantom-lancer` | phantom_lancer | 267 | 9.7 | 1 | 0 | 0 |
| `heroes-crystal-bristleback` | bristleback | 30 | 4.2 | 0 | 0 | 0 |
| `heroes-crystal-maiden-cirno` | crystal_maiden | 226 | 20.0 | 1 | 14 | 0 |
| `heroes-crystal-maiden-tamamo-no-mae` | crystal_maiden | 165 | 33.8 | 5 | 0 | 0 |
| `heroes-cursed-queen-of-pain` | queen_of_pain, queenofpain | 487 | 25.8 | 0 | 0 | 0 |
| `heroes-cursed-shadow-fiend` | nevermore, shadow_fiend | 339 | 5.1 | 13 | 0 | 0 |
| `heroes-curseding-anti-mage` | antimage | 185 | 9.3 | 22 | 4 | 0 |
| `heroes-custom-largo` | largo, bard | 163 | 92.0 | 26 | 0 | 0 |
| `heroes-custom-slark` | slark | 79 | 0.7 | 0 | 1 | 0 |
| `heroes-dark-ancient-slark` | slark, invoker | 672 | 9.7 | 20 | 0 | 0 |
| `heroes-dark-reaper-invoker` | invoker, invoker_kid | 691 | 8.7 | 20 | 0 | 0 |
| `heroes-dark-souls-razor` | razor | 127 | 18.1 | 6 | 0 | 0 |
| `heroes-dark-willow-saccharine-saboteur` | dark_willow | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-darkness-crimson-naga-siren` | siren, naga | 333 | 19.0 | 26 | 0 | 0 |
| `heroes-darkness-naga-siren` | siren | 182 | 6.3 | 19 | 0 | 0 |
| `heroes-darkness-weaver` | weaver | 452 | 22.5 | 23 | 89 | 0 |
| `heroes-dawnbreaker-death-knight` | dawnbreaker | 407 | 59.8 | 26 | 16 | 0 |
| `heroes-dawnbreaker-moonbreaker` | dawnbreaker | 324 | 25.4 | 27 | 0 | 0 |
| `heroes-dawnbreaker-starlorn-adjudicator` | dawnbreaker | 7 | 0.3 | 0 | 0 | 0 |
| `heroes-dazzle-darkclaw` | dazzle | 23 | 2.3 | 0 | 2 | 0 |
| `heroes-dazzle-lifeless-devotion` | dazzle | 231 | 14.8 | 5 | 1 | 0 |
| `heroes-dazzle-needlepoint-necromance` | dazzle | 5 | 0.4 | 0 | 0 | 0 |
| `heroes-death-prophet-bb` | death_prophet | 397 | 5.9 | 0 | 12 | 0 |
| `heroes-death-prophet-unicorn` | death_prophet | 539 | 35.3 | 0 | 9 | 0 |
| `heroes-death-prophet-yuyuko` | death_prophet | 232 | 19.5 | 0 | 15 | 29 |
| `heroes-defiree-disruptor` | disruptor | 259 | 8.3 | 17 | 0 | 0 |
| `heroes-devil-legion-commander` | legion_commander | 286 | 29.4 | 3 | 0 | 0 |
| `heroes-devil-skywrath-mage` | skywrath_mage | 85 | 8.4 | 8 | 0 | 0 |
| `heroes-disruptor-tandem-storm` | disruptor | 22 | 1.1 | 1 | 0 | 0 |
| `heroes-doom-fallen-khorne` | doom, doom_bringer | 16 | 0.7 | 0 | 0 | 0 |
| `heroes-doom-jeanne-alter` | doom, doom_bringer | 286 | 2.9 | 0 | 8 | 0 |
| `heroes-dragon-knight-burning-scale` | dragon_knight | 19 | 2.8 | 0 | 0 | 0 |
| `heroes-dragon-knight-silverwurm-sacrifice` | dragon_knight | 7 | 0.8 | 0 | 0 | 0 |
| `heroes-drow-ranger-arcana` | drow | 99 | 71.4 | 2 | 0 | 0 |
| `heroes-drow-ranger-diretide` | drow | 378 | 15.5 | 15 | 0 | 0 |
| `heroes-drow-ranger-stranger-arcana` | drow | 142 | 76.0 | 16 | 0 | 0 |
| `heroes-earth-spirit-slitehering-stone` | earth_spirit | 12 | 1.5 | 0 | 0 | 0 |
| `heroes-earthshaker-deep-magma` | earthshaker | 140 | 24.5 | 4 | 0 | 0 |
| `heroes-earthshaker-purple-arcana` | earthshaker | 355 | 32.9 | 24 | 0 | 0 |
| `heroes-earthshaker-red-arcana` | earthshaker | 371 | 27.2 | 14 | 0 | 0 |
| `heroes-ember-spirit-cinder-sensei` | ember_spirit | 18 | 0.4 | 0 | 0 | 0 |
| `heroes-ember-spirit-green-samurai` | ember_spirit | 233 | 13.3 | 21 | 0 | 0 |
| `heroes-ember-spirit-master-of-searing-path` | ember_spirit | 17 | 0.4 | 0 | 0 | 0 |
| `heroes-ember-spirit-shana` | ember_spirit | 594 | 45.6 | 0 | 2 | 0 |
| `heroes-emerald-dragon-knight` | dragon_knight | 78 | 6.3 | 14 | 0 | 0 |
| `heroes-emerald-shadow-fiend` | shadow_fiend, nevermore | 613 | 91.7 | 2 | 0 | 0 |
| `heroes-enchantress-darkwood-witness` | enchantress | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-enchantress-shiro` | enchantress | 206 | 6.6 | 0 | 1 | 0 |
| `heroes-enigma-angel-of-the-abyss` | enigma | 6 | 4.0 | 0 | 0 | 0 |
| `heroes-faceless-void-arcana` | faceless_void | 54 | 10.1 | 0 | 2 | 0 |
| `heroes-faceless-void-esdeath` | faceless_void | 287 | 4.5 | 0 | 1 | 0 |
| `heroes-faceless-void-rita` | faceless_void | 671 | 49.8 | 13 | 9 | 0 |
| `heroes-flameling-morphling` | morphling | 959 | 55.4 | 32 | 14 | 0 |
| `heroes-fog-ember-spirit` | ember_spirit | 212 | 12.2 | 13 | 0 | 0 |
| `heroes-frostivus-alchemist` | alchemist | 14 | 0.4 | 0 | 0 | 0 |
| `heroes-galactic-io` | wisp | 139 | 0.2 | 0 | 0 | 0 |
| `heroes-ghost-shadow-fiend` | shadow_fiend, nevermore | 712 | 24.3 | 25 | 14 | 0 |
| `heroes-ghost-void-spirit` | void_spirit | 1021 | 59.0 | 35 | 14 | 0 |
| `heroes-ghosty-slark` | slark | 477 | 9.0 | 24 | 15 | 0 |
| `heroes-goffren-broodmother` | broodmother | 4 | 0.5 | 0 | 0 | 0 |
| `heroes-gopo-pudge` | pudge | 57 | 10.5 | 16 | 0 | 0 |
| `heroes-gopo-tidehunter` | tidehunter | 71 | 20.2 | 12 | 1 | 0 |
| `heroes-gothic-lifestealer` | life_stealer | 81 | 6.6 | 12 | 0 | 0 |
| `heroes-green-earth-spirit` | earth_spirit | 241 | 8.1 | 11 | 0 | 0 |
| `heroes-grimstroke-arong` | grimstroke | 309 | 19.0 | 2 | 9 | 0 |
| `heroes-grimstroke-gloom-shroud` | grimstroke | 469 | 11.3 | 10 | 71 | 0 |
| `heroes-gyrocopter-kongou` | gyrocopter, gyro | 262 | 7.4 | 0 | 15 | 0 |
| `heroes-halloween-treant-protector` | treant_protector | 7 | 0.2 | 0 | 0 | 0 |
| `heroes-harrows-harvest-ringmaster` | ringmaster | 4 | 0.8 | 0 | 0 | 0 |
| `heroes-hell-muerta` | muerta | 400 | 59.3 | 0 | 1 | 0 |
| `hero-items-alchemist-radiance-blades` | alchemist | 6 | 0.1 | 0 | 0 | 0 |
| `hero-items-beastmaster-primal-paean` | beastmaster | 4 | 0.1 | 0 | 0 | 0 |
| `hero-items-beastmaster-primal-peacemaker` | beastmaster | 3 | 0.3 | 0 | 0 | 0 |
| `hero-items-bloodseeker-thirst-of-eztzhok` | bloodseeker | 9 | 0.1 | 2 | 0 | 0 |
| `hero-items-bounty-hunter-hunters-hoard` | bounty_hunter | 8 | 0.1 | 0 | 0 | 0 |
| `hero-items-bounty-hunter-tines-of-tybara` | bounty_hunter | 4 | 0.4 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-cavalcade` | centaur | 5 | 0.4 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-chieftain` | centaur | 4 | 0.1 | 0 | 0 | 0 |
| `hero-items-centaur-infernal-menace` | centaur | 5 | 0.1 | 0 | 0 | 0 |
| `hero-items-crystal-maiden-ice-blossom` | crystal_maiden | 3 | 0.3 | 0 | 0 | 0 |
| `hero-items-crystal-maiden-yulsarias-glacier` | crystal_maiden | 3 | 0.0 | 1 | 0 | 0 |
| `hero-items-crystal-maiden-yulsarias-mantle` | crystalmaiden | 6 | 0.3 | 3 | 0 | 0 |
| `hero-items-dark-seer-forlorn-precipice` | dark_seer | 7 | 0.9 | 0 | 0 | 0 |
| `hero-items-dazzle-gates-of-nothl` | dazzle | 5 | 0.2 | 0 | 0 | 0 |
| `hero-items-disruptor-faceless-rex` | disruptor | 10 | 1.5 | 0 | 0 | 0 |
| `hero-items-earthshaker-carrot` | earthshaker | 16 | 0.7 | 8 | 0 | 0 |
| `hero-items-faceless-void-battlefury` | faceless_void | 3 | 0.0 | 1 | 0 | 0 |
| `hero-items-invoker-magus-accord` | invoker | 9 | 0.2 | 1 | 0 | 0 |
| `hero-items-invoker-magus-apex` | invoker | 10 | 0.2 | 0 | 0 | 0 |
| `hero-items-kez-guts-sword` | kez | 9 | 1.5 | 3 | 12 | 0 |
| `hero-items-kez-zangetsu-sword` | kez | 189 | 2.1 | 0 | 0 | 0 |
| `hero-items-kunkka-shadow-blade` | kunkka | 4 | 0.0 | 0 | 0 | 0 |
| `hero-items-kunkka-whale-blade` | kunkka | 5 | 0.0 | 0 | 0 | 0 |
| `hero-items-lion-fin-kings-charm` | lion | 6 | 0.4 | 1 | 0 | 0 |
| `hero-items-lion-hell-spar-anathema` | lion | 8 | 0.1 | 0 | 0 | 0 |
| `hero-items-lion-mulctant-pall` | lion | 3 | 0.1 | 0 | 0 | 0 |
| `hero-items-lone-druid-cauldron-of-xahryx` | lone_druid | 3 | 0.0 | 0 | 0 | 0 |
| `hero-items-lone-druid-claws-of-nuranu` | lone_druid | 4 | 0.3 | 1 | 0 | 0 |
| `hero-items-luna-moonfall` | luna | 6 | 0.0 | 0 | 0 | 0 |
| `hero-items-luna-twilight-schism` | luna | 7 | 0.2 | 0 | 0 | 0 |
| `hero-items-magnus-eyes-of-ardenok` | magnataur | 4 | 0.3 | 0 | 0 | 0 |
| `hero-items-magnus-shock-of-the-anvil` | magnataur | 6 | 0.3 | 0 | 0 | 0 |
| `hero-items-mars-progenitors-bane` | mars | 6 | 0.4 | 0 | 0 | 0 |
| `hero-items-monkey-king-staff-of-gun-yu` | monkey_king | 7 | 3.4 | 0 | 0 | 0 |
| `hero-items-phantom-assassin-avowance` | phantom_assassin | 5 | 0.2 | 0 | 0 | 0 |
| `hero-items-phantom-assassin-codicil` | phantom_assassin | 4 | 0.2 | 0 | 0 | 0 |
| `hero-items-phantom-lancer-concord-dominion` | phantom_lancer | 3 | 0.1 | 0 | 0 | 0 |
| `hero-items-pudge-hood-of-defiance` | pudge | 1 | 0.0 | 0 | 0 | 0 |
| `hero-items-shadow-fiend-arms-of-desolation` | nevermore | 6 | 0.0 | 3 | 0 | 0 |
| `hero-items-shadow-fiend-eternal-harvest` | shadow_fiend | 1 | 0.0 | 0 | 0 | 0 |
| `hero-items-sniper-dragon-lore` | sniper | 71 | 2.9 | 8 | 0 | 0 |
| `hero-items-spirit-breaker-savage-mettle` | spirit_breaker | 4 | 0.1 | 0 | 0 | 0 |
| `hero-items-storm-spirit-corridan-maestro` | storm_spirit | 1 | 0.1 | 0 | 0 | 0 |
| `hero-items-storm-spirit-lightning-orchid` | stormspirit | 6 | 0.1 | 3 | 0 | 0 |
| `hero-items-storm-spirit-stormborn-mandate` | stormspirit | 4 | 0.1 | 2 | 0 | 0 |
| `hero-items-sven-guts-sword` | sven | 57 | 2.5 | 4 | 0 | 0 |
| `hero-items-tinker-cripple` | tinker | 21 | 16.1 | 0 | 0 | 0 |
| `hero-items-tiny-banana` | tiny | 7 | 0.1 | 0 | 0 | 0 |
| `hero-items-undying-hallows-within` | undying | 7 | 0.8 | 0 | 0 | 0 |
| `hero-items-ursa-swift-claw` | ursa | 3 | 0.0 | 0 | 0 | 0 |
| `hero-items-wraith-king-winterblight` | skeletonking | 4 | 0.0 | 1 | 0 | 0 |
| `heroes-hollow-kez` | kez | 677 | 29.4 | 8 | 42 | 0 |
| `heroes-homelander-invoker` | invoker | 79 | 15.8 | 6 | 0 | 0 |
| `heroes-hoodwink-bilge-rat-buccaneer` | hoodwink | 5 | 0.4 | 0 | 0 | 0 |
| `heroes-hoodwink-tomokan-footsoldier` | hoodwink | 4 | 0.3 | 0 | 0 | 0 |
| `heroes-huskar-cthugha` | huskar | 254 | 9.5 | 0 | 1 | 0 |
| `heroes-huskar-flashpoint-proselyte` | huskar | 50 | 1.1 | 0 | 0 | 0 |
| `heroes-ice-chaos-knight` | chaos_knight | 198 | 10.3 | 13 | 0 | 0 |
| `heroes-ice-phoenix` | phoenix | 333 | 5.9 | 10 | 0 | 0 |
| `heroes-ice-venomancer` | venomancer | 342 | 7.7 | 13 | 0 | 0 |
| `heroes-ifrit-monkey-king` | monkey_king | 343 | 9.4 | 13 | 0 | 0 |
| `heroes-immortal-ancient-apparition` | ancient_apparition | 15 | 0.9 | 0 | 0 | 0 |
| `heroes-immortal-anti-mage` | antimage | 38 | 2.1 | 1 | 0 | 0 |
| `heroes-immortal-beastmaster` | beastmaster | 9 | 0.5 | 0 | 0 | 0 |
| `heroes-immortal-hoodwink` | hoodwink | 8 | 0.8 | 0 | 0 | 0 |
| `heroes-immortal-jakiro` | jakiro | 13 | 0.6 | 0 | 0 | 0 |
| `heroes-immortal-lich` | lich | 16 | 4.3 | 0 | 0 | 0 |
| `heroes-immortal-lina` | lina | 167 | 0.8 | 0 | 0 | 0 |
| `heroes-immortal-medusa` | medusa | 11 | 0.4 | 0 | 0 | 0 |
| `heroes-immortal-necrophos` | necrolyte | 20 | 0.7 | 0 | 0 | 0 |
| `heroes-immortal-phantom-lancer` | phantom_lancer | 16 | 0.5 | 0 | 0 | 0 |
| `heroes-immortal-primal-beast` | primal_beast | 12 | 0.8 | 0 | 0 | 0 |
| `heroes-immortal-puck` | puck | 10 | 0.3 | 0 | 0 | 0 |
| `heroes-immortal-queen-of-pain` | queenofpain | 52 | 2.4 | 13 | 2 | 0 |
| `heroes-immortal-slardar` | slardar | 13 | 1.1 | 0 | 0 | 0 |
| `heroes-immortal-storm-spirit` | storm_spirit, stormspirit | 20 | 1.8 | 0 | 0 | 0 |
| `heroes-immortal-templar-assassin` | templar_assassin, lanaya | 22 | 0.6 | 0 | 0 | 0 |
| `heroes-invoker-bloody-wizard` | invoker | 526 | 13.4 | 15 | 0 | 0 |
| `heroes-invoker-dark-artistry` | invoker | 16 | 0.2 | 0 | 0 | 0 |
| `heroes-invoker-dark-artistry-v2` | invoker | 19 | 0.4 | 1 | 0 | 0 |
| `heroes-invoker-dark-artistry-v3` | invoker | 55 | 1.3 | 1 | 0 | 0 |
| `heroes-invoker-dark-divine` | invoker | 931 | 19.6 | 12 | 0 | 0 |
| `heroes-invoker-depths-covenant` | invoker | 11 | 1.4 | 0 | 0 | 0 |
| `heroes-invoker-fate-meridian` | invoker | 8 | 0.5 | 0 | 0 | 0 |
| `heroes-invoker-maryday` | invoker | 35 | 0.7 | 2 | 0 | 0 |
| `heroes-invoker-patchouli` | invoker | 755 | 23.4 | 17 | 41 | 0 |
| `heroes-io-histoire` | wisp, io | 192 | 11.6 | 0 | 5 | 0 |
| `heroes-io-madame-scrio` | wisp | 5 | 0.7 | 0 | 0 | 0 |
| `heroes-io-pink-arcana` | wisp | 209 | 11.6 | 20 | 1 | 0 |
| `heroes-io-purple` | wisp | 97 | 0.4 | 0 | 0 | 0 |
| `heroes-jakiro-himera` | jakiro | 144 | 8.3 | 0 | 0 | 0 |
| `heroes-jakiro-kiyohime` | jakiro | 275 | 11.4 | 0 | 1 | 0 |
| `heroes-joker-ringmaster` | ringmaster, obsidian_destroyer | 277 | 48.1 | 24 | 0 | 0 |
| `heroes-juggernaut-ancient-exile` | juggernaut | 6 | 0.4 | 0 | 0 | 0 |
| `heroes-juggernaut-arcana-purple` | juggernaut | 565 | 11.8 | 7 | 0 | 0 |
| `heroes-juggernaut-ice-and-fire` | juggernaut | 590 | 53.6 | 0 | 1 | 0 |
| `heroes-juggernaut-okita-souji` | juggernaut | 611 | 12.0 | 5 | 25 | 0 |
| `heroes-juggernaut-samurai-of-wind` | juggernaut | 98 | 1.9 | 8 | 1 | 0 |
| `heroes-kai-meepo` | meepo | 487 | 24.8 | 16 | 0 | 0 |
| `heroes-keeper-of-the-light-renegade` | keeper_of_the_light | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-keeper-of-the-light-tamamo` | keeper_of_the_light | 462 | 32.0 | 21 | 13 | 0 |
| `heroes-kez-bersexer` | kez | 563 | 42.5 | 14 | 0 | 0 |
| `heroes-kez-captain-anchorbeak` | kez | 55 | 6.3 | 0 | 0 | 0 |
| `heroes-kez-nanami-yachiyo-kai` | kez | 374 | 7.3 | 0 | 1 | 0 |
| `heroes-kez-peacock` | kez | 35 | 5.6 | 0 | 0 | 0 |
| `heroes-kez-tensura` | kez | 1945 | 38.3 | 13 | 1 | 0 |
| `heroes-legion-commander-crimson-black` | legion_commander | 175 | 7.0 | 11 | 0 | 0 |
| `heroes-legion-commander-holy-paladin` | legion_commander, legion | 450 | 19.8 | 13 | 0 | 0 |
| `heroes-legion-commander-nero` | legion_commander | 170 | 4.9 | 0 | 10 | 0 |
| `heroes-legion-commander-persona` | legion_commander | 547 | 109.9 | 0 | 1 | 0 |
| `heroes-lich-miyuki-shiba` | lich | 239 | 4.7 | 0 | 8 | 0 |
| `heroes-lich-shokuhou-misaki` | lich | 753 | 33.0 | 3 | 1 | 0 |
| `heroes-lifestealer-cong-yu` | lifestealer, life_stealer | 782 | 25.2 | 0 | 1 | 0 |
| `heroes-lifestealer-diretide-shimmer` | life_stealer | 4 | 0.1 | 0 | 0 | 0 |
| `heroes-lifestealer-golden-maw` | life_stealer | 47 | 5.4 | 1 | 0 | 0 |
| `heroes-lina-aerial-aphrodite-flame` | lina | 235 | 26.8 | 10 | 0 | 0 |
| `heroes-lion-cannonroar-confessor` | lion | 53 | 4.2 | 14 | 0 | 0 |
| `heroes-lion-immortal-chimera` | lion | 40 | 1.6 | 0 | 0 | 0 |
| `heroes-lion-witcher` | lion | 290 | 17.2 | 25 | 0 | 0 |
| `heroes-lollipop-juggernaut` | juggernaut | 58 | 1.4 | 8 | 0 | 0 |
| `heroes-luna-bloodmoon-stalker` | luna | 34 | 0.7 | 0 | 0 | 0 |
| `heroes-luna-moon-stalker` | luna | 238 | 17.5 | 2 | 0 | 0 |
| `heroes-luna-winged-sentinel` | luna | 14 | 1.8 | 0 | 0 | 0 |
| `heroes-lycan-blood-moon` | lycan | 8 | 1.5 | 0 | 0 | 0 |
| `heroes-lycan-red-wolf-clan` | lycan | 6 | 1.0 | 0 | 0 | 0 |
| `heroes-magnus-kirito` | magnataur | 237 | 6.5 | 0 | 1 | 0 |
| `heroes-magnus-magnataur` | magnataur | 152 | 13.3 | 15 | 0 | 0 |
| `heroes-magnus-ornate-cruelty` | magnataur | 5 | 0.2 | 0 | 0 | 0 |
| `heroes-magnus-vortexbull` | magnataur | 279 | 14.8 | 1 | 1 | 0 |
| `heroes-magnus-white-tiger` | magnataur | 165 | 4.1 | 12 | 0 | 0 |
| `heroes-maid-marci` | marci | 44 | 40.8 | 0 | 0 | 0 |
| `heroes-marci-atri` | marci | 127 | 58.9 | 0 | 1 | 0 |
| `heroes-marci-bloomjewel-brawler` | marci | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-marci-faithful-fortune` | marci | 4 | 0.5 | 0 | 0 | 0 |
| `heroes-marci-laurels-of-the-downstar` | marci | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-marci-school-of-the-solar-fang` | marci | 4 | 0.5 | 0 | 0 | 0 |
| `heroes-mars-diretide-shimmer` | mars | 106 | 1.3 | 0 | 0 | 0 |
| `heroes-medusa-gilgamesh` | medusa | 333 | 6.9 | 0 | 8 | 0 |
| `heroes-medusa-gorgon` | medusa | 332 | 37.3 | 10 | 0 | 0 |
| `heroes-meepo-diretide` | meepo | 12 | 1.6 | 0 | 0 | 0 |
| `heroes-meepo-lotus-bear` | meepo | 6 | 0.4 | 0 | 0 | 0 |
| `heroes-meepo-mocking-bird` | meepo | 6 | 0.7 | 0 | 0 | 0 |
| `heroes-monkey-king-arcana` | monkey_king | 6 | 0.3 | 0 | 0 | 0 |
| `heroes-monkey-king-simian-sovereign` | monkey_king | 4 | 2.4 | 0 | 0 | 0 |
| `heroes-monkey-king-water-demon` | monkey_king | 242 | 8.8 | 5 | 0 | 0 |
| `heroes-morphling-abyss-revenge` | morphling | 13 | 3.1 | 2 | 0 | 0 |
| `heroes-morphling-automaton` | morphling | 29 | 19.2 | 0 | 1 | 0 |
| `heroes-morphling-darktrench-purple` | morphling | 245 | 11.4 | 12 | 0 | 0 |
| `heroes-morphling-darktrench-tears` | morphling | 13 | 2.2 | 0 | 0 | 0 |
| `heroes-morphling-fluid-frenzy` | morphling | 45 | 9.9 | 0 | 0 | 0 |
| `heroes-morphling-megalodon` | morphling | 176 | 16.1 | 14 | 0 | 0 |
| `heroes-muerta-blackwing-bandolera` | muerta | 5 | 1.6 | 0 | 0 | 0 |
| `heroes-muerta-soul-warden` | muerta | 5 | 1.3 | 0 | 0 | 0 |
| `heroes-naga-siren-allure-of-the-deep` | siren | 7 | 1.4 | 0 | 0 | 0 |
| `heroes-naga-siren-izayoi-miku` | siren | 287 | 12.6 | 10 | 1 | 0 |
| `heroes-natures-prophet-allfather` | furion | 8 | 0.6 | 0 | 0 | 0 |
| `heroes-natures-prophet-saya` | furion | 494 | 31.4 | 0 | 1 | 0 |
| `heroes-natures-prophet-spectral-sage` | furion | 193 | 13.8 | 31 | 0 | 0 |
| `heroes-necrophos-eucliwood-halsace` | necrolyte | 805 | 18.5 | 0 | 1 | 0 |
| `heroes-necrophos-serpent-spite` | necrolyte | 5 | 0.6 | 0 | 0 | 0 |
| `heroes-night-stalker-dark-oath` | nightstalker, night_stalker | 18 | 1.0 | 0 | 0 | 0 |
| `heroes-night-stalker-feasts-of-forever` | nightstalker | 16 | 4.8 | 0 | 0 | 0 |
| `heroes-night-stalker-frost-walker` | nightstalker | 96 | 6.6 | 17 | 0 | 0 |
| `heroes-night-stalker-relict` | nightstalker | 117 | 30.2 | 6 | 1 | 0 |
| `heroes-night-stalker-remilia` | nightstalker | 174 | 13.6 | 4 | 12 | 0 |
| `heroes-night-ursa` | ursa | 144 | 7.1 | 16 | 0 | 0 |
| `heroes-nightmare-chaos-knight` | chaos_knight | 480 | 26.7 | 23 | 14 | 0 |
| `heroes-nuclear-clockwerk` | rattletrap, clockwerk | 110 | 4.3 | 14 | 0 | 0 |
| `heroes-nude-drow-ranger` | drow | 64 | 7.2 | 6 | 0 | 0 |
| `heroes-nude-marci` | marci | 286 | 79.3 | 0 | 7 | 0 |
| `heroes-nude-naga-siren` | siren | 15 | 1.0 | 7 | 0 | 0 |
| `heroes-nude-queen-of-pain` | queenofpain, queen_of_pain | 160 | 27.3 | 0 | 0 | 0 |
| `heroes-ogre-magi-amakusa-shiro` | ogre_magi | 261 | 4.0 | 0 | 9 | 0 |
| `heroes-ogre-magi-custom-arcana` | ogre_magi | 108 | 11.4 | 16 | 1 | 0 |
| `heroes-ogre-magi-freeboot-fortunes` | ogre_magi | 6 | 0.8 | 0 | 0 | 0 |
| `heroes-ogre-magi-pyrexae-polymorph` | ogre_magi | 6 | 0.7 | 0 | 0 | 0 |
| `heroes-old-shadow-fiend` | shadow_fiend | 82 | 1.8 | 13 | 1 | 0 |
| `heroes-old-storm-spirit` | storm_spirit, stormspirit | 120 | 1.7 | 0 | 11 | 0 |
| `heroes-omniknight-akashi` | omniknight | 285 | 44.7 | 10 | 5 | 0 |
| `heroes-omniknight-blinding-bastion` | omniknight | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-oracle-immortal-silence` | oracle | 18 | 0.6 | 0 | 0 | 0 |
| `heroes-oracle-irisviel-von-einzbern` | oracle | 730 | 124.9 | 8 | 1 | 0 |
| `heroes-outworld-destroyer-dark-metter` | obsidian_destroyer | 229 | 8.1 | 23 | 0 | 0 |
| `heroes-outworld-destroyer-yukari` | obsidian_destroyer, outworld_devourer | 364 | 34.9 | 1 | 18 | 0 |
| `heroes-pangolier-gentleman` | pangolier, antimage | 200 | 11.9 | 13 | 0 | 0 |
| `heroes-phantom-assassin-arcana` | phantom_assassin | 426 | 5.9 | 0 | 6 | 0 |
| `heroes-phantom-assassin-persona` | phantom_assassin | 113 | 46.0 | 0 | 0 | 0 |
| `heroes-phantom-assassin-rory` | phantom_assassin | 464 | 5.8 | 0 | 19 | 0 |
| `heroes-phantom-assassin-skull-arcana` | phantom_assassin | 319 | 16.2 | 17 | 0 | 0 |
| `heroes-phantom-lancer-crimson-v2` | phantom_lancer | 268 | 9.7 | 1 | 0 | 0 |
| `heroes-phantom-lancer-dark-carnival` | phantom_lancer | 5 | 0.5 | 0 | 0 | 0 |
| `heroes-phantom-lancer-humble-drifter` | phantom_lancer | 4 | 0.1 | 0 | 0 | 0 |
| `heroes-phantom-lancer-immortal-avilliva` | phantom_lancer | 10 | 0.4 | 0 | 0 | 0 |
| `heroes-phantom-lancer-immortal-felidae` | phantom_lancer | 21 | 0.6 | 0 | 0 | 0 |
| `heroes-phantom-lancer-purple-avilliva` | phantom_lancer | 9 | 0.5 | 0 | 0 | 0 |
| `heroes-phantom-lancer-sun-warrior` | phantom_lancer | 7 | 0.1 | 0 | 0 | 0 |
| `heroes-phite-phantom-assassin` | phantom_assassin | 287 | 9.1 | 23 | 0 | 0 |
| `heroes-pink-nyx-assassin` | nyx_assassin, nerubian_assassin | 418 | 8.1 | 29 | 0 | 0 |
| `heroes-pink-shadow-fiend` | shadow_fiend, nevermore | 719 | 36.6 | 16 | 1 | 0 |
| `heroes-pinkie-sven` | sven | 239 | 6.4 | 18 | 14 | 0 |
| `heroes-primal-beast-dark-behemoth` | primal_beast | 12 | 9.6 | 0 | 0 | 0 |
| `heroes-primal-beast-prehistoric-predator` | primal_beast | 39 | 16.2 | 0 | 0 | 0 |
| `heroes-primal-beast-snowbeast` | primal_beast | 33 | 13.2 | 14 | 0 | 0 |
| `heroes-pudge-arcana-elephant` | pudge | 290 | 38.2 | 13 | 0 | 0 |
| `heroes-pudge-candy-cat` | pudge | 7 | 0.6 | 0 | 0 | 0 |
| `heroes-pudge-clown-toy` | pudge | 416 | 11.6 | 9 | 1 | 0 |
| `heroes-pudge-cm` | pudge | 17 | 0.2 | 6 | 0 | 0 |
| `heroes-pudge-iron-hog` | pudge | 7 | 0.2 | 0 | 0 | 0 |
| `heroes-pudge-mindless-slaughter` | pudge | 7 | 0.5 | 0 | 0 | 0 |
| `heroes-pudge-rotzo-the-clown` | pudge | 11 | 0.8 | 0 | 0 | 0 |
| `heroes-pudge-toy-butcher` | pudge | 173 | 51.7 | 0 | 0 | 0 |
| `heroes-pugna-nightwither-revenant` | pugna | 468 | 8.8 | 3 | 0 | 0 |
| `heroes-purple-kez` | kez | 531 | 30.1 | 18 | 15 | 0 |
| `heroes-purple-legion-commander` | legion, legion_commander | 322 | 11.5 | 24 | 0 | 0 |
| `heroes-purple-queen-of-pain` | queen_of_pain, queenofpain | 692 | 37.5 | 21 | 0 | 0 |
| `heroes-purple-witch-broodmother` | broodmother | 170 | 9.7 | 24 | 0 | 0 |
| `heroes-queen-of-pain-rose` | queen_of_pain, queenofpain | 1112 | 72.4 | 26 | 14 | 0 |
| `heroes-razor-rainbow-statics` | razor | 146 | 3.4 | 14 | 4 | 0 |
| `heroes-razor-rizelencer` | razor | 96 | 1.2 | 12 | 0 | 0 |
| `heroes-razor-silver-rrevenant` | razor | 8 | 0.2 | 0 | 0 | 0 |
| `heroes-red-abaddon` | abaddon | 148 | 3.6 | 21 | 0 | 0 |
| `heroes-red-monkey-king` | monkey_king | 445 | 14.3 | 0 | 1 | 0 |
| `heroes-riki-hunger-howling-wilds` | riki, rikimaru | 46 | 0.6 | 0 | 0 | 0 |
| `heroes-rubick-ancient-magus` | rubick | 20 | 0.5 | 0 | 0 | 0 |
| `heroes-rubick-dark-mage` | rubick | 180 | 2.4 | 4 | 0 | 0 |
| `heroes-rubick-plagueroad-apothacary` | rubick | 53 | 0.5 | 0 | 0 | 0 |
| `heroes-russian-ursa` | ursa | 58 | 8.7 | 7 | 0 | 0 |
| `heroes-sakura-pink-invoker` | invoker | 740 | 25.7 | 29 | 4 | 0 |
| `heroes-sand-king-black-sands` | sand_king | 8 | 0.5 | 2 | 0 | 0 |
| `heroes-scarlet-keeper-of-the-light` | keeper_of_the_light | 996 | 55.2 | 27 | 14 | 0 |
| `heroes-sf-broodmother` | broodmother | 83 | 0.4 | 0 | 0 | 0 |
| `heroes-shadow-fiend-1000-7` | shadow_fiend, nevermore | 202 | 2.7 | 0 | 2 | 0 |
| `heroes-shadow-fiend-arcana` | nevermore, shadow_fiend | 30 | 5.8 | 0 | 0 | 0 |
| `heroes-shadow-fiend-black` | shadow_fiend, nevermore | 366 | 18.3 | 13 | 1 | 0 |
| `heroes-shadow-fiend-desolation` | shadow_fiend | 3 | 0.1 | 0 | 0 | 0 |
| `heroes-shadow-fiend-eternal-harvest` | shadow_fiend | 3 | 0.2 | 0 | 0 | 0 |
| `heroes-shadow-fiend-king-of-vipers` | shadow_fiend | 3 | 0.4 | 0 | 0 | 0 |
| `heroes-shadow-fiend-souls-tyrant` | shadow_fiend | 3 | 0.2 | 0 | 0 | 0 |
| `heroes-shadow-fiend-sundered-souls` | shadow_fiend, nevermore | 7 | 0.5 | 0 | 1 | 0 |
| `heroes-shadow-fiend-trenbolone` | nevermore, shadow_fiend | 33 | 4.1 | 2 | 0 | 0 |
| `heroes-shadow-fiend-white` | shadow_fiend, nevermore | 423 | 8.7 | 9 | 0 | 0 |
| `heroes-shadow-shaman-immortal-crimson` | shadowshaman, shadow_shaman | 21 | 1.0 | 2 | 0 | 0 |
| `heroes-shadow-shaman-purple` | shadowshaman | 32 | 1.0 | 0 | 0 | 0 |
| `heroes-shadow-timbersaw` | shredder | 122 | 2.8 | 5 | 0 | 0 |
| `heroes-shaolin-master-oracle` | oracle | 209 | 9.6 | 24 | 0 | 0 |
| `heroes-sherman-crystal-maiden` | crystal_maiden, crystalmaiden | 344 | 17.9 | 15 | 0 | 0 |
| `heroes-silencer-whispered-wing` | silencer | 6 | 0.8 | 0 | 0 | 0 |
| `heroes-skywrath-mage-arcana` | skywrath_mage | 90 | 44.5 | 0 | 0 | 0 |
| `heroes-skywrath-mage-thanatos` | skywrath_mage | 232 | 19.1 | 17 | 0 | 0 |
| `heroes-slardar-naga-guardian` | slardar | 281 | 18.7 | 0 | 0 | 0 |
| `heroes-slark-barracuda-re` | slark | 447 | 21.1 | 0 | 0 | 0 |
| `heroes-slark-shadow-in-the-deep` | slark | 9 | 0.3 | 0 | 0 | 0 |
| `heroes-snapfire-desert-ardalan` | snapfire | 6 | 0.9 | 0 | 0 | 0 |
| `heroes-snapfire-whippersnapper` | snapfire | 5 | 0.8 | 0 | 0 | 0 |
| `heroes-sniper-ardalan-interdictor` | sniper | 277 | 9.0 | 0 | 0 | 0 |
| `heroes-sniper-howling-wolf` | sniper | 15 | 1.1 | 0 | 0 | 0 |
| `heroes-sniper-odogaron-armor` | sniper | 11 | 2.1 | 0 | 0 | 0 |
| `heroes-snowing-marci` | marci | 341 | 7.3 | 4 | 0 | 0 |
| `heroes-spectre-arcana` | spectre | 37 | 5.5 | 0 | 0 | 0 |
| `heroes-spectre-phantom-advent` | spectre | 73 | 10.5 | 17 | 0 | 0 |
| `heroes-spectre-shadowveil` | spectre | 21 | 0.9 | 0 | 0 | 0 |
| `heroes-spectre-toxin` | spectre | 762 | 44.2 | 28 | 14 | 0 |
| `heroes-spirit-breaker-crystal-colossus` | spirit_breaker | 9 | 0.5 | 0 | 0 | 0 |
| `heroes-spirit-breaker-elemental-imperator` | spirit_breaker | 9 | 0.2 | 0 | 0 | 0 |
| `heroes-spirit-breaker-ghost-monster` | spirit_breaker | 180 | 10.0 | 13 | 2 | 0 |
| `heroes-steel-sand-king` | sandking, sand_king | 103 | 2.7 | 16 | 0 | 0 |
| `heroes-storm-spirit-beast-of-thunder` | storm_spirit | 5 | 0.4 | 2 | 0 | 0 |
| `heroes-storm-spirit-corridan-maestro` | storm_spirit | 5 | 0.3 | 2 | 0 | 0 |
| `heroes-sven-berseker` | sven | 196 | 6.5 | 9 | 0 | 0 |
| `heroes-sven-blue-angel` | sven | 315 | 27.1 | 16 | 0 | 0 |
| `heroes-sven-indomitable-legacy` | sven | 22 | 0.5 | 0 | 0 | 0 |
| `heroes-sven-yatogami-tohka` | winter_wyvern, sven | 563 | 37.2 | 0 | 21 | 0 |
| `heroes-sven-yatogami-tohka-v2` | sven | 321 | 5.6 | 0 | 20 | 0 |
| `heroes-techies-alien` | techies | 42 | 5.9 | 0 | 0 | 0 |
| `heroes-techies-arab` | techies | 17 | 5.0 | 0 | 0 | 0 |
| `heroes-techies-sodbustin-saboteurs` | techies | 5 | 0.7 | 0 | 0 | 0 |
| `heroes-techies-war-rig-pachyderm` | techies | 9 | 1.1 | 0 | 0 | 0 |
| `heroes-templar-assassin-playboy` | templar_assassin, lanaya | 180 | 13.3 | 4 | 1 | 0 |
| `heroes-templar-assassin-royal-squire` | templar_assassin, lanaya | 28 | 0.6 | 0 | 0 | 0 |
| `heroes-templar-assassin-whisper-gates` | lanaya | 29 | 4.4 | 0 | 0 | 0 |
| `heroes-terrorblade-666-fractal-horns` | terrorblade | 84 | 1.0 | 5 | 0 | 0 |
| `heroes-terrorblade-crimson-fractal-horns` | terrorblade | 183 | 1.8 | 0 | 0 | 0 |
| `heroes-terrorblade-green-fractal-horns` | terrorblade | 185 | 3.7 | 1 | 0 | 0 |
| `heroes-terrorblade-immortal-arcana` | terrorblade | 19 | 1.2 | 0 | 1 | 0 |
| `heroes-terrorblade-unblinking-horror` | terrorblade | 179 | 20.7 | 23 | 0 | 0 |
| `heroes-terrorblade-white-fractal-horns` | terrorblade | 212 | 1.7 | 0 | 0 | 0 |
| `heroes-terrorblade-wrong-arcana` | terrorblade | 483 | 34.7 | 8 | 0 | 0 |
| `heroes-tidehunter-insatiable-seas` | tidehunter | 33 | 8.8 | 0 | 0 | 0 |
| `heroes-tidehunter-squid-girl` | tidehunter | 322 | 11.2 | 5 | 1 | 0 |
| `heroes-timbersaw-clearcut-cavalier` | shredder | 134 | 2.9 | 0 | 0 | 0 |
| `heroes-tinker-deathsman` | tinker | 30 | 4.2 | 0 | 0 | 0 |
| `heroes-tinker-deep-hornet` | tinker | 18 | 0.9 | 3 | 0 | 0 |
| `heroes-tinker-fortified-fabricator` | tinker | 6 | 0.7 | 0 | 0 | 0 |
| `heroes-tinker-sakuya` | tinker | 138 | 15.4 | 0 | 17 | 0 |
| `heroes-tinker-submerged-hazard` | tinker | 31 | 0.4 | 0 | 0 | 0 |
| `heroes-tiny-majesty-of-the-colossus` | tiny | 32 | 3.1 | 1 | 0 | 0 |
| `heroes-tiny-stone-giant-stronghold` | tiny | 6 | 3.3 | 1 | 0 | 0 |
| `heroes-toxicpunk-timbersaw` | shredder, timbersaw | 155 | 4.3 | 16 | 0 | 0 |
| `heroes-treant-protector-frost-hideaway` | treant_protector | 4 | 0.4 | 0 | 0 | 0 |
| `heroes-troll-warlord-reimu` | troll_warlord | 178 | 20.6 | 0 | 11 | 0 |
| `heroes-tusk-immortal` | tuskarr | 9 | 0.3 | 3 | 0 | 0 |
| `heroes-tusk-king-kong` | tusk, tuskarr | 204 | 6.8 | 14 | 0 | 1 |
| `heroes-tusk-odobenus-footman` | tusk, tuskarr | 145 | 13.2 | 33 | 0 | 0 |
| `heroes-tusk-odobenus-frost-mage` | tusk, tuskarr | 200 | 17.8 | 36 | 0 | 0 |
| `heroes-underlord-immortal-ravenous` | underlord, abyssal_underlord | 21 | 0.8 | 8 | 0 | 0 |
| `heroes-undying-archange` | undying | 223 | 13.5 | 13 | 0 | 0 |
| `heroes-undying-grim-reformation` | undying | 6 | 1.0 | 0 | 0 | 0 |
| `heroes-ursa-alpine-stalker` | ursa | 4 | 1.1 | 0 | 0 | 0 |
| `heroes-ursa-crystal-drift` | ursa | 4 | 0.2 | 0 | 0 | 0 |
| `heroes-ursa-feast-of-the-damned` | ursa | 4 | 0.2 | 0 | 0 | 0 |
| `heroes-ursa-immortal-hallowed-hunt` | ursa | 10 | 1.0 | 0 | 0 | 0 |
| `heroes-ursa-roshan-hunter` | ursa | 4 | 0.1 | 0 | 0 | 0 |
| `heroes-ursa-white-and-red` | ursa | 284 | 7.3 | 11 | 1 | 0 |
| `heroes-vengeful-spirit-seraph-scion` | vengeful | 25 | 0.4 | 0 | 0 | 0 |
| `heroes-venomancer-peashooter` | venomancer | 40 | 8.3 | 0 | 0 | 0 |
| `heroes-venomancer-poisonous-assassin` | venomancer | 5 | 0.6 | 0 | 0 | 0 |
| `heroes-viper-butch-dog` | viper | 209 | 20.5 | 27 | 0 | 0 |
| `heroes-viper-elephant-hawk-moth` | viper | 28 | 3.4 | 12 | 0 | 0 |
| `heroes-viper-stuffed-menace` | viper | 18 | 3.5 | 0 | 0 | 0 |
| `heroes-visage-azure-phantom` | visage | 199 | 9.9 | 0 | 2 | 0 |
| `heroes-visage-grimfeather-orpse` | visage | 115 | 2.2 | 0 | 1 | 0 |
| `heroes-void-morphling` | morphling | 810 | 25.1 | 19 | 0 | 0 |
| `heroes-void-spirit-hidden-vector` | void_spirit | 83 | 1.5 | 0 | 0 | 0 |
| `heroes-void-spirit-keqing` | void_spirit | 851 | 27.8 | 1 | 2 | 0 |
| `heroes-warden-phantom-assassin` | phantom_assassin | 519 | 17.3 | 6 | 23 | 0 |
| `heroes-warlock-puppet-bear` | warlock | 16 | 1.1 | 0 | 0 | 0 |
| `heroes-weaver-kurumi` | weaver | 328 | 12.5 | 3 | 10 | 0 |
| `heroes-white-natures-prophet` | furion | 278 | 3.4 | 6 | 0 | 0 |
| `heroes-white-shadow-shaman` | shadowshaman | 137 | 2.4 | 14 | 0 | 0 |
| `heroes-windranger-autumn-arcana` | windrunner | 162 | 31.4 | 24 | 0 | 0 |
| `heroes-winter-ember-spirit` | ember_spirit | 711 | 29.9 | 26 | 0 | 0 |
| `heroes-winter-lycan` | lycan | 287 | 25.3 | 29 | 0 | 0 |
| `heroes-wraith-king-arcana` | wraith_king, skeletonking | 336 | 12.6 | 6 | 2 | 0 |
| `heroes-wraith-king-grim-destiny` | wraith_king | 6 | 0.4 | 0 | 0 | 0 |
| `heroes-wraith-king-itsuka-kotori` | wraith_king, skeletonking | 449 | 7.0 | 5 | 16 | 0 |
| `heroes-wraith-king-pizdec` | wraith_king, skeletonking | 32 | 1.4 | 3 | 0 | 0 |
| `heroes-wraith-king-skeleton` | skeletonking, wraith_king | 251 | 3.0 | 6 | 0 | 0 |
| `heroes-wraith-king-unbroken-arcana` | wraith_king, skeletonking | 40 | 9.3 | 5 | 0 | 0 |
| `heroes-wraith-king-veil-tyrant` | wraith_king | 6 | 0.4 | 0 | 0 | 0 |
| `heroes-young-invoker` | invoker | 182 | 13.5 | 0 | 0 | 0 |
| `heroes-young-invoker-dark-artistry` | invoker | 191 | 16.2 | 3 | 0 | 0 |
| `heroes-zeus-arcana` | zeus, zuus | 19 | 5.6 | 0 | 0 | 0 |
| `heroes-zuljin-troll-warlord` | troll_warlord | 756 | 47.0 | 7 | 14 | 0 |
