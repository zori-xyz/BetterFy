# BetterFy Windows test checklist

Use this checklist for the first native pass before production Dota deployment
is enabled. The goal is to verify Telegram device ownership, credential-vault
restoration, discovery, runtime control, BetterFy-owned staging, Steam profile
activation, recovery, and the installer boundary.

## 1. Install the internal build

1. Open the latest successful [Windows build workflow](https://github.com/zori-xyz/BetterFy/actions/workflows/windows-build.yml).
2. Download the `BetterFy-Windows-x64-nsis` artifact.
3. Extract it and run the unsigned NSIS installer.
4. Start BetterFy from the installed shortcut, not from the repository.

The CI artifact is for internal testing only. Windows may warn about an unknown
publisher until release signing is configured.

## 2. Verify Telegram account and device ownership

1. Start from a signed-out BetterFy installation.
2. Choose **Open BetterFy Bot** and verify that Windows opens only the
   `@BeterFyBot` device-confirmation link.
3. Confirm the request in Telegram. BetterFy must show a settled confirmation
   before entering the application, and the profile/avatar must match the
   Telegram account.
4. Sign out, repeat the flow, and deny the request. The app must remain signed
   out and offer a fresh request or the six-digit fallback.
5. Start another request and let it expire. It must not be redeemable after ten
   minutes.
6. Restart BetterFy after a successful login. The session should restore through
   Credential Manager without exposing a token in the interface or logs.
7. Revoke another desktop/web session from Profile and confirm that the revoked
   session can no longer refresh.
8. Verify the six-digit fallback separately. A used or expired code must fail.

Never share a challenge link, six-digit code, access credential, or copied
Credential Manager value in a bug report.

## 3. Run the safe readiness report

Open **Settings → Connection diagnostics → Run diagnostics**. Verify that the
report includes six checks:

- Windows support;
- Dota 2 installation;
- Steam and Dota runtime state;
- Steam profile readiness;
- BetterFy staging recovery state;
- verified BetterFy content-store state.

Use **Copy safe report** when reporting a problem. The exported JSON contains no
game paths, Steam IDs, account names, or Telegram data.

## 4. Verify Dota discovery

- Test the normal Steam library.
- If available, test a second Steam library on another drive.
- Test manual folder selection.
- Select an unrelated directory and verify that BetterFy rejects it.
- Restart BetterFy and verify that the saved installation is validated again.

## 5. Verify runtime and Steam activation

1. Open Steam and Dota 2, then rerun diagnostics. Runtime should require
   attention rather than pretending to be ready.
2. Select a neutral Steam profile inside BetterFy.
3. Confirm the shutdown step.
4. Verify that BetterFy asks Dota and Steam to close normally.
5. Complete activation and verify that BetterFy starts Steam only.
6. Start Dota manually.
7. Test the visible rollback action and confirm that unrelated Steam edits are
   never overwritten.

BetterFy must not force-terminate processes and must never launch Dota itself.

## 6. Verify fixture build and recovery

- Open the fixture build.
- Resolve the demonstrated conflict.
- Run the staging build to completion.
- Rerun diagnostics and verify that the content-store check reports at least one
  verified fixture package.
- Repeat with the simulated failure.
- Restore staging and rerun diagnostics; no recoverable staging operation should
  remain.

This flow writes only inside BetterFy application data. The game-deployment
transaction exists behind a strict verified-VPK boundary. Tree Mod, Show Net
Worth, and Repopulate Unit Query HUD are the only live pilot packages; no other
catalog item may be treated as installable. Dutch has founder-observed in-game
evidence for Tree Mod, not a general compatibility result for other languages or mods.

## 7. Tree Mod internal pilot

- Confirm `pak66_dir.vpk` is absent, or is identified by BetterFy as its own prior
  install. A foreign file in that slot must block the operation.
- Run with Dota and Steam open. After clicking Install, confirm BetterFy requests
  graceful shutdown before any write and refuses to continue if either remains
  open after the timeout. No force-kill should occur.
- Select only Tree Mod in My build. Prepare the 21 resources and check the verified
  plan. Choose the game language before installation, then check that both the
  receipt and actual target file use `game/dota_<selected-language>/pak66_dir.vpk`.
  A BetterFy success state alone is not proof that Dota mounted the file.
- Before confirming installation, verify that merely previewing the selected
  destination did not create a new `dota_<selected-language>` folder in `game`.
- During preparation, cancel once, reopen the app, and resume. The verified file
  count must continue without publishing an unverified resource.
- Choose the intended Steam profile before installation. Verify BetterFy adds
  `-language <selected-language>` only to that profile and restarts Steam, but does not launch
  Dota. Repeat without a selected profile: Steam should restart without a profile
  edit, and the command should be available to copy for manual entry.
- Start Dota manually. Record the visible game language and whether it can be
  changed without losing the Tree Mod effect.
- Confirm the default-terrain trees are replaced as described by the pilot for
  each newly selected language. Dutch has already been observed by the founder;
  Russian, Korean and Simplified Chinese have not.
- Restore the existing Dutch operation before switching to another language.
  Check that the prior VPK and BetterFy-owned launch option are removed and no
  unrelated Steam option or language-folder file changes.
  Founder evidence now covers the visible rollback and return to normal trees;
  retain exact before/after hashes in the next pass before marking byte-perfect
  restoration verified.
- Do not switch the installer to `dota_betterfy` until a separate Windows test
  demonstrates that the current Dota client mounts that exact folder.
- Close Dota and Steam, then roll back in BetterFy. Verify both the exact
  previous target bytes and Steam launch options return, or the initial BetterFy
  target disappears when no previous file existed.
- Restart BetterFy between install and rollback once. The installed operation
  and its restore action must still be discoverable.
- For version 0.1.3, restart BetterFy after automatic Steam profile activation
  and verify that the same Steam profile and its rollback operation are found
  without browser storage. Then restore and confirm the VPK and the exact
  previous launch options are both restored. A pre-0.1.3 Steam edit has no
  durable Tree Mod link and must be checked separately.
- Interrupt the Steam profile change in an internal failure-injection build.
  BetterFy must require Steam recovery before allowing VPK rollback, and the
  recovery action must not overwrite an unrelated Steam edit.
- In a disposable Windows test installation, remove only BetterFy's pinned
  downloaded-resource cache after install, then restart BetterFy. The owned VPK
  and file-restore action should remain visible, while automatic Steam setup is
  blocked until the package can be verified again. Do not delete the deployment
  journal or its private backup for this test.
- Repeat with a failure-injection internal build on both sides of atomic publish.
  Recovery must be deterministic and diagnostics must remain safe.

### Controlled recovery pass in the stress artifact

The `Windows build` workflow produces an internal artifact whose name starts with
`BetterFy-Windows-Stress`. The public Early Access release intentionally does not
contain these controls.

1. Prepare the selected package set and choose one language. Do not run this pass
   over an active BetterFy installation; restore it first.
2. Open **Windows test report → Controlled recovery**.
3. Run **Interrupt before publish**. It must report `PASS`, leave the target
   unchanged, and record a failed pre-commit journal.
4. Run **Interrupt after publish**. It must report `PASS`, remove or restore the
   target, and record `rollbackVerified: true`.
5. Install normally without automatic Steam activation. The game VPK remains
   installed so the Steam transaction can be tested independently.
6. Choose a Steam profile which does not already contain the selected BetterFy
   language, then run **Steam · before publish** and **Steam · after publish**.
   Both must report `PASS`; unrelated launch options must remain byte-for-byte
   unchanged.
7. Copy the Windows test report after every language/package matrix. The JSON is
   designed to be shareable: verify visually that it contains no paths, Steam ID,
   account name, profile token, or authentication data.
8. Finish by restoring the normal installation and rerunning diagnostics. No
   recoverable deployment or Steam transaction may remain.

## 8. Verify installation and updates

Before installer/update checks, run one multi-mod pass:

- select Tree Mod, Show Net Worth, and Repopulate Unit Query HUD and note their visible priority order;
- prepare the bundle and confirm it reports 3 packages and 25 resources;
- install through Dutch, restart BetterFy, and confirm the same three-package
  operation is rediscovered with all three package IDs;
- launch Dota manually and verify the tree replacement, net-worth HUD, and
  unit-query HUD changes; a BetterFy success state alone is not proof;
- reverse the priority and prepare again; the plan identity must change even if
  the packages do not collide;
- restore and confirm normal trees/HUD and the previous exact target bytes.

Show Net Worth and Repopulate Unit Query HUD stay internal unverified pilots
until this pass succeeds.

- Install the same internal version over the existing installation and confirm
  that settings survive.
- A real in-app update can only be tested after a newer, signed release and its
  signed updater manifest are published. Unsigned workflow artifacts do not
  activate the public updater.

## 9. BetterFy Setup (the branded installer)

`installer/` has not run on Windows at all before this pass. Nothing here may
be assumed to work; run every step.

1. Uninstall any existing BetterFy install first (Settings → Apps, or the
   Windows 8 uninstall flow) so this starts from a clean machine.
2. Run `BetterFy-Setup.exe`. Confirm the sidebar shows the real Telegram
   banner image (not a placeholder), switch RU ↔ EN with the top-right toggle
   and confirm every screen's text changes, and confirm **Cancel** actually
   closes the window — this exact button silently did nothing in the first
   real build until a missing capability grant was found and fixed.
3. Confirm the install path reads `%LOCALAPPDATA%\BetterFy`, and — only if
   this machine genuinely lacks WebView2 — the WebView2 warning appears and
   its button opens Microsoft's official download page.
4. Install with both checkboxes on. Confirm BetterFy launches, a Start Menu
   shortcut exists under a `BetterFy` folder, and a Desktop shortcut exists
   at the real Desktop location — this matters specifically if this machine's
   Desktop has been relocated by OneDrive.
5. Open **Settings → Apps → Installed apps** and confirm BetterFy is listed
   with the correct version, publisher, and size.
6. **Run `BetterFy-Setup.exe` again over the existing install.** Confirm the
   welcome screen now reads "Update" (not "Install"), shows the installed
   version, and offers an **Uninstall BetterFy** link inline — this is the
   in-app path, separate from Windows' own Apps & Features. With BetterFy
   itself running, attempt the update and confirm it reports `app_running`
   with a plain-language message rather than corrupting the install; close
   BetterFy and confirm Retry then succeeds.
7. **Update-in-place through the real updater, the compatibility check that
   matters most:** with this BetterFy Setup install still in place, trigger a
   signed update (a newer signed release, applied the normal way through the
   in-app updater). Confirm the existing installation is updated — same
   folder, same shortcuts, same registry entry — and that a second, orphaned
   copy is not created anywhere. This is the specific risk documented in
   `ROADMAP.md`: the updater silently reruns the NSIS installer, and it must
   recognize this installation as the one to replace.
8. Uninstall BetterFy from the installer's own **Uninstall BetterFy** button
   (not just Apps & Features). Confirm the Start Menu and Desktop shortcuts
   and the registry entry are gone, and that BetterFy's own account/session
   data under `%APPDATA%\app.betterfy.desktop\` is untouched. A leftover
   `uninstall.exe` in an otherwise-empty `%LOCALAPPDATA%\BetterFy` folder is
   a known, accepted gap — record it, do not treat it as a new finding.
9. Force an error deliberately (for example, rename `payload/` before
   building so `run_install` returns `payload_not_embedded`, or simply pull
   the network/deny a file permission) and confirm the error screen shows a
   plain-language explanation plus working Retry and Contact-support
   buttons, not just a bare code.

## Send back after the pass

- the copied safe readiness report;
- screenshots of any broken layout or state;
- the exact action that preceded the failure;
- whether Steam and Dota were running;
- the installed BetterFy version and Windows version.

Do not send personal Steam files, Telegram codes, or full filesystem paths.
