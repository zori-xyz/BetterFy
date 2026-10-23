# English text layer Windows test plan

This plan covers the first native Windows pass of the English text layer: an
English-language Dota installed through `game/dota_dutch` with
`-language dutch`, with the player's own English text added to BetterFy's VPK
(see [ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md), "Language slots"). It
follows the evidence rules of [EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md):
every result names the exact artifact, what was actually observed, and what
remains unverified.

**Status: not run on Windows, not run in Dota.** The engine side was exercised on
a Mac with a synthetic game archive (unit tests only); no real Dota archive was
available there. Whether Dota honors `dota_dutch` and shows English text from the
layer is the open question this plan answers. The trigger is one tester report
(the `betterfy` slot installed and did nothing) and the upstream guide, not a
BetterFy observation.

## Before starting

- Windows PC with Dota 2 and BetterFy; note the BetterFy build (artifact name,
  commit) and the Dota build ID from `steamapps/appmanifest_570.acf`.
- In Steam, set Dota 2's language to English (Properties, Language) and let it
  finish updating. Start Dota once, confirm the interface is English, close it.
- Have no BetterFy build installed (restore first) and note what is in
  `game/dota_dutch`, if the folder exists.

## Steps

1. Open BetterFy, connect Dota, pick a small build: one hero skin and one small
   tuning mod.
2. In the language field, "Same as Dota now - English" should be recommended,
   with the explanation that Dota ignores custom language folders. Record what is
   shown. The `English (own folder)` card should be marked as not working.
3. Prepare the build. The review must show a line "English text: N files, X MB,
   from your own game". **Record N and X**: the engine's limits (32,768 files,
   128 MiB) are estimates until this number is known.
4. Install, then let BetterFy set the Steam launch option. Confirm
   `-language dutch` is in Dota's launch options and `game/dota_dutch` holds one
   `pakNN_dir.vpk` only; record its size.
5. Start Dota. Check, and record with screenshots:
   - the interface text is English (main menu, hero pages, settings), not Dutch;
   - the hero skin is visible on the hero page;
   - anything shown in Dutch or as a raw token (`#DOTA_...`). Such text means the
     layer is not enough on its own: Minify's English Fix also overwrites
     `*_dutch.txt` on disk under `game/core` and `game/dota_addons`, which BetterFy
     does not do. Record exactly which screens are affected.
6. Close Dota and restore in BetterFy. Check that `dota_dutch` is gone (or back
   to what it held), the `-language dutch` option is removed from Steam, and
   `game/dota/pak01_dir.vpk` is untouched (compare size and date).
7. Prepare the same build again without changing anything: the plan ID in the
   journal line must be identical.
8. After a Dota update (when one comes), prepare again: a different plan ID is
   expected if the English text changed, and BetterFy must ask for a new review.

## Also check

- A Dutch-language Dota (or an English one with Dutch picked by hand) installs
  without the English text line.
- Picking `English (own folder)` shows its warning. Optional: install it and note
  whether anything shows in game, to learn whether it is impossible on this build.

## What to hand back

The BetterFy build, the Dota build ID, N and X from step 3, the size of the
installed VPK, screenshots of steps 5 and 6, and the plain result: English shown /
Dutch shown / mixed, skin visible yes/no, restore clean yes/no.
