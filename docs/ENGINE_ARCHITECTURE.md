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
`pak66_dir.vpk`; the current CSS fixtures cannot satisfy that contract. The
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

The normal release build and the internal stress build share the same transaction
code. Only the CI `windows-build` artifact enables the `internal-stress-test`
feature. That feature exposes two deterministic stop points on each game-file and
Steam-profile transaction: after the verified temporary state is journaled, and
immediately after the atomic replacement. Each control immediately invokes the
normal recovery path and requires a matching recovery receipt. The Early Access
release workflow does not enable these controls.

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
outside the `LaunchOptions` value, and adds only `-language dutch`. An existing
foreign `-language` argument is reported as a conflict instead of being replaced.
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
requires one verified `pak66_dir.vpk`, rehashes and reopens it, then targets only
`game/dota_dutch/pak66_dir.vpk` for the generic debug path. The verified-build pilot
instead chooses an allowlisted language folder before installation and records
it in the journal and receipt. The runtime must already prove Steam and Dota are
closed. An existing target is replaceable only when BetterFy's ownership record
matches its current hash; an unknown target is a hard conflict.

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
so their manifests record no verified language. Packages that need Panorama CSS or
XML changes or Minify's script hooks are still not installable.

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
