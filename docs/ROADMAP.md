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
  Windows; the founder has run the ordered three-package install, the visible
  rollback, and the internal stress artifact's controlled recovery pass
  (interrupt before/after publish, Steam before/after publish, all `PASS`) as a
  single visually confirmed session; a recorded Windows test report and
  exact-byte comparison against it remain open;
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

The founder has run a native Windows multi-package pass: the ordered
three-package bundle installed and was visually confirmed in Dota, and the
internal stress artifact's controlled recovery pass reported `PASS` on every
check. This is one visually confirmed session on one machine, without a saved
Windows test report; a recorded, repeatable result is still open. The
deterministic merge, priority accounting, per-package effective-resource
report, and transactional boundaries otherwise remain covered synthetically.

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

## BetterFy Setup (branded installer)

`installer/` is a standalone Tauri application that replaces the NSIS wizard
as the installer a new user downloads and runs. It exists because NSIS's
Back/Next/Cancel/Finish buttons belong to the outer Windows dialog frame and
cannot be recolored without unverified low-level window subclassing; this
installer's UI is plain HTML and CSS instead. NSIS is not retired — the signed
auto-updater depends on its artifact format and keeps using it internally.

Implemented, confirmed on a real `windows-latest` CI runner through the build
step (fmt, clippy, and a full `npx tauri build` with the real app embedded),
but not yet run as an installer by a person on Windows:

- copy the embedded main-app payload into `$LOCALAPPDATA\BetterFy`, matching
  Tauri's own NSIS `currentUser` install path exactly, so the auto-updater's
  NSIS pass finds and updates this installation rather than creating a second,
  orphaned one;
- refuse to overwrite a running `betterfy.exe`, reporting `app_running`
  instead of a half-overwritten install;
- detect an existing installation (binary on disk plus registry version) and
  switch the welcome screen to an Update framing with an in-app Uninstall
  button, rather than presenting every run as a first install;
- a Start Menu shortcut under a `BetterFy` folder and a Desktop shortcut
  resolved via the real `FOLDERID_Desktop` known folder (not a hardcoded
  `%USERPROFILE%\Desktop` guess, which misses a OneDrive-relocated Desktop),
  matching the NSIS template's own layout;
- an `HKCU\...\Uninstall\BetterFy` registry entry (name, version, publisher,
  install location, icon, size, uninstall command, help and about links)
  matching every value Tauri's own NSIS template writes;
- a `run_uninstall` command reachable from the welcome screen's own button or
  via `--uninstall`, reusing the same binary as its own uninstaller;
- Russian and English, switchable in the UI; finite, named install steps
  instead of an indeterminate looping progress bar; `prefers-reduced-motion`
  honored; error screens with a plain-language explanation plus Retry and
  Contact-support actions, not just a raw code;
- the sidebar is a live CSS/HTML build of the reference design, with the
  "Join Our Telegram" text positioned near the QR per direct founder
  feedback on an earlier draft;
- confirming Uninstall uses a real, styled screen rather than the browser's
  native `confirm()`, after that dialog was found to be silently suppressed
  in this app's actual WebView2 environment — clicking Uninstall did nothing
  visible at all, which is exactly what a founder test run reported;
- a Microsoft-documented WebView2 Runtime presence check that points a user at
  Microsoft's official download page when missing, rather than fetching and
  running a binary itself;
- `cargo fmt`/`cargo clippy -D warnings` clean on both the native target and
  `x86_64-pc-windows-msvc`, confirmed for real on CI's Windows runner.

A capability-permission bug was found and fixed along the way: closing the
window from the page's own Cancel/Finish/Close buttons silently did nothing
in the first real build, because Tauri v2 gates the built-in window-close
command behind a capability grant that this project never declared. Custom
commands this crate defines are not gated the same way, which is why
installation itself worked while the close buttons did not — see
`installer/README.md`'s "Why capabilities matter".

Still required before this can replace NSIS as the public download:

- a native Windows install/uninstall/update pass by a person — nothing in
  `installer/` has been run as an installer on Windows yet, only compiled
  there;
- an actual update-in-place test: install with BetterFy Setup, then let the
  signed updater's NSIS pass run against that install and confirm it updates
  rather than duplicates;
- WebView2 provisioning beyond detection, if pointing users at Microsoft's
  page proves insufficient in practice;
- the self-delete-after-uninstall step is intentionally not implemented (the
  reliable version needs cmd.exe quoting around a path that may contain
  spaces, which cannot be verified without Windows); accepted for now as one
  leftover file in an otherwise-empty folder;
- its own code-signing certificate, since it is now the first executable a
  new user runs.

Exit condition: a native Windows pass installs, updates in place through the
existing signed updater, and uninstalls cleanly, matching every registry and
path detail NSIS itself would have used.

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
