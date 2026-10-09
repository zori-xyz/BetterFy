# Skin Windows test plan: Scarlet Keeper of the Light

This plan covers the first native Windows pass of a third-party hero skin
(wardrobe content) through the verified install pipeline: the one allowlisted
archive, installed alone, played, restored, and then installed together with
tuning packages. It follows the evidence rules of
[EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md): every result names the
exact artifact, what was actually observed, and what remains unverified. A
BetterFy success state is not proof that Dota loaded a change; only an in-game
observation is.

**Status: not run in Dota.** Nothing in this plan has been run on Windows. The
engine side was exercised on a Mac only (unit tests, and a local run against
the real archive; see "What the Mac runs already show"). Results are recorded in
the table at the end, item by item.

## Artifact under test

Fill in before starting; do not test an artifact that is not recorded here.

- Branch: `feature/skin-engine`
- Source commit: `<record>`
- Workflow: Windows build, run `<record>` (started with
  `gh workflow run windows-build.yml --ref feature/skin-engine`)
- Artifact: `BetterFy-Windows-x64-nsis`, installer file name and SHA-256
  `<record>`
- Skin: catalog ID `heroes-scarlet-keeper-of-the-light`, engine ID
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
  stored once (58 MB), the build is staged (53 MB), backed up and deployed.

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

## 1. The skin alone

1. Catalog, Heroes: the Scarlet Keeper of the Light card carries the Windows
   pilot mark. Cards of other skins do not. Add it to a build. The Build
   screen's header should say Windows pilot, not preview only.
2. Prepare the build with the network on. This is the first real download of
   the archive through BetterFy's transport. Record how long it takes, and on
   failure the exact error code (for example `download_redirect_blocked`,
   `download_peer_unverified`, `download_hash_mismatch`).
3. Check `%APPDATA%\app.betterfy.desktop\content-v1\objects\sha256\`: exactly
   one new 58,096,981-byte object named `12137a74...a190`, and no `.part`
   leftover. The ZIP itself is not stored.
4. Read the review block (Skin in the build). Expected, from the Mac run:

   | Field | Expected |
   | --- | --- |
   | Into the build | 951 files, 50.8 MB (53,280,352 bytes) |
   | Hero files | 214 |
   | Author's files | 737 |
   | Shared game files | 45 left out, each listed |
   | Dropped | 14, all uncompiled `.vpcf`, each listed |
   | Overlaps | none |

   Open both lists and read them. The shared list must not contain any path
   under `models/heroes`, `particles/units/heroes/hero_keeper_of_the_light` or
   `darkness/`. Record any difference from the table.
5. The plan's VPK size and hash should equal what the Mac produced:
   53,327,413 bytes, SHA-256
   `c64e4a2913fc83ac111dac259faeb24b3acb1a1edf3baaa37142c555533bdcfa`, plan ID
   `sha256:327e5d7f140805159422bd713948918c95c1380b47f4bdbcf945d45a27cce4aa`.
   After the install in step 6 the installed
   `game/dota_<language>/pak66_dir.vpk` must have the same SHA-256. A mismatch
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
   - Spirit Form specifically: its status effect file is among the shared files
     that were left out, so note whether it looks wrong.
8. Check what was not meant to change. Look at two or three other heroes'
   attacks and spells, and at metal or glossy items and heroes. The stripped
   shared files (`particles/basic_*`, `materials/default`, cubemaps) must
   still be the game's own. Note any other hero that looks different.
9. Note every visible failure of the skin: invisible effects, missing or
   checkerboard textures, a crash, a hang on load. This is the answer to the
   open question in [SKIN_ARCHIVE_AUDIT.md](SKIN_ARCHIVE_AUDIT.md): whether the
   skin looks right without the 45 shared files, and whether it works in the
   current Dota patch at all. If it does not, do not conclude the engine is
   wrong: save the screenshots and stop.
10. Quit Dota. In BetterFy restore the build. Check that the installed
    `pak66_dir.vpk` is gone (or equal to the machine record), that the launch
    options equal the machine record, and that the evidence report says the
    rollback was verified. Start Dota once more and confirm Keeper of the Light
    is back to the game's own look.
11. Repeat install then restore twice more. Every install must produce the same
    `pak66_dir.vpk` hash.

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

## What to capture

For every step above that touches a file or the game:

- the in-app evidence report (saved, with its file name);
- SHA-256 before and after of `game/dota_<language>/pak66_dir.vpk`, the backup,
  and the stored archive object;
- the review block as a screenshot (counts and both lists), once per plan;
- the plan ID shown in the journal, to compare with the table in section 1;
- screenshots or a recording of each in-game check, labelled with the step;
- the exact error code of any failure, copied from the interface or journal;
- who observed it: the founder in game, the tester through screen access, or
  only the app's report. Keep these distinct.

## What the Mac runs already show

Run on a Mac, not Windows, and not in Dota:

- the analyzer's findings on the real archive and the pinned identities (see
  [SKIN_ARCHIVE_AUDIT.md](SKIN_ARCHIVE_AUDIT.md));
- extraction of the pinned VPK from the real ZIP, and a build of the 951 files
  into one 53,327,413-byte VPK, byte-identical on a second build;
- the redirect response of the real download URL, seen once with a `HEAD`
  request. The engine's own download of it has not been run.

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
| 1.9 Failures and the shared-files question | NOT RUN (not run in Dota) | | |
| 1.10 Restore and clean state | NOT RUN | | |
| 1.11 Three install and restore cycles | NOT RUN | | |
| 2.1 Skin with Tree Mod and Show NetWorth | NOT RUN (not run in Dota) | | |
| 2.2 Conflict list and confirmation | NOT RUN | | |
| 2.3 Conflicting build in Dota | NOT RUN (not run in Dota) | | |
| 3.1 Cancel during download | NOT RUN | | |
| 3.2 Network off | NOT RUN | | |
| 3.3 Missing and tampered store object | NOT RUN | | |
| 3.4 Close before restore | NOT RUN | | |
| 3.5 Stress install | NOT RUN | | |
| 3.6 Dota or Steam running | NOT RUN | | |
