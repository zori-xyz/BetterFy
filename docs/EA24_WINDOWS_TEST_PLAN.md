# Engine Windows test plan (EA.24)

This plan covers the first native Windows pass of the generic package engine:
blacklist packages, Panorama style packages, Panorama layout packages and the
two audited scripts. It follows the evidence rules of
[EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md): every result names the
exact artifact, what was actually observed, and what remains unverified. A
BetterFy success state is not proof that Dota loaded a change; only an in-game
observation is.

Nothing here has been run on Windows yet. Results are recorded in the
"Results" section at the end, item by item, as the pass proceeds.

## Release identity

- Release: `BetterFy v0.1.4 · Early Access 24`
- Tag: `v0.1.4-ea.24`
- Source commit: `b57b513c3ce372c72cafdf1ce077c0d7760122aa`
- User-facing artifact: `BetterFy-Setup.exe`
- Artifact size: `33,352,704` bytes
- SHA-256: `054ba9ac631ca1de219e263e136c37f05a63163f8e5ebf8344b311bade9e670a`
- Release workflow: [run 37146668892](https://github.com/zori-xyz/BetterFy/actions/runs/37146668892)
- Signed package catalog: sequence 7, 26 manifests (25 offered; the original
  Repopulate Unit Query HUD package is superseded and hidden)

## Ground rules for the session

- Read the repository's working rules first. They apply here unchanged,
  including the rule against any trace of assistant tooling in commits or docs.
- The founder signs in (Telegram, Steam) and enters every password personally.
  The tester never types credentials, never reads Credential Manager values,
  and never pastes challenge links or six-digit codes into reports.
- BetterFy must never force-terminate Dota or Steam and must never launch Dota
  itself. Dota is always started by hand from Steam.
- Record before/after SHA-256 of every file BetterFy writes or restores
  (`Get-FileHash -Algorithm SHA256`), and save the in-app evidence report for
  every install and restore.
- One change at a time: when a step fails, stop, save the evidence report and
  the safe diagnostics report, and do not continue on a dirty state.

## 0. Machine record

Record once, before any test:

- Windows edition and build (`winver`), CPU architecture.
- Dota 2 install path and Steam library layout (one library or several drives).
- Dota game language and the language slot BetterFy will use.
- Whether a previous BetterFy (EA.23 or older) is installed, and its version.
- Contents of `game/dota_<language>/` before the first install (file list and
  hashes), and the current Steam launch options for the test profile.

## 1. Install and update

1. Over the existing EA.23 install: run `BetterFy-Setup.exe`. Setup must
   recognize the installation and update it in place: one entry in
   Apps & features, one Start Menu shortcut, install path
   `%LOCALAPPDATA%\BetterFy`, `DisplayVersion` matching the release.
2. Launch from the shortcut. Signed-in state and saved Dota installation must
   survive the update (user data lives in `%APPDATA%\app.betterfy.desktop\`).
3. Catalog: the Worker serves sequence 7 (deployed 2026-10-03, bytes identical
   to the committed, signature-tested files). The app should report it as the
   network catalog. Record what the interface shows for source and
   sequence. With the network off, the app must fall back to the cached or
   embedded catalog without blocking the tuning tab.
4. Tuning tab: 25 packages offered; Repopulate Unit Query HUD appears once.

## 2. Package matrix

For every package: add it alone to a build, prepare, check the plan, install,
start Dota by hand, observe, then restore and confirm the game is back to its
previous state (hashes of `game/dota_<language>/` and launch options equal to
the machine record).

| Package | Mechanism | What to observe in Dota |
| --- | --- | --- |
| Tree Mod | files | Default-terrain trees replaced (baseline package) |
| Show NetWorth | files | Net worth visible as described by the package |
| Remove River | files | River water layer removed |
| Dark Terrain | files, requires Remove Foilage | Darker terrain; preparation must refuse it without Remove Foilage |
| Remove Foilage | blacklist | Grass and foliage gone |
| Remove Weather Effects | blacklist | Weather effects from equipped/environment weather gone |
| Remove Sprays | blacklist | Sprays not drawn |
| Remove Pings | blacklist, conflicts with Revert Ping Sounds | Map pings not shown/heard as described |
| Revert Ping Sounds | files + blacklist | Old ping sounds |
| Mute Ambient Sounds | blacklist | Ambient map sounds silent |
| Mute Default Announcer | blacklist | Default announcer silent |
| Mute Taunt Sounds | blacklist | Taunt sounds silent |
| Mute Voice Line Sounds | blacklist | Hero voice lines silent |
| Minify Base Attacks | blacklist | Simplified base-attack effects |
| Minify Spells & Items | blacklist | Simplified spell and item effects |
| Misc Optimization | blacklist | Effects removed as listed in its blacklist |
| Remove Hero Renders | styles | Hero renders gone from the listed screens |
| Remove Main Menu Background | styles | Main menu background removed |
| Remove Showcases | styles | Profile showcases and their animation gone |
| Reposition & Rescale HUD | styles | HUD moved and rescaled |
| Transparent HUD | styles | HUD panels transparent |
| Revamp Hero Grid Layout | styles + blacklist | New hero grid layout in hero selection |
| Auto Accept Match | script + layout + settings menu | Settings has a "Minify" section with an "Auto Accept" delay slider (0 to 40, 40 disables); a found match is accepted after the set delay |
| Stat Site Buttons | script + styles + layout | DB/OD/ST buttons next to the match ID on the post-game page and in other players' profiles; each opens the right site |
| Repopulate Unit Query HUD | files + layout | Selected-unit HUD (bottom left) shows the repopulated layout: innate/facet display, Aghanim's status, stats block |

For blacklist packages, record the counts the engine stored at preparation
(`%APPDATA%\app.betterfy.desktop\engine-v1\blacklist\<id>.json`: number of
paths, missing paths, unmatched rules, unsupported types). For style and layout
packages, record which originals were captured under `engine-v1\panorama\` and
`engine-v1\layout\`.

The open question for styles and layouts: the edited files keep the CRC of the
original source text. If Dota rejects such a file, the symptom is that the
whole screen or layout falls back or fails to load, not just the mod's change.
Note exactly what is seen on the affected screen.

## 3. Combinations and priority

1. Conflict: select Remove Pings and Revert Ping Sounds together. The interface
   must warn and preparation must fail with `package_conflict`.
2. Dependency: select Dark Terrain alone. Expect `package_dependency_missing`.
   Add Remove Foilage: preparation succeeds.
3. Shared styles: Transparent HUD + Reposition & Rescale HUD together; both
   effects visible. Swap their order in the build and record whether the result
   changes where both touch the same rules.
4. Full build: every offered package except one side of the ping conflict
   (24 packages). Prepare, install, verify a sample of effects from each
   mechanism in one Dota session, restore, and confirm the machine record.
5. Repeat install then restore three times on the full build. The fourth
   install must produce the same VPK hash as the first.

## 4. Interruptions and failures

1. Install with Dota and Steam running. BetterFy must ask both to close
   normally and refuse to write if either stays open past the timeout.
2. Cancel during resource download, close BetterFy, reopen, resume. Verified
   counts continue; no unverified file is published.
3. Network off during preparation: expect a download error with the retry
   message; verified files are kept.
4. Close BetterFy after install and before restore; reopen. The installed build
   and its restore action must still be found.
5. Run the built-in stress install (failure injection at each boundary the app
   offers) and save each evidence report. Recovery must report `PASS`.
6. After an install, use Steam's "Verify integrity of game files", then start
   Dota. Record whether the build is still active, then restore in BetterFy and
   confirm a clean state.

## 5. Update and uninstall

1. With a build installed, install this artifact over itself (reinstall).
   The installed build and restore action remain.
2. Restore the build, then uninstall BetterFy from inside the app. Confirm the
   install directory is removed (a leftover empty `uninstall.exe` directory is
   a known limitation) and that user data under `%APPDATA%` is untouched.
3. Reinstall and confirm sign-in and saved Dota installation return.

## Results

Record each item as `PASS`, `FAIL` or `NOT RUN`, with what was actually seen,
the evidence report file name, and hashes where relevant. Keep "observed by the
founder", "observed by the tester through screen access" and "reported by the
app" distinct.

| Item | Result | Observed | Evidence |
| --- | --- | --- | --- |
