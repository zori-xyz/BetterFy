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
deploy it to one selected, allowlisted language slot, prove it in Dota on Windows, and
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
- a single BetterFy-owned `pak66_dir.vpk` target under the selected language
  folder; the generic debug path remains fixed to `game/dota_dutch`;
- refusal to replace a target not proven to be owned by BetterFy;
- verified backup, same-directory publish, installed-hash verification, journal,
  crash recovery, and rollback that refuses external edits;
- injected failures immediately before and after publication.

Now covered by the internal verified-build pilot (not a product enablement claim):

- a fixed 21-resource contract, pinned HTTPS acquisition with exact size/hash
  checks, immutable cache reads, deterministic VPK construction, reopen, and
  journaled staging/rollback on macOS;
- two additional pinned HUD contracts and an ordered three-package build whose
  plan identity changes with priority even when output resources do not collide;

Still required before public enablement (the fixed internal UI path is now wired):

- review distribution and notices for compiled game-derived resources;
- exercise the cancellable per-resource intake worker on Windows and confirm
  that an interrupted download resumes from the verified cache;
- validate the confirmed staging/deployment and post-restart recovery journey on
  Windows; Dutch install and the visible rollback have founder evidence, while
  exact-byte comparison, post-restart discovery, and interruption recovery remain open;
- repeat containment and output limits if an archive/extraction package is added;
- add manifest signature and key-rotation policy;
- pass the native Windows matrix, including interrupted deploy and rollback.

Implemented locally as the foundation for the next slice:

- ordered multi-package bundle planning where the first selected package wins a
  differing-resource collision;
- identical-resource deduplication and an explicit override report;
- one deterministic embedded VPK instead of blindly renaming upstream
  `pak##_dir.vpk` files;
- verified embedded-VPK normalization back into resource maps before merging;
- reviewed plan identities and staging journals bound to the complete ordered
  input set, including resources shadowed by higher-priority packages.

The three-package path remains unverified in Dota until a native Windows
multi-package pass exists. Its deterministic merge, priority accounting,
per-package effective-resource report, and transactional boundaries are covered
synthetically.

Exit condition: Tree Mod is visibly active after a confirmed patch, Steam alone
restarts, and every tested interruption returns to an explainable recoverable state.

## Remaining pilot integration

- complete provenance, redistribution, and signed-manifest review for the
  pinned resources;
- verify the per-resource cancellation and progress UI under slow and broken
  network conditions on Windows;
- validate the connected Steam-profile activation and interrupted-operation
  recovery actions on Windows. The user still starts Dota manually;
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
