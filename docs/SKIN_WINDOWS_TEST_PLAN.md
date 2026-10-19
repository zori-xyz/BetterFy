# Skin Windows test plan

This plan covers the native Windows passes of third-party hero skins (wardrobe
content) through the verified install pipeline. Round one is Scarlet Keeper of
the Light: installed alone, played, restored, and then installed together with
tuning packages. Round two adds three skins of other shapes. Round three (section 6) tries one item of every other catalog category. It follows the
evidence rules of [EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md): every
result names the exact artifact, what was actually observed, and what remains
unverified. A BetterFy success state is not proof that Dota loaded a change;
only an in-game observation is.

**Status: not run in Dota.** Nothing in this plan has been run on Windows
through BetterFy. The engine side was exercised on a Mac only (unit tests, and
local runs against the real archives; see "What the Mac runs already show").
The founder reports that skin VPKs copied by hand into the language folder show
the full new look on Windows; that is evidence about the files, not about
BetterFy's merged install, which is what this plan tests. Results are recorded
in the tables at the end, item by item.

By default a skin is installed with **every file the analyzer accepts**,
including the shared game files it overrides. A per-skin switch in the build
review, "Keep the game's shared files untouched", is off by default and leaves
those files out. Both modes are tested below, because the second is an opt-in
that may change how the skin looks.

## Artifact under test

Fill in before starting; do not test an artifact that is not recorded here.

- Branch: `feature/wardrobe-catalog`
- Source commit: `<record>`
- Workflow: Windows build, run `<record>` (started with
  `gh workflow run windows-build.yml --ref feature/wardrobe-catalog`)
- Artifact: `BetterFy-Windows-x64-nsis`, installer file name and SHA-256
  `<record>`
- Round one skin: catalog ID `heroes-scarlet-keeper-of-the-light`, engine ID
  `wardrobe.scarlet-keeper-of-the-light`
- Pinned ZIP: `Scarlet Keeper.zip`, 24,071,988 bytes, SHA-256
  `f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7`
- Pinned VPK inside it: `pak59_dir.vpk`, 58,096,981 bytes, SHA-256
  `12137a743ff91c8dc1341e32718580109f86c70df735e24b022929e38cc9a190`

## Ground rules

- Read the repository's working rules first. They apply unchanged.
- The founder signs in (Telegram, Steam) and enters every password personally.
- BetterFy never force-terminates Dota or Steam and never launches Dota. Dota is
  always started by hand from Steam.
- Use the language slot already observed in game on Windows (`russian` or
  `dutch`), so a failure here is not confused with the open question of other
  slots.
- Record before and after SHA-256 of every file BetterFy writes or restores
  (`Get-FileHash -Algorithm SHA256`), and save the in-app evidence report for
  every install and restore.
- One change at a time. When a step fails, stop, save the evidence report and
  the safe diagnostics report, and do not continue on a dirty state.
- Plan for about 300 MB of free space on the drive that holds
  `%APPDATA%\app.betterfy.desktop` and on the Dota drive: the pinned VPK is
  stored once (58 MB), the build is staged (58 MB), backed up and deployed.
  The big skin of round two needs about 20 MB more per copy.

## 0. Machine record

Record once, before any test:

- Windows edition and build (`winver`), CPU architecture.
- Dota 2 install path, Steam library layout, game language, and the language
  slot used here.
- Contents of `game/dota_<language>/` before the first install (file list and
  hashes), and the current Steam launch options for the test profile.
- Whether a BetterFy build is already installed. Restore it first, so this plan
  starts from the machine record.
- Whether `%APPDATA%\app.betterfy.desktop\content-v1\objects\sha256\` already
  holds `12137a74...a190` (it should not on a fresh machine).

## 1. Scarlet Keeper of the Light alone (default mode)

1. Catalog, Heroes: the Scarlet Keeper of the Light card carries the Windows
   pilot mark, and so do the other hero and hero-item skins that passed the
   audit (500 cards in all). Add it to a build. The Build screen's header should
   say Windows pilot, not preview only.
2. Prepare the build with the network on and Dota connected. This is the first
   real download of the archive through BetterFy's transport (the first mirror,
   GitHub, then Hugging Face if that fails). Record how long it takes, which
   mirror served it if the log shows it, and on failure the exact error code
   (for example `download_redirect_blocked`, `download_peer_unverified`,
   `download_hash_mismatch`, `download_http_failed`).
3. Check `%APPDATA%\app.betterfy.desktop\content-v1\objects\sha256\`: exactly
   one new 58,096,981-byte object named `12137a74...a190`, and no `.part`
   leftover. The ZIP itself is not stored.
4. Read the review block (Skin in the build). Expected, from the Mac run:

   | Field | Expected |
   | --- | --- |
   | Into the build | 996 files, 55.2 MB (57,855,637 bytes) |
   | Hero files | 214 |
   | New files (not in the game) | 755 |
   | Also replaces shared game files | 27, each listed |
   | Dropped | 14, all uncompiled `.vpcf`, each listed |
   | Compared with your Dota | yes; "skin files your game ships now" empty or short |
   | Overlaps | none |

   Open the lists and read them. The shared list must not contain any path under
   `models/heroes/keeper_of_the_light` or `particles/units/heroes/hero_keeper_of_the_light`.
   Record any difference from the table, and any "your game ships now" file.
5. The plan's VPK size and hash should equal what the Mac produced:
   57,904,917 bytes, SHA-256
   `69de05a590aa33a01479b166034198ef360ca554c7a0efbf530967a57fcd586b`, plan ID
   `sha256:ba18ccfe2988d4266a7f9baaf145de4822e03597841948a8df8fe4c4eaf56fa3`.
   After the install in step 6 the installed
   `game/dota_<language>/pakNN_dir.vpk` (the name in the receipt) must have the same SHA-256. A mismatch
   is a finding, not a rounding error: the build is meant to be deterministic.
6. Language, Steam profile, install, with Dota and Steam closed by BetterFy
   normally. Save the evidence report. The installed file's SHA-256 and the
   backup's SHA-256 are recorded as in every other pass.
7. Start Steam, then Dota by hand. In the main menu open the hero grid and the
   Keeper of the Light page. Then take control of Keeper of the Light in a hero
   demo or a bot match and watch, with a screenshot or short recording of each:
   - the hero's model (hood, mount, skirt, staff are replaced) and the author's
     ambient effects;
   - the hero portrait in the grid and the 10 spell icons in the HUD (these are
     the 11 image wrappers);
   - the base attack, and each ability once (Illuminate, Blinding Light, Mana
     Leak, Radiant Bind, Recall, Spirit Form, Will-O-Wisp, Chakra Magic);
   - Spirit Form specifically.
8. Check what else changed. The 27 shared files (default textures, cubemaps,
   `net_color`, status effects) are installed in this mode, as they are when the
   VPK is copied by hand. Look at two or three other heroes' attacks and spells,
   and at metal or glossy items and heroes, and note any other hero that looks
   different. Because the founder's hand-copied runs showed the full look, a
   difference between that look and this one is the finding to chase.
9. Note every visible failure of the skin: invisible effects, missing or
   checkerboard textures, a crash, a hang on load. If something fails, do not
   conclude the engine is wrong: save the screenshots and stop.
10. Quit Dota. In BetterFy restore the build. Check that the installed
    `pakNN_dir.vpk` is gone (or equal to the machine record), that the launch
    options equal the machine record, and that the evidence report says the
    rollback was verified. Start Dota once more and confirm Keeper of the Light
    is back to the game's own look.
11. Repeat install then restore twice more. Every install must produce the same
    `pakNN_dir.vpk` hash.

## 1b. Scarlet Keeper with the shared game files left out (opt-in)

1. Prepare the build again and, in the review, turn on "Keep the game's shared
   files untouched". The plan is prepared again; the build's package is now
   `wardrobe.scarlet-keeper-of-the-light.strip`. Expected: 969 files, 51.2 MB
   (53,708,285 bytes), the 27 shared files listed as left out, a different plan
   ID and a VPK of 53,756,163 bytes (the hash is recorded on the machine; if the
   game ships extra files of the skin the plan lists them and is smaller).
2. Install, start Dota, and compare with section 1: models, icons, abilities,
   Spirit Form, textures of metal and glossy things. Note exactly what looks
   different, if anything. This is the question the opt-in answers: whether
   leaving those files out changes the look.
3. Restore and confirm the machine record. Turn the switch off again and check
   that the plan returns to the 996-file plan of section 1.

## 2. The skin with tuning packages

1. Build: Scarlet Keeper of the Light, Tree Mod and Show NetWorth. Prepare. The
   review should show the skin block and no conflict block. Install, start
   Dota, and check that the trees and the net worth display (the known
   pilot packages) and the skin are all there. Restore and compare with the
   machine record.
2. Conflict handling. Add a tuning package that may write the same paths as the
   skin (Minify Base Attacks or Minify Spells & Items replace spell and attack
   effect files with blanks; whether they hit Keeper of the Light's paths is
   not known). Prepare and read the plan.
   - If the review lists conflicting paths, the install button must stay
     disabled until the confirmation box is ticked. Tick it, and note which
     package keeps each path (the first one in the build). Then swap the order
     in the build and record whether the winner swaps.
   - If the review lists none, record that, and that the plan's "Overlaps"
     count agrees.
3. Install the conflicting build only if the conflict list is understood. In
   Dota, check that the winner's change is what is visible, restore, and
   confirm the machine record.

## 3. Interruptions and failures

1. Cancel during the archive download, close BetterFy, reopen. No object may be
   published for the VPK, and the next preparation must start cleanly.
2. Network off during preparation: expect the retry message and no partial
   object.
3. Delete the stored VPK object from `content-v1\objects\sha256\` and prepare
   again: BetterFy must download and verify it again. Then change one byte of
   the stored object and prepare: it must refuse it (`content_store_corrupt`)
   and not install.
4. Close BetterFy after install and before restore, reopen: the installed build
   and its restore action must still be found, and the installed-state check
   must still match (it rebuilds the VPK from the stored archive, so allow a few
   seconds).
5. Run the built-in stress install on a skin-only build, if the account has the
   developer controls (the stress path never acknowledges conflicts, so use the
   skin alone). Recovery must report `PASS`.
6. Install with Dota or Steam running: BetterFy must ask them to close normally
   and refuse if they stay open.
7. Mirrors. With the network on, block `raw.githubusercontent.com` (for example
   in the hosts file) and prepare a build whose archive is not yet stored: the
   download must fall through to Hugging Face and succeed. Undo the block. If
   both are blocked the preparation must fail with the last mirror's error and
   store nothing.

## 4. Round two: three skins of other shapes

Do these after round one has produced a clean machine record. Each is installed
alone in default mode, played, and restored; the numbers are from the Mac
build and are the expected values in the review and in the installed file.

| Skin | Catalog ID | Why | Files / bytes (default) | Shared | Dropped | VPK bytes, SHA-256 | Plan ID |
| --- | --- | --- | --- | ---: | ---: | --- | --- |
| Pudge, Hood of Defiance | `hero-items-pudge-hood-of-defiance` | tiny hero item: one file, no shared file | 1 / 21,799 | 0 | 0 | 21,864; `60606ccaa3b03de4118a05decbb87e99d2973bbcacfd3a9e9396029ec7bab7d9` | `sha256:ec7d8aaac7e142473011457834f321ca918cd4857c4846954891d88adfb58bec` |
| Cursed Shadow Fiend | `heroes-cursed-shadow-fiend` | one hero, two internal names (`shadow_fiend` and `nevermore`) | 339 / 5,322,735 | 13 | 0 | 5,338,879; `432e6c759476c2435609b4e37337c15122bdb5728748b875700cc9d4536f52f0` | `sha256:ebbd44d5298e1f3b975ae491e793c8981526900394a5de40e6edc57ff3e50f60` |
| Bloody Gore Spectre | `heroes-bloody-gore-spectre` | the biggest allowlisted install | 1,126 / 63,934,624 | 20 | 4 | 63,993,491; `162acd1629a922949d24d1ed013dc032c81e2684c1c81486559c4e5b6fa0bc34` | `sha256:ee38299638d39089f1a18ae87f733a25b702381ff5a797f1fdc7fcc0baf80601` |

For each: the card has the pilot mark; the review matches the table; the plan
hash and the installed `pakNN_dir.vpk` hash equal the table (when the review
says "compared with your Dota" and lists "your game ships now" files, the
default build is still identical, because those files are information only);
the skin is visible on the right hero in a demo; restore leaves the machine
record. Extra checks:

- Pudge, Hood of Defiance: the one file is `models/heroes/pudge/hair.vmdl_c`;
  Pudge's hood/hair shows the skin's model. The shared list is empty and the
  opt-in switch is not offered.
- Cursed Shadow Fiend: the review shows both hero names; the abilities and
  particles (the `nevermore` names) are the skin's. Then turn the opt-in on
  (326 files, 4,298,547 bytes, plan `sha256:3fb082653df217c99300d3c4bb7419b0d40127207c183ffbb7e8a5b796d29e29`)
  and compare the look with the default.
- Bloody Gore Spectre: preparation downloads a 26,443,411-byte ZIP and stores a
  64,085,118-byte object; note the time to prepare, install and restore. Select
  it together with Scarlet Keeper (122 MB of file data, under the 256 MiB limit)
  and read the plan: the two skins share no path, so there is no conflict block.
  Then select five of the largest skins (for example this one,
  `heroes-dawnbreaker-death-knight`, `heroes-hell-muerta`,
  `heroes-ghost-void-spirit`, `heroes-marci-atri`): the build screen should warn
  that the skins add about 298 MB against the 256 MB limit, and preparing must
  stop with the "build is too big" notice before downloading anything.
- Conflicts between two skins: select `heroes-shadow-fiend-black` and
  `heroes-shadow-fiend-white` together. The review must list conflicting paths,
  and the install button must stay disabled until the confirmation is ticked.
  The Mac plan counts 244 conflicting paths for these two plus the others in the
  spot check; record the number the Windows review shows for just these two.

## 5. Installing next to Minify's output

BetterFy picks its file name at install time (see "File names and coexistence with
Minify" in `ENGINE_ARCHITECTURE.md`). This pass checks it on a machine that also uses
Minify. Do not use files you care about: work on a copy of the language folder if you
can.

1. With BetterFy not installed, run Minify once so that `game/dota_<language>/` holds
   its `pak66_dir.vpk` (and `pak66_000.vpk`, ... when it chunks). Record the SHA-256
   of every `pak66*` file (`Get-FileHash`) and the folder listing.
2. In BetterFy prepare a build and open the review. Expected: a line beside the file
   name says the folder already has `pak66_dir.vpk` (named like Minify's) and that
   BetterFy will use `pak65_dir.vpk`. Record what it shows.
3. Install. Expected: the journal says "Using pak65_dir.vpk in dota_<language>;
   pak66_dir.vpk ... is left alone", the receipt and the evidence report carry
   `vpkName: pak65_dir.vpk`, and every Minify file has the same SHA-256 as in step 1.
4. Restore. Expected: `pak65_dir.vpk` is gone, every Minify file again has the step 1
   SHA-256, the folder listing equals step 1.
5. Install again, then run Minify so that it rewrites `pak66*`. Open BetterFy: BetterFy's
   `pak65_dir.vpk` must still match its record (the install is intact), and Minify's
   files are not BetterFy's concern. Restore and check the hashes again.
6. The reported case, on a build that installed to `pak66_dir.vpk` before this change
   (the old record has no `vpkName`): run Minify so that it overwrites `pak66_dir.vpk`.
   Expected: the notice says BetterFy's file was replaced by another program that uses
   the same name, names the file, and offers "Let go of the old install"; the file facts
   (name, size, modified time) are under Details. After letting go, install: BetterFy
   takes `pak65_dir.vpk` and Minify's files keep their hashes.
7. Optional: fill every `pak50` to `pak65` number in a folder with dummy files and try
   to install. Expected: `slot_full`, nothing written.
8. Start Dota with `-language <language>` and note whether the BetterFy changes and
   Minify's changes are both visible. Whether Dota loads both, and which wins on a
   conflicting path (the upstream guide says the lower number wins, so BetterFy's
   `pak65`), is not yet observed.

## 6. Round three: one item of every other kind

The non-hero categories (see [CATALOG_CATEGORIES.md](CATALOG_CATEGORIES.md)) install
whole, with no opt-in mode. Each pass below is: add the item alone, prepare, compare the
review with the numbers here, install, check in Dota, restore. The Mac numbers are
from real plans built from the cached archives (one VPK per item, byte-identical on a second
build); they are not a Windows or in-game result. Compare the plan ID and the VPK SHA-256
the app shows with the table.

| Item (catalog ID) | Source | Files | Payload bytes | VPK bytes | VPK SHA-256 | Plan ID |
| --- | --- | ---: | ---: | ---: | --- | --- |
| `mega-kill-diretide` (sound pack) | raw VPK | 15 | 5,619,599 | 5,620,187 | `9553eb3bddde305209a6ae6163350cfc6819b3165e158bb23c88c462de34afae` | `sha256:66032dbe903446cbee082eae9ff457a0dd1b53a952cd45b94c540dce30ce5a35` |
| `couriers-billy-bounceback` (courier) | raw VPK | 4 | 588,750 | 588,916 | `df3b8eecf2fb3317ab77a6f77f44b5e914cc3bfc59f8be5daa414c39bfddf90b` | `sha256:6ed12d11abc418c210c4e5ae5fb4c6c90059c624493c2cf1975ea6f4c8133146` |
| `creeps-nemestice-radiant-creeps` (creep skin) | raw VPK | 34 | 19,402,196 | 19,404,725 | `ff7ae1f69a23c5b5ecb41d6f67bcaa712564e06d5e806905b9b62cbd671ea518` | `sha256:6b5d93db539e94b5b4a0b0898267ea4e5ea35eb955c0f9a91aca67e1b107cb4b` |
| `emblems-sunken-emblem` (emblem) | raw VPK | 1 | 2,361 | 2,441 | `48b0e91d6c59a52e03cd7b4f39335dbf959f1fa136524608a8a1fc51333ba9e7` | `sha256:2ad2706daff782e0828f3b7c0d5c5663778523764b07901f7e4f2a2224288b89` |
| `shaders-aghanim-labyrinth` (shader) | raw VPK | 1 | 2,348 | 2,424 | `960b23aa82a317eb5921bc1d66934d6a4dd67d982e65b696959ed06012f5f1f6` | `sha256:aac94c2599460c7327620eb4de0ddf1d07a75bfde59b5c0ff1a28e6f31e1ca8d` |
| `river-frozen-river` (river) | raw VPK | 18 | 113,184 | 113,977 | `932febe0a03a4d4bda5bdb2cbd676be80e8372cc2108c4387a8712bb7b6c9728` | `sha256:482be9f19f97deff8174bc463cb8cc625bdf03181b27d1e52d47831da7018662` |
| `trees-lowpoly-shrub-trees` (trees) | raw VPK | 34 | 236,288 | 237,657 | `1b422e5cca94bf93d283398f2e741abdd45ba23f8caa7e4eee49a868eb709684` | `sha256:9c353cb07d01e9d9e39610cb320b04a152522cd145ab2dee7560c84504325585` |
| `item-effects-ti6-bottle` (item effect) | ZIP | 2 | 2,600 | 2,765 | `db4047058e5458c33fc8bbd96508110e9ddacbaaf2c31952bd4b73163f5a468f` | `sha256:d54a3b143003590f6ea19ab4e8412f23f949f7b6e1b676fe3b4dbf798d7d119e` |
| `ranks-normis-rank-icons` (rank icons) | raw VPK | 20 | 618,152 | 618,770 | `0fc2ced607051ac08bf9ce09b0a72e3246f481f1e74f92bd6ce45539ada555c8` | `sha256:d13ae283728c35cb60a4390e2f05cc69ea63ebce3b315a2949d2ed8f47c73adc` |
| `hero-sounds-sven-already-red` (hero sound) | ZIP | 1 | 70,689 | 70,768 | `878310605291820b6134140cfef450a92b8d66b579b818f4cce131151b965755` | `sha256:fe1653485e55c462bd2edf1432f5fb6fcb128d64493ee53a241ab08125564be0` |

For every item:

1. Add only that item. In the catalog card and the detail dialog expect the
   "Windows pilot" mark, the category help line and the note that it has not been run in
   Dota. For the build review expect: no hero line, no "Keep the game's shared files
   untouched" switch, the file and size counts above, and the list of game files it replaces.
2. Prepare. Record the download time (the first time the archive comes through BetterFy's
   own transport) and whether the plan ID and VPK SHA-256 equal the table.
3. Install, then start Dota with `-language <language>` and check the item:
   - sound pack and hero sound: play or trigger the sound (a bot match; a Mega Kill needs a
     kill streak, say so if you cannot trigger one);
   - courier, creep skin, river, trees: look at it in a bot match or the demo;
   - emblem: select your hero; if it does not show, run `r_draw_selected_ring 0`, then
     `r_draw_selected_ring 1` in the console before the match and make sure Minify's Misc
     Optimization mod is off (upstream advice), and record whether that helped;
   - shader: look at the changed materials; if it looks checkered, turn off World Lighting
     (upstream advice) and record whether that helped;
   - item effect: spawn the bottle in the demo and use it;
   - rank icons: open the profile or rank screen.
4. Restore and confirm the folder state equals before the install.

Also:

- A mix: `heroes-bare-kez`, `mega-kill-diretide`, `couriers-baby-roshan-2018`,
  `item-effects-earthshaker-gold-blink-dagger`, `emblems-sunken-emblem`,
  `shaders-aghanim-labyrinth`, `river-frozen-river`, `trees-lowpoly-shrub-trees`,
  `ranks-normis-rank-icons` together. Mac: 140 files, 14,399,530 payload bytes, VPK
  14,406,026 bytes, SHA-256
  `a30feaf958b02a1758900b20b3a5abaa2d7af2812053b97848ad7f7dc88387f2` (the full hash is in
  the Mac log; compare the first 16 hex characters), 0 conflicts.
- Conflicting packs: `mega-kill-nedotrax-mega-kill` with `mega-kill-deus-ex` (Mac: 23
  conflicting paths, install disabled until confirmed) and
  `creeps-woodland-radiant-creeps` with `creeps-diretide-radiant-creeps` (6). Record the
  numbers the review shows and which pack's lines or models Dota uses.
- Size refusal: `music-deadmau5-dieback`, `music-the-international-10`,
  `terrains-simple-dark-terrain-minify`, `trees-metal-barrel-trees` and `heroes-nude-marci`
  together (426,583,923 bytes of file data against the 256 MiB bound): preparing must stop
  with the build-too-big notice before anything is downloaded.
- Largest single packs: `music-deadmau5-dieback` (92 MB VPK) and
  `trees-metal-barrel-trees` (46 MB): record the time and memory use to prepare, install and
  restore.
## 6. Menu backgrounds, HUD skins and versus screens

The three Panorama decor categories are installable and marked Windows pilot (see
[PANORAMA_DECOR_AUDIT.md](PANORAMA_DECOR_AUDIT.md) and
[CATALOG_CATEGORIES.md](CATALOG_CATEGORIES.md)); none has been run on Windows or in
Dota. Minimum run: one of each kind, installed alone, checked in game, then
restored with Restore the game. One item of each kind, chosen to
cover the shapes the audit found. Do each alone first, then restore. Record the
Dota build (`steam.inf`) and whether the Windows machine has a plain (non-ultrawide)
display; backgrounds behave differently on 21:9.

**6a. A background with a video** (suggest `backgrounds-ichigo`, about 7.5 MB).
1. Install. The review should show the proof lines (what was proven, what is not
   proven, how to undo), the files going in (layouts, styles and the one video),
   and `panorama/version.txt` as dropped (file type not installed). Record exactly
   what it shows.
2. Start Dota. Expected: the main menu shows the video behind the dashboard.
   Check, and write down which of these still work, because the archive replaces
   Valve's `dashboard.vxml_c` and `dashboard_page_home.vxml_c` with its own copies:
   the Play, Heroes, Armory, Watch, Learn and Mods tabs, the search box, the
   friends list, the settings button, Dota Plus menu, notifications, the Quit
   button, and opening the Heroes page (the hero loadout background style is also
   replaced). Anything missing, misplaced or unstyled is a finding, not a pass.
3. Does the video loop, and does it have sound? (5 of 61 videos carry audio.)
4. Restore. Expected: the default menu is back, `pakNN_dir.vpk` is gone.

**6b. A HUD skin** (suggest `huds-green-hud`, 7.3 MB, then `huds-blue-web-hud`,
textures only).
1. Install, start a bot match (hero, abilities, inventory visible).
2. Expected: the HUD shows the skin's colours and textures. Write down any element
   that is unstyled, overlapping or illegible, the minimap, ability bar,
   inventory, the Roshan timer and the top bar.
3. Restore, and check the default HUD is back.

**6c. A versus screen** (suggest `versus-screens-winter-versus-screen`, 4.7 KB).
1. Install, start a match with a hero pick and watch the versus screen before the
   loading finishes.
2. Expected: the chosen scene instead of the default. If the default scene shows,
   record it: it is possible that Dota does not read this layout from a mod VPK.
3. Check the debug buttons that the layout contains (Play Scene, Fade Out When
   Complete) are not visible during the normal screen.
4. Restore.

**6d. Failure behaviour.** Take a background archive, change one byte of its
`dashboard.vxml_c` payload in a copy (the audit pins the archive hash, so the
build must stop with the audit mismatch before anything is written). Record the
code shown. This checks the pin, not the proof.

Record for 6a to 6c: what Dota shows, what the review showed, the plan ID, the
evidence report, and whether the main menu was usable throughout.

## What to capture

For every step above that touches a file or the game:

- the in-app evidence report (saved, with its file name);
- SHA-256 before and after of `game/dota_<language>/pakNN_dir.vpk` (the name in the receipt), the backup,
  and the stored archive object;
- the review block as a screenshot (counts and the lists), once per plan;
- the plan ID shown in the journal, to compare with the tables;
- screenshots or a recording of each in-game check, labelled with the step;
- the exact error code of any failure, copied from the interface or journal;
- who observed it: the founder in game, the tester through screen access, or
  only the app's report. Keep these distinct.

## What the Mac runs already show

Run on a Mac, not Windows, and not in Dota:

- the analyzer's findings on the real archives and the pinned identities (see
  [SKIN_ARCHIVE_AUDIT.md](SKIN_ARCHIVE_AUDIT.md) and
  [WARDROBE_AUDIT_REPORT.md](WARDROBE_AUDIT_REPORT.md));
- extraction of the pinned VPK from each real ZIP, and builds of the skins named
  here into one VPK each, with the sizes and hashes above, byte-identical on a
  second build for Scarlet Keeper;
- the mirrors answering `HEAD` requests with the pinned sizes. The engine's own
  download has not been run against the network; the archives were fetched with
  `curl` and their SHA-256 matched the pins.

## Results

Record each item as `PASS`, `FAIL` or `NOT RUN`, with what was actually seen,
the evidence report file name, and hashes where relevant.

| Item | Result | Observed | Evidence |
| --- | --- | --- | --- |
| 1.1 Catalog mark and header | NOT RUN | | |
| 1.2 Archive download | NOT RUN | | |
| 1.3 Content store object | NOT RUN | | |
| 1.4 Review block and lists | NOT RUN | | |
| 1.5 VPK hash equals the Mac build | NOT RUN | | |
| 1.6 Install and backup | NOT RUN | | |
| 1.7 Skin visible in Dota | NOT RUN (not run in Dota) | | |
| 1.8 Other heroes unchanged | NOT RUN (not run in Dota) | | |
| 1.9 Failures | NOT RUN (not run in Dota) | | |
| 1.10 Restore and clean state | NOT RUN | | |
| 1.11 Three install and restore cycles | NOT RUN | | |
| 1b.1 Opt-in plan (969 files) | NOT RUN | | |
| 1b.2 Opt-in look compared with default | NOT RUN (not run in Dota) | | |
| 1b.3 Restore and switch back | NOT RUN | | |
| 2.1 Skin with Tree Mod and Show NetWorth | NOT RUN (not run in Dota) | | |
| 2.2 Conflict list and confirmation | NOT RUN | | |
| 2.3 Conflicting build in Dota | NOT RUN (not run in Dota) | | |
| 3.1 Cancel during download | NOT RUN | | |
| 3.2 Network off | NOT RUN | | |
| 3.3 Missing and tampered store object | NOT RUN | | |
| 3.4 Close before restore | NOT RUN | | |
| 3.5 Stress install | NOT RUN | | |
| 3.6 Dota or Steam running | NOT RUN | | |
| 3.7 Mirror fallback | NOT RUN | | |
| 4.1 Pudge, Hood of Defiance | NOT RUN (not run in Dota) | | |
| 4.2 Cursed Shadow Fiend, default and opt-in | NOT RUN (not run in Dota) | | |
| 4.3 Bloody Gore Spectre | NOT RUN (not run in Dota) | | |
| 4.4 Size limit warning and refusal | NOT RUN | | |
| 4.5 Two skins of one hero: conflicts | NOT RUN | | |
| 6.1 Sound pack (mega-kill) | NOT RUN (not run in Dota) | | |
| 6.2 Courier | NOT RUN (not run in Dota) | | |
| 6.3 Creep skin | NOT RUN (not run in Dota) | | |
| 6.4 Emblem | NOT RUN (not run in Dota) | | |
| 6.5 Shader | NOT RUN (not run in Dota) | | |
| 6.6 River | NOT RUN (not run in Dota) | | |
| 6.7 Trees | NOT RUN (not run in Dota) | | |
| 6.8 Item effect (ZIP) | NOT RUN (not run in Dota) | | |
| 6.9 Rank icons | NOT RUN (not run in Dota) | | |
| 6.10 Hero sound (ZIP) | NOT RUN (not run in Dota) | | |
| 6.11 Nine-item mix | NOT RUN (not run in Dota) | | |
| 6.12 Conflicting packs | NOT RUN | | |
| 6.13 Size refusal | NOT RUN | | |
| 6.14 Largest packs: time and memory | NOT RUN | | |
