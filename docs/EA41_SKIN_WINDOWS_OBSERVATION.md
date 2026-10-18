# EA.41 skins on Windows: first observation

The first time audited hero skins were installed by BetterFy on Windows and
seen in Dota. It records what the founder observed on one machine, on one day,
with the evidence that backs it, and keeps what is still unverified apart.

## Artifact

- Release: `BetterFy v0.5.0 · Early Access 41`, tag `v0.5.0-ea.41`
- User-facing artifact: `BetterFy-Setup.exe`, `34,759,168` bytes
- SHA-256: `037b540d0d281a3f628b82e2d803ea85a01284bdd1d3c1ed9aa86d895daa0890`
  (the published sidecar matched the downloaded bytes)
- Identification of the build from the founder's report: the evidence report
  names app version `0.5.0` and was generated at 2026-10-18 11:02:22 UTC; EA.41
  was published at 10:38 UTC and EA.42 at 11:20 UTC, so EA.41 is the build the
  report came from. The founder did not state the build number himself.

## What was done

On a Windows machine that also uses Minify, with the Dota language folder
`dota_russian`:

1. A build of three catalog skins was prepared and installed through the
   builder: Scarlet Keeper of the Light (`wardrobe.scarlet-keeper-of-the-light`),
   Juggernaut Arcana Purple (`wardrobe.juggernaut-arcana-purple`) and Phantom
   Lancer Crimson V2 (`wardrobe.phantom-lancer-crimson-v2`). Four other
   selections were removed from the build first.
2. The journal reads: files verified 3 of 3 and the plan ready; the Russian
   language folder checked; Dota and Steam closed gracefully; the file written
   with a verified backup; "Using pak65_dir.vpk in dota_russian"; `-language
   russian` added to the Steam profile with the settings backed up; Steam
   started; the report saved; Steam asked to start Dota 2.
3. Dota was started and the Heroes page was opened for each of the three heroes.

## Evidence

- The saved evidence report (schema 1, app `0.5.0`, platform `windows`) holds
  operation 12: language `russian`, phase `committed`, active, `vpkName`
  `pak65_dir.vpk`, the three wardrobe package IDs, installed SHA-256
  `046fbb18b14210962dda3a2e927489b07d2d810702671f2f3cd1db20f737d5a5`,
  bundle plan `sha256:03bac41e9ea05b0a4f1be5217459981ec42316084cd772953860d88a1e6a0d83`,
  backup verified, no error code.
- Operation 11 in the same report is the earlier install in the same language
  folder (a 24-package tuning build), which carries `released_stale`: it was
  let go with the new "Let go of the old install" action and left the folder
  alone. This is the first recorded use of that action.
- The new install took `pak65_dir.vpk`, not the `pak66_dir.vpk` that BetterFy
  and Minify both used before, which is the allocation added after Minify's
  file name was found to collide with BetterFy's.
- Three photographs of the screen, taken by the founder, show the hero page of
  each hero with the skin applied: Scarlet Keeper of the Light with the red and
  white colors and effects, Juggernaut in purple, and Phantom Lancer in red. The
  founder's verdict: everything works. The photographs are not part of the
  repository.

## Not verified

- Only the hero page was looked at, not a match: ability effects, the Spirit
  Form of Keeper of the Light, in-match models, item and spell icons.
- The restore of this three-skin build was not part of the report, and a second
  install and restore with an identical hash was not run.
- Other heroes and shared effects were not checked for unintended changes.
- Whether Minify's own file was present beside BetterFy's in this session, and
  which file wins on a conflicting path, was not recorded.
- The Dota build number was not recorded.
- Only these three of the 500 allowlisted skins were seen. Peak memory and time
  for the largest skins (about 207 MiB) are unmeasured.
- This is one machine and one session, not a repeatable report.
