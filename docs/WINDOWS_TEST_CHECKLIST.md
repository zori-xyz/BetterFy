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

This flow writes only inside BetterFy application data. Production Dota file
deployment is still disabled.

## 7. Verify installation and updates

- Install the same internal version over the existing installation and confirm
  that settings survive.
- A real in-app update can only be tested after a newer, signed release and its
  signed updater manifest are published. Unsigned workflow artifacts do not
  activate the public updater.

## Send back after the pass

- the copied safe readiness report;
- screenshots of any broken layout or state;
- the exact action that preceded the failure;
- whether Steam and Dota were running;
- the installed BetterFy version and Windows version.

Do not send personal Steam files, Telegram codes, or full filesystem paths.
