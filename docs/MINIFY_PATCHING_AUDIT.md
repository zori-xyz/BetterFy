# Minify patching audit and BetterFy compatibility contract

This document pins the behavior BetterFy is studying before any production write
to a Dota 2 installation. The audited upstream is the official
[`Egezenn/dota2-minify`](https://github.com/Egezenn/dota2-minify) repository at
commit `3a85572029f2c264e2a17cee1c9b54ce93e4fd93` (2026-08-02).

The upstream repository is GPL-3.0. BetterFy therefore treats this as a behavioral
and file-format study. We do not copy its Python implementation or execute its mod
scripts. Any compatible implementation is written against the contract below and
must pass BetterFy-owned tests.

## What Minify actually does

The upstream patcher performs these stages:

1. Locks its UI and refuses to patch while a process named `dota2.exe` (`dota2` on
   non-Windows systems) is running. It does not close Dota itself.
2. Verifies helper binaries. `ripgrep` is used to expand blacklist patterns.
   Source2Viewer and Dota's `resourcecompiler.exe` are required for mods that
   change Panorama XML/CSS or contain uncompiled resources.
3. Reads the selected mod manifests, resolves dependencies, rejects declared
   conflicts, and gathers all requested transformations.
4. Extracts requested files from `game/dota/pak01_dir.vpk` and
   `game/core/pak01_dir.vpk`, decompiles resources, applies XML/CSS changes, then
   recompiles them through Workshop Tools.
5. Builds side-loaded outputs in `game/dota_<locale>`:
   - `pak66_dir.vpk` for native/compiled Minify transformations;
   - `pak65_dir.vpk` for merged VPK mods and normal-priority browser mods;
   - `pak67_dir.vpk` for high-priority D2PFX categories;
   - `maps/dota.vpk` for a selected terrain.
6. Adds `-language <locale>` to each selected Steam account's Dota launch options
   in `userdata/<steam-id>/config/localconfig.vdf`, preserving the other options.
   English is routed through the `dutch` language slot and has a separate fix.
7. If launch options were changed on Windows, invokes `steam.exe -exitsteam`,
   waits briefly for Steam to disappear, and starts Steam again. Launching Dota is
   a separate, optional `steam://rungameid/570` action.

The important consequence is persistent activation: the language launch option
and side-loaded VPK files remain in place. A player can later launch Dota normally
through Steam without opening the patcher again.

## Supported transformation vocabulary

Minify mods use several mechanisms. BetterFy will represent supported mechanisms
as validated data, never executable mod code:

| Upstream mechanism | Effect | BetterFy operation |
| --- | --- | --- |
| `files/` | Copy already compiled resources into output | `copy_compiled` |
| `files_uncompiled/` | Compile source assets with Workshop Tools | `compile_resource` |
| `blacklist.txt` | Replace matching resources with typed blank resources | `blank_resource` |
| `replacer.json` | Extract one base-VPK resource under another path | `replace_from_base` |
| `styling.css` | Decompile and extend Panorama CSS | `patch_panorama_css` |
| `xml.json` | Add/move elements, includes, and attributes | `patch_panorama_xml` |
| input VPK | Merge files into an output package | `merge_vpk` |
| terrain VPK | Install one `maps/dota.vpk` | `install_terrain` |

The upstream also loads arbitrary Python hooks at loop, after-decompile,
after-recompile, after-patch, uninstall, and utility stages. Existing hooks fetch
guides and hero grids, replace fonts, transform custom backgrounds, patch English
localization, and modify user configuration. BetterFy does not import or execute
these hooks. Each desired feature needs a dedicated, allowlisted Rust operation
with its own destination policy, download policy, backup, and rollback tests.

At the pinned commit, the bundled native catalog contains 33 mod directories:
14 use blacklists, 11 use styling patches, 6 use XML patches, and 18 hook scripts
exist across the scripted features. Ten directories provide explicit manifests;
the remainder rely on their folder content and scanner defaults. No bundled mod
uses `replacer.json` at this commit, although the engine supports it. This inventory
is a compatibility snapshot, not a reason to accept future files automatically.

## Paths touched by the compatibility layer

Read-only inputs:

- `steamapps/libraryfolders.vdf`
- `steamapps/appmanifest_570.acf`
- `dota 2 beta/game/dota/pak01_dir.vpk`
- `dota 2 beta/game/core/pak01_dir.vpk`
- `dota 2 beta/game/dota/steam.inf`
- the selected Steam account's Dota launch options

Potential production destinations, all requiring a verified restore point:

- `dota 2 beta/game/dota_<managed-locale>/pak65_dir.vpk`
- `dota 2 beta/game/dota_<managed-locale>/pak66_dir.vpk`
- `dota 2 beta/game/dota_<managed-locale>/pak67_dir.vpk`
- `dota 2 beta/game/dota_<managed-locale>/maps/dota.vpk`
- `Steam/userdata/<id>/config/localconfig.vdf`

Temporary compile directories under Dota's `content/dota_addons` and
`game/dota_addons` are not production destinations. BetterFy should prefer its own
app-data staging directory and use Dota's compiler input only for the narrow time
that `resourcecompiler.exe` requires it. Every temporary path must be removed or
recoverable after interruption.

## BetterFy process contract

The user-approved lifecycle is stricter than upstream Minify:

```text
preflight
  -> obtain explicit patch confirmation
  -> request Dota shutdown
  -> verify dota2.exe is absent
  -> request Steam shutdown
  -> verify Steam and all Dota processes are absent
  -> acquire BetterFy operation lock
  -> freeze immutable BuildPlan
  -> stage and verify all outputs
  -> create and verify backups
  -> atomically deploy and journal every change
  -> verify installed hashes and launch options
  -> release lock
  -> start Steam only
  -> report ready; the user starts Dota
```

Rules:

- No build, compile, launch-option edit, or deployment begins while Dota or Steam
  is still running.
- Closing is a visible preflight stage. If graceful shutdown times out, the build
  stops and explains which process remains. Force termination must be a separate
  explicit user choice; it is never a silent fallback.
- Steam is relaunched only after the journal is durably committed and installed
  outputs have been re-read and verified.
- BetterFy never auto-launches Dota after patching. The player launches it from
  Steam and can continue doing so without BetterFy while the profile is active.
- An interrupted operation is recovered before another operation may start.
- A Dota update (`steam.inf` change) invalidates cached decompilation and marks the
  active profile as requiring rebuild; it does not trigger an unattended write.

## Steam launch-option ownership

BetterFy owns only the launch argument it adds for its managed language slot. It
must parse the existing option string, preserve unrelated flags and commands, and
write a verified backup of `localconfig.vdf` before replacement. Uninstall removes
only the BetterFy-owned language argument. It must not rewrite all Steam accounts
unless the user explicitly selected that scope.

Steam must be stopped before `localconfig.vdf` is replaced so the client cannot
overwrite the change from memory. The replacement uses a same-directory temporary
file, parse verification, flush, and atomic rename. The journal stores before and
after hashes, not account names or authentication data.

## Recovery contract

Minify's emergency wipe can delete an entire `game/dota_<locale>` directory.
BetterFy will not use that strategy. Recovery is ownership-based:

- restore the exact files recorded in the last committed journal;
- delete only outputs that carry the matching BetterFy operation marker and whose
  current hash still matches the installed hash;
- never delete an unknown language directory or a file changed after deployment;
- restore launch options from a verified backup or remove only the owned token;
- make rollback resumable and idempotent.

## Security differences from upstream

The compatibility target is the visible output, not every upstream behavior.
BetterFy additionally requires:

- canonical containment checks for every archive entry and destination;
- rejection of traversal, absolute/drive/device paths, ADS, links, case-folding
  collisions, oversized archives, and unexpected executable content;
- HTTPS downloads with pinned SHA-256 and bounded size/time;
- fixed executable paths and argument arrays for Source2Viewer,
  `resourcecompiler.exe`, Steam, and any optional converter;
- no arbitrary Python, JavaScript, DLL loading, shell strings, or process-memory
  modification from catalog packages;
- deterministic plans and reproducible output hashes;
- honest compatibility labels tied to a Dota build and validator version.

Claims such as “VAC-safe” are not inferred from Minify and are not product
guarantees. BetterFy reports only what it can verify.

## Implementation gates

Production deployment remains disabled until all gates pass:

1. Windows process inspection and graceful stop/start lifecycle tested on a real
   Steam installation.
2. Launch-option parser and atomic VDF replacement tested with fixtures and a real
   disposable account configuration.
3. VPK generation is deterministic and opens successfully after writing.
4. Backup, deploy, crash recovery, and rollback pass injected-failure tests.
5. A real Windows matrix covers default and secondary Steam libraries, paths with
   spaces/non-ASCII characters, RU/EN UI, Steam already closed, Dota running,
   shutdown timeout, Dota update, and insufficient disk space.

Current evidence: gates 3 and 4 pass synthetic Rust tests. Gates 1, 2, and 5 still
need the recorded native Windows pass; synthetic evidence is not treated as a
successful Dota patch.

## First production pilot: Tree Mod

The first package is Minify `Tree Mod` at upstream commit
`3a85572029f2c264e2a17cee1c9b54ce93e4fd93`. It is intentionally chosen because
its `files/` directory contains 21 already compiled `.vmat_c`, `.vtex_c`, and
`.vmdl_c` resources and uses no hook script, generic Python patch action, Panorama
decompile, XML patch, or Workshop Tools invocation. Upstream describes the result
as replacing trees with small round bushes and requires the default terrain.

BetterFy will not copy the upstream patcher or silently vendor these game-derived
resources. Before enabling the item, every resource needs a pinned path, byte size,
SHA-256, source notice, and accepted distribution decision. Only those bytes may
enter the deterministic VPK builder. The output is fixed to BetterFy's owned
`game/dota_dutch/pak66_dir.vpk` slot.
