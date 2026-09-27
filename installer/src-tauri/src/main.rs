#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
#[tauri::command]
fn run_install(options: InstallOptions) -> Result<InstallReport, String> {
    if Payload::iter().next().is_none() {
        return Err("payload_not_embedded".to_string());
    }

    let target = install_dir()?;
    std::fs::create_dir_all(&target).map_err(|_| "install_dir_failed".to_string())?;

    let mut installed_bytes: u64 = 0;
    for file in Payload::iter() {
        let asset = Payload::get(&file).ok_or_else(|| "payload_read_failed".to_string())?;
        installed_bytes += asset.data.len() as u64;
        let dest = target.join(file.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|_| "install_dir_failed".to_string())?;
        }
        std::fs::write(&dest, asset.data.as_ref())
            .map_err(|_| "payload_write_failed".to_string())?;
    }

    let main_binary = target.join(MAIN_BINARY_NAME);
    install_uninstaller(&target)?;
    write_uninstall_registry(&target, &main_binary, installed_bytes)?;
    create_shortcuts(&main_binary, options.create_desktop_shortcut)?;

    if options.launch_after {
        std::process::Command::new(&main_binary)
            .spawn()
            .map_err(|_| "launch_failed".to_string())?;
    }

    Ok(InstallReport {
        install_dir: target.display().to_string(),
        main_binary: main_binary.display().to_string(),
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
    key.set_value("DisplayVersion", &env!("CARGO_PKG_VERSION"))
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

fn desktop_shortcut() -> Result<PathBuf, String> {
    Ok(std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "missing_userprofile".to_string())?
        .join("Desktop")
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
#[tauri::command]
fn run_uninstall() -> Result<(), String> {
    perform_uninstall()
}

#[cfg(windows)]
fn perform_uninstall() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let target = install_dir()?;
    let _ = std::fs::remove_file(start_menu_shortcut()?);
    let _ = std::fs::remove_file(desktop_shortcut()?);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);

    // The running process is `uninstall.exe` inside `target`. Windows will
    // not let it delete its own open executable, so everything else is
    // removed and the now near-empty folder (just `uninstall.exe`) is left
    // behind. A batch-script self-delete trick could clear it too, but that
    // needs correct cmd.exe quoting around a path that may contain spaces
    // (a username with a space is common), which cannot be verified without
    // a Windows run — left as a known, cosmetic limitation instead of
    // shipping unverified quoting logic. See `installer/README.md`.
    remove_dir_all_except_self(&target)
}

#[cfg(windows)]
fn remove_dir_all_except_self(target: &std::path::Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|_| "uninstall_failed".to_string())?;
    for entry in std::fs::read_dir(target).map_err(|_| "uninstall_failed".to_string())? {
        let entry = entry.map_err(|_| "uninstall_failed".to_string())?;
        let path = entry.path();
        if path == current {
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
        .invoke_handler(tauri::generate_handler![
            default_install_dir,
            check_webview2,
            run_install,
            run_uninstall,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running the BetterFy installer");
}
