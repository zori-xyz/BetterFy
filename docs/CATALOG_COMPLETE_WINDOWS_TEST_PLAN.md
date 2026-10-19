# Complete catalog: Windows test plan

This plan covers everything the "complete the catalog" branch
(`feature/catalog-complete`, pull request #45) adds or changes: the cleaned-up
screens, the 17 newly installable wardrobe archives, the game's own loose files
(cursor packs, font packs, OpenDotaGuides), the five Minify mods that take the
player's input (User Styles, Custom Fonts, Custom Backgrounds, Custom Hero
Grids, and the guides), and the catalog's packs. It follows the evidence rules
of [EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md): every result names the
exact artifact, what was actually observed, and what remains unverified. A
BetterFy success state is not proof that Dota loaded a change; only an in-game
observation is.

**Status: not run on Windows.** Everything below was built and tested on a Mac
(unit tests, the regenerated allowlists against the real archives, real fonts
renamed and checked with an independent parser) and in CI. None of it has been
installed on Windows or seen in Dota.

## Artifact under test

The first build of this branch published for testing:

- Branch: `feature/catalog-complete`
- Source commit: `0c0cc11` (version 0.6.0)
- Workflow: Early Access Windows installer, run `39297436418`
- Release: `v0.6.0-ea.47` (prerelease, unsigned)
- Artifact: `BetterFy-Setup.exe`, SHA-256
  `8bc3894679c17e265e0a02459be68abc494895dff599a759e84405a063f8c249`

A later build replaces these lines; record the one actually installed.

## Ground rules

- The repository's working rules apply unchanged.
- The founder signs in and enters every password personally.
- BetterFy never force-terminates Dota or Steam and never launches Dota.
- Use the language slot already observed in game on Windows (`russian`).
- Before the first install of a section, record the SHA-256 of the files that
  section touches (`Get-FileHash -Algorithm SHA256`), so a restore can be
  compared byte for byte:
  - cursors: every file in `game\dota\resource\cursor\` (one command:
    `Get-ChildItem "...\game\dota\resource\cursor" -File | Get-FileHash`);
  - fonts: every file in `game\dota\panorama\fonts\`;
  - guides: every file in `game\dota\itembuilds\`;
  - hero grids: `Steam\userdata\<id>\570\remote\cfg\hero_grid_config.json`.
- One change at a time. On a failure, stop, save the evidence report, and do
  not continue on a dirty state.

## 1. Screens

1. Catalog, both tabs: no "Windows pilot", "preview only", checksum, file path
   or "not checked yet" text anywhere. Tuning tab shows 30 mods; looks tab about
   1,090 items including 34 cursors, 10 fonts and 6 packs.
2. A mod's detail panel: author, size, what it changes, "Install it into Dota
   from My build". No status line.
3. Build review with a skin: size, what it changes, the "Change only this hero"
   switch, the one-line restore note. No file lists or hashes.
4. Settings → About: the early-access line.

## 2. Newly unblocked wardrobe archives

Install each alone, check in Dota, restore:

| Item | What was wrong before | Expected |
| --- | --- | --- |
| `heroes-mirana-suigintou` | sound names with `{guid}` | the skin and its voice lines |
| `heroes-enigma-blackhole-chan` | `(14)` in a texture name, the Workshop addon template | the skin |
| `heroes-legion-commander-chen` | 300 MB, apostrophes in sound names | the skin and its sounds |
| `heroes-phantom-assassin-yor-forger` | 467 MiB from the second dataset | the skin (largest build: watch time and memory) |
| `huds-black-hud` | a Windows "— копия" texture (dropped) | the black HUD |
| `terrains-darkpoly` | `(carve.photos)` in a texture name | the terrain |
| `heroes-black-witch-doctor` | an empty VPK entry with a stale offset | the skin |

## 3. Cursor and font packs (game files)

1. Install a cursor pack (`cursors-ti9-cursor`). The cursor in the menu and in a
   match is the pack's. The files in `resource\cursor` match the pack.
2. Restore. Every file in `resource\cursor` is byte for byte as recorded.
3. Install a font pack (`fonts-monocraft`). Interface text uses the font,
   including Russian. Restore: `panorama\fonts` is byte for byte as recorded
   (including the fonts the pack removed).
4. Install a cursor pack, then install a build without it: the game's cursors
   come back.
5. Stress: interrupt during the write (close BetterFy from the task manager
   while installing a font pack), reopen, use Check under Details: the fonts
   are as recorded.
6. Steam "Verify integrity of game files" after installing a font pack, then
   open BetterFy: the warning that Dota put its own cursors or fonts back
   appears; restore does not overwrite the game's files.

## 4. Minify mods with the player's input

1. **User Styles.** Tick "Remove the hero portrait from the HUD" and "Remove
   dims and blurs behind popups". Install. In a match the portrait is gone; in
   the menu popups have no dim. Restore.
2. **Custom Fonts.** Pick `C:\Windows\Fonts\consola.ttf` (or any font with
   Cyrillic). Install. The interface uses it in every weight; Russian is
   readable. Restore: `panorama\fonts` byte for byte as recorded.
3. **Custom Backgrounds.** Pick "The International 2024 · 1". Install. The
   main menu background is that loading screen. Then pick a short WebM video
   (VP9) and install again: the menu plays it. Restore.
4. **Custom Hero Grids.** Install. In the hero grid, three grids named
   "... #BetterFy" are there beside your own. Edit one of your own grids in the
   game, restore: your edit stays, the three are gone.
5. **OpenDotaGuides.** Install. A hero's guides in the shop are
   "ODG <hero> 2026-10-19". Restore: the default guides are back.
6. **Auto Accept Match.** Settings → Minify section: the delay slider is there.

## 5. Packs

1. Add "Winter Pack": 16 items are added to the build, one per slot. Install,
   check a few in Dota, restore.
2. Add a pack whose items take a slot already in the build: the pack's item
   replaces it.

## Results

| Step | Artifact | Observed | Evidence | Open |
| --- | --- | --- | --- | --- |
| | | | | |
