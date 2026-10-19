# Map terrain Windows test plan

This plan covers the first native Windows pass of the standard map terrains of
the Dota2PornFx catalog (whole-map packages installed as
`game/dota_<language>/maps/dota.vpk`) through the verified install pipeline. It
follows the evidence rules of [EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md)
and the shape of [SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md): every
result names the exact artifact, what was actually observed, and what remains
unverified. A BetterFy success state is not proof that Dota loaded a change; only
an in-game observation is.

**Status: not run on Windows, not run in Dota.** Nothing in this plan has been
run through BetterFy on Windows. The engine side was exercised on a Mac only (unit
tests, and local runs against the 12 real archives; see "Facts from the Mac
runs"). The founder reports that VPKs of this catalog copied by hand into the
language folder work; there is no separate report about the map packages, and
BetterFy's way of writing the file (backup, journal, write-through replace,
folder handling) is what this plan tests. How the design works is in
[ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md), "Map terrains"; what was audited
is in [MAP_AUDIT_REPORT.md](MAP_AUDIT_REPORT.md).

What the upstream guide says (https://d2pfxwiki.pages.dev/en/troubleshoot.html,
"Issues with terrains", and each ZIP's `Guide.txt`), and what this plan therefore
expects to observe, none of it confirmed by BetterFy yet:

- The map goes into the language folder as `maps/dota.vpk`; `dota.vpk` is not
  moved or renamed.
- A standard terrain shows only in matches, not in Demo mode.
- The Dota+ terrain must be switched off in the game settings, or it conflicts
  with the map.

BetterFy's own risk statement, shown in the build review: the file replaces the
game's map package as a whole, so a Dota patch could make it stale. That is an
unverified risk. This plan does not try to age the file; it records the Dota build
number so a later failure can be compared.

## Artifact under test

Fill in before starting; do not test an artifact that is not recorded here.

- Branch: `feature/map-terrains`
- Source commit: `<record>`
- Workflow: Windows build, run `<record>` (started with
  `gh workflow run windows-build.yml --ref feature/map-terrains`)
- Artifact: `BetterFy-Windows-x64-nsis`, installer file name and SHA-256
  `<record>`
- Map used in most steps: catalog ID `terrains-ti6-immortal-gardens`, engine ID
  `map.ti6-immortal-gardens`
  - Pinned ZIP `TI6 Immortal Gardens.zip`: 20,219,973 bytes, SHA-256
    `197ae67961dbd9345346f531d7e6269fe359a1ecd0c4393a2abf2be4af071561`
  - Pinned `maps/dota.vpk`: 35,428,003 bytes (33.8 MB in the review), SHA-256
    `8c421a47c862f7b06aaaa66e4f458843c61d874303a48d44ce868df083f57754`,
    1,291 entries
- Second map (update and swap): `terrains-ti5-desert-terrain`,
  `maps/dota.vpk` 36,768,159 bytes, SHA-256
  `39462e9cee057a623be04c8d13ec97e2dbbde5ddd39cc8f420d034665853ff7b`
- Hero skin for the pak test: `heroes-scarlet-keeper-of-the-light`
  (`wardrobe.scarlet-keeper-of-the-light`); its installed pak is
  57,904,917 bytes, SHA-256
  `69de05a590aa33a01479b166034198ef360ca554c7a0efbf530967a57fcd586b`
  (see [SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md))
- Optional large file: `terrains-crownfall`, 53,299,338 bytes

## Ground rules

- Read the repository's working rules first. They apply unchanged.
- The founder signs in (Telegram, Steam) and enters every password personally.
- BetterFy never force-terminates Dota or Steam and never launches Dota. Dota is
  always started by hand from Steam.
- Use a language slot already observed in game on Windows (`russian` or `dutch`).
- Record SHA-256 of every file BetterFy writes or restores
  (`Get-FileHash -Algorithm SHA256`) before and after each step, and save the
  in-app evidence report after every install and restore.
- One change at a time. When a step fails, stop, save the evidence report and the
  safe diagnostics report, and do not continue on a dirty state.
- Plan for about 400 MB of free space on the drive holding
  `%APPDATA%\app.betterfy.desktop` and on the Dota drive: the pinned VPK is stored
  once (35 MB), staged (35 MB), backed up when it replaces something, and
  deployed; the skin pairing adds 58 MB of the same four copies.

## 0. Machine record

Record once, before any test:

- Windows edition and build (`winver`), CPU architecture, the Dota 2 build number
  Steam shows (Steam: Dota 2, Properties, Updates, or the `buildid` in
  `steamapps\appmanifest_570.acf`), install path, Steam library layout, game
  language, and the language slot used here.
- A recursive listing with hashes of `game\dota_<language>\` before the first
  install, **including `maps\`**: `Get-ChildItem -Recurse -Force | Get-FileHash`.
  Note any existing `maps\dota.vpk` and where it came from. Minify's `d2pfx`
  browser also writes `maps\dota.vpk` into a language folder, so a machine that
  used it may already have one; BetterFy treats it as someone else's file (step 7a
  expects exactly that).
- Whether a BetterFy build is already installed. Restore it first.
- The Dota+ terrain setting in the game (turn it off for the in-game steps).

## 1. A map alone: review and download

1. Catalog, Terrains: the twelve standard terrains carry the Windows pilot mark
   and the other eight (raw `pakNN_dir.vpk`) follow whatever the generic catalog
   path says. Open the TI6 Immortal Gardens card: the detail dialog says it is a
   whole map, that a BetterFy install has not been run in Dota, that it shows only
   in matches, and that Dota+ terrain must be off. Add it to a build alone.
2. Prepare the build with the network on and Dota connected. This is the first
   real download of the archive through BetterFy's transport (GitHub first, then
   Hugging Face). Record the time, the mirror if the log shows it, and on failure
   the exact error code.
3. `%APPDATA%\app.betterfy.desktop\content-v1\objects\sha256\` gets exactly one new
   35,428,003-byte object named `8c421a47...7754`, no `.part` leftover. The ZIP is
   not stored.
4. The review block "Map in the build" should read:

   | Field | Expected |
   | --- | --- |
   | Where it is written | `game/dota_<language>/maps/dota.vpk` |
   | File size | 33.8 MB |
   | Files inside the map package | 1291 |
   | Files also in Valve's pak01 list | none (1291 paths checked) |
   | Map SHA-256 | `8c421a47c862f7b0…` |
   | Archive SHA-256 | `197ae67961dbd934…` |

   and the three notes: what a map is and that it is one file beside the pak, the
   "What to know about maps" box (matches only, Dota+ off, whole-file replacement
   as an unverified risk, install not run in Dota yet), and the foreign-file
   promise. The step list says "map" with 33.8 MB, not a file count. No skin block.
5. Plan ID, from the Mac run:
   `sha256:088da2ad7f18dd002b9e8938281244a8ba170aab354235ddd41d071e35111cf6`. A
   different ID is a finding.

## 2. A map alone: install, hash and journal

1. Language, Steam profile, install, with Dota and Steam closed by BetterFy.
2. The journal reads, in order, among others: folder check, shutdown, "Writing the
   map: maps/dota.vpk", "Build written, backup verified" with
   `dota_<language>/maps/dota.vpk` in its detail, "Wrote the map maps/dota.vpk
   into dota_<language>", the Steam steps. Copy it.
3. On disk: `game\dota_<language>\maps\dota.vpk` is 35,428,003 bytes and its
   SHA-256 is `8c421a47...7754`. **No** `pakNN_dir.vpk` was written by this
   install. No `.betterfy-*.tmp` is left in `maps\` or the language folder.
   Whether `maps\` and the language folder existed before is recorded in step 0;
   BetterFy created only what was missing.
4. The saved evidence report: the newest entry has `"hasPak": false`, `"vpkName":
   ""`, `"installedSha256"` equal to the map's hash, `"phase": "committed"`,
   `"active": true`, and `"map": {"file": "maps/dota.vpk", "beforeSha256": null,
   "installedSha256": "8c421a47...", "backupVerified": true, ...}`. The report
   holds no path.

## 3. A map alone: in the game

With the Dota+ terrain off, start Steam and Dota by hand.

1. Start a bot match (Play, Practice with bots) and look at the base, the river,
   the trees and the lanes at the start of the match. Take screenshots of the same
   spots with the map and, after step 4, without it. Record which terrain the
   match shows and whether anything is missing or broken (floor, trees, cliffs,
   lighting, minimap).
2. Start Demo mode (Hero Demo). The upstream guide says a standard terrain does
   **not** show there; record what is seen.
3. Turn the Dota+ terrain on for one match and record the result (upstream says
   it conflicts); turn it off again.
4. Record the Dota build number and whether Dota logged or showed any error on
   start. A BetterFy success state with an unchanged map in game is a finding.

## 4. Restore

1. "Restore the game" (BetterFy closes Steam and Dota gracefully).
2. `maps\dota.vpk` is gone; if the step 0 listing showed no `maps\` folder, it is
   gone too, and if it showed no `dota_<language>` folder, that is gone as well.
   Anything else in those folders (a file you put there) is untouched and the
   folders stay.
3. The folder listing with hashes equals the step 0 listing.
4. The evidence report gets the rolled-back entry with `"rollbackVerified": true`
   for the map; the journal says the original files are back.
5. Start Dota and play a bot match: the normal terrain is back.

## 5. A map beside a hero skin

1. Build: Scarlet Keeper of the Light and TI6 Immortal Gardens. Prepare. The review
   shows both blocks. Plan ID, from the Mac run:
   `sha256:052c8f98631e74c4cb07c53c4b20fa51ee4794c77936e200208599b85759318c`.
   The merged VPK is the same as with the skin alone: 57,904,917 bytes, SHA-256
   `69de05a590aa33a01479b166034198ef360ca554c7a0efbf530967a57fcd586b`. The map
   does not change it; only the plan ID changes.
2. Install. Two files are written: `pakNN_dir.vpk` (the name in the receipt, SHA-256
   `69de05a5...`) and `maps\dota.vpk` (`8c421a47...`). Journal: "Using
   pakNN_dir.vpk in dota_<language>" and "Wrote the map ...". Report: `"hasPak":
   true`, `"vpkName": "pakNN_dir.vpk"`, the map object as in step 2.
3. In game: the skin on the hero page and in a match, and the map in the match.
   Record both separately.
4. Restore: both files are restored together; the listing equals step 0.

## 6. Updates and the chain

Each step is a normal install over the previous one, with Restore tried after
each. Record hashes at every step.

1. Skin alone installed. Add TI6 to the build and install: the pak keeps its name;
   the map is new (backup of the pak only).
2. Swap TI6 for TI5 Desert Terrain (`39462e9c...`), install: the map is replaced,
   with a backup of the TI6 file. (The catalog puts all terrains in one slot: with
   both in the build, the screen asks to keep one, and the engine refuses two maps
   as `map_terrain_conflict`.)
3. Remove the map from the build, install: BetterFy removes `maps\dota.vpk`, with a
   backup; the pak stays. The evidence entry's `map.installedSha256` is `null`.
4. Restore the last install: the TI5 file is back, byte for byte
   (`39462e9c...`). Restore again: TI6 is back. Again: the file is gone and the pak
   is as at step 1.

## 7. Failure cases

Record the notice shown, the exact code under Details, and the file facts after
each. Nothing here may change a file BetterFy did not write.

a. **A foreign map.** Put a file at `game\dota_<language>\maps\dota.vpk` (create
   `maps\` first; copy any harmless file, record its hash). Prepare and install a
   map build. Expected: before Steam and Dota are closed, the notice "A map from
   someone else is already there" (`map_target_foreign`) with the file's size under
   Details; the file is byte-identical afterwards; nothing else was written (also
   no pak file when the build had a skin); the Steam profile is untouched; no new
   operation appears in the report. Move the file elsewhere, press Retry: the
   install works. Restore: the moved file is not touched or recreated.
b. **A map that BetterFy wrote, changed behind its back.** After an install, append
   a byte to `maps\dota.vpk` (or delete it). Expected: install says "BetterFy's map
   in the game changed" (or "is gone"), Restore says the same with the restore
   wording (`map_conflict` / `rollback_conflict`), neither overwrites the file;
   "Let go of the old install" works and the file stays as it is. A new install
   then treats that file as foreign (a). Note whether a pak file BetterFy also owns
   stays in the folder (it does: letting go forgets, it deletes nothing).
c. **A `maps` that is not a folder.** Rename a file to `maps` in the language
   folder. Expected: `map_folder_invalid`, nothing written.
d. **A link.** `mklink /J maps <another folder>` in the language folder. Expected:
   `deployment_path_unsafe`, nothing written, the other folder untouched.
e. **Interrupted writes** (developer account, `internal-stress-test` build): run
   both stress tests with a build that has a skin and a map. "Interrupt before
   publish" and "Interrupt after publish" must both report PASS; afterwards no
   `.betterfy-*.tmp` is left in the language folder or `maps\`, a `maps\` folder
   BetterFy created is gone, and the listing equals step 0.
f. **Dota open.** Start an install with Dota running: `runtime_busy`, nothing
   written.
g. **The two stress tests on a map-only build** (no pak file): same expectations.

## What to capture

- The evidence report JSON after every install, restore and failure (the in-app
  Copy button, or `Folder` in the report section), with the build number.
- The journal text of every step (Copy).
- `Get-FileHash` and a recursive listing of `game\dota_<language>\` before and
  after every step, and of `%APPDATA%\app.betterfy.desktop\engine-v1\` once at the
  end (`game-deployment\journals`, `operations\*\before*.vpk`).
- Screenshots: the review block, the notice of each refusal, the match with and
  without the map, Demo mode with the map installed.
- Timings of Prepare and Install, and BetterFy's peak memory while preparing and
  installing the largest map (Crownfall, 53 MB), unmeasured so far. The engine
  reads each file whole into memory a few times; a 53 MB file is expected to be
  fine, not measured.
- The Dota build number at each in-game observation.

## Facts from the Mac runs

All 12 ZIPs were fetched with `curl` (GitHub), and every one was fetched again from
Hugging Face and compared by SHA-256: byte-identical. Each ZIP holds exactly
`maps/dota.vpk` and the same `Guide.txt` (3,292 bytes, SHA-256
`716b14d5e198b98d748e5478318fa3fde1bd40becebc977007bd87b3cf10403f`). Every VPK is
version 2 with all entries embedded, every entry CRC checks, no section after the
data except Valve's 48-byte checksum block, every path under `maps/`, 12 file
types (compiled maps, models, textures and map data; no script type), and none of
its 1,214 to 2,308 paths is in Valve's `pak01_dir.txt` list. Planning a single map
from the content store took 0.41 to 0.65 s in a release build on a Mac.

| Engine ID | `maps/dota.vpk` bytes | SHA-256 | Entries | Plan ID (the map alone) |
| --- | ---: | --- | ---: | --- |
| `map.crownfall` | 53,299,338 | `f2447a6703d72053e1c307775f8418025912829426f996fcb66de6f6b7d9d8ed` | 1,895 | `sha256:a334c2275abebec71c15d3293e212039147343caa454a8bf845ee2e50b1c5ef5` |
| `map.dota-autumn` | 34,537,287 | `4c491002520fd207cb4206d600fd13d316549a4b0a8b3768f336b0934fa3639f` | 1,214 | `sha256:adccfec0825c61b70857cbe45e68d24bff3f8c8518d45ea0ea9241057ef508df` |
| `map.dota-spring` | 35,460,094 | `d05f96d135c53acc4a04b36948e842bbc7148961fd07f171d4d030350d003856` | 1,282 | `sha256:f8e9cf92263a3b6961a3818d22c7bdcf94338bca6ed01c13cd3fbbd565c17055` |
| `map.dota-summer` | 34,778,111 | `d3aa35cf139c33c44bedbb1ab5c9428be5073b6294b62ee3a30227debcd5697c` | 1,214 | `sha256:0ac1082480b20972df32d89ec0c76f652c99b0f037a2d0d7916ce5f030fc5ce7` |
| `map.dota-winter` | 37,622,865 | `8cade537206904aaa6b27606fd184d3461210f4a79f5e6dec33294181334106d` | 1,336 | `sha256:1d21ad9ade7411dd56fc62d87fbe4d81a3b98b398ad02de472348dd7066a581a` |
| `map.the-kings-new-journey` | 36,802,575 | `a854fd446849a8e2ee8704a1fc59bac71eb4566ff36218efa38785d35de2f187` | 1,354 | `sha256:80b155717c478b14d68b52a87d4637ed61eb06347b0dd4864f2d8a0e8d9a899b` |
| `map.ti10-sanctum-of-the-divine` | 39,027,093 | `b13f961c613dc98f326a42ffa284655bfa44559d850634ffad9e487a83689964` | 1,375 | `sha256:3c4287cb531ab6b7c4feb0b6804ff7aa67937578e1032e6781b599f7d15f750c` |
| `map.ti5-desert-terrain` | 36,768,159 | `39462e9cee057a623be04c8d13ec97e2dbbde5ddd39cc8f420d034665853ff7b` | 1,337 | `sha256:5efa63e946d5df6bfdee6b58fcfab7e4261828445d190749c5d3087af68245b3` |
| `map.ti6-immortal-gardens` | 35,428,003 | `8c421a47c862f7b06aaaa66e4f458843c61d874303a48d44ce868df083f57754` | 1,291 | `sha256:088da2ad7f18dd002b9e8938281244a8ba170aab354235ddd41d071e35111cf6` |
| `map.ti7-reefs-edge` | 39,998,774 | `03706dc6a079fc56b1b89a970ee9c10b40f7c19197a2eb7ce00ad0e720de67af` | 2,308 | `sha256:b79ed3bb257e6dc1da19fd8757cdc858b32fb76ce70f8217bf2b839c9cf0a087` |
| `map.ti8-emerald-abyss` | 35,590,789 | `2a849a5fe789da4be79a1547ca0ff4521eb3f6119890574b29935b03ad6a5fc1` | 1,274 | `sha256:1ca654336eb9d3a1595d32ed59a127f12caf359e1c470b6c6210d5c65e88655f` |
| `map.ti9-overgrown-empire` | 37,514,537 | `6775f7a0e5804bd1875b37f303c86de4ba396c25cda96fa65743e358845a4592` | 1,234 | `sha256:32a6cbdfdd71706db57537682fd2e05d7c4f56c02db5a0ec97cb1853c032c9ac` |

Full audit, including the ZIP hashes and the contents of each package:
[MAP_AUDIT_REPORT.md](MAP_AUDIT_REPORT.md).

## Results

Fill in one row per step as it is run. "Observed" is what was actually seen;
"Not verified" is what the step could not show.

| Step | Artifact | Observed | Evidence kept | Not verified |
| --- | --- | --- | --- | --- |
| 1 Review and download | | | | |
| 2 Install, map alone | | | | |
| 3 In game | | | | |
| 4 Restore | | | | |
| 5 Map beside a skin | | | | |
| 6 Updates and chain | | | | |
| 7a Foreign map | | | | |
| 7b Changed map | | | | |
| 7c `maps` not a folder | | | | |
| 7d Link | | | | |
| 7e Interrupted writes | | | | |
| 7f Dota open | | | | |
| 7g Stress, map alone | | | | |
