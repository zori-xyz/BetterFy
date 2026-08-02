# BetterFy

BetterFy is a desktop manager for Dota 2 mods and skin packs, built with Tauri,
React, and TypeScript.

The current build is an interactive product prototype. Authentication, backups,
patching, and launch progress are simulated. Tauri game discovery performs a
read-only marker-file check for standard Steam locations; browser discovery is
explicitly simulated. BetterFy does not modify Dota 2 files yet.

## Run locally

```bash
npm install
npm run tauri dev
```

For browser-only UI work:

```bash
npm run dev
```

## Build on Windows

On a Windows PC with Node.js, stable Rust, and Microsoft C++ Build Tools:

```powershell
.\scripts\build-windows.ps1
```

The script installs locked dependencies, builds the UI, runs Rust tests, and
creates an unsigned NSIS installer. Detailed prerequisites and artifact paths
are in [docs/WINDOWS_BUILD.md](docs/WINDOWS_BUILD.md). GitHub Actions also
produces a Windows installer artifact for every push to `main` and pull request.

## Checks

```bash
npm run check
```

## Current surface

- Russian and English interface
- sign-in and first-launch scenarios
- read-only Tauri Dota path discovery and validation
- deterministic fixture `BuildPlan` with conflict detection
- Home workspace and complete preview Build journey: conflict resolution,
  progress, success, and recovery
- mod catalog, selection states, guide, details, and conflict resolver
- curated BetterFy skin packs
- imported Dota2PornFxWeb catalog
- settings and profile flows
- contextual BetterFy community surfaces linked to `@BetterFyBot`
- mock engine behind a replaceable `EngineBridge`
- Telegram auth bridge with demo fallback and a configurable HTTPS backend boundary
- local config manager with validated Rust persistence, JSON import/export, and
  three built-in BetterFy Workshop compositions

## Production boundary

Config storage, Steam/Dota marker validation, and deterministic fixture planning
have real Rust boundaries. Build execution, downloads, VPK assembly, deployment,
rollback, game launch, and production authentication are not implemented yet.
The interface continues to label those paths as preview behavior.

The planned Rust boundary and safe first implementation slice are documented in
[docs/ENGINE_ARCHITECTURE.md](docs/ENGINE_ARCHITECTURE.md).
