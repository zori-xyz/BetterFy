# BetterFy engine architecture

BetterFy is still running on the mock bridge in `src/engine.ts`. The UI contract is
already isolated from the implementation, so the Rust engine can replace the mock
without changing the screens.

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
    | "steam_start_failed";
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
execute_build(request) -> BuildReceipt
restore_backup(backup_id) -> RestoreReceipt
list_backups() -> BackupSummary[]
inspect_runtime() -> RuntimeState
prepare_runtime_for_patch(confirmation_id) -> RuntimeState
start_steam(operation_id) -> RuntimeState
list_presets() -> PresetRecord[]
save_preset(request) -> PresetRecord
delete_preset(preset_id) -> ()
export_preset(preset_id) -> String
import_preset(payload) -> PresetRecord
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

Build this before downloads, authentication, subscriptions, or Steam launch:

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
the validated BetterFy-owned operation directory. Repeating rollback is safe.
Tests inject failures after a staged write and before verification.

This is not deployment: no command writes to Dota, creates a game backup, assembles
a production VPK, downloads catalog content, stops processes, or launches Steam.
Those boundaries stay closed until the staging and recovery contract is verified
on Windows. The researched production lifecycle is pinned in
`docs/MINIFY_PATCHING_AUDIT.md`.

The runtime preflight is now implemented behind typed Tauri commands. Windows
process enumeration uses Tool Help APIs and recognizes the Steam client, Web
Helper, overlay, and Dota processes without invoking a shell. With explicit
confirmation, `prepare_runtime_for_patch` requests a normal `WM_CLOSE` for Dota,
waits for it to exit, invokes the registry-resolved `steam.exe -exitsteam`, and
then waits until both products are absent. It never force-terminates a process.
Timeout, unavailable Dota window, missing Steam, and unsupported platform have
stable error codes. This command is intentionally not connected to the current
staging-only preview; it becomes mandatory immediately before production deploy.

Steam launch-option planning now has a BetterFy-owned, lossless VDF boundary.
The parser walks the nested KeyValues path for app `570`, preserves every byte
outside the `LaunchOptions` value, and adds only `-language dutch`. An existing
foreign `-language` argument is reported as a conflict instead of being replaced.
Missing launch options are inserted into the existing Dota app object, and the
updated document is parsed again in tests. Plans expose before/after SHA-256
values but perform no filesystem write. Account selection, verified backup,
same-directory atomic replacement, and rollback journaling remain deployment
gates before this planner can touch a real `localconfig.vdf`.

Preset persistence is implemented as a separate BetterFy-owned boundary. The
backend validates the schema and identifiers, rejects symlinks and oversized
records, and commits JSON records through a temporary file and rollback-aware
rename. Import creates a new local preset and cannot overwrite built-in
workshop entries.

## Definition of done for filesystem writes

- Unit tests cover path validation, traversal attempts, conflicts, and journal
  recovery.
- Integration tests use temporary directories only.
- A killed process can be restarted and recovered from the journal.
- The same ordered input produces the same build plan and checksums.
- Logs contain paths and operation IDs but never auth tokens or personal data.
