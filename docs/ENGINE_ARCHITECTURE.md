# BetterFy engine architecture

BetterFy uses a typed bridge in `src/engine.ts`: the Tauri desktop runtime invokes
Rust commands, while browser preview returns explicit fixture or unsupported
states. React presents the result but does not decide whether a path, profile, or
operation is safe.

## Product rules

- Windows is the primary runtime. macOS must support UI development, catalog work,
  validation, and a dry-run build plan without requiring Dota 2.
- Never modify the original game files before a verified backup exists.
- Build inside a BetterFy staging directory first. Move the verified result into
  place only at the final commit step.
- Every filesystem operation must be represented in a build journal so an
  interrupted build can be rolled back.
- Treat catalog archives as untrusted input: validate paths, file types, size, and
  checksums before extraction.
- Do not promise compatibility or safety from metadata alone. Record which Dota 2
  build and BetterFy validator produced the result.

## Modules

1. `discovery` — locate Steam libraries and Dota 2, then validate the selected path.
2. `catalog` — read curated and personal manifests without touching game files.
3. `download` — fetch into a cache with resumable downloads and checksums.
4. `archive` — reject path traversal, links, unexpected executables, and oversized
   payloads before extraction.
5. `resolver` — produce a deterministic file map and explicit conflict list.
6. `backup` — create and verify a restore point for every destination that will
   change.
7. `builder` — assemble the selected profile in an isolated staging directory.
8. `verifier` — compare the build plan, staged output, checksums, and required files.
9. `installer` — commit the verified result atomically and write the journal.
10. `runtime` — inspect Dota/Steam, stop both before any production patch, and
    restart Steam only after a successful verified commit. BetterFy never
    auto-launches Dota.
11. `recovery` — restore the last verified backup or finish an interrupted rollback.

## State machine

The backend should emit the states already supported by the UI:

```text
idle
  -> checking
  -> resolving
  -> building
  -> verifying
  -> ready
```

Failures do not become another progress state. Return a structured error containing:

```ts
type EngineError = {
  code:
    | "game_not_found"
    | "invalid_game_path"
    | "catalog_invalid"
    | "download_failed"
    | "archive_rejected"
    | "conflict_unresolved"
    | "backup_failed"
    | "build_failed"
    | "verification_failed"
    | "commit_failed"
    | "runtime_busy"
    | "shutdown_failed"
    | "steam_start_failed"
    | "steam_profile_not_found"
    | "steam_config_confirmation_required"
    | "steam_config_plan_stale"
    | "steam_config_locked"
    | "steam_recovery_required"
    | "steam_config_commit_failed"
    | "steam_config_rollback_conflict";
  message: string;
  recoverable: boolean;
  journalId?: string;
};
```

Messages shown to users stay in React localization. Rust should emit stable codes,
progress, and factual details rather than Russian or English UI copy.

## IPC boundary

Start with these Tauri commands:

```text
discover_game() -> GameInstallation[]
validate_game_path(path) -> GameInstallation
plan_build(request) -> BuildPlan
execute_build(mod_ids, expected_plan_id, confirmed) -> BuildReceipt
restore_backup(backup_id) -> RestoreReceipt
list_backups() -> BackupSummary[]
inspect_runtime() -> RuntimeState
prepare_runtime_for_patch(confirmation_id) -> RuntimeState
list_steam_profiles() -> SteamProfileSummary[]
preview_steam_launch_options(profile_token) -> SteamLaunchOptionPreview
apply_steam_launch_options(request) -> SteamConfigReceipt
rollback_steam_launch_options(request) -> SteamConfigReceipt
recover_steam_launch_options(confirmed) -> SteamConfigReceipt[]
start_steam_after_profile(request) -> RuntimeState
list_presets() -> PresetRecord[]
save_preset(request) -> PresetRecord
delete_preset(preset_id) -> ()
export_preset(preset_id) -> String
import_preset(payload) -> PresetRecord
collect_system_diagnostics(game_path?) -> SystemDiagnosticReport
intake_fixture_content(request) -> ContentReceipt[]
begin_content_download(package_id) -> ContentDownloadStatus
content_download_status(operation_id) -> ContentDownloadStatus
cancel_content_download(operation_id) -> ContentDownloadStatus
```

Long-running commands will emit one event once deployment is introduced:

```text
betterfy://engine-progress
```

Payload:

```ts
type EngineProgress = {
  operationId: string;
  phase: "checking" | "resolving" | "building" | "verifying" | "ready";
  progress: number;
  completedItems?: number;
  totalItems?: number;
};
```

The current staging-only slice reports progress through the existing bridge while
the command runs. The event contract is reserved for the later cancellable worker;
the frontend must then listen only while its operation is active and always remove
the listener when the screen unmounts.

## First vertical slice

The first staging slice was built before downloads, authentication, subscriptions,
or Steam activation:

1. Detect Dota 2 on Windows and validate a manually selected folder.
2. Load two local fixture mods from the repository.
3. Produce and display a dry-run `BuildPlan`.
4. Detect a real path conflict between the fixtures.
5. Build into a temporary staging directory.
6. Verify the staged checksums.
7. Write a build journal.
8. Roll the staging directory back.

This slice proves discovery, manifests, conflict resolution, progress events, and
recovery without modifying the user's game.

### Implemented discovery boundary

The current Tauri prototype exposes `discover_game` and
`validate_game_path`. Both are read-only. Rust canonicalizes the candidate,
requires the `dota 2 beta` directory name, and checks the platform executable
plus `game/dota/pak01_dir.vpk`. Browser preview uses an explicitly unverified
demo result. Additional libraries are read from `libraryfolders.vdf`. Windows
candidates also come from the current-user and machine Steam registry keys plus
both Program Files locations. Every candidate still passes canonical marker
validation; registry data is never trusted as proof of an installation.

`plan_build` is implemented for two repository-owned fixture manifests. It accepts
only allowlisted fixture IDs, rejects absolute, drive-prefixed, backslash, and
traversing destinations, sorts operations deterministically, hashes embedded
payloads with SHA-256, estimates staged bytes, and reports a real same-destination
conflict.

`execute_build` executes a conflict-free fixture plan only inside
`app_data/engine-v1/operations/<operation>/staging`. It creates files with
`create_new`, verifies size and SHA-256 after writing, and commits an atomically
recoverable JSON journal after every material step. `list_engine_operations`
recovers interrupted journal renames, and `rollback_engine_operation` removes only
the validated BetterFy-owned operation directory. Execution requires explicit
confirmation and the exact `planId` returned for the reviewed selection; a stale
or substituted plan is rejected before an operation journal is created. Repeating
rollback is safe. Tests inject failures after a staged write and before verification.

This fixture flow is not deployment. A separate game-deployment boundary now
exists, but it accepts only a `Ready` journal containing exactly one verified
VPK (staged as `pak66_dir.vpk`); the current CSS fixtures cannot satisfy that contract. The
confirmed activation step stops Dota and Steam, commits BetterFy's owned launch
option, verifies the profile against its journal, and starts Steam only. The
generic game-directory write remains closed in release builds. The fixed Tree
verified-build pilot now stages and re-verifies an ordered Tree Mod / Show Net
Worth / Repopulate Unit Query HUD bundle before calling the deployment transaction. The deployment ownership
record and journal persist both the complete bundle-plan identity and ordered
package IDs, so restart detection and rollback cannot mistake two builds that
produce a different selected composition. The founder has observed the ordered
three-package path through the Dutch slot on Windows, including the visible
results in Dota and the recovery controls described in `TREE_MOD_PILOT.md`.
This remains one founder-observed machine without a retained report JSON, not
general compatibility evidence.

The normal release build and the stress controls share the same transaction
code. The CI `windows-build` artifact and the Early Access release both compile
the `internal-stress-test` feature, which adds two deterministic stop points on
each game-file and Steam-profile transaction: after the verified temporary state
is journaled, and immediately after the atomic replacement. Each control
immediately invokes the normal recovery path and requires a matching recovery
receipt. The app offers and accepts these controls only when the auth service's
current profile marks the signed-in account as a developer. That mark comes from
the Worker setting `BETTERFY_DEVELOPER_LOGINS` (BetterFy ID logins, never
committed), so every other account, a signed-out app, or an offline app sees no
stress controls.

The runtime preflight is now implemented behind typed Tauri commands. Windows
process enumeration uses Tool Help APIs and recognizes the Steam client, Web
Helper, overlay, and Dota processes without invoking a shell. With explicit
confirmation, `prepare_runtime_for_patch` requests a normal `WM_CLOSE` for Dota,
waits for it to exit, invokes the registry-resolved `steam.exe -exitsteam`, and
then waits until both products are absent. It never force-terminates a process.
Timeout, unavailable Dota window, missing Steam, and unsupported platform have
stable error codes. This preflight is used by the visible profile-activation step
and remains mandatory immediately before future production deploy.

Steam launch-option planning now has a BetterFy-owned, lossless VDF boundary.
The parser walks the nested KeyValues path for app `570`, preserves every byte
outside the `LaunchOptions` value, and adds or replaces only the `-language`
argument for the selected slot. Restore undoes only that argument (see "Language
slots and Steam language detection").
Missing launch options are inserted into the existing Dota app object, and the
updated document is parsed again in tests. Plans expose before/after SHA-256
values. Steam profile discovery exposes only an opaque path-derived token and a
neutral ordinal; Steam IDs, account names, and `userdata` paths never cross IPC
or enter the journal. Applying a plan requires the exact confirmation token for
its before/after hashes and repeats discovery and hash validation while holding
an exclusive transaction lock.

The production write boundary creates and verifies a private BetterFy backup,
writes a same-directory temporary VDF, reparses it, verifies its SHA-256, and on
Windows publishes it with `ReplaceFileW` plus write-through. The journal records
only operation/profile tokens, hashes, phase, and a BetterFy-relative backup
path. An interruption before or after replacement is recovered idempotently
from the verified backup; an unrelated post-commit Steam edit blocks rollback
instead of being overwritten. Apply, rollback, and recovery Tauri commands are
Windows-only and reject the operation unless both Steam and Dota are stopped.
The visible Build success route exposes this transaction as a separate, honest
activation step. It lists profiles by neutral ordinal, requires explicit shutdown
confirmation, applies the selected preview, verifies either the matching committed
journal or an already-managed profile, then resolves `steam.exe` from the registry
and starts Steam. Dota is never launched. A changed transaction exposes its exact
rollback path. The internal verified-build pilot binds one selected, allowlisted
`-language` value to the same language folder used for its VPK. Earlier Dutch
operations remain readable and restorable.

### Implemented game-deployment transaction foundation

Rust now contains a clean-room deterministic VPK v1 writer and reader for bounded,
embedded data entries. Paths are lowercase relative ASCII, traversal and case-fold
collisions are rejected, CRC32 is recorded per entry, and the finished archive is
opened and checked again. External archive parts are not accepted.

The multi-package layer now merges verified embedded resources into one
BetterFy-owned VPK. Selected package order is explicit: the first package has
priority when different bytes target the same resource path. Identical resources
are deduplicated, differing collisions are recorded as overrides, and all inputs —
including shadowed bytes — are bound into the reviewed SHA-256 plan identity. The
same ordered package list is persisted in the staging journal. The deterministic
merge remains covered synthetically and the complete three-package bundle has
also been observed on one Windows machine through the pinned pilot. This does not
open general catalog deployment. Each package's
input, effective, deduplicated, and shadowed resource counts are included in the
reviewed plan so a selected package cannot silently contribute zero effective
resources.

Verified embedded upstream VPKs can be reopened into their resource maps before
the merge, so repeated upstream names such as `pak66_dir.vpk` do not become target
filenames and do not require blind numeric renaming. External numbered archive
parts remain unsupported and are rejected rather than partially copied.

`deploy_staged_vpk` cannot receive a source or destination from the interface. It
resolves a confirmed BetterFy staging journal, requires its exact reviewed plan ID,
requires one verified staged VPK (staged as `pak66_dir.vpk`; that is the staging name
only), rehashes and reopens it, then targets only `game/dota_dutch` for the generic
debug path. The verified-build pilot
instead chooses an allowlisted language folder before installation and records
it in the journal and receipt. The runtime must already prove Steam and Dota are
closed. The file name inside the language folder is chosen at install time (see
"File names and coexistence with Minify"). An existing owned target is replaceable
only when BetterFy's ownership record matches its current hash; BetterFy never
writes over a file it did not write.

The transaction writes and verifies a private backup, writes a same-directory
temporary VPK, publishes with Windows write-through APIs, reopens and rehashes the
installed bytes, and commits an ownership record plus journal. Rollback restores
the exact prior bytes and prior ownership chain, or removes an initial install. It
refuses to overwrite a post-install external edit. Recovery distinguishes an
interruption before publish from one after publish and either marks the untouched
operation failed or rolls the published bytes back. Synthetic tests inject both
failures. Only pinned Tree Mod, Show Net Worth, and Repopulate Unit Query HUD selections can invoke the internal live-deploy
path on Windows. No other catalog selection can invoke a live deploy, and no
general Windows compatibility result is claimed. The founder has observed the
intended Tree Mod result in Dota through the Dutch slot and completed the visible
rollback, after which the game returned to its normal tree state.

Installed-state discovery now reads the ownership record and committed journal,
rehashes the on-disk target, and checks a required previous-version backup without
depending on the resource cache or browser storage. The Tree Mod screen separately
reports whether the installed hash still matches a VPK rebuilt from the pinned
resources. If that second check is unavailable, file rollback remains discoverable
but automatic Steam activation is not offered. This is synthetic recovery coverage,
not proof of exact-byte Windows restoration or interruption recovery.

The internal Tree Mod Steam write now carries the validated VPK deployment ID
inside the Steam journal. On restart, Rust locates the matching committed Steam
operation by that ID; React no longer relies on localStorage for either rollback
identifier. An interrupted linked Steam journal blocks VPK rollback until Steam
recovery runs, preventing an orphaned launch option. Legacy Steam changes that
predate this link cannot be inferred safely and require a separate manual check.

Preset persistence is implemented as a separate BetterFy-owned boundary. The
backend validates the schema and identifiers, rejects symlinks and oversized
records, and commits JSON records through a temporary file and rollback-aware
rename. Import creates a new local preset and cannot overwrite built-in
workshop entries.

### Language slots and Steam language detection

The language allowlist is Valve's Steam API code for each of the 27 languages
besides English that Dota 2's store entry lists (`supported_languages` from the
Steam store API for app 570, matched against the Steamworks language table):
`brazilian`, `bulgarian`, `czech`, `danish`, `dutch`, `finnish`, `french`,
`german`, `greek`, `hungarian`, `italian`, `japanese`, `koreana`, `latam`,
`norwegian`, `polish`, `portuguese`, `romanian`, `russian`, `schinese`, `spanish`,
`swedish`, `tchinese`, `thai`, `turkish`, `ukrainian`, `vietnamese`, plus
`betterfy`. Deserialization accepts only these exact lowercase names; `english`
stays rejected because no `dota_english` slot exists. Dutch remains the default
for journals that predate the choice. Every slot uses the same backup, ownership,
rollback and targeted Steam restore, and changing slot still requires a restore
first. Only `russian` and `dutch` have been observed in game on Windows.

`betterfy` stands in for English: `game/dota_betterfy/pakNN_dir.vpk` (the name
chosen as for any slot) with `-language betterfy`. The folder holds nothing but BetterFy's VPK and is removed on
restore when BetterFy created it. Evidence from the pinned Minify commit
`3a85572029f2c264e2a17cee1c9b54ce93e4fd93`:

- Minify writes only VPKs into a language folder (`pak65`, `pak66`, `pak67` and
  `pak99_dir.vpk`, plus `maps/dota.vpk` from its `d2pfx` browser). `gameinfo.gi`
  appears only in a copy Minify makes for its resource-compiler tools, never in a
  language folder, so BetterFy writes no extra file there either.
- At that commit Minify no longer uses its own slot. Its commit `a9ef0592bb`
  ("Base dynamic fix for 2026-07-23 patch") removed `dota_minify`, and
  `db8c43d7df` moves a saved `minify` choice and `dota_minify` path to an
  `english` option that writes into `dota_dutch` and forces its `#English Fix`
  mod. That mod's notes call it a workaround for a Dota patch that enforces valid
  languages. It packs every `*_english.txt` and `*_english.vtt` from
  `game/dota/pak01_dir.vpk`, renamed to `*_dutch`, into `pak99_dir.vpk`, and
  overwrites `*_dutch.txt` files under `game/core` and `game/dota_addons` with
  their English copies.
- The `betterfy` slot therefore mirrors Minify before that patch (the setup that
  left a populated `dota_minify` on the founder's PC), not Minify now. Whether the
  current Dota client still mounts a non-Valve `-language` folder has not been
  observed on Windows. Minify's current workaround is not copied: it rewrites
  Valve's own files in `game/core` outside the single BetterFy-owned VPK, and it
  needs Valve's multi-part VPKs, which this engine rejects.

`detect_dota_language` is read-only. It validates the Dota folder like manual
selection, then reads `steamapps/appmanifest_570.acf` of the library holding it:
no symlink, at most 256 KiB, `appid` must be `570`. It uses the same strict
KeyValues parser as `localconfig.vdf`, now limited to 64 nesting levels. It
returns Valve's raw name from `AppState/UserConfig/language`, falling back to
`AppState/MountedConfig/language` (the language still installed while a switch
downloads), and the matching slot: `english` → `betterfy`, a Dota language →
itself, anything else → no slot. A missing manifest or language returns nothing;
an unreadable or malformed one returns `steam_manifest_invalid`. The parser was
run once against a published Dota 2 manifest fixture with LF and CRLF line ends.
The founder reported `russian` in his manifest; detection itself has not been run
on Windows.

### Implemented Windows readiness report

`collect_system_diagnostics` gives the interface one factual preflight before a
native test or future production transaction. Rust revalidates the stored Dota
path, inspects the Windows process snapshot, aggregates neutral Steam-profile
states, and counts recoverable BetterFy staging and game-deployment journals. The
report contains stable codes, states, counts, application version, platform, and
generation time.

The report never contains filesystem paths, Steam IDs, account names, Telegram
data, or authentication material. Diagnostics may prepare empty BetterFy-owned
app-data roots and listing staging journals may finish an interrupted journal
rename there; it does not modify Dota, Steam, launch options, or game content.
Browser and unsupported platforms return an explicit unsupported state instead
of imitating Windows readiness.

`collect_tree_pilot_evidence` exports at most the latest 100 deployment journal
records for the validated installation. It includes the app version, platform,
language, package order, phases, timestamps, SHA-256 values, backup verification,
and durable rollback verification. It deliberately omits filesystem paths,
target identities, Steam profile tokens, account identifiers, and authentication
data. A rollback is marked verified only after the restored target hashes to the
recorded pre-install value, or after an initially absent target is confirmed
absent again.

### Implemented trusted content foundation

The fixture build now consumes a versioned package manifest and verified bytes
from `content-v1`, not directly from the interface or an arbitrary filesystem
path. Rust validates required localized metadata, provenance, permission state,
artifact format, byte size, lowercase SHA-256, relationships, recipe version,
compatibility state, and explicit signature state. Unknown fields and unknown
package IDs are rejected.

Artifacts are addressed by SHA-256 and published without replacement. BetterFy
writes and syncs a uniquely named temporary file, verifies it again, and uses a
same-directory hard link to claim the final object name atomically. A competing
or repeated intake must verify the existing bytes; it cannot overwrite them.
The normalized manifest is published through the same no-clobber boundary.
Interruption before object publication leaves no final object. Interruption
between object and manifest publication is repaired by an idempotent retry.

The visible fixture build automatically ensures its selected packages are in
this store, then re-reads and re-hashes the stored object before staging. The
package version, recipe version, artifact filename, byte size, and SHA-256 must
also match the declarative build recipe. That content identity is included in the
deterministic plan ID. A tampered object or drifted recipe blocks the build before
an operation journal or Dota-facing state exists. The current commands accept
repository-owned fixture IDs only. A cancellable worker can stream their
commit-pinned HTTPS artifacts into a bounded BetterFy-owned sidecar, with proxy
bypass, DNS/peer checks, manual same-origin redirects, exact size and SHA-256
verification, and no-clobber publication. The frontend receives only an opaque
operation ID and factual phases; URLs and paths stay inside Rust.

A ZIP metadata preflight rejects traversal, links, ambiguous names, executable
content, unsupported compression, and archive-bomb limits without extracting any
entry. No ZIP package is enabled in the registry yet. Local imports, signatures,
archive extraction, and generic live Dota deployment remain disabled. The fixed
Tree Mod internal pilot is separate. The full threat model is documented in
`docs/CONTENT_INTAKE_SECURITY.md`.

### Package manifests (PackageManifest v1)

Installable packages are declared, not coded. Each one is a JSON file in
`src-tauri/packages/`: engine ID and catalog ID, Russian and English names,
author, pinned source (repository, 40-character commit, directory, license),
distribution state, verified languages, and the exact resource list (lowercase
relative path, byte size, SHA-256). `package_registry.rs` embeds the files at
build time and validates them on first use: trusted repository only, pinned
commit, no traversal or case-folding collisions, bounded sizes, unknown fields
rejected. Download URLs, bundle contracts and the allowlist of installable
packages all come from the registry; there is no per-package branch in Rust.
The interface imports the same files, so its list of installable mods cannot
drift from the engine's.

A resource may name a different repository path in `from`. That is how a Minify
`blacklist.txt` line is represented: the listed game path receives the matching
`Minify/bin/blank-files/blank.<ext>` placeholder, fetched and hash-checked like any
other file. The placeholder must have the same extension as the target. A manifest
may instead carry a whole `blacklist.txt` (see "Blacklist packages" below), which also
covers Minify's `**` and `>>` patterns. Zero-length resources are allowed
when their hash is the SHA-256 of empty input; they need no download and are written
as zero-length VPK entries, which is how Minify silences sounds.

`minify.remove-river` is the first package added only as data: 9 files, 5 zero-length
sounds and 9 blacklist placeholders from the same pinned commit. The engine
downloaded and verified all 23 resources and built a four-package VPK. It has not been
checked in Dota yet, and its manifest records no verified language.

The three pilot packages were moved from Rust constants into manifests without
changing any path, size or hash. A test pins a fingerprint of the contracts, and
building the ordered three-package bundle from the real pinned resources gave the
same plan ID and VPK SHA-256 before and after the move, so existing installations
and their journals are still recognised.

### Blacklist packages

A manifest may name a Minify `blacklist.txt` in a `blacklist` block: its repository
path, size and SHA-256, plus one pinned `Minify/bin/blank-files/blank.<ext>`
placeholder per compiled type. The file and the placeholders are downloaded through
the same pinned path and content-addressed store as resources. The block is part of
the package contract; manifests without it keep their previous contract hashes.

`src-tauri/src/blacklist.rs` reads the file line by line: `#` comments and blank
lines are ignored, `>>dir` selects every resource under that directory, `**regex`
selects every resource whose path the expression matches anywhere (Minify runs
ripgrep over the game's file list the same way), and any other line is an exact
path. Lines are matched against the resource paths of the installed game's own
`game/dota/pak01_dir.vpk` and `game/core/pak01_dir.vpk`, read from their directory
trees only (`vpk::list_directory_paths`, VPK versions 1 and 2). Exact paths the game
does not have, rules that match nothing and matches of a type without a placeholder
are counted and skipped, never guessed. Regular expressions use the `regex` crate,
whose matching time is linear in the input, and every file, line and result count is
bounded.

The expansion happens once, when a build is prepared for a specific installation,
and is stored per package under `engine-v1/blacklist/`. Every later rebuild of the
same bundle (plan verification before install, Steam setup, installed-state check,
restore) reads that stored expansion, so a Dota update does not change what BetterFy
considers installed. Preparing again re-expands against the current game. In a
bundle, a file a package ships itself always wins over a blank for the same path.

Manifests may also declare `conflicts` and `requires` (package IDs), mirroring
Minify's own manifest rules: Remove Pings and Revert Ping Sounds exclude each other,
and Dark Terrain requires Remove Foilage. The engine rejects such a selection before
anything is downloaded; the interface explains it before preparation.

Thirteen packages were added this way from the same pinned commit: Minify Base
Attacks, Minify Spells & Items, Misc Optimization, Mute Ambient Sounds, Mute Default
Announcer, Mute Taunt Sounds, Mute Voice Line Sounds, Remove Foilage, Remove Pings,
Remove Sprays, Remove Weather Effects, Revert Ping Sounds (8 files plus its
blacklist) and Dark Terrain (300 files). None of them has been checked in game yet,
so their manifests record no verified language.

### Panorama style packages

A manifest may carry a Minify `styling.css` in `panoramaStyles` (repository path, size,
SHA-256). The file is split into sections by its `/* g:panorama/styles/... */` (from
`game/dota/pak01_dir.vpk`) and `/* c:... */` (from `game/core/pak01_dir.vpk`) headers;
each section is CSS that Minify appends to that compiled style.

BetterFy does this without Valve's resource compiler (`src-tauri/src/panorama.rs`). A
compiled style is a Source 2 resource: a 16-byte header, a block table with offsets
relative to each entry, and 16-byte-aligned blocks. Its `DATA` block holds a CRC, an
image table and the plain CSS text. The engine appends the package's CSS to that text
and rebuilds the container; every other block (dependencies, source map, image table,
the CRC) is kept byte for byte, and appending keeps the source map's offsets valid. New
`url(...)` images cannot be added this way, and none of the pinned `styling.css` files
uses them. Whether the game client accepts a style whose text no longer matches the
kept CRC is the open question; Valve ships styles where the two already differ, and the
first in-game check on Windows answers it.

When a build is prepared, the game's own styles named by the selected packages are read
from the installation's archives, including numbered side archives (`pak01_NNN.vpk`),
checked against the directory CRC, stored by SHA-256, and listed per package under
`engine-v1/panorama/`. Rebuilds use these stored originals, as with blacklists. When
several packages extend the same style, their CSS is joined lowest priority first so the
highest-priority package's rules come last, and the result belongs to that package in
the bundle.

Seven packages use this: Remove Hero Renders, Remove Main Menu Background, Remove
Showcases, Reposition & Rescale HUD, Transparent HUD, Revamp Hero Grid Layout (also a
blacklist) and Stat Site Buttons (also a layout package, below).

### Panorama layout packages

A manifest may carry a Minify `xml.json` in `panoramaLayouts` (repository path, size,
SHA-256), optionally with a settings `menu.xml`. Each key names a game layout
(`panorama/layout/....xml`, compiled as `.vxml_c` in `game/dota/pak01_dir.vpk`); each
value is a list of Minify actions: `add_script`, `add_style_include`, `set_attribute`,
`add_child`, `insert_after`, `insert_before` and `move_into`. Minify decompiles the
layout to XML, applies the actions with ElementTree and recompiles with the Workshop
Tools. BetterFy edits the compiled layout directly (`src-tauri/src/panorama_layout.rs`,
`src-tauri/src/kv3.rs`):

- A compiled layout keeps its tree as binary KeyValues3 in the `LaCo` block
  (`m_AST.m_pRoot`; nodes with `eType`, `name`, `child`/`vecChildren`,
  `sourceLineColumn`). The reader handles KV3 versions 2 to 5, uncompressed or LZ4;
  the edited tree is written back as uncompressed version 5. Every other block is kept
  byte for byte.
- Selectors follow Minify's grammar (`tag#id.class[attr=value]`) and its two different
  "first match" orders: document order for `set_attribute`, `add_child` and the
  `move_into` destination; parent-by-parent for `insert_*` and the moved element.
- The compiler leaves `m_ChildResourceList` empty even for layouts that include scripts
  and styles, so includes resolve by path at load time and editing the tree alone
  matches the compiler's output. New nodes reuse the source position of the place they
  are inserted at; the `DATA` CRC of the source XML is kept, as with styles.
- A settings `menu.xml` goes into a "Minify" section appended to
  `PopupSettingsRebornSettingsBody` in `popup_settings_reborn`, as Minify's Auto Accept
  Match script does. BetterFy needs no Workshop Tools, so the slider is always added.

Layout actions can carry script in event attributes (`onload`, `onactivate`), and
`add_script` loads a compiled Panorama script. Both are accepted only from an
allowlist compiled into the app: `xml.json`/`menu.xml` files by exact repository path,
size and SHA-256, and `vjs_c` resources by exact path, size and SHA-256. A signed
catalog cannot extend either list. The two audited scripts are Auto Accept Match's
`popup_auto_accept_match.vjs_c` (waits the slider's delay, then dispatches
`DOTAPlayAcceptMatch`) and Stat Site Buttons' `ssb.vjs_c` (opens the match or profile
on Dotabuff, OpenDota or Stratz through Dota's own browser events).

When a build is prepared, the game layouts each package edits are read, every action is
applied once as a check, and the originals are stored under `engine-v1/layout/`. A target
that a Dota update moved or removed fails preparation with
`panorama_layout_target_missing` instead of producing a half-edited layout. When several
packages edit the same layout, the lowest priority edits first and the result belongs to
the highest-priority package.

Verified on this Mac only: exact tree round trips through the Rust reader and writer on
four Valve-compiled layouts (KV3 v2 with LZ4 and v5), the Rust output read back by an
independent Python parser, and Minify's Auto Accept edits applied to a Valve-compiled
`popup_accept_match.vxml_c`. Not yet run on Windows or in game.

Auto Accept Match, Stat Site Buttons and Repopulate Unit Query HUD use this. The first
published Repopulate package shipped only the override styles, which nothing loads
without the `xml.json` edits that include them. Its contract cannot change (see "Signed
package catalog"), so the full version is a new package,
`minify.repopulate-unit-query-hud-v2`, which takes over the catalog entry. The old
package keeps its ID and contract, so builds that include it stay verifiable and
restorable; it carries `supersededBy` and a `-legacy` catalog ID, and the interface no
longer offers it. A remote catalog that still lists the old package under its original
catalog ID collides with the new one and is rejected by this build, so the catalog and
the app release ship together.

### Wardrobe items (audited hero skins)

A wardrobe item is a third-party hero skin installed through the same verified pipeline
as the tuning packages: one entry in the BetterFy VPK, the same plan identity, backup,
journal, deployment and rollback. It differs in where its bytes come from and in how
much of them is trusted. The rules come from
[`SKIN_ARCHIVE_AUDIT.md`](SKIN_ARCHIVE_AUDIT.md) and live in `skin_archive.rs` (the
analyzer), `wardrobe.rs` (the allowlist and the load path) and `src-tauri/wardrobe/`
(the entries). The entries are kept apart from `src-tauri/packages` so the signed package
catalog is unchanged.

**Install everything by default.** An earlier version of this section said shared game
files are stripped from a skin by default. That rule is superseded by evidence from real
Windows use, reported by the founder: skin VPKs dropped into the language folder as they
are give the full look, and leaving files out can break textures and materials. The
default is now to install every file the analyzer accepts. Leaving the shared game files
out is an explicit per-skin opt-in (see "Install modes").

**Allowlist.** A wardrobe item is installable only if it has an entry compiled into the
app, like the audited scripts: a signed catalog cannot add one, and the package registry
reserves the `wardrobe.` namespace and the catalog IDs of the entries. All entries are one
generated JSON array, `src-tauri/wardrobe/manifests.json` (compact, sorted by ID, about
0.87 MB for the 500 entries), and `index.json` is the interface's copy (names, authors,
heroes, sizes; about 0.14 MB). A test requires both files to be exactly what the
generator would write for the embedded manifests, so the interface cannot list an item the
engine does not know. An entry pins

- the downloaded ZIP and the single VPK inside it by size and SHA-256, and the ordered
  list of mirrors they come from (see "Download hosts");
- the heroes the skin is for (a set of internal names: `shadow_fiend` and `nevermore`),
  and the *shared paths*: the files of the archive that Valve's own file list also has
  and that are not the heroes'. The list is computed at audit time from SteamDatabase's
  `game/dota/pak01_dir.txt` (its SHA-256 and entry count are in the entry) and decides
  which files count as shared; no name heuristic does;
- what the analyzer must find: the entry count and archive identity, the number of
  dropped and shared files, the default install (file count, size, identity) and the
  opt-in install (file count, size, identity). Identities are SHA-256 over sorted path
  and payload hash;
- how permission was established, as recorded (`founder_reported_author_consent`: no
  licence file ships in the archives).

**Install modes.** Each item is installed in one of two modes. `install` (default)
puts every accepted file into the build, including the files that override a Valve file
outside the hero's own and the files that replace nothing; the shared ones are counted
and listed in the review as information. `strip` leaves the shared files out. The mode
is part of the package ID: `wardrobe.<name>` is the default and `wardrobe.<name>.strip`
the opt-in, so the mode travels wherever the package list travels (plan, journal,
ownership, rebuilds, restore) and is bound into the plan identity with no new field. A
build holds a skin once, in one mode. The review shows, per skin, a switch "Keep the
game's shared files untouched" (default off); turning it on prepares the plan again with
the `.strip` ID, and the install button works only for the plan that was reviewed.

**Pipeline.** Preparing a build with a wardrobe item downloads the ZIP (bounded, proxy
bypass, public-address and peer checks, SHA-256), reads exactly the one pinned entry out of
it behind the ZIP preflight and a hard length bound, checks its SHA-256 and publishes only
that VPK to the content store, where every later build re-reads and re-hashes it. Each
build then runs the analyzer on those bytes in the requested mode and requires its result
to equal that mode's pinned audit; otherwise it stops with `wardrobe_audit_mismatch`. A
change to the analyzer's rules therefore cannot silently change which files an audited
archive installs. The files to install become an ordinary bundle package, merged by the
existing `mod_bundle` with the tuning packages. The content store holds at most 256 MiB per
object (the same cap as the merged VPK), so an archive whose VPK is larger is not in the allowlist. Paths may contain spaces and plus signs, which Valve's own sound and material names use; traversal, backslashes, control characters, empty or space-padded segments stay rejected.

**Analyzer.** It reads a VPK v1 or v2 directory with every entry embedded (side archives
and CRC mismatches refuse the file) and classifies each path:

- *Type.* Only compiled `vpcf_c`, `vsnap_c`, `vtex_c`, `vmat_c`, `vmdl_c`, `vmesh_c`,
  `vanim_c`, `vsndevts_c`, `vsnd_c`, `vmorf_c`, `vagrp_c`, `vphys_c` and `vseq_c` pass.
  Scripts and executables, case-colliding names and every other type are rejected;
  scripts, executables and unsafe paths also block the whole install. A zero-length
  file of an allowed type is a deliberate blank override and installs. One file may
  be as large as the install, and the install as large as the merged VPK less
  8 MiB for its directory (`skin_archive::MAX_INSTALL_BYTES`). A `vxml_c` passes only
  when `panorama_layout::image_wrapper_dependency` proves the image-wrapper shape
  from the parsed structure, in either of the two containers seen in third-party
  archives (a `LaCo` tree, or the older XML text in `DATA`); a layout with a script
  body, include or event attribute is rejected as `panorama_scripted`, anything else
  unprovable as `panorama_unproven`. The `audited scripts only` rule is untouched:
  no skin can bring a script. Rejected files are split in the report into *dropped*
  (scripts, executables, unprovable layouts, types the game may load such as `webm`
  and `vcss_c`) and *ignored* (uncompiled sources and tool or reading leftovers, which
  the game does not load from a VPK); the review shows "Not game files, ignored: N"
  apart from "Dropped". The evidence for the wrapper shape is in
  [`SKIN_ARCHIVE_AUDIT.md`](SKIN_ARCHIVE_AUDIT.md).
- *Names.* Upper-case letters are folded to lower case and counted in the report; two
  names equal once folded are both rejected.
- *Scope.* Hero-scoped (the hero's folders, sounds, icons, sound events and status
  effects; see the audit document), shared (a path in the pinned shared list) or
  author-owned (everything else: files the game does not ship). The classes feed the
  report; under the default mode they change nothing about what is installed.

The result is a typed, serializable `SkinReport` (counts and bytes per class, rejected and
shared paths with reasons, the identities, folded names, and files the player's game ships
now) carried in the plan.

**Plan, conflicts and size.** `TreePilotPlan` gains `wardrobe` (one report per item, with
its mode, the sizes and the shared and rejected lists) and `wardrobeConflicts`. A path that
a wardrobe item and another selected package both write, with different bytes, is a
conflict. The first package in the selection wins, as for tuning packages, but the plan
lists every such path and the install command refuses to run unless the request carries
`acknowledgedConflicts`, which the interface sends only after the person ticks a
confirmation under that list. Identical bytes are a duplicate, not a conflict. Both are
already part of the plan identity. No Tauri command was added.

One merged VPK holds at most 256 MiB of file data and 65,536 entries (`mod_bundle.rs`).
The audited install sizes are known up front, so preparing a build whose selected skins
(in their chosen modes) already exceed either limit stops at once with
`bundle_limit_exceeded`, before anything is downloaded, and the build screen shows the
running total against the limit as skins are added. The merged build is still checked
against the same limits when it is made, which catches the tuning packages too.
Staging the verified VPK for install is bounded by the same 256 MiB
(`build_engine::MAX_STAGED_BYTES`; it used to be 64 MiB, below the audited installs of
the larger skins, and refused them with `build_too_large`). A unit test pins the cap; no
staging or install of a VPK of that size has been run on Windows.

**The player's own game.** When a game folder is connected, preparing a build also reads
the file list of `game/dota/pak01_dir.vpk` (read-only, bounded) and notes which of the
skin's author-owned files the game ships now although the audit's Valve list did not (a
file added since the audit). Under the default mode this is information only (listed in
the review as files "your game ships now"). Under the opt-in mode those files are also
left out, because that mode's meaning is to leave the game's files untouched. The result
is stored per item and read back by every later rebuild, like a blacklist expansion, so a
Dota update does not change what is considered installed. With no game folder, or an
unreadable file list, the item is recorded as "not checked", the review says so, and the
plan never fails because of it.

**Download hosts.** The archives come from the Dota2PornFx project's GitHub repository
(`raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/<category>/<file>`) and
its Hugging Face dataset (`huggingface.co/datasets/hrdq/Dota2PornFx/resolve/main/...`). A
manifest lists up to one URL per host, in the order they are tried, all naming the same
file; every URL must match a trusted prefix exactly. A mirror that answers with an HTTP
error, cannot be reached, or serves bytes that do not match the pinned size and hash is
skipped and the next is tried; the download fails only when no mirror yields the pinned
bytes (with the last mirror's error). Cancelling stops at once. Only mirrors that answered
200 with the pinned ZIP size to a `HEAD` request are listed (all 500 have both). GitHub
answered with 200 directly; Hugging Face answers a `resolve/...` URL with a redirect to a
CDN host under `cdn.hf.co` with a signed query string (observed with a `HEAD` request
from a Mac on 2026-10-09: `us.aws.cdn.hf.co`). The transport therefore follows redirects
to hosts ending in `.cdn.hf.co` in addition to the original origin, and allows a query
string only there; every hop is still HTTPS on port 443 with resolved addresses checked
and pinned. Other regions of that CDN are matched by the suffix and have not been seen.
The SHA-256 pin, not the host, decides whether the bytes are accepted.

**The generator.** The entries are not written by hand. The `#[ignore]`d test
`wardrobe_audit::generate_wardrobe_allowlist` (local only, driven by environment
variables: a folder of downloaded ZIPs, the fetch manifest, the mirror check, the catalog
and Valve's file list) audits every `heroes` and `hero-items` catalog item: it opens the
ZIP, reads the VPK, infers the hero names from the paths Valve's list also has, computes
the shared paths, runs the analyzer in both modes, lets the app's own load path reproduce
the audit, and writes `manifests.json`, `index.json` and
[`WARDROBE_AUDIT_REPORT.md`](WARDROBE_AUDIT_REPORT.md). An item that cannot be audited
cleanly (no hero found, an unsafe path, a container the reader refuses, an archive over
the 256 MiB bound, no archive) is not emitted and is listed with its reason. Output is
sorted and carries no timestamp. A second ignored test, `spot_check_embedded_items`,
unpacks chosen cached ZIPs as the app does and builds plans from them. The generated
files are checked by ordinary tests on every run: every embedded manifest is validated,
the files must be canonical, and the interface copy must be derived from them.

**What has and has not been run.**

- Compiles and passes unit tests on macOS: the analyzer against small synthetic VPKs
  (v1 and v2; hero, author-owned and shared paths; both modes; folded and colliding
  names; uncompiled sources, scripts, executables, traversal and oversized entries;
  passing and failing image wrappers; several hero names), the allowlist and its
  hostile-entry rejections, the mode IDs, ZIP extraction, the audit match and its drift
  cases, the player-side comparison, build and conflict behaviour with synthetic items,
  the size refusal, and the transport's redirect rules and mirror fallback (with fake
  transports, not the network).
- Run locally on a Mac and not committed: the audit of the 512 catalog items from
  cached downloads (500 allowlisted; numbers in the report); the real Scarlet Keeper
  archive through extraction, the content store and a build into one VPK of 996 files
  (57,904,917 bytes, reproduced byte for byte on a second build); and plans built from
  other cached archives (a one-file hero item, a two-name hero, a 61 MiB skin, six
  skins together including two for one hero, with 244 conflicting paths, and five large
  skins that together are refused as over the limit). The archives were fetched with `curl`, not through
  BetterFy's transport; the transport itself has only been exercised against fakes and a
  `HEAD` request.
- Not run: Windows CI for this change (see the PR), an install on Windows, and anything
  in Dota. Nothing here claims a skin loads or looks right in the game when BetterFy
  installs it.

### Letting go of a stale installation

A Dota update or a Steam file check can replace or delete the file BetterFy
wrote. Install and restore both refuse in that state (`deployment_conflict`,
`rollback_conflict`), because BetterFy's backup describes a file that is no
longer there and writing it back could undo a game update.

`release_stale_deployment` is the one way out. It runs only after the user asks
for it in the builder, only when the file is missing or differs from the
recorded hash (a healthy install answers `deployment_not_stale`), and it
touches nothing in the game folder. The ownership record is renamed to
`ownership.released-<ms>.json`, the journal keeps `released_stale` as its error
code, and the installed-profile record is cleared. The next install then treats
whatever sits in the slot as foreign and does not touch it: it takes the first
free file name (see below), so the file that replaced BetterFy's stays where it is
and the new build goes in beside it. Steam's `-language` option has its own
restore button. Covered by unit tests; seen on Windows only as the report that led
to it, not yet exercised there.

### File names and coexistence with Minify

Until this change BetterFy owned one fixed file name, `pak66_dir.vpk`. Minify
(Egezenn/dota2-minify, `Minify/patch/__init__.py` and `vpk_utils.py`) writes its
output into the same language folder under that same name, `pak66_dir.vpk`, plus
`pak66_000.vpk`, ... chunk files unless its `non_chunked_vpk` option is set. The two
tools overwrote each other, and BetterFy, which refuses to touch a file it cannot
prove it wrote, then stopped with `deployment_conflict`, `rollback_conflict` or
`deployment_target_foreign`. Reading Minify's source showed that `pak66` is the only
number its patch step writes (the older `pak65`/`pak67`/`pak99` names noted above
belong to the pinned commit, and the allocation below does not depend on either).

What Dota loads comes from the upstream install guide
(https://d2pfxwiki.pages.dev/en/install.html): any `pak01_dir.vpk` to `pak99_dir.vpk`
in the language folder; names outside that range are ignored or crash the game; a
`!` prefix means top priority; on a conflict the lower number wins; an archive may be
split into `pakNN_dir.vpk` plus `pakNN_000.vpk`, ... chunk files. These rules come from
that guide. BetterFy has not run them in game.

The name is now a recorded fact, not a constant:

- A **new install** takes the first number of a fixed preference list, 65 down to 50,
  such that no entry named `pakNN_dir.vpk`, `!pakNN_dir.vpk` or `pakNN_<digits>.vpk`
  (case-insensitive; directories count) exists in that language folder. 66 is never
  chosen because it is Minify's; 65 to 50 stay clear of it and of the numbers most
  mods use, and sit below 66 so that on a conflict BetterFy's files win over Minify's
  blanks (per the guide, not observed). BetterFy never writes, renames or deletes a
  file it did not write. If all sixteen numbers are taken the install stops with
  `slot_full` and writes nothing.
- The chosen name is stored as `vpkName` in the ownership record and the journal, and
  returned in the receipt and the evidence report. A record without the field means
  `pak66_dir.vpk`, so every earlier record, receipt and backup keeps working. Names in
  a record must be exactly `pak` + two digits + `_dir.vpk`; anything else is
  `deployment_journal_invalid`.
- An **existing healthy install keeps its recorded name**: an update of an owned
  install, rollback to the previous chain entry (which must carry the same name),
  recovery, `current_owned_deployment` and `release_stale_deployment` all use the
  recorded name. Language changes still require a restore first.
- Steam activation names no file, so nothing there changed.

`inspect_language_slot(gamePath, language?)` is the read-only explanation. It lists the
`pak*.vpk` entries of the language folder (at most 64) with name, size, modified time and
a label: `betterfy` (the recorded file, same SHA-256), `minify_like` (named
`pak66_dir.vpk` or `pak66_<digits>.vpk` and not BetterFy's; only the name is known, the
contents are never read) or `unknown`. It also returns BetterFy's recorded file and its
state (`intact`, `missing`, `replaced`), the name a new install would take now, and
whether the slot is full. It hashes only BetterFy's own recorded file, creates nothing,
and returns no paths or contents; with an existing install it describes that install's
folder whatever language is asked for. The builder calls it when an install, restore or
prepare stops with one of the folder codes and shows the cause (for example BetterFy's
file was replaced by another program that uses the same name) and the next step, with
the file facts under Details; before an install it says where BetterFy's file will go
next to files named like Minify's, and after an install it writes one journal line naming
the file taken and what was left alone.

**What has and has not been run.** Compiles, passes clippy and the unit tests on macOS:
a foreign `pak66_dir.vpk` plus `pak66_000.vpk` left byte-identical through install,
update and rollback; the first free number with `!`-prefixed, chunk-only and directory
entries; `slot_full`; an old record without a name through update and rollback; the
reported Minify-replaced case through inspect, refusal, release and a fresh install
beside it; and the notice wording in the browser preview against the demo bridge. Not
run: Windows CI for this change (see the PR), an install next to a real Minify output on
Windows, and whether Dota loads BetterFy's `pak65` beside Minify's `pak66` or resolves
conflicts in the order the guide describes.

### Map terrains: `maps/dota.vpk` as a second owned target

A "standard" terrain of the Dota2PornFx catalog is not a skin. It is a complete
Valve map package, `maps/dota.vpk` (34-53 MB), that the game reads in place of
its own. The project's guide
(https://d2pfxwiki.pages.dev/en/troubleshoot.html, "Issues with terrains") says
what to do by hand: move the `maps` folder holding `dota.vpk` into the language
folder, do not move or rename `dota.vpk`, standard terrains show only in matches
and not in Demo mode, and the Dota+ terrain must be switched off in the game
settings or it conflicts with the mod. The ZIP's own `Guide.txt` says the same
about the folder (`game/dota_<language>/maps`). BetterFy installs that one file
as a second owned target of the same reviewed, journaled, crash-recoverable
transaction that owns `pakNN_dir.vpk`. The eight raw `pakNN_dir.vpk` terrains of
the catalog are ordinary VPKs and take the generic path, not this one.

**Where it goes and what is never done.** The target is
`game/dota_<language>/maps/dota.vpk`, below the same language folder as the pak
file. The map is never merged into the pak file and is never renamed. BetterFy
writes it only when nothing is at that path, or when the file there is exactly
the one BetterFy wrote (same hash as its ownership record). Any other
`maps/dota.vpk` (Valve's, another tool's, the player's, even byte-identical to the
new map: BetterFy cannot prove it wrote it) stops the install with
`map_target_foreign` before anything is written, and the builder checks this
before Steam and Dota are shut down (`preview_tree_language` takes `withMap`).
BetterFy backs up and replaces only files it owns; there is no "back up a foreign
map" path, because the semantics of overwriting a Valve-shipped or user file could
not be shown to be reversible to the exact original. A `maps` that is a file or
another non-folder is `map_folder_invalid`; a link (symlink, and on Windows any
reparse point or junction) at the `maps` folder or the map file is
`deployment_path_unsafe`. The `maps` folder is created only when needed.

**Records.** One journal and one ownership record still describe an operation;
nothing else was added. The changes are additive and defaulted, and a build
without a map writes records that are byte-identical to before (a test pins it):

| Record | New field | Meaning |
| --- | --- | --- |
| journal | `map` (absent without a map) | `installedSha256` (absent value: the operation removed BetterFy's map), `beforeSha256`, `backupRelativePath` (`before-map.vpk`), `mapsDirectoryCreated` |
| journal | `noPak` (absent when false) | the build has no `pakNN_dir.vpk`; `installedSha256` is empty |
| ownership | `map` (absent without a map) | `installedSha256` of the owned map |
| ownership | `noPak` (absent when false) | BetterFy owns only a map; `vpkName` and `installedSha256` are empty |

For a map-only build the receipt's `installedSha256` is the map's hash and
`vpkName` is empty (`hasPak` false). With a pak the pak's hash identifies the
build, as before, and the map is reported in `map` (`file`, `installedSha256`,
`beforeSha256`, `backupVerified`). Each record is validated for contradictions
(`deployment_journal_invalid`): a name for a pak the operation never touched, a
backup without a hash, an operation that leaves nothing installed. An older
record that predates the fields means "a pak, no map".

**Transaction.** Each operation changes up to two parts. A part has a hash before
(what BetterFy owned) and a hash after (what the build leaves): absent to present
is an install, present to present an update, present to absent a removal (a build
that drops its map, or a map-only build replacing a pak build, removes the file
BetterFy owned, with a backup like any change). The steps are the old ones for
every part: check, create the folders the build needs and record whether this
operation created them, back up each owned file and read the backup back, write
the journal (`backed_up`), write and verify a temporary copy beside each target,
journal `prepared`, publish the pak and then the map (Windows write-through
replace or move), read both back, write the ownership record, journal
`committed`. Rollback checks every part first and changes nothing if one is not
as the operation left it (`rollback_conflict`), puts the map back before the pak,
restores the previous ownership (the chain is validated before the first file
moves, which the single-file code did after), removes only empty folders the
operation created (the `maps` folder, then the language folder) and verifies
every part equals its "before". A part that an earlier restore attempt already
put back is accepted, so a restore that stopped between the two files is finished
by the next one; a file that is neither state is still a conflict.
Recovery of an operation that never committed works per part: a part found as it
was is left alone, one found as the operation leaves it is rolled back, anything
else is `deployment_recovery_conflict`; leftover temporary copies are removed. A
stale install (`release_stale_deployment`) is one where any file BetterFy owns,
pak or map, is missing or changed; letting go forgets the whole record and touches
no game file. Failures were injected before publish, between the two publishes and
after both (`FailurePoint::BetweenPublishes` exists only in tests); the stress
stop points the app offers to a developer account are unchanged.

**Plan identity.** A build without a map keeps its bundle plan's identity. A build
with a map has `planId = sha256` over a length-prefixed tag, the bundle plan's
identity (empty for a map alone), the full ordered package list, the map's engine
ID, its pinned SHA-256 and the analyzer's contents identity. So the same pak with
a different map, or without one, is a different reviewed plan, and the merged VPK
and its hash are unchanged by adding a map. The plan carries `hasPak`, `map` (one
`MapPlanItem`) and the full `packageIds`; the deployment records those ids in the
journal and ownership, and installed-state checks rebuild the build from them and
compare through `VerifiedTreeVpk::matches_receipt`. Staging copies both files
(`pak66_dir.vpk`, `maps/dota.vpk`) and reopens them, and the install compares the
reopened bytes with the reviewed ones.

**One map at a time.** Two maps in one selection are refused with
`map_terrain_conflict` before anything is downloaded. The catalog already puts all
terrains in one slot, so the interface offers the choice first.

**The allowlist.** `src-tauri/maps/manifests.json` (generated, compact, sorted by
ID) and `src-tauri/maps/index.json` (the interface's copy) are kept apart from the
wardrobe files on purpose; `map_terrain.rs` loads and validates them, and the
package registry reserves the `map.` namespace and these catalog IDs. An entry
pins the ZIP (size, SHA-256, the mirrors that served exactly those bytes), the
`maps/dota.vpk` inside it and the `Guide.txt` beside it (size and SHA-256), and
what the analyzer found. The download uses the same transport and trusted hosts as
a skin; the ZIP must hold exactly the two pinned entries; only the verified VPK is
stored. Every build re-reads it from the content store (size and SHA-256), re-runs
`map_package::analyze` and requires the result to equal the pinned audit
(`map_audit_mismatch` otherwise).

`map_package.rs` is not the skin analyzer, and its shared-path logic does not
apply. It reads the VPK (version 1 or 2, every entry embedded, section sizes that
account for every byte of the file, a CRC check of every entry), refuses
signature or archive-checksum sections and side archives, and requires lower-case
ASCII paths without empty or dot segments, all under `maps/`, no duplicates,
`maps/dota.vmap_c` present, and only the 12 compiled-map file types seen in the
audited archives. Scripts and programs by extension are refused as
`map_entry_script`. That is a check on names and types: an entity lump
(`vents_c`) could still name scripts the game itself ships, and nothing in the
report says a map is harmless in game. The report records the entry count, the
types, the folders under `maps/`, a contents identity and how many of the map's
paths Valve's own `pak01_dir.vpk` file list also has (0 of 1,214 to 2,308 in every
audited map: the package is a separate namespace; what it replaces is the game's
map package as a whole, a whole-file replacement that a Dota patch could make
stale, which is an unverified risk and not a measured one).

**The generator.** The ignored tests `map_audit::generate_map_allowlist` and
`spot_check_embedded_maps` (local only; the environment variables are listed in
the file header and `scripts/map-terrains/fetch_maps.py` produces the fetch
report) write the two JSON files and [`MAP_AUDIT_REPORT.md`](MAP_AUDIT_REPORT.md)
from archives downloaded on a Mac, and check the app's own extraction and load path
against them. Output carries no timestamp. Ordinary tests check on every run that
the embedded files are canonical and valid.

**What has and has not been run.**

- Compiles, passes clippy and the unit tests on macOS (also with the
  `internal-stress-test` feature): installs of a map alone, of a pak and a map, and
  of each transition between the two (added, changed, dropped, the pak dropped for a
  map alone) with exact-byte restore step by step; refusal of a foreign file, a
  non-folder and (on Unix) links in the map path with nothing written; cleanup of
  empty folders and refusal to remove non-empty or pre-existing ones; stale and
  missing maps; failures before publish, between the two publishes and after both,
  each recovered; a restore that stopped halfway; records of a build without a map
  unchanged on disk; plan identity and staging with synthetic maps; the analyzer
  against hostile synthetic packages.
- Run locally on a Mac and not committed: the 12 real ZIPs (fetched with `curl`,
  GitHub and Hugging Face both byte-identical by SHA-256 for all 12) through the
  generator, extraction, the content store and plans, and a plan of one real map
  beside a real hero skin (numbers in the test plan). The archives did not go
  through BetterFy's transport.
- Not run: Windows CI for this change (see the PR), an install of a map on Windows
  (so no Windows rule above is observed: write-through replace of a 50 MB file, the
  reparse-point check, removal of the `maps` folder), and anything in Dota: whether
  the game mounts `dota_<language>/maps/dota.vpk` from BetterFy's install, shows
  the map in a match, and what a later Dota patch does to it.

### Installed profile

After a committed pilot install, Rust writes one derived record to
`<app_data>/engine-v1/installed-profile.json` (schema version 1): deployment
operation ID, plan ID, ordered package IDs, language, installed SHA-256, UTC
install time, the Steam Dota build number seen at install time, the linked Steam
operation ID once launch options are applied, and the app version. It holds no
filesystem paths, Steam IDs or account names. The file is written to a synced
temporary file in the same directory and renamed over the previous one;
symlinked paths, unknown fields and malformed values are rejected.

The profile is informational. Ownership record and journals remain the source of
truth, and their schema and transaction steps are unchanged. A profile whose
operation, hash, plan, language or package order differs from the current owned
deployment is ignored as stale. A failed profile write or clear never fails or
alters the install or rollback result. A successful rollback clears the profile,
so a deployment restored from an earlier chain, or one installed before this
record existed, has no profile until it is installed again.

`current_tree_pilot` returns the matching profile and a `dotaPatched` flag,
true only when both the recorded and the current build numbers are known and
differ. This replaces the browser-storage build baseline the Tree Mod screen
used before. The write, stale-profile, privacy and failed-write paths are covered
by synthetic tests on macOS; the profile has not yet been observed on a Windows
install.

### Signed package catalog

The manifests are also published as a signed catalog so new or updated packages
can reach existing installs without an app release. `npm run catalog:publish`
writes `website/public/bot/catalog/index.json` (schema 1, a sequence number one
higher than the previous one, issue and expiry times 180 days apart, and every
manifest) and signs it with `tauri signer` using a catalog key that is separate
from the updater key. The auth Worker serves these files as static assets at
`/catalog/index.json` and `/catalog/index.json.sig`. `npm run catalog:check`
fails CI when the published catalog no longer matches `src-tauri/packages` or
expires within seven days; a Rust test fails when the committed signature does
not verify.

At startup the desktop calls `refresh_catalog` (serialized per process).
`catalog_index.rs` re-verifies the cached catalog, then fetches the remote one
over HTTPS with bounded sizes. A remote catalog is accepted only if:

- the signature over the exact bytes verifies against the embedded public key
  (the catalog files are committed with `-text` in `.gitattributes`, because a
  Windows checkout rewriting line endings broke exactly this in CI);
- its sequence is at least the highest verified one seen on the device and at
  least the one committed with the build (`build.rs` embeds it as a floor), is
  not more than one million above that, and a reused sequence has identical
  bytes;
- it is unexpired, issued no more than a day in the future, and valid for at
  most 200 days;
- every manifest passes the embedded validation, including an allowlist of
  compiled data extensions (`vcss_c`, `vmat_c`, `vmdl_c`, `vpcf_c`, `vsnd_c`,
  `vtex_c`, `vxml_c`; compiled Panorama scripts are rejected);
- no package ID changes its contract (source plus every resource). This holds
  for packages the build ships with and for packages first accepted on this
  device, whose contract hashes are kept in `engine-v1/catalog/contracts.json`.
  Installed builds are re-verified against their package contracts, so new
  content needs a new package ID.

Accepted manifests may update metadata of shipped packages and add packages; a
catalog cannot remove one. An expired cached catalog is still activated for the
packages it provided, so installed builds stay verifiable offline; only the
remote catalog must be fresh. Any failure keeps the current set active.

The private key lives outside the repository (by default
`~/.betterfy/catalog-signing.key`). Without it no new catalog can be signed; after
the published one expires, clients keep the cached set and ignore newer content
until an app release embeds a new public key. A leaked key cannot change what an
existing package ID installs, use another host, write outside the language
folder, or ship scripts. It could add packages built from data files at some
commit reachable under the trusted repository path (GitHub may also serve fork
commits there) and change metadata such as verified languages. Rotation is an
app release that embeds the new public key, followed by a catalog signed with it.

## Definition of done for filesystem writes

- Unit tests cover path validation, traversal attempts, conflicts, and journal
  recovery.
- Integration tests use temporary directories only.
- A killed process can be restarted and recovered from the journal.
- The same ordered input produces the same build plan and checksums.
- Logs contain paths and operation IDs but never auth tokens or personal data.
