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
user-facing game-directory write remains closed until the Tree Mod package and
native Windows evidence exist.

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
rollback path. This transaction currently owns only BetterFy's `-language dutch`
argument.

### Implemented game-deployment transaction foundation

Rust now contains a clean-room deterministic VPK v1 writer and reader for bounded,
embedded data entries. Paths are lowercase relative ASCII, traversal and case-fold
collisions are rejected, CRC32 is recorded per entry, and the finished archive is
opened and checked again. External archive parts are not accepted.

`deploy_staged_vpk` cannot receive a source or destination from the interface. It
resolves a confirmed BetterFy staging journal, requires its exact reviewed plan ID,
requires one verified `pak66_dir.vpk`, rehashes and reopens it, then targets only
`game/dota_dutch/pak66_dir.vpk`. The runtime must already prove Steam and Dota are
closed. An existing target is replaceable only when BetterFy's ownership record
matches its current hash; an unknown target is a hard conflict.

The transaction writes and verifies a private backup, writes a same-directory
temporary VPK, publishes with Windows write-through APIs, reopens and rehashes the
installed bytes, and commits an ownership record plus journal. Rollback restores
the exact prior bytes and prior ownership chain, or removes an initial install. It
refuses to overwrite a post-install external edit. Recovery distinguishes an
interruption before publish from one after publish and either marks the untouched
operation failed or rolls the published bytes back. Synthetic tests inject both
failures. The Tree Mod resource package and native Windows validation are still
gates; therefore no current catalog selection can invoke a live deploy.

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
archive extraction, Tree Mod package wiring, and live Dota deployment remain
disabled. The full threat model is documented in
`docs/CONTENT_INTAKE_SECURITY.md`.

## Definition of done for filesystem writes

- Unit tests cover path validation, traversal attempts, conflicts, and journal
  recovery.
- Integration tests use temporary directories only.
- A killed process can be restarted and recovered from the journal.
- The same ordered input produces the same build plan and checksums.
- Logs contain paths and operation IDs but never auth tokens or personal data.
