#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(RustEmbed)]
#[folder = "payload"]
struct Payload;

const APP_NAME: &str = "BetterFy";
// The Cargo package in `src-tauri/Cargo.toml` is named "betterfy" (lowercase)
// and `tauri.conf.json` sets no `mainBinaryName` override, so Tauri's actual
// compiled binary is `betterfy.exe`, not "BetterFy.exe". Verified against
// Tauri's own docs, not assumed — NTFS is case-insensitive so the wrong case
// would likely have worked anyway, but every reference (shortcut target,
// registry DisplayIcon, the CI payload copy) should agree exactly.
const MAIN_BINARY_NAME: &str = "betterfy.exe";
const UNINSTALLER_NAME: &str = "uninstall.exe";
const WEBSITE_URL: &str = "https://zori-xyz.github.io/BetterFy/";
const HELP_URL: &str = "https://t.me/BeterFyBot";
// The WebView2 download URL itself lives in installer/ui/app.js, which is
// the only place `open_url` needs it; `open_url` validates any https:// URL
// generically rather than special-casing this one.
//
// The documented Microsoft Edge WebView2 Runtime identity, checked the way
// Microsoft's own distribution guide checks it: a non-empty, non-zero `pv`
// value under either registry hive. NSIS provisions this at install time for
// the main app; this installer only detects it and points the user at the
// official download page rather than fetching and running a binary itself.
const WEBVIEW2_CLIENT_KEY: &str =
    r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

// This exact path, registry key, and Start Menu layout are load-bearing: they
// must match what `src-tauri/windows/installer.nsi` (Tauri's own NSIS
// bundler template) uses for `installMode: "currentUser"`, because the
// signed auto-updater silently re-runs that same NSIS installer for updates.
// If the two installers disagreed, an update would create a second, orphaned
// copy instead of replacing this one. Verified against Tauri's NSIS template
// source, not assumed: see `installer/README.md`.
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\BetterFy";

// Backs the window's CloseRequested handler: while true, the OS title bar's
// own close button is refused, so a person cannot close the window mid-write
// and leave the install directory in whatever state the swap/registry/
// shortcut sequence happened to be in. Commands hold this only around their
// actual commit phase, not for the whole command (network-free file and
// registry operations are fast; there is no reason to block close any longer
// than that).
struct BusyState(std::sync::Mutex<bool>);

struct BusyGuard<'a>(&'a std::sync::Mutex<bool>);

impl<'a> BusyGuard<'a> {
    fn new(flag: &'a std::sync::Mutex<bool>) -> Self {
        if let Ok(mut guard) = flag.lock() {
            *guard = true;
        }
        BusyGuard(flag)
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = false;
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallOptions {
    create_desktop_shortcut: bool,
    launch_after: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallReport {
    install_dir: String,
    main_binary: String,
    // The install itself is committed before the launch is attempted, so a
    // failed launch is reported alongside success rather than as an error
    // (an error screen would offer Retry, which reinstalls for nothing).
    launch_failed: bool,
}

fn install_dir() -> Result<PathBuf, String> {
    let local_app_data =
        std::env::var_os("LOCALAPPDATA").ok_or_else(|| "missing_local_app_data".to_string())?;
    Ok(PathBuf::from(local_app_data).join(APP_NAME))
}

#[tauri::command]
fn default_install_dir() -> Result<String, String> {
    install_dir().map(|p| p.display().to_string())
}

// Runs `finish` (registering the uninstaller, registry entry, and shortcuts)
// and, if it fails after an update, restores `main_binary` from
// `backup_binary` before returning the original error, rather than leaving
// an install that is neither the old working version nor a registered new
// one. On success the backup is removed. `backup_binary` existing is what
// distinguishes an update from a fresh install (see `run_install`, which
// only creates it when there was something to back up). Deliberately plain
// std::fs plus a generic closure — no Windows-only API — so this is testable
// without a Windows machine; see the tests below.
fn commit_or_rollback(
    backup_binary: &std::path::Path,
    main_binary: &std::path::Path,
    finish: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let is_update = backup_binary.exists();
    match finish() {
        Ok(()) => {
            if is_update {
                let _ = std::fs::remove_file(backup_binary);
            }
            Ok(())
        }
        Err(err) => {
            if is_update {
                let _ = std::fs::rename(backup_binary, main_binary);
            }
            Err(err)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExistingInstall {
    version: String,
}

// Checked two ways, not just the registry: a stale uninstall entry left over
// from a prior failed uninstall would otherwise report "already installed"
// for an app that no longer exists on disk, matching the caution this
// repository's own engine takes with stale journal state.
#[cfg(windows)]
#[tauri::command]
fn existing_install() -> Option<ExistingInstall> {
    use winreg::enums::*;
    use winreg::RegKey;

    let target = install_dir().ok()?;
    if !target.join(MAIN_BINARY_NAME).is_file() {
        return None;
    }
    let version = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(UNINSTALL_KEY)
        .and_then(|key| key.get_value::<String, _>("DisplayVersion"))
        .unwrap_or_else(|_| "?".to_string());
    Some(ExistingInstall { version })
}

#[cfg(not(windows))]
#[tauri::command]
fn existing_install() -> Option<ExistingInstall> {
    None
}

#[cfg(windows)]
fn webview2_installed() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let has_version = |root: RegKey, path: &str| -> bool {
        root.open_subkey(path)
            .and_then(|key| key.get_value::<String, _>("pv"))
            .map(|version| !version.is_empty() && version != "0.0.0.0")
            .unwrap_or(false)
    };

    let per_user = has_version(
        RegKey::predef(HKEY_CURRENT_USER),
        &format!(r"Software\{WEBVIEW2_CLIENT_KEY}"),
    );
    let per_machine = has_version(
        RegKey::predef(HKEY_LOCAL_MACHINE),
        &format!(r"SOFTWARE\WOW6432Node\{WEBVIEW2_CLIENT_KEY}"),
    );
    per_user || per_machine
}

#[tauri::command]
fn check_webview2() -> bool {
    #[cfg(windows)]
    {
        webview2_installed()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

// The main app's built Windows output is embedded from `payload/`, which CI
// populates before compiling this installer. Nothing is embedded yet: see
// `installer/README.md`. Failing closed here (rather than "installing" an
// empty directory) matches the rest of this repository's engine commands.
#[cfg(windows)]
// Install and uninstall write many files. A sync command runs on the main
// thread, which would stop the event loop for the whole operation: the window
// could be reported as "Not responding" and the CloseRequested guard below
// could never run. `async` moves the work to the async runtime.
#[tauri::command(async)]
fn run_install(
    options: InstallOptions,
    busy: tauri::State<'_, BusyState>,
) -> Result<InstallReport, String> {
    if Payload::iter().next().is_none() {
        return Err("payload_not_embedded".to_string());
    }
    let _guard = BusyGuard::new(&busy.0);

    let target = install_dir()?;
    std::fs::create_dir_all(&target).map_err(|_| "install_dir_failed".to_string())?;

    // Reinstalling/updating over a running BetterFy would try to overwrite an
    // open, locked executable. Detected the same way Windows itself reports
    // it — a sharing-violation on open — rather than enumerating processes.
    let existing_binary = target.join(MAIN_BINARY_NAME);
    let is_update = existing_binary.is_file();
    if is_update {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&existing_binary)
            .map_err(|_| "app_running".to_string())?;
    }

    // Keep the previous binary around until every later step (uninstaller
    // stub, registry, shortcuts) has actually succeeded. The payload rename
    // below is already safe on its own; this covers the remaining gap where
    // an update could swap in a new exe and then fail to register it,
    // leaving neither a clean old install nor a working new one.
    let backup_binary = target.join(format!("{MAIN_BINARY_NAME}.bak"));
    if is_update {
        std::fs::copy(&existing_binary, &backup_binary)
            .map_err(|_| "install_dir_failed".to_string())?;
    }

    // Two-phase write: stage every payload file under a sibling `.new` name
    // and verify its bytes on disk before touching any real destination.
    // Each `rename` below is then atomic on the same volume (Windows
    // MoveFileEx with replace-existing), so a failure here can only ever
    // leave the previous, working files in place — never a half-written
    // `betterfy.exe`. (With today's single-file payload this is fully
    // transactional; if the payload grows to multiple files, a failure
    // partway through the rename loop could still leave some files updated
    // and others not — real per-file atomicity, not yet a whole-payload one.)
    let mut installed_bytes: u64 = 0;
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for file in Payload::iter() {
        let asset = Payload::get(&file).ok_or_else(|| "payload_read_failed".to_string())?;
        installed_bytes += asset.data.len() as u64;
        let dest = target.join(file.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|_| "install_dir_failed".to_string())?;
        }
        let temp_name = format!(
            "{}.new",
            dest.file_name()
                .ok_or_else(|| "payload_write_failed".to_string())?
                .to_string_lossy()
        );
        let temp = dest.with_file_name(temp_name);
        std::fs::write(&temp, asset.data.as_ref())
            .map_err(|_| "payload_write_failed".to_string())?;
        let written = std::fs::read(&temp).map_err(|_| "payload_write_failed".to_string())?;
        if written != asset.data.as_ref() {
            let _ = std::fs::remove_file(&temp);
            return Err("payload_verify_failed".to_string());
        }
        staged.push((temp, dest));
    }
    for (temp, dest) in &staged {
        std::fs::rename(temp, dest).map_err(|_| "payload_write_failed".to_string())?;
    }

    let main_binary = target.join(MAIN_BINARY_NAME);
    commit_or_rollback(&backup_binary, &main_binary, || {
        install_uninstaller(&target)?;
        write_uninstall_registry(&target, &main_binary, installed_bytes)?;
        create_shortcuts(&main_binary, options.create_desktop_shortcut)?;
        Ok(())
    })?;

    let launch_failed =
        options.launch_after && std::process::Command::new(&main_binary).spawn().is_err();

    Ok(InstallReport {
        install_dir: target.display().to_string(),
        main_binary: main_binary.display().to_string(),
        launch_failed,
    })
}

#[cfg(not(windows))]
#[tauri::command]
fn run_install(_options: InstallOptions) -> Result<InstallReport, String> {
    Err("windows_only".to_string())
}

// Copies this running installer into the install directory under a fixed
// name, the same way NSIS drops an `uninstall.exe` stub. Reusing one binary
// for both roles (it re-execs itself with `--uninstall`) avoids maintaining
// a second build target.
#[cfg(windows)]
fn install_uninstaller(target: &std::path::Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|_| "uninstaller_copy_failed".to_string())?;
    std::fs::copy(&current, target.join(UNINSTALLER_NAME))
        .map_err(|_| "uninstaller_copy_failed".to_string())?;
    Ok(())
}

#[cfg(windows)]
fn write_uninstall_registry(
    install_dir: &std::path::Path,
    main_binary: &std::path::Path,
    installed_bytes: u64,
) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(UNINSTALL_KEY)
        .map_err(|_| "registry_write_failed".to_string())?;

    let uninstaller = install_dir.join(UNINSTALLER_NAME);
    let estimated_size_kb = (installed_bytes / 1024).max(1) as u32;
    let fail = |_| "registry_write_failed".to_string();
    key.set_value("DisplayName", &APP_NAME).map_err(fail)?;
    key.set_value("DisplayVersion", &env!("BETTERFY_APP_VERSION"))
        .map_err(fail)?;
    key.set_value("Publisher", &"BetterFy").map_err(fail)?;
    key.set_value("MainBinaryName", &MAIN_BINARY_NAME)
        .map_err(fail)?;
    key.set_value("InstallLocation", &install_dir.display().to_string())
        .map_err(fail)?;
    key.set_value("DisplayIcon", &format!("\"{}\"", main_binary.display()))
        .map_err(fail)?;
    key.set_value(
        "UninstallString",
        &format!("\"{}\" --uninstall", uninstaller.display()),
    )
    .map_err(fail)?;
    key.set_value("URLInfoAbout", &WEBSITE_URL).map_err(fail)?;
    key.set_value("HelpLink", &HELP_URL).map_err(fail)?;
    key.set_value("NoModify", &1u32).map_err(fail)?;
    key.set_value("NoRepair", &1u32).map_err(fail)?;
    key.set_value("EstimatedSize", &estimated_size_kb)
        .map_err(fail)?;
    Ok(())
}

fn start_menu_shortcut() -> Result<PathBuf, String> {
    Ok(std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "missing_appdata".to_string())?
        .join(r"Microsoft\Windows\Start Menu\Programs")
        .join(APP_NAME)
        .join(format!("{APP_NAME}.lnk")))
}

// `dirs::desktop_dir()` resolves the real FOLDERID_Desktop known folder
// rather than assuming `%USERPROFILE%\Desktop`, so it still finds the right
// place when OneDrive's Known Folder Move has relocated Desktop.
fn desktop_shortcut() -> Result<PathBuf, String> {
    Ok(dirs::desktop_dir()
        .ok_or_else(|| "missing_desktop_dir".to_string())?
        .join(format!("{APP_NAME}.lnk")))
}

#[cfg(windows)]
fn create_shortcuts(main_binary: &std::path::Path, desktop: bool) -> Result<(), String> {
    use mslnk::ShellLink;

    let mut link = ShellLink::new(main_binary).map_err(|_| "shortcut_failed".to_string())?;
    link.set_working_dir(main_binary.parent().map(|p| p.display().to_string()));

    let start_menu = start_menu_shortcut()?;
    if let Some(parent) = start_menu.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "shortcut_failed".to_string())?;
    }
    link.create_lnk(&start_menu)
        .map_err(|_| "shortcut_failed".to_string())?;

    if desktop {
        link.create_lnk(desktop_shortcut()?)
            .map_err(|_| "shortcut_failed".to_string())?;
    }

    Ok(())
}

#[cfg(windows)]
#[tauri::command(async)]
fn run_uninstall(busy: tauri::State<'_, BusyState>) -> Result<(), String> {
    let _guard = BusyGuard::new(&busy.0);
    perform_uninstall()
}

#[cfg(windows)]
fn perform_uninstall() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let target = install_dir()?;
    let main_binary = target.join(MAIN_BINARY_NAME);

    // Refuse up front if BetterFy is running, the same way run_install does.
    // Without this, the shortcuts and registry entry would disappear while
    // the locked exe survived, and the "uninstalled" screen would be lying.
    if main_binary.is_file() {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&main_binary)
            .map_err(|_| "app_running".to_string())?;
    }

    let start_menu = start_menu_shortcut()?;
    let desktop = desktop_shortcut()?;
    let _ = std::fs::remove_file(&start_menu);
    // The shortcut lives in its own "BetterFy" Start Menu folder; remove it
    // too. `remove_dir` only succeeds on an empty folder, so anything else a
    // user put there is left alone.
    if let Some(folder) = start_menu.parent() {
        let _ = std::fs::remove_dir(folder);
    }
    let _ = std::fs::remove_file(&desktop);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);

    remove_dir_all_except_self(&target)?;

    // This repository's rule for any privileged operation: verify the result
    // actually happened rather than trusting that each step above returned
    // without erroring. Only the running uninstall.exe itself is allowed to
    // remain (see the doc comment below).
    let registry_gone = hkcu.open_subkey(UNINSTALL_KEY).is_err();
    if main_binary.exists() || start_menu.exists() || desktop.exists() || !registry_gone {
        return Err("uninstall_incomplete".to_string());
    }

    // The running process is `uninstall.exe` inside `target`. Windows will
    // not let it delete its own open executable, so it is deliberately
    // excluded from the checks above and left behind in the now near-empty
    // folder. A batch-script self-delete trick could clear it too, but that
    // needs correct cmd.exe quoting around a path that may contain spaces
    // (a username with a space is common), which cannot be verified without
    // a Windows run — left as a known, cosmetic limitation instead of
    // shipping unverified quoting logic. See `installer/README.md`.
    Ok(())
}

// Plain std::fs, no Windows-only API, parametrized on the path to keep
// rather than reading `std::env::current_exe()` itself — so this is testable
// directly; see the tests below.
fn remove_dir_all_except(target: &std::path::Path, keep: &std::path::Path) -> Result<(), String> {
    for entry in std::fs::read_dir(target).map_err(|_| "uninstall_failed".to_string())? {
        let entry = entry.map_err(|_| "uninstall_failed".to_string())?;
        let path = entry.path();
        if path == keep {
            continue;
        }
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn remove_dir_all_except_self(target: &std::path::Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|_| "uninstall_failed".to_string())?;
    remove_dir_all_except(target, &current)
}

#[cfg(not(windows))]
#[tauri::command]
fn run_uninstall() -> Result<(), String> {
    Err("windows_only".to_string())
}

#[cfg(windows)]
fn main() {
    if std::env::args().any(|arg| arg == "--uninstall") {
        std::process::exit(match perform_uninstall() {
            Ok(()) => 0,
            Err(_) => 1,
        });
    }
    run_app();
}

#[cfg(not(windows))]
fn main() {
    run_app();
}

// Hands a URL to the OS default browser via `explorer.exe`, which accepts a
// URL argument and forwards it — the same mechanism Windows' own "Open"
// verbs use. No shell interpretation of the argument, so this is safe for
// the fixed, hardcoded URLs this installer opens.
#[cfg(windows)]
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("unsupported_url".to_string());
    }
    std::process::Command::new("explorer")
        .arg(url)
        .spawn()
        .map_err(|_| "open_failed".to_string())?;
    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
fn open_url(_url: String) -> Result<(), String> {
    Err("windows_only".to_string())
}

fn run_app() {
    tauri::Builder::default()
        .manage(BusyState(std::sync::Mutex::new(false)))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let busy = window.app_handle().state::<BusyState>();
                let is_busy = busy.0.lock().map(|guard| *guard).unwrap_or(false);
                if is_busy {
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            default_install_dir,
            existing_install,
            check_webview2,
            run_install,
            run_uninstall,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running the BetterFy installer");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("betterfy-installer-test-{label}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn commit_or_rollback_leaves_a_fresh_install_alone_on_failure() {
        let dir = scratch_dir("fresh-install");
        let backup = dir.join("betterfy.exe.bak");
        let main_binary = dir.join("betterfy.exe");
        fs::write(&main_binary, b"new build").unwrap();

        // No backup exists, so this is a fresh install, not an update: there
        // is nothing to roll back to, and the binary that was just written
        // must be left exactly as it is.
        let result = commit_or_rollback(&backup, &main_binary, || Err("boom".to_string()));

        assert_eq!(result, Err("boom".to_string()));
        assert_eq!(fs::read(&main_binary).unwrap(), b"new build");
    }

    #[test]
    fn commit_or_rollback_restores_the_previous_binary_on_failure() {
        let dir = scratch_dir("rollback-on-failure");
        let backup = dir.join("betterfy.exe.bak");
        let main_binary = dir.join("betterfy.exe");
        fs::write(&backup, b"old build").unwrap();
        fs::write(&main_binary, b"new build").unwrap();

        let result = commit_or_rollback(&backup, &main_binary, || {
            Err("registry_write_failed".to_string())
        });

        assert_eq!(result, Err("registry_write_failed".to_string()));
        assert_eq!(fs::read(&main_binary).unwrap(), b"old build");
        assert!(
            !backup.exists(),
            "the backup should be consumed by the restore"
        );
    }

    #[test]
    fn commit_or_rollback_removes_the_backup_on_success() {
        let dir = scratch_dir("commit-on-success");
        let backup = dir.join("betterfy.exe.bak");
        let main_binary = dir.join("betterfy.exe");
        fs::write(&backup, b"old build").unwrap();
        fs::write(&main_binary, b"new build").unwrap();

        let result = commit_or_rollback(&backup, &main_binary, || Ok(()));

        assert_eq!(result, Ok(()));
        assert_eq!(fs::read(&main_binary).unwrap(), b"new build");
        assert!(!backup.exists());
    }

    #[test]
    fn remove_dir_all_except_keeps_only_the_named_path() {
        let dir = scratch_dir("remove-except");
        let keep = dir.join("uninstall.exe");
        let other_file = dir.join("betterfy.exe");
        let other_dir = dir.join("some-subdir");
        fs::write(&keep, b"keep me").unwrap();
        fs::write(&other_file, b"delete me").unwrap();
        fs::create_dir_all(&other_dir).unwrap();
        fs::write(other_dir.join("nested.txt"), b"delete me too").unwrap();

        remove_dir_all_except(&dir, &keep).unwrap();

        assert!(keep.exists());
        assert!(!other_file.exists());
        assert!(!other_dir.exists());
    }
}
