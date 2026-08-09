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
  desktop credentials, avatar proxying, and per-device revocation;

## Active milestone: trusted content intake

The next engine slice turns catalog entries into verifiable local inputs without
touching the Dota installation.

Implemented foundation:

- strict version 1 package manifests with explicit provenance, permission,
  compatibility, and signature states;
- SHA-256 content identity and byte-size verification;
- immutable no-clobber publication under BetterFy application data;
- idempotent retry across both sides of object publication;
- fixture build consumption from the verified store;
- recipe-to-package cross-checking and exact reviewed-plan confirmation;
- diagnostics for an empty, verified, or corrupt content store.
- cancellable, bounded HTTPS acquisition for commit-pinned repository fixtures;
- proxy bypass, public-address DNS pinning, connected-peer verification, and
  bounded same-origin redirects;
- hash-before-publication and a second read before the immutable store;
- metadata-only ZIP rejection for traversal, links, ambiguous names, executables,
  unsupported compression, and archive-bomb limits;
- typed queued/downloading/verifying/ready/failed/cancelled operation states.

Still required before production catalog content is accepted:

- normalize accepted packages into immutable staging content;
- repeat containment and output limits during extraction;
- add manifest signature and key-rotation policy;
- connect retry/progress UI only after the production catalog registry exists.

Exit condition: a catalog item can become a verified staged package, and every
failure leaves the game and Steam untouched.

## Next: production build transaction

- resolve dependencies and conflicts against real staged packages;
- produce a deterministic deployment plan and review summary;
- close Dota 2 and Steam before any privileged write;
- create and verify a private backup of every affected target;
- build the required game payload in BetterFy-owned staging;
- publish atomically where the filesystem permits it;
- verify installed bytes and persist a durable recovery journal;
- restart Steam only; the player starts Dota 2;
- restore safely without overwriting unrelated external edits.

Exit condition: interrupted execution can always be explained and either
resumed or recovered without relying on interface state.

## Later milestones

### Production services

- finish secure browser cookies, account deletion/retention, and native Windows
  vault/Telegram interaction evidence defined in
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
