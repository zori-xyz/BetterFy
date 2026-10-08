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

EA.18 is the verified Windows Early Access baseline. Native Windows CI built
the real embedded application and the founder subsequently completed the
Setup and application pass on Windows: install, existing-install detection,
update, uninstall/reinstall, launch, sign-in, and the pinned three-package
pilot path were observed working. The immutable artifact identity, exact scope
of the human evidence, and retained limitations are recorded in
[`docs/EA18_WINDOWS_BASELINE.md`](../docs/EA18_WINDOWS_BASELINE.md). This is an
Early Access baseline, not a signed Stable release.

Implemented:

- an eight-screen UI (welcome, confirm-uninstall, installing, finish,
  uninstalling, uninstalled, error, plus an inline WebView2 warning) in the
  BetterFy dark theme (the app's violet/near-black tokens, Days One for
  headings, Manrope for text), in Russian and English with a segmented
  switcher whose choice is remembered; the left brand panel is live CSS/HTML
  (wordmark, a short tagline, and the Telegram QR with its "Join our
  Telegram" text kept near it, per direct founder feedback on where it
  should sit) rather than a static export;
- the welcome screen shows the fixed install path as a readonly chip with a
  note on why it is fixed (updates must replace the same copy), and the
  options as toggles; the finish screen offers a "Launch BetterFy" action
  (the `launch_app` command) when the app was not already started by the
  install step;
- detects an existing installation (binary present on disk, version read
  from the registry) and switches the welcome screen to an Update framing,
  offering an in-app Uninstall button next to it — reachable without going
  through Windows' own Apps & Features; confirming goes to a real, styled
  screen rather than a native `confirm()` dialog (see "Why there is no
  `window.confirm()` anywhere" below), and the uninstalled screen offers an
  "Install again" button back to a fresh welcome screen;
- `run_install` refuses to overwrite a running `betterfy.exe` (detected via
  an open-for-write probe, not process enumeration) and reports
  `app_running` rather than failing into a half-overwritten install;
- the payload is staged to a sibling `.new` file per entry, read back and
  compared byte-for-byte, and only then renamed over the real destination —
  each rename is atomic on the same volume, so a failure can only ever leave
  the previous working files in place, never a half-written `betterfy.exe`;
- `run_uninstall` checks the app isn't running before touching anything, then
  verifies afterward that the binary, both shortcuts, and the registry key
  are actually gone before reporting success — it does not just trust that
  each removal call returned without erroring;
- the embedded app's own version (read from the root `package.json` at
  compile time, not this installer's own Cargo.toml version) is what's
  written to the registry and shown in the "already installed" text — these
  are two different version numbers and conflating them was an earlier bug;
- installing shows three named, finite steps (files, shortcuts, registry)
  and a progress bar. `run_install` reports once, when it returns, so the bar
  and the highlighted step are paced by elapsed time (an estimate that eases
  toward about 92%) and the bar only reaches 100%, and the steps are only
  marked done together, when the command actually finishes. The rotating
  one-line status messages are cosmetic. The same bar is used for uninstall;
  `prefers-reduced-motion: reduce` is honored globally, and every button and
  checkbox has a visible `:focus-visible` ring;
- errors show a plain-language explanation and Retry/Contact-support actions
  next to the raw code, instead of only the code;
- Rust commands, gated to Windows, that: copy the embedded payload into
  `%LOCALAPPDATA%\BetterFy`, write a Start Menu shortcut under a `BetterFy`
  folder and an optional Desktop shortcut (resolved via `dirs::desktop_dir()`
  against the real `FOLDERID_Desktop` known folder, so a OneDrive-relocated
  Desktop is still found), register an uninstall entry under
  `HKCU\...\Uninstall\BetterFy` with the same fields Tauri's NSIS template
  writes, and optionally launch the app;
- `run_uninstall`, reachable from the welcome screen's Uninstall button or by
  relaunching this same binary with `--uninstall` (the registry's
  `UninstallString` does exactly that — no second build target to maintain);
- a `core:window:allow-close` capability grant, without which the page's own
  Cancel/Finish/Close buttons silently do nothing — see "Why capabilities
  matter" below;
- a WebView2 Runtime presence check against the exact registry keys
  Microsoft's own distribution guide documents, surfaced as a warning with a
  link to Microsoft's official download page — this installer does not fetch
  or run an external binary itself;
- `cargo fmt` / `cargo clippy -D warnings` and `npx tauri build` all succeed
  on a real `windows-latest` GitHub Actions runner, and a full
  `early-access-release.yml` run has produced a working `BetterFy-Setup.exe`
  with the real app payload embedded.

## Why capabilities matter

Tauri v2 gates built-in core commands — including closing a window — behind
a capability grant; commands this crate defines itself
(`run_install`, `existing_install`, ...) are not gated the same way, which is
why installation worked in the first real test while every button that only
closed the window (Cancel, Finish, the error screen's Close) did not. The
failure mode is silent: the JS promise rejects, nothing throws visibly, and
without opening the webview's devtools the window just looks unresponsive.
`src-tauri/capabilities/default.json` grants exactly `core:window:allow-close`
to the `main` window (which the window config now labels explicitly, rather
than relying on Tauri's default label matching); nothing broader.

## Why there is no `window.confirm()` anywhere

An early build asked "Uninstall BetterFy?" via the browser's native
`confirm()`. In the actual WebView2 environment this runs in, that dialog was
silently answered `false` with no dialog ever shown, so clicking Uninstall
did nothing visible at all — a category of embedded-browser dialog
suppression later reproduced directly during preview testing, confirming the
mechanism rather than just guessing at it. Confirmation is now its own
screen (`confirm-uninstall`), styled and worded like the rest of the app,
with explicit Yes/Cancel buttons wired to real click handlers, not a native
dialog whose availability this installer cannot rely on.

## Why the exact paths matter

Verified against Tauri's own NSIS bundler template
(`tauri-apps/tauri/crates/tauri-bundler/.../nsis/installer.nsi`), not
assumed:

| What | NSIS (`installMode: currentUser`) | This installer |
| --- | --- | --- |
| Install directory | `$LOCALAPPDATA\BetterFy` | same |
| Uninstall registry key | `HKCU\...\Uninstall\BetterFy` | same |
| Start Menu shortcut | `Start Menu\Programs\BetterFy\BetterFy.lnk` | same |
| Desktop shortcut | resolved known folder `\BetterFy.lnk` | same |
| Main binary name | `betterfy.exe` (the Cargo package name; no `mainBinaryName` override is set) | same |

If these ever drifted, the signed updater's silent NSIS pass would not
recognize an install this tool made — it would install a second, orphaned
copy instead of updating the first one. An early draft of this installer got
the install directory, the binary name's case, and the Start Menu layout
wrong before this was checked against Tauri's actual template source; get
this table wrong again and it will happen silently, not as a build error.

## Known gaps after the EA.18 Windows pass

- **The evidence is one founder-run Windows pass.** EA.18 install, update,
  uninstall/reinstall and launch behavior is confirmed, but the machine
  metadata and a machine-readable report were not retained. A second machine
  and a saved repeatable report remain open before Stable.
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
- **Install progress is reported in one batch, not live per step.** The bar
  is a time-paced estimate and the three steps are all marked done together
  when `run_install` returns, because
  reporting them as they actually happen needs Tauri's event system, and its
  `core:event:allow-emit`/`allow-listen` permissions have open community
  reports of being inconsistently recognized — not a risk worth taking for a
  cosmetic improvement given the `core:window:allow-close` lesson above.
  Worth revisiting if a verified recipe turns up.
- **No per-machine (admin) install mode, no code signing for this binary
  itself.** Both are real requirements before a public, non-Early-Access
  release; see `docs/ROADMAP.md`.

## Run it locally

This directory has its own `package.json`, pinning `@tauri-apps/api` and
`@tauri-apps/cli` to match the Rust `tauri` crate version. Without it,
`tauri build` (though not `tauri dev`) fails hard on a version-mismatch check
against the root project's own, older-pinned `@tauri-apps/api` — a failure a
real Windows CI run caught before this was added.

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
