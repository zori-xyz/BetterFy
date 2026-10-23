//! What BetterFy knows about how its previous run ended.
//!
//! Two small files in the app data folder:
//!
//! * `session-alive.json` is written when the app starts and removed on a
//!   normal exit. If it is still there at the next start, the previous run did
//!   not end normally (a crash, a kill, a power cut).
//! * `last-panic.json` is written by the panic hook: the message, the source
//!   location, and the time. It never holds a stack, file contents, account
//!   data or full paths: the user's home folder and `Users\<name>` segments
//!   are replaced by `~` before anything is written.
//!
//! The interface asks once with [`take_previous_run`] and decides what to show
//! or send; nothing here leaves the machine by itself.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const ALIVE: &str = "session-alive.json";
const PANIC: &str = "last-panic.json";
const MAX_TEXT: usize = 300;
const MAX_FILE: u64 = 16 * 1024;

static DIR: OnceLock<PathBuf> = OnceLock::new();
static PREVIOUS: Mutex<Option<PreviousRun>> = Mutex::new(None);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PanicNote {
    pub at_ms: u64,
    pub message: String,
    pub location: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviousRun {
    /// The previous run did not end through the normal exit.
    pub unclean_exit: bool,
    pub started_at_ms: Option<u64>,
    pub version: Option<String>,
    pub panic: Option<PanicNote>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Alive {
    started_at_ms: u64,
    version: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_millis() as u64)
}

/// Removes the user's home folder and `Users\<name>` / `Users/<name>` segments
/// and bounds the length, so a message can be shown or sent without a name.
pub(crate) fn sanitize(text: &str) -> String {
    let mut text = text.replace('\\', "/");
    for variable in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(variable) {
            let home = home.replace('\\', "/");
            if home.len() > 3 {
                text = text.replace(&home, "~");
            }
        }
    }
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(index) = rest.to_ascii_lowercase().find("users/") {
        let (head, tail) = rest.split_at(index + "users/".len());
        out.push_str(head);
        out.push('~');
        let end = tail.find('/').unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out.chars().take(MAX_TEXT).collect()
}

fn read_small<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Reads how the previous run ended, then marks this run as started. Safe to
/// call once per process; a second call only refreshes the marker.
pub fn begin(dir: &Path, version: &str) {
    let _ = fs::create_dir_all(dir);
    let alive: Option<Alive> = read_small(&dir.join(ALIVE));
    let panic: Option<PanicNote> = read_small(&dir.join(PANIC));
    let _ = fs::remove_file(dir.join(PANIC));
    if alive.is_some() || panic.is_some() {
        if let Ok(mut slot) = PREVIOUS.lock() {
            *slot = Some(PreviousRun {
                unclean_exit: alive.is_some(),
                started_at_ms: alive.as_ref().map(|value| value.started_at_ms),
                version: alive.map(|value| value.version),
                panic,
            });
        }
    }
    let marker = Alive {
        started_at_ms: now_ms(),
        version: version.to_string(),
    };
    if let Ok(bytes) = serde_json::to_vec(&marker) {
        let _ = fs::write(dir.join(ALIVE), bytes);
    }
    if DIR.set(dir.to_path_buf()).is_ok() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(dir) = DIR.get() {
                let message = info
                    .payload()
                    .downcast_ref::<&str>()
                    .map(|value| (*value).to_string())
                    .or_else(|| info.payload().downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "panic".to_string());
                let location = info.location().map_or_else(String::new, |place| {
                    let file = sanitize(place.file());
                    let tail = file.rsplit('/').take(2).collect::<Vec<_>>();
                    format!(
                        "{}:{}",
                        tail.into_iter().rev().collect::<Vec<_>>().join("/"),
                        place.line()
                    )
                });
                write_panic(dir, &message, &location);
            }
            previous(info);
        }));
    }
}

fn write_panic(dir: &Path, message: &str, location: &str) {
    let note = PanicNote {
        at_ms: now_ms(),
        message: sanitize(message),
        location: sanitize(location),
    };
    if let Ok(bytes) = serde_json::to_vec(&note) {
        let _ = fs::write(dir.join(PANIC), bytes);
    }
}

/// Called on a normal exit: the next start will see a clean ending.
pub fn clean_exit() {
    if let Some(dir) = DIR.get() {
        let _ = fs::remove_file(dir.join(ALIVE));
    }
}

/// How the previous run ended, once; `None` when it ended normally.
#[tauri::command]
pub fn take_previous_run() -> Option<PreviousRun> {
    PREVIOUS.lock().ok().and_then(|mut slot| slot.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "betterfy-crash-{label}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&path).expect("dir");
        path
    }

    // `begin` also installs the process-wide hook and sets the global
    // directory once, so these tests exercise the file logic directly.
    fn previous_from(dir: &Path) -> Option<PreviousRun> {
        let alive: Option<Alive> = read_small(&dir.join(ALIVE));
        let panic: Option<PanicNote> = read_small(&dir.join(PANIC));
        (alive.is_some() || panic.is_some()).then(|| PreviousRun {
            unclean_exit: alive.is_some(),
            started_at_ms: alive.as_ref().map(|value| value.started_at_ms),
            version: alive.map(|value| value.version),
            panic,
        })
    }

    #[test]
    fn a_start_without_a_clean_exit_is_reported_once() {
        let dir = temp("unclean");
        assert_eq!(previous_from(&dir), None);
        fs::write(
            dir.join(ALIVE),
            serde_json::to_vec(&Alive {
                started_at_ms: 5,
                version: "0.5.0".into(),
            })
            .unwrap(),
        )
        .unwrap();
        let previous = previous_from(&dir).expect("unclean");
        assert!(previous.unclean_exit);
        assert_eq!(previous.version.as_deref(), Some("0.5.0"));
        fs::remove_file(dir.join(ALIVE)).unwrap();
        assert_eq!(previous_from(&dir), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_panic_note_carries_no_home_folder_or_user_name() {
        let dir = temp("panic");
        write_panic(
            &dir,
            "failed at C:\\Users\\Ivan Petrov\\AppData\\x.json",
            "C:\\Users\\Ivan Petrov\\src\\main.rs:12",
        );
        let note: PanicNote = read_small(&dir.join(PANIC)).expect("note");
        assert!(!note.message.contains("Ivan"));
        assert!(!note.location.contains("Petrov"));
        assert!(note.message.contains("Users/~"));
        assert!(note.message.chars().count() <= MAX_TEXT);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sanitize_bounds_the_length_and_keeps_plain_text() {
        assert_eq!(sanitize("plain message"), "plain message");
        assert_eq!(sanitize(&"a".repeat(1000)).chars().count(), MAX_TEXT);
    }
}
