use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub platform_supported: bool,
    pub steam_running: bool,
    pub dota_running: bool,
    pub patch_ready: bool,
}

fn classify_processes<'a>(names: impl IntoIterator<Item = &'a str>) -> RuntimeState {
    let mut steam_running = false;
    let mut dota_running = false;

    for name in names {
        let normalized = name.trim().to_ascii_lowercase();
        steam_running |= matches!(normalized.as_str(), "steam.exe" | "steamwebhelper.exe");
        dota_running |= matches!(
            normalized.as_str(),
            "dota2.exe" | "dota2launcher.exe" | "dota2cfg.exe"
        );
    }

    RuntimeState {
        platform_supported: cfg!(target_os = "windows"),
        steam_running,
        dota_running,
        patch_ready: !steam_running && !dota_running,
    }
}

#[cfg(target_os = "windows")]
fn process_names() -> Result<Vec<String>, String> {
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

    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut names = Vec::new();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;

    while has_entry {
        let end = entry
            .szExeFile
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(entry.szExeFile.len());
        names.push(String::from_utf16_lossy(&entry.szExeFile[..end]));
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }

    unsafe {
        CloseHandle(snapshot);
    }
    Ok(names)
}

#[cfg(not(target_os = "windows"))]
fn process_names() -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

pub fn inspect_runtime() -> Result<RuntimeState, String> {
    let names = process_names()?;
    Ok(classify_processes(names.iter().map(String::as_str)))
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
        let state = classify_processes(["SteamWebHelper.EXE", "DOTA2.exe"]);
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
}
