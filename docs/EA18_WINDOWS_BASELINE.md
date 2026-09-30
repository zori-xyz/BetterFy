# EA.18 Windows baseline

This record fixes the verified Early Access baseline before BetterFy moves from
the pinned three-package pilot toward a generic package engine. It separates
what CI proved, what the founder observed on Windows, and what still requires a
repeatable artifact or a broader compatibility matrix.

## Release identity

- Release: `BetterFy v0.1.4 · Early Access 18`
- Tag: `v0.1.4-ea.18`
- Source commit: `97df2f88e3ddbc2842a55c77effffcf6548c3cfc`
- User-facing artifact: `BetterFy-Setup.exe`
- Artifact size: `31,146,496` bytes
- SHA-256: `f1a8a5df56c303eb61cd59c78ebcc9bcf1d2d4fee2d00c7b719d36e9cc275126`
- Windows release workflow: [run 36337097018](https://github.com/zori-xyz/BetterFy/actions/runs/36337097018)

## Automated evidence

The tag's `Early Access Windows installer` workflow completed successfully on a
native `windows-latest` runner. It ran the root frontend/engine check, Rust
formatting and Clippy for the engine and installer, built the internal NSIS
updater artifact, embedded the real `betterfy.exe` payload into BetterFy Setup,
built the branded installer, generated the SHA-256 sidecar, and published the
immutable prerelease.

The downloaded release asset was independently identified as a PE32+ x86-64
Windows GUI executable, and its bytes matched the published SHA-256 sidecar.

## Founder-observed Windows evidence

On 2026-09-27 the founder reported completing the Windows application and Setup
pass for EA.18. The observed baseline includes:

- BetterFy Setup renders and installs the embedded application;
- the installed application launches and the account sign-in path works;
- rerunning Setup recognizes the installation and supports the update flow;
- the in-app uninstall flow and reinstall entry point work;
- the pinned Tree Mod, Show Net Worth, and Repopulate Unit Query HUD bundle
  installs through the verified pilot path and is visible in Dota through the
  tested Dutch language slot;
- restoring the pilot returns the tested game state and Steam is restarted by
  the application;
- the internal recovery controls previously reported `PASS` before and after
  the VPK publish and Steam-profile publish boundaries.

This is founder-reported human verification on one Windows machine. It is not a
portable automated evidence bundle: the machine's Windows build, installation
topology, and the test report JSON were not retained in the repository.

## Accepted Early Access limitations

- The installer is unsigned and Windows may show an unknown-publisher warning.
- `uninstall.exe` may remain in an otherwise empty install directory because a
  running Windows executable cannot delete itself.
- Setup detects a missing WebView2 Runtime and links to Microsoft; it does not
  provision the runtime itself.
- Install progress names finite steps but reports their completion as one batch.
- The Setup payload is currently one application executable. Its staged,
  byte-verified rename is atomic for that payload; a future multi-file payload
  requires a whole-payload transaction and rollback plan.
- The signed in-app updater's NSIS compatibility path still needs evidence from
  a published signed update, not an unsigned Early Access artifact.
- The retained evidence does not prove every language folder, a second machine,
  every Steam library layout, universal mod compatibility, VAC safety, or ban
  immunity.
- General catalog deployment remains closed. Only the three pinned pilot
  packages may request a live Dota write.

## Baseline decision

EA.18 is the verified Windows Early Access baseline for continuing engine work.
It is not a signed Stable release. New installer work is limited to reliability,
signing, updater compatibility, and regressions; the active product milestone is
the generic package engine described in [ROADMAP.md](ROADMAP.md).
