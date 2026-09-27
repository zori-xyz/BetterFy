# BetterFy installer

A standalone Tauri/WebView application that replaces the NSIS installer as
the thing a person downloads and runs. It exists because NSIS cannot render
arbitrary UI: the wizard's Back/Next/Cancel/Finish buttons belong to the
outer Windows dialog frame and cannot be recolored without unverified
low-level window subclassing. This installer's whole interface is plain HTML
and CSS, so branding it — including a real violet primary button — needs no
such risk.

NSIS is not retired. `tauri-plugin-updater` is tied to Tauri's own NSIS/MSI
bundler output and signature format, and on Windows it silently re-runs the
NSIS installer, in passive mode, to apply updates. This installer's install
path, registry key, and Start Menu layout are therefore deliberately copied
from Tauri's own NSIS template rather than chosen independently — see
"Why the exact paths matter" below.

## Current status

This is a working scaffold with the update-compatibility details reasoned
through carefully. It is not a release candidate: nothing in this directory
has executed on a real Windows machine yet.

Implemented:

- a five-screen UI (welcome, installing, finish, error, plus an inline
  WebView2 warning) in the BetterFy dark theme, reusing the product's
  wordmark, fonts, and the Telegram QR banner; the primary button is a real
  `<button>` colored in CSS, not a recolored native control;
- Rust commands, gated to Windows, that: copy the embedded payload into
  `%LOCALAPPDATA%\BetterFy`, write a Start Menu shortcut under a `BetterFy`
  folder and an optional Desktop shortcut, register an uninstall entry under
  `HKCU\...\Uninstall\BetterFy` with the same fields Tauri's NSIS template
  writes (name, version, publisher, location, icon, estimated size,
  uninstall command, about/help links), and optionally launch the app;
- `run_uninstall`, reachable by relaunching this same binary with
  `--uninstall` (the registry's `UninstallString` does exactly that — no
  second build target to maintain);
- a WebView2 Runtime presence check against the exact registry keys
  Microsoft's own distribution guide documents, surfaced as a warning with a
  link to Microsoft's official download page — this installer does not fetch
  or run an external binary itself;
- `cargo fmt` / `cargo clippy` clean on macOS (where Windows-only code
  compiles to a `windows_only` stub), and confirmed on a real
  `windows-latest` GitHub Actions runner via `windows-build.yml`: `cargo
  clippy -- -D warnings` and `npx tauri build --debug` both succeed there.
  That real run is also what caught two mistakes this section used to gloss
  over — a genuinely dead `WEBVIEW2_DOWNLOAD_URL` constant, and a hard
  `tauri build` failure (not just a warning, as `tauri dev` suggested) from
  the Rust `tauri` crate's minor version not matching the root project's
  older-pinned `@tauri-apps/api`, fixed by giving this directory its own
  `package.json`. This Mac has no Windows resource compiler (`llvm-rc`), so a
  full local cross-build for the icon-embedding step still isn't possible
  here — CI's Windows runner remains the actual gate, and it now passes
  through the build step, though not yet an actual install/run.

## Why the exact paths matter

Verified against Tauri's own NSIS bundler template
(`tauri-apps/tauri/crates/tauri-bundler/.../nsis/installer.nsi`), not
assumed:

| What | NSIS (`installMode: currentUser`) | This installer |
| --- | --- | --- |
| Install directory | `$LOCALAPPDATA\BetterFy` | same |
| Uninstall registry key | `HKCU\...\Uninstall\BetterFy` | same |
| Start Menu shortcut | `Start Menu\Programs\BetterFy\BetterFy.lnk` | same |
| Desktop shortcut | `Desktop\BetterFy.lnk` | same |
| Main binary name | `betterfy.exe` (the Cargo package name; no `mainBinaryName` override is set) | same |

If these ever drifted, the signed updater's silent NSIS pass would not
recognize an install this tool made — it would install a second, orphaned
copy instead of updating the first one. An early draft of this installer got
the install directory, the binary name's case, and the Start Menu layout
wrong before this was checked against Tauri's actual template source; get
this table wrong again and it will happen silently, not as a build error.

## Known gaps, and why they're gaps rather than bugs waiting to happen

- **The payload is populated by CI, not committed.** `src-tauri/payload/` is
  embedded into the binary at compile time via `rust-embed`. CI builds the
  main app first and copies `betterfy.exe` in before compiling this
  installer (see `.github/workflows/early-access-release.yml` and
  `windows-build.yml`). `run_install` fails closed with
  `payload_not_embedded` if that step is skipped, rather than "installing"
  an empty directory.
- **No native Windows pass yet.** Every claim above about shortcuts,
  registry values, and paths is verified against documentation and, where
  possible, cross-target type-checking — not against a real run. Treat it
  the way this repository treats any engine change: unverified until
  `docs/WINDOWS_TEST_CHECKLIST.md` §9 passes.
- **No self-delete after uninstall.** `run_uninstall` removes the app,
  shortcuts, and registry entry, but the running `uninstall.exe` cannot
  delete its own open file, so it is left behind in an otherwise-empty
  folder. NSIS solves this with a `cmd /C ping ... & rmdir` self-delete
  trick; reproducing it needs correct cmd.exe quoting around a path that may
  contain spaces (a username with a space is common), which cannot be
  verified without a Windows run. Left as a documented, cosmetic gap rather
  than shipped unverified.
- **No WebView2 auto-provisioning.** The installer detects a missing runtime
  and links to Microsoft's official download page; it does not download and
  run the bootstrapper itself. Revisit if this proves insufficient in
  practice for the Early Access audience.
- **No per-machine (admin) install mode, no code signing for this binary
  itself.** Both are real requirements before a public, non-Early-Access
  release; see `docs/ROADMAP.md`.

## Run it locally

This directory has its own `package.json`, pinning `@tauri-apps/api` and
`@tauri-apps/cli` to match the Rust `tauri` crate version. Without it,
`tauri build` (though not `tauri dev`) fails hard on a version-mismatch check
against the root project's own, older-pinned `@tauri-apps/api` — the exact
failure a real Windows CI run caught before this was added.

```bash
cd installer
npm install
npx tauri dev
```

Without a populated `payload/`, install/uninstall return
`payload_not_embedded` — expected, and lets you iterate on the screens
without a Windows machine. `default_install_dir` and `check_webview2` still
return real answers off-Windows (the former from environment variable
names; the latter defaults to `true` so the preview isn't blocked on it).

To build the way CI does, after building the main app:

```powershell
Copy-Item ..\src-tauri\target\release\betterfy.exe src-tauri\payload\betterfy.exe
npx tauri build
```

The unsigned installer is written to
`src-tauri\target\release\betterfy-installer.exe`.
