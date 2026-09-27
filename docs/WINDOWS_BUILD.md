# BetterFy Windows build

The release target is Windows. Native Windows bundles must be built on Windows;
the macOS development machine cannot verify Steam discovery or produce a trusted
Windows release artifact.

## Prerequisites

- Windows 10 or 11
- Node.js 22 LTS or newer
- Rust stable installed through rustup
- Microsoft C++ Build Tools with the **Desktop development with C++** workload
- WebView2 (included with current Windows 10/11 installations)

## One-command local build

Open `cmd.exe` or PowerShell in the repository root and run:

```powershell
.\scripts\build-windows.ps1
```

The default output is an unsigned NSIS installer under:

```text
src-tauri\target\release\bundle\nsis\
```

Alternative bundles:

```powershell
.\scripts\build-windows.ps1 -Bundle msi
.\scripts\build-windows.ps1 -Bundle all
```

The script uses `npm ci`, builds the TypeScript interface, runs the locked Rust
test suite, builds Tauri, and fails if no installer is produced.

## Installer artwork

The NSIS bundle uses the BetterFy wordmark as its primary identity and two
release-owned bitmaps:

```text
src-tauri/windows/installer/sidebar.bmp   164 × 314
src-tauri/windows/installer/header.bmp    150 × 57
```

Both are opaque Windows BMP files and are checked during `npm run check`.
Russian and English are bundled into the same installer; Windows chooses the
matching language and falls back to Russian. Installation is scoped to the
current Windows account, so the normal path does not request administrator
access.

`src-tauri/windows/installer.nsi` is pinned to the official Tauri CLI 2.11.4
NSIS template and carries a deliberately small BetterFy patch. The native
finish page keeps the launch checkbox, offers the desktop-shortcut checkbox,
and adds explicit buttons for the BetterFy website and the founder's GitHub.
The installation page retains native progress behavior while applying the
BetterFy violet/ivory control colors. Updater, uninstall, WebView2 and passive
install behavior remain inherited from the matching Tauri template.

On the macOS design machine, regenerate the bitmaps and the review images with:

```bash
npm run installer:render
```

The review images are written to `artifacts/installer-preview/`. They reproduce
the intended native NSIS composition for visual review; the Windows CI artifact
remains the source of truth for packaging and runtime behavior.

## GitHub Actions

`.github/workflows/windows-build.yml` runs the same checks on `windows-latest`
and uploads an unsigned NSIS workflow artifact. CI installs `rustfmt` and
`clippy`, rejects formatting drift, and treats every Rust warning as a build
failure. It does not publish a release, sign the installer, or enable the
updater.

## BetterFy Setup, the branded installer

`installer/` is a second, standalone Tauri application: the user-facing
installer. It exists because NSIS's Back/Next/Cancel/Finish buttons belong to
the outer Windows dialog frame and cannot be recolored without unverified
low-level window subclassing — this installer's whole interface is plain HTML
and CSS instead, so branding it needs no such risk. See
[`installer/README.md`](../installer/README.md) for its full design and
current status.

NSIS is not retired. `tauri-plugin-updater` on Windows is tied to Tauri's own
NSIS/MSI bundler output and signature format, and it silently re-runs that
same NSIS installer, in passive mode, to apply updates. BetterFy Setup and the
NSIS build must therefore install to the exact same place: this was verified
against Tauri's own NSIS template source (`$LOCALAPPDATA\BetterFy`, the
`HKCU\...\Uninstall\BetterFy` registry key, and the `BetterFy` Start Menu
folder), not assumed. Diverging would make an update create a second, orphaned
copy instead of replacing the first. So: NSIS keeps building and keeps feeding
the updater; BetterFy Setup replaces it only as the thing a new user downloads
and runs once.

Build it locally on Windows after building the main app:

```powershell
.\scripts\build-windows.ps1
Copy-Item src-tauri\target\release\betterfy.exe installer\src-tauri\payload\betterfy.exe
cd installer
npx tauri build
```

The unsigned installer is written to
`installer\src-tauri\target\release\betterfy-installer.exe`. CI does the same
two-stage build in `early-access-release.yml` and `windows-build.yml`.

## Release boundary

An unsigned CI artifact is for internal testing only. Public distribution still
requires a Windows code-signing certificate, signed updater configuration,
hash publication, dependency/license review, and real Steam/Dota integration
tests on Windows. BetterFy Setup additionally still needs: a native Windows
verification pass of install, uninstall, and update-in-place behavior (nothing
in `installer/` has run on Windows yet); WebView2 provisioning beyond
detection (it currently points a user at Microsoft's download page rather
than installing the runtime itself); and its own code-signing certificate,
since it is now the first executable a new user runs.
