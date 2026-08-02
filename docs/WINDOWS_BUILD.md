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
