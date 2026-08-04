use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(250);
const SHUTDOWN_MAX_POLLS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub platform_supported: bool,
    pub steam_running: bool,
    pub dota_running: bool,
    pub patch_ready: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePrepareRequest {
    pub confirmed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SteamStartRequest {
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessEntry {
    pid: u32,
    name: String,
}

fn classify_processes<'a>(names: impl IntoIterator<Item = &'a str>) -> RuntimeState {
    let mut steam_running = false;
    let mut dota_running = false;

    for name in names {
        let normalized = name.trim().to_ascii_lowercase();
        steam_running |= matches!(
            normalized.as_str(),
            "steam.exe" | "steamwebhelper.exe" | "gameoverlayui.exe"
        );
        dota_running |= is_dota_process(&normalized);
    }

    RuntimeState {
        platform_supported: cfg!(target_os = "windows"),
        steam_running,
        dota_running,
        patch_ready: !steam_running && !dota_running,
    }
}

fn is_dota_process(normalized_name: &str) -> bool {
    matches!(
        normalized_name,
        "dota2.exe" | "dota2launcher.exe" | "dota2cfg.exe"
    )
}

fn state_from_entries(entries: &[ProcessEntry]) -> RuntimeState {
    classify_processes(entries.iter().map(|entry| entry.name.as_str()))
}

fn wait_for_patch_ready<F>(
    mut inspect: F,
    max_polls: usize,
    interval: Duration,
) -> Result<RuntimeState, String>
where
    F: FnMut() -> Result<RuntimeState, String>,
{
    for poll in 0..=max_polls {
        let state = inspect()?;
        if state.patch_ready {
            return Ok(state);
        }
        if poll < max_polls && !interval.is_zero() {
            thread::sleep(interval);
        }
    }
    Err("shutdown_timeout".to_string())
}

fn wait_for_steam_started<F>(
    mut inspect: F,
    max_polls: usize,
    interval: Duration,
) -> Result<RuntimeState, String>
where
    F: FnMut() -> Result<RuntimeState, String>,
{
    for poll in 0..=max_polls {
        let state = inspect()?;
        if state.steam_running && !state.dota_running {
            return Ok(state);
        }
        if poll < max_polls && !interval.is_zero() {
            thread::sleep(interval);
        }
    }
    Err("steam_start_timeout".to_string())
}

#[cfg(target_os = "windows")]
fn process_entries() -> Result<Vec<ProcessEntry>, String> {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("runtime_inspection_failed".to_string());
    }

    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut entries = Vec::new();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;

    while has_entry {
        let end = entry
            .szExeFile
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(entry.szExeFile.len());
        entries.push(ProcessEntry {
            pid: entry.th32ProcessID,
            name: String::from_utf16_lossy(&entry.szExeFile[..end]),
        });
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }

    unsafe {
        CloseHandle(snapshot);
    }
    Ok(entries)
}

#[cfg(not(target_os = "windows"))]
fn process_entries() -> Result<Vec<ProcessEntry>, String> {
    Ok(Vec::new())
}

#[cfg(target_os = "windows")]
fn request_dota_close(entries: &[ProcessEntry]) -> Result<(), String> {
    use std::collections::HashSet;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
    };

    struct CloseContext {
        target_pids: HashSet<u32>,
        messages_sent: usize,
    }

    unsafe extern "system" fn close_dota_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        let context = &mut *(lparam as *mut CloseContext);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if context.target_pids.contains(&pid) && PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0 {
            context.messages_sent += 1;
        }
        1
    }

    let target_pids = entries
        .iter()
        .filter(|entry| is_dota_process(&entry.name.to_ascii_lowercase()))
        .map(|entry| entry.pid)
        .collect::<HashSet<_>>();
    if target_pids.is_empty() {
        return Ok(());
    }

    let mut context = CloseContext {
        target_pids,
        messages_sent: 0,
    };
    let enumerated = unsafe {
        EnumWindows(
            Some(close_dota_window),
            (&mut context as *mut CloseContext) as LPARAM,
        )
    };
    if enumerated == 0 || context.messages_sent == 0 {
        return Err("dota_close_unavailable".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn request_dota_close(_entries: &[ProcessEntry]) -> Result<(), String> {
    Err("platform_not_supported".to_string())
}

#[cfg(target_os = "windows")]
fn resolve_steam_executable() -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};
    use winreg::RegKey;

    let candidates = [
        (
            HKEY_CURRENT_USER,
            "Software\\Valve\\Steam",
            "SteamPath",
            KEY_READ,
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\Valve\\Steam",
            "InstallPath",
            KEY_READ | KEY_WOW64_32KEY,
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\WOW6432Node\\Valve\\Steam",
            "InstallPath",
            KEY_READ,
        ),
    ];

    for (hive, key_path, value_name, flags) in candidates {
        let hive = RegKey::predef(hive);
        let Ok(key) = hive.open_subkey_with_flags(key_path, flags) else {
            continue;
        };
        let Ok(root) = key.get_value::<String, _>(value_name) else {
            continue;
        };
        let executable = PathBuf::from(root.trim()).join("steam.exe");
        let Ok(canonical) = executable.canonicalize() else {
            continue;
        };
        if canonical.is_file()
            && canonical
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("steam.exe"))
        {
            return Ok(canonical);
        }
    }
    Err("steam_not_found".to_string())
}

#[cfg(target_os = "windows")]
fn request_steam_close() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    let executable = resolve_steam_executable()?;
    Command::new(executable)
        .arg("-exitsteam")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|_| "steam_shutdown_failed".to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn request_steam_start() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    let executable = resolve_steam_executable()?;
    Command::new(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|_| "steam_start_failed".to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn request_steam_start() -> Result<(), String> {
    Err("platform_not_supported".to_string())
}

#[cfg(not(target_os = "windows"))]
fn request_steam_close() -> Result<(), String> {
    Err("platform_not_supported".to_string())
}

pub fn inspect_runtime() -> Result<RuntimeState, String> {
    let entries = process_entries()?;
    Ok(state_from_entries(&entries))
}

pub fn prepare_runtime_for_patch(request: RuntimePrepareRequest) -> Result<RuntimeState, String> {
    if !request.confirmed {
        return Err("runtime_confirmation_required".to_string());
    }
    if !cfg!(target_os = "windows") {
        return Err("platform_not_supported".to_string());
    }

    let mut entries = process_entries()?;
    let initial = state_from_entries(&entries);
    if initial.dota_running {
        request_dota_close(&entries)?;
        wait_for_patch_ready(
            || {
                let state = inspect_runtime()?;
                Ok(RuntimeState {
                    patch_ready: !state.dota_running,
                    ..state
                })
            },
            SHUTDOWN_MAX_POLLS,
            SHUTDOWN_POLL_INTERVAL,
        )?;
        entries = process_entries()?;
    }

    if state_from_entries(&entries).steam_running {
        request_steam_close()?;
    }

    wait_for_patch_ready(inspect_runtime, SHUTDOWN_MAX_POLLS, SHUTDOWN_POLL_INTERVAL)
}

pub fn start_steam(request: SteamStartRequest) -> Result<RuntimeState, String> {
    if !request.confirmed {
        return Err("runtime_confirmation_required".to_string());
    }
    if !cfg!(target_os = "windows") {
        return Err("platform_not_supported".to_string());
    }

    let state = inspect_runtime()?;
    if state.dota_running {
        return Err("runtime_busy".to_string());
    }
    if state.steam_running {
        return Ok(state);
    }

    request_steam_start()?;
    wait_for_steam_started(inspect_runtime, SHUTDOWN_MAX_POLLS, SHUTDOWN_POLL_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_process_list_is_ready() {
        let state = classify_processes([]);
        assert!(!state.steam_running);
        assert!(!state.dota_running);
        assert!(state.patch_ready);
    }

    #[test]
    fn matching_is_case_insensitive_and_includes_helpers() {
        let state = classify_processes(["SteamWebHelper.EXE", "GameOverlayUI.exe", "DOTA2.exe"]);
        assert!(state.steam_running);
        assert!(state.dota_running);
        assert!(!state.patch_ready);
    }

    #[test]
    fn unrelated_processes_do_not_block_patch() {
        let state = classify_processes(["explorer.exe", "steamservice.exe", "discord.exe"]);
        assert!(!state.steam_running);
        assert!(!state.dota_running);
        assert!(state.patch_ready);
    }

    #[test]
    fn wait_accepts_a_delayed_clean_state() {
        let mut calls = 0;
        let state = wait_for_patch_ready(
            || {
                calls += 1;
                Ok(RuntimeState {
                    platform_supported: true,
                    steam_running: calls < 3,
                    dota_running: false,
                    patch_ready: calls >= 3,
                })
            },
            3,
            Duration::ZERO,
        )
        .expect("ready state");
        assert!(state.patch_ready);
        assert_eq!(calls, 3);
    }

    #[test]
    fn wait_returns_a_stable_timeout_code() {
        let result = wait_for_patch_ready(
            || {
                Ok(RuntimeState {
                    platform_supported: true,
                    steam_running: true,
                    dota_running: false,
                    patch_ready: false,
                })
            },
            1,
            Duration::ZERO,
        );
        assert_eq!(result.err().as_deref(), Some("shutdown_timeout"));
    }

    #[test]
    fn preparation_requires_explicit_confirmation() {
        let result = prepare_runtime_for_patch(RuntimePrepareRequest { confirmed: false });
        assert_eq!(
            result.err().as_deref(),
            Some("runtime_confirmation_required")
        );
    }

    #[test]
    fn steam_start_wait_accepts_a_delayed_client() {
        let mut calls = 0;
        let state = wait_for_steam_started(
            || {
                calls += 1;
                Ok(RuntimeState {
                    platform_supported: true,
                    steam_running: calls >= 3,
                    dota_running: false,
                    patch_ready: calls < 3,
                })
            },
            3,
            Duration::ZERO,
        )
        .expect("started state");
        assert!(state.steam_running);
        assert_eq!(calls, 3);
    }

    #[test]
    fn steam_start_requires_explicit_confirmation() {
        let result = start_steam(SteamStartRequest { confirmed: false });
        assert_eq!(
            result.err().as_deref(),
            Some("runtime_confirmation_required")
        );
    }
}
