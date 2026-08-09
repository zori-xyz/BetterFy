# BetterFy roadmap

The roadmap is ordered by trust, not by spectacle. Each milestone must be
recoverable and observable before the next one can write closer to the game.

## Foundation

Completed:

- Windows-first Tauri shell with React and Rust boundaries;
- Russian and English interface, dark and light themes, reduced-motion support;
- responsive desktop layouts and automated visual journeys;
- Dota 2 discovery across Steam libraries with marker verification;
- deterministic fixture planning, staging, journaling, verification, rollback;
- local preset manager with validated JSON import and export;
- signed updater and Windows installer infrastructure;
- Steam profile selection and verified launch-option activation;
- privacy-safe Windows readiness diagnostics for discovery, runtime, profiles,
  and staging recovery;
- deployed Telegram identity with challenge-bound native approval, rotating
  desktop credentials, primary-consistent one-time redemption, avatar proxying,
  and per-device revocation;

## Active milestone: one real patch, end to end

The next engine slice is deliberately narrow: package the pinned Minify Tree Mod,
deploy it to BetterFy's fixed language slot, prove it in Dota on Windows, and
restore the previous state exactly. No Workshop or community layer starts first.

Implemented foundation:

- strict version 1 package manifests with explicit provenance, permission,
  compatibility, and signature states;
- SHA-256 content identity and byte-size verification;
- immutable no-clobber publication under BetterFy application data;
- idempotent retry across both sides of object publication;
- fixture build consumption from the verified store;
- recipe-to-package cross-checking and exact reviewed-plan confirmation;
- diagnostics for an empty, verified, or corrupt content store;
- cancellable, bounded HTTPS acquisition for commit-pinned repository fixtures;
- proxy bypass, public-address DNS pinning, connected-peer verification, and
  bounded same-origin redirects;
- hash-before-publication and a second read before the immutable store;
- metadata-only ZIP rejection for traversal, links, ambiguous names, executables,
  unsupported compression, and archive-bomb limits;
- typed queued/downloading/verifying/ready/failed/cancelled operation states.

The deployment foundation now also includes:

- deterministic VPK v1 construction with embedded entries and CRC verification;
- a second VPK open and validation pass before a staged artifact is accepted;
- a single fixed BetterFy target, `game/dota_dutch/pak66_dir.vpk`;
- refusal to replace a target not proven to be owned by BetterFy;
- verified backup, same-directory publish, installed-hash verification, journal,
  crash recovery, and rollback that refuses external edits;
- injected failures immediately before and after publication.

Still required before the pilot is enabled:

- pin the 21 Tree Mod resources as an explicit package contract without importing
  Minify's Python hooks or generic patcher;
- normalize that package into immutable staging content and build `pak66_dir.vpk`;
- repeat containment and output limits during extraction;
- add manifest signature and key-rotation policy;
- pass the native Windows matrix, including interrupted deploy and rollback.

Exit condition: Tree Mod is visibly active after a confirmed patch, Steam alone
restarts, and every tested interruption returns to an explainable recoverable state.

## Remaining pilot integration

- turn the accepted Tree Mod ledger into the first real content recipe;
- acquire and verify every listed resource without accepting package-defined code;
- emit one reviewed VPK plan and one verified staging receipt;
- connect the existing confirmed runtime, deploy, recovery, Steam activation, and
  rollback commands as one resumable operation;
- expose factual progress and recovery states without presenting success before
  the final installed-byte verification;
- record the native Windows evidence in the test checklist.

Exit condition: an interrupted Tree Mod operation can always be explained and
recovered without relying on interface state.

## Later milestones

### Production services

- finish secure browser cookies, account deletion/retention, and native Windows
  vault/Telegram approve-deny-expiry interaction evidence defined in
  `IDENTITY_AND_WEB_ARCHITECTURE.md`;
- catalog delivery with authenticated manifests;
- release signing, public updater endpoints, and rollback policy;
- privacy and retention documentation for every remote service.

### BetterFy Workshop

- signed community submissions with source and author attribution;
- moderation and compatibility review;
- shareable presets that reference immutable content identities;
- clear separation between functional mods and wardrobe content.

### Release asset pass

- replace temporary Dota imagery with approved assets;
- finalize the application icon and installer artwork;
- complete third-party notices and select the source license;
- verify every public claim against the production build.

## Release gates

A public build does not ship until all applicable gates are green:

- native Windows checks and installer build pass;
- updater signatures and endpoints are configured outside the repository;
- privileged operations have failure-injection and recovery coverage;
- RU and EN journeys are complete in both themes;
- keyboard navigation and reduced motion remain usable;
- no temporary Dota assets or unverified safety claims are present;
- release notes distinguish implemented behavior from preview behavior.
