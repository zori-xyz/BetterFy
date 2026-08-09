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

## Release boundary

An unsigned CI artifact is for internal testing only. Public distribution still
requires a Windows code-signing certificate, signed updater configuration,
hash publication, dependency/license review, and real Steam/Dota integration
tests on Windows.
