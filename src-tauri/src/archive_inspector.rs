use serde::Serialize;
use std::collections::BTreeSet;
use std::io::Cursor;
use std::path::Component;
use zip::read::ZipArchive;
use zip::CompressionMethod;

const MAX_ENTRIES: usize = 2_048;
const MAX_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_NAME_BYTES: usize = 240;
const MAX_COMPRESSION_RATIO: u64 = 200;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveReport {
    pub entries: usize,
    pub files: usize,
    pub expanded_bytes: u64,
    pub compressed_bytes: u64,
}

fn unsafe_extension(name: &str) -> bool {
    const BLOCKED: [&str; 16] = [
        "exe", "dll", "bat", "cmd", "ps1", "com", "scr", "msi", "js", "jse", "vbs", "vbe", "lnk",
        "sys", "drv", "ocx",
    ];
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| BLOCKED.contains(&extension.to_ascii_lowercase().as_str()))
}

fn windows_reserved(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

fn normalized_name(name: &str) -> Result<(String, bool), String> {
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || !name.is_ascii()
        || name.contains(['\\', ':', '\0'])
    {
        return Err("archive_path_unsafe".to_string());
    }
    let is_directory = name.ends_with('/');
    let trimmed = name.trim_end_matches('/');
    if trimmed.is_empty() || trimmed.starts_with('/') || trimmed.contains("//") {
        return Err("archive_path_unsafe".to_string());
    }
    let path = std::path::Path::new(trimmed);
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err("archive_path_unsafe".to_string());
        };
        let component = component
            .to_str()
            .ok_or_else(|| "archive_path_unsafe".to_string())?;
        if component.ends_with([' ', '.']) || windows_reserved(component) {
            return Err("archive_path_unsafe".to_string());
        }
    }
    if !is_directory && unsafe_extension(trimmed) {
        return Err("archive_content_unsafe".to_string());
    }
    Ok((trimmed.to_ascii_lowercase(), is_directory))
}

pub fn inspect_zip_bytes(bytes: &[u8]) -> Result<ArchiveReport, String> {
    if bytes.is_empty() || bytes.len() as u64 > crate::content_store::MAX_ARTIFACT_BYTES {
        return Err("archive_size_invalid".to_string());
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| "archive_format_invalid".to_string())?;
    if archive.offset() != 0 || archive.is_empty() || archive.len() > MAX_ENTRIES {
        return Err("archive_format_invalid".to_string());
    }

    let mut names = BTreeSet::new();
    let mut file_paths = BTreeSet::new();
    let mut directory_paths = BTreeSet::new();
    let mut files = 0usize;
    let mut expanded_bytes = 0u64;
    let mut compressed_bytes = 0u64;

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| "archive_format_invalid".to_string())?;
        if entry.encrypted() || entry.is_symlink() || (!entry.is_file() && !entry.is_dir()) {
            return Err("archive_content_unsafe".to_string());
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| "archive_path_unsafe".to_string())?;
        if enclosed
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("archive_path_unsafe".to_string());
        }
        let (normalized, is_directory) = normalized_name(entry.name())?;
        if !names.insert(normalized.clone()) {
            return Err("archive_path_collision".to_string());
        }
        let parents = std::path::Path::new(&normalized)
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if parents.iter().any(|parent| file_paths.contains(parent)) {
            return Err("archive_path_collision".to_string());
        }
        if is_directory {
            if file_paths.contains(&normalized) {
                return Err("archive_path_collision".to_string());
            }
            directory_paths.insert(normalized);
            directory_paths.extend(parents);
            continue;
        }
        if directory_paths.contains(&normalized) {
            return Err("archive_path_collision".to_string());
        }
        directory_paths.extend(parents);
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err("archive_compression_unsupported".to_string());
        }
        if entry.size() > MAX_ENTRY_BYTES {
            return Err("archive_size_invalid".to_string());
        }
        if entry.size() > 1024 * 1024
            && (entry.compressed_size() == 0
                || entry.size() / entry.compressed_size().max(1) > MAX_COMPRESSION_RATIO)
        {
            return Err("archive_ratio_invalid".to_string());
        }
        expanded_bytes = expanded_bytes
            .checked_add(entry.size())
            .ok_or_else(|| "archive_size_invalid".to_string())?;
        compressed_bytes = compressed_bytes
            .checked_add(entry.compressed_size())
            .ok_or_else(|| "archive_size_invalid".to_string())?;
        if expanded_bytes > MAX_EXPANDED_BYTES {
            return Err("archive_size_invalid".to_string());
        }
        file_paths.insert(normalized);
        files += 1;
    }
    if files == 0 {
        return Err("archive_format_invalid".to_string());
    }
    Ok(ArchiveReport {
        entries: archive.len(),
        files,
        expanded_bytes,
        compressed_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn archive_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut bytes));
            for (name, contents) in entries {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .expect("start file");
                writer.write_all(contents).expect("write file");
            }
            writer.finish().expect("finish archive");
        }
        bytes
    }

    #[test]
    fn accepts_a_small_data_only_archive() {
        let bytes = archive_with(&[("panorama/styles/betterfy.css", b".root {}")]);
        let report = inspect_zip_bytes(&bytes).expect("valid archive");
        assert_eq!(report.files, 1);
        assert_eq!(report.expanded_bytes, 8);
    }

    #[test]
    fn rejects_traversal_and_case_collisions() {
        let traversal = archive_with(&[("../escape.css", b"bad")]);
        assert_eq!(
            inspect_zip_bytes(&traversal).err().as_deref(),
            Some("archive_path_unsafe")
        );
        let duplicate = archive_with(&[("A.css", b"a"), ("a.CSS", b"b")]);
        assert_eq!(
            inspect_zip_bytes(&duplicate).err().as_deref(),
            Some("archive_path_collision")
        );
    }

    #[test]
    fn rejects_executable_and_reserved_names() {
        let executable = archive_with(&[("payload.EXE", b"bad")]);
        assert_eq!(
            inspect_zip_bytes(&executable).err().as_deref(),
            Some("archive_content_unsafe")
        );
        let reserved = archive_with(&[("assets/CON.txt", b"bad")]);
        assert_eq!(
            inspect_zip_bytes(&reserved).err().as_deref(),
            Some("archive_path_unsafe")
        );
    }

    #[test]
    fn rejects_file_directory_aliases_in_either_order() {
        for bytes in [
            archive_with(&[("a", b"file"), ("a/child.css", b"child")]),
            archive_with(&[("a/child.css", b"child"), ("a", b"file")]),
        ] {
            assert_eq!(
                inspect_zip_bytes(&bytes).err().as_deref(),
                Some("archive_path_collision")
            );
        }
    }
}
