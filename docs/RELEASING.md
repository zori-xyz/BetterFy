# BetterFy Windows releases

BetterFy updates are distributed only through signed GitHub Releases. Ordinary
push and pull-request builds remain test artifacts and are never offered by the
in-app updater.

The public website may offer a clearly labelled unsigned Early Access installer
for Windows testing. Run the `Early Access Windows installer` workflow manually;
it creates an immutable prerelease with a fixed installer filename and a SHA-256
sidecar. These prereleases are never used by the in-app updater. Once a signed
stable release exists, the website and authenticated release resolver prefer it.

The published, user-facing installer is `BetterFy-Setup.exe`, built from
`installer/` — see [WINDOWS_BUILD.md](WINDOWS_BUILD.md#betterfy-setup-the-branded-installer).
NSIS is still built on every release: `tauri-plugin-updater` depends on its
signature format and silently re-runs it to apply updates, so it remains the
release's internal update mechanism even though it is no longer the download
a person runs first.

EA.18 is the current founder-verified unsigned Windows baseline. Its commit,
artifact size, SHA-256, CI run, observed Windows behavior, and limitations are
recorded in [EA18_WINDOWS_BASELINE.md](EA18_WINDOWS_BASELINE.md). A later
prerelease produced by the same workflow is not automatically human-verified;
record a new manual observation when its installer behavior changes.

## One-time repository setup

The updater public key is committed in `src-tauri/tauri.conf.json`. Its private
half is stored locally in the ignored `.release-signing/updater.key` file and
must never be committed or posted in an issue, log, or chat.

After authenticating GitHub CLI for `zori-xyz/BetterFy`, register the private
key as an Actions secret:

```powershell
gh auth login -h github.com
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo zori-xyz/BetterFy < .release-signing/updater.key
```

Back up `.release-signing/updater.key` in a secure password manager or encrypted
vault. Losing it prevents existing installations from accepting future updates.

## Publish a release

1. Increase the same SemVer in `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml`.
2. Run `npm run check` and `npm run visual:check`.
3. Commit and push `main`.
4. Create and push the matching tag, for example `v0.2.0`.

The `Signed Windows release` workflow builds the NSIS installer, its signature,
and `latest.json`. Installed copies check that feed on launch and offer an
in-place passive update. Never replace the updater public key for a routine
release.
