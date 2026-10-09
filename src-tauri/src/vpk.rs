use crc32fast::Hasher as Crc32;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const VPK_SIGNATURE: u32 = 0x55aa_1234;
const VPK_VERSION: u32 = 1;
const DIRECTORY_ARCHIVE_INDEX: u16 = 0x7fff;
const ENTRY_TERMINATOR: u16 = 0xffff;
const MAX_VPK_BYTES: usize = 256 * 1024 * 1024;
const MAX_VPK_ENTRIES: usize = 65_536;
const MAX_TREE_STRING_BYTES: usize = 260;

#[derive(Clone)]
pub struct VpkInput<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpkReport {
    pub version: u32,
    pub entries: usize,
    pub payload_bytes: u64,
}

struct NormalizedInput<'a> {
    extension: String,
    directory: String,
    stem: String,
    path: String,
    bytes: &'a [u8],
}

pub(crate) fn validate_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 512
        || value.contains('\\')
        || value.contains(':')
        || value.contains('\0')
        || !value.is_ascii()
    {
        return Err("vpk_path_invalid".to_string());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("vpk_path_invalid".to_string());
    }
    if value.bytes().any(|byte| {
        !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-/ .".contains(&byte))
    }) {
        return Err("vpk_path_invalid".to_string());
    }
    Ok(())
}

fn normalize_input<'a>(input: VpkInput<'a>) -> Result<NormalizedInput<'a>, String> {
    validate_path(input.path)?;
    // Zero-length entries are valid VPK v1 entries and are how Minify ships
    // silenced sounds; only the upper bound is enforced.
    if input.bytes.len() > MAX_VPK_BYTES {
        return Err("vpk_payload_invalid".to_string());
    }
    let path = Path::new(input.path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "vpk_path_invalid".to_string())?;
    let (stem, extension) = file_name
        .rsplit_once('.')
        .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
        .ok_or_else(|| "vpk_path_invalid".to_string())?;
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .and_then(|parent| parent.to_str())
        .unwrap_or(" ")
        .to_string();
    Ok(NormalizedInput {
        extension: extension.to_string(),
        directory,
        stem: stem.to_string(),
        path: input.path.to_string(),
        bytes: input.bytes,
    })
}

fn write_cstring(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(value.as_bytes());
    output.push(0);
}

pub fn build(inputs: Vec<VpkInput<'_>>) -> Result<Vec<u8>, String> {
    if inputs.is_empty() || inputs.len() > MAX_VPK_ENTRIES {
        return Err("vpk_payload_invalid".to_string());
    }
    let mut normalized = inputs
        .into_iter()
        .map(normalize_input)
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort_by(|left, right| left.path.cmp(&right.path));
    let mut unique = BTreeSet::new();
    let mut total_payload = 0usize;
    for input in &normalized {
        if !unique.insert(input.path.to_ascii_lowercase()) {
            return Err("vpk_path_collision".to_string());
        }
        total_payload = total_payload
            .checked_add(input.bytes.len())
            .ok_or_else(|| "vpk_payload_invalid".to_string())?;
    }
    if total_payload > MAX_VPK_BYTES {
        return Err("vpk_payload_invalid".to_string());
    }

    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<&NormalizedInput<'_>>>> = BTreeMap::new();
    for input in &normalized {
        grouped
            .entry(&input.extension)
            .or_default()
            .entry(&input.directory)
            .or_default()
            .push(input);
    }

    let mut tree = Vec::new();
    let mut data = Vec::with_capacity(total_payload);
    for (extension, directories) in grouped {
        write_cstring(&mut tree, extension);
        for (directory, entries) in directories {
            write_cstring(&mut tree, directory);
            for entry in entries {
                let offset =
                    u32::try_from(data.len()).map_err(|_| "vpk_payload_invalid".to_string())?;
                let length = u32::try_from(entry.bytes.len())
                    .map_err(|_| "vpk_payload_invalid".to_string())?;
                let mut crc = Crc32::new();
                crc.update(entry.bytes);
                write_cstring(&mut tree, &entry.stem);
                tree.extend_from_slice(&crc.finalize().to_le_bytes());
                tree.extend_from_slice(&0u16.to_le_bytes());
                tree.extend_from_slice(&DIRECTORY_ARCHIVE_INDEX.to_le_bytes());
                tree.extend_from_slice(&offset.to_le_bytes());
                tree.extend_from_slice(&length.to_le_bytes());
                tree.extend_from_slice(&ENTRY_TERMINATOR.to_le_bytes());
                data.extend_from_slice(entry.bytes);
            }
            tree.push(0);
        }
        tree.push(0);
    }
    tree.push(0);

    let tree_size = u32::try_from(tree.len()).map_err(|_| "vpk_payload_invalid".to_string())?;
    let capacity = 12usize
        .checked_add(tree.len())
        .and_then(|value| value.checked_add(data.len()))
        .ok_or_else(|| "vpk_payload_invalid".to_string())?;
    if capacity > MAX_VPK_BYTES {
        return Err("vpk_payload_invalid".to_string());
    }
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(&VPK_SIGNATURE.to_le_bytes());
    output.extend_from_slice(&VPK_VERSION.to_le_bytes());
    output.extend_from_slice(&tree_size.to_le_bytes());
    output.extend_from_slice(&tree);
    output.extend_from_slice(&data);
    inspect(&output)?;
    Ok(output)
}

fn read_u16(bytes: &[u8], cursor: &mut usize, end: usize) -> Result<u16, String> {
    let next = cursor
        .checked_add(2)
        .filter(|next| *next <= end)
        .ok_or_else(|| "vpk_invalid".to_string())?;
    let value = u16::from_le_bytes(
        bytes[*cursor..next]
            .try_into()
            .map_err(|_| "vpk_invalid".to_string())?,
    );
    *cursor = next;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize, end: usize) -> Result<u32, String> {
    let next = cursor
        .checked_add(4)
        .filter(|next| *next <= end)
        .ok_or_else(|| "vpk_invalid".to_string())?;
    let value = u32::from_le_bytes(
        bytes[*cursor..next]
            .try_into()
            .map_err(|_| "vpk_invalid".to_string())?,
    );
    *cursor = next;
    Ok(value)
}

fn read_cstring(bytes: &[u8], cursor: &mut usize, end: usize) -> Result<String, String> {
    if *cursor >= end {
        return Err("vpk_invalid".to_string());
    }
    let remaining = &bytes[*cursor..end];
    let length = remaining
        .iter()
        .position(|byte| *byte == 0)
        .filter(|length| *length <= MAX_TREE_STRING_BYTES)
        .ok_or_else(|| "vpk_invalid".to_string())?;
    let value = std::str::from_utf8(&remaining[..length])
        .map_err(|_| "vpk_invalid".to_string())?
        .to_string();
    *cursor += length + 1;
    Ok(value)
}

pub(crate) fn extract_embedded(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if bytes.len() < 15 || bytes.len() > MAX_VPK_BYTES {
        return Err("vpk_invalid".to_string());
    }
    let mut header_cursor = 0usize;
    if read_u32(bytes, &mut header_cursor, bytes.len())? != VPK_SIGNATURE {
        return Err("vpk_invalid".to_string());
    }
    let version = read_u32(bytes, &mut header_cursor, bytes.len())?;
    if version != VPK_VERSION {
        return Err("vpk_version_unsupported".to_string());
    }
    let tree_size = read_u32(bytes, &mut header_cursor, bytes.len())? as usize;
    let tree_end = 12usize
        .checked_add(tree_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| "vpk_invalid".to_string())?;
    let data_start = tree_end;
    let mut cursor = 12usize;
    let mut paths = BTreeSet::new();
    let mut resources = BTreeMap::new();
    let mut entries = 0usize;
    let mut extracted_bytes = 0usize;
    loop {
        let extension = read_cstring(bytes, &mut cursor, tree_end)?;
        if extension.is_empty() {
            break;
        }
        loop {
            let directory = read_cstring(bytes, &mut cursor, tree_end)?;
            if directory.is_empty() {
                break;
            }
            loop {
                let stem = read_cstring(bytes, &mut cursor, tree_end)?;
                if stem.is_empty() {
                    break;
                }
                entries += 1;
                if entries > MAX_VPK_ENTRIES {
                    return Err("vpk_invalid".to_string());
                }
                let expected_crc = read_u32(bytes, &mut cursor, tree_end)?;
                let preload_length = read_u16(bytes, &mut cursor, tree_end)? as usize;
                if read_u16(bytes, &mut cursor, tree_end)? != DIRECTORY_ARCHIVE_INDEX {
                    return Err("vpk_archive_unsupported".to_string());
                }
                let offset = read_u32(bytes, &mut cursor, tree_end)? as usize;
                let length = read_u32(bytes, &mut cursor, tree_end)? as usize;
                if read_u16(bytes, &mut cursor, tree_end)? != ENTRY_TERMINATOR {
                    return Err("vpk_invalid".to_string());
                }
                let preload_end = cursor
                    .checked_add(preload_length)
                    .filter(|end| *end <= tree_end)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let preload = &bytes[cursor..preload_end];
                cursor = preload_end;
                let data_offset = data_start
                    .checked_add(offset)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let data_end = data_offset
                    .checked_add(length)
                    .filter(|end| *end <= bytes.len())
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let directory = if directory == " " { "" } else { &directory };
                let path = if directory.is_empty() {
                    format!("{stem}.{extension}")
                } else {
                    format!("{directory}/{stem}.{extension}")
                };
                validate_path(&path).map_err(|_| "vpk_invalid".to_string())?;
                if !paths.insert(path.to_ascii_lowercase()) {
                    return Err("vpk_path_collision".to_string());
                }
                let mut crc = Crc32::new();
                crc.update(preload);
                crc.update(&bytes[data_offset..data_end]);
                if crc.finalize() != expected_crc {
                    return Err("vpk_crc_mismatch".to_string());
                }
                let payload_length = preload_length
                    .checked_add(length)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                extracted_bytes = extracted_bytes
                    .checked_add(payload_length)
                    .filter(|total| *total <= MAX_VPK_BYTES)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let mut payload = Vec::with_capacity(payload_length);
                payload.extend_from_slice(preload);
                payload.extend_from_slice(&bytes[data_offset..data_end]);
                resources.insert(path, payload);
            }
        }
    }
    if cursor != tree_end || entries == 0 {
        return Err("vpk_invalid".to_string());
    }
    Ok(resources)
}

/// One entry of a third-party VPK directory, exactly as the archive lists it.
///
/// `path` is the raw joined path (directory, name, extension). It is neither
/// validated nor lowercased: classifying and rejecting paths is the caller's
/// job, so one hostile entry cannot hide the rest of an archive from a report.
/// The bytes were checked against the directory CRC.
pub(crate) struct DirectoryEntry<'a> {
    pub path: String,
    preload: &'a [u8],
    data: &'a [u8],
}

impl<'a> DirectoryEntry<'a> {
    pub(crate) fn len(&self) -> usize {
        self.preload.len() + self.data.len()
    }

    pub(crate) fn payload(&self) -> std::borrow::Cow<'a, [u8]> {
        if self.preload.is_empty() {
            std::borrow::Cow::Borrowed(self.data)
        } else {
            let mut joined = Vec::with_capacity(self.len());
            joined.extend_from_slice(self.preload);
            joined.extend_from_slice(self.data);
            std::borrow::Cow::Owned(joined)
        }
    }
}

/// Reads every entry of a VPK version 1 or 2 whose data is embedded in the
/// directory file itself (archive index `0x7fff`). An entry that lives in a
/// numbered side archive refuses the whole file, and so does a CRC mismatch.
/// Version 2 trailers (the archive and signature sections after the data) are
/// not read: the per-entry CRC is what is verified.
pub(crate) fn read_embedded_directory(bytes: &[u8]) -> Result<Vec<DirectoryEntry<'_>>, String> {
    if bytes.len() < 15 || bytes.len() > MAX_VPK_BYTES {
        return Err("vpk_invalid".to_string());
    }
    let mut header_cursor = 0usize;
    if read_u32(bytes, &mut header_cursor, bytes.len())? != VPK_SIGNATURE {
        return Err("vpk_invalid".to_string());
    }
    let version = read_u32(bytes, &mut header_cursor, bytes.len())?;
    let tree_size = read_u32(bytes, &mut header_cursor, bytes.len())? as usize;
    let tree_start = match version {
        1 => 12usize,
        2 => 28usize,
        _ => return Err("vpk_version_unsupported".to_string()),
    };
    let tree_end = tree_start
        .checked_add(tree_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| "vpk_invalid".to_string())?;
    let mut cursor = tree_start;
    let mut entries = Vec::new();
    loop {
        let extension = read_cstring(bytes, &mut cursor, tree_end)?;
        if extension.is_empty() {
            break;
        }
        loop {
            let directory = read_cstring(bytes, &mut cursor, tree_end)?;
            if directory.is_empty() {
                break;
            }
            loop {
                let stem = read_cstring(bytes, &mut cursor, tree_end)?;
                if stem.is_empty() {
                    break;
                }
                if entries.len() >= MAX_VPK_ENTRIES {
                    return Err("vpk_invalid".to_string());
                }
                let expected_crc = read_u32(bytes, &mut cursor, tree_end)?;
                let preload_length = read_u16(bytes, &mut cursor, tree_end)? as usize;
                if read_u16(bytes, &mut cursor, tree_end)? != DIRECTORY_ARCHIVE_INDEX {
                    return Err("vpk_archive_unsupported".to_string());
                }
                let offset = read_u32(bytes, &mut cursor, tree_end)? as usize;
                let length = read_u32(bytes, &mut cursor, tree_end)? as usize;
                if read_u16(bytes, &mut cursor, tree_end)? != ENTRY_TERMINATOR {
                    return Err("vpk_invalid".to_string());
                }
                let preload_end = cursor
                    .checked_add(preload_length)
                    .filter(|end| *end <= tree_end)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let preload = &bytes[cursor..preload_end];
                cursor = preload_end;
                let data_start = tree_end
                    .checked_add(offset)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let data_end = data_start
                    .checked_add(length)
                    .filter(|end| *end <= bytes.len())
                    .ok_or_else(|| "vpk_invalid".to_string())?;
                let data = &bytes[data_start..data_end];
                let mut crc = Crc32::new();
                crc.update(preload);
                crc.update(data);
                if crc.finalize() != expected_crc {
                    return Err("vpk_crc_mismatch".to_string());
                }
                let path = if directory == " " {
                    format!("{stem}.{extension}")
                } else {
                    format!("{directory}/{stem}.{extension}")
                };
                entries.push(DirectoryEntry {
                    path,
                    preload,
                    data,
                });
            }
        }
    }
    if cursor != tree_end || entries.is_empty() {
        return Err("vpk_invalid".to_string());
    }
    Ok(entries)
}

/// Test helper: writes a VPK (version 1 or 2, every entry embedded) from
/// arbitrary path strings without any of `build`'s path checks, so tests can
/// build the hostile archives `build` refuses to write.
#[cfg(test)]
pub(crate) fn build_unchecked(version: u32, entries: &[(&str, &[u8])]) -> Vec<u8> {
    type Files<'a> = Vec<(String, &'a [u8])>;
    let mut grouped: BTreeMap<String, BTreeMap<String, Files<'_>>> = BTreeMap::new();
    for (path, bytes) in entries {
        let (directory, file) = path.rsplit_once('/').unwrap_or((" ", path));
        let (stem, extension) = file.rsplit_once('.').expect("test path has an extension");
        grouped
            .entry(extension.to_string())
            .or_default()
            .entry(directory.to_string())
            .or_default()
            .push((stem.to_string(), *bytes));
    }
    let mut tree = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    for (extension, directories) in grouped {
        write_cstring(&mut tree, &extension);
        for (directory, files) in directories {
            write_cstring(&mut tree, &directory);
            for (stem, bytes) in files {
                write_cstring(&mut tree, &stem);
                let mut crc = Crc32::new();
                crc.update(bytes);
                tree.extend_from_slice(&crc.finalize().to_le_bytes());
                tree.extend_from_slice(&0u16.to_le_bytes());
                tree.extend_from_slice(&DIRECTORY_ARCHIVE_INDEX.to_le_bytes());
                tree.extend_from_slice(&(data.len() as u32).to_le_bytes());
                tree.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                tree.extend_from_slice(&ENTRY_TERMINATOR.to_le_bytes());
                data.extend_from_slice(bytes);
            }
            tree.push(0);
        }
        tree.push(0);
    }
    tree.push(0);
    let mut output = Vec::new();
    output.extend_from_slice(&VPK_SIGNATURE.to_le_bytes());
    output.extend_from_slice(&version.to_le_bytes());
    output.extend_from_slice(&(tree.len() as u32).to_le_bytes());
    if version == 2 {
        // Embedded data size, then the three trailer section sizes.
        output.extend_from_slice(&(data.len() as u32).to_le_bytes());
        output.extend_from_slice(&[0u8; 12]);
    }
    output.extend_from_slice(&tree);
    output.extend_from_slice(&data);
    output
}

/// Valve's own archives (`game/dota/pak01_dir.vpk`) are version 2 and keep
/// their data in numbered side archives. Only the directory tree is read here.
const GAME_VPK_MAX_DIRECTORY_BYTES: usize = 1024 * 1024 * 1024;
const GAME_VPK_MAX_ENTRIES: usize = 2_000_000;

/// Lists every resource path in a game VPK directory file (version 1 or 2),
/// lowercased, without reading any resource data.
pub(crate) fn list_directory_paths(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() < 12 || bytes.len() > GAME_VPK_MAX_DIRECTORY_BYTES {
        return Err("game_vpk_invalid".to_string());
    }
    let mut cursor = 0usize;
    if read_u32(bytes, &mut cursor, bytes.len())? != VPK_SIGNATURE {
        return Err("game_vpk_invalid".to_string());
    }
    let version = read_u32(bytes, &mut cursor, bytes.len())?;
    let tree_size = read_u32(bytes, &mut cursor, bytes.len())? as usize;
    let tree_start = match version {
        1 => 12usize,
        2 => 28usize,
        _ => return Err("game_vpk_version_unsupported".to_string()),
    };
    let tree_end = tree_start
        .checked_add(tree_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| "game_vpk_invalid".to_string())?;
    let mut cursor = tree_start;
    let mut paths = Vec::new();
    loop {
        let extension = read_cstring(bytes, &mut cursor, tree_end)?;
        if extension.is_empty() {
            break;
        }
        loop {
            let directory = read_cstring(bytes, &mut cursor, tree_end)?;
            if directory.is_empty() {
                break;
            }
            loop {
                let stem = read_cstring(bytes, &mut cursor, tree_end)?;
                if stem.is_empty() {
                    break;
                }
                // crc, preload length, archive index, offset, length, terminator
                read_u32(bytes, &mut cursor, tree_end)?;
                let preload = read_u16(bytes, &mut cursor, tree_end)? as usize;
                read_u16(bytes, &mut cursor, tree_end)?;
                read_u32(bytes, &mut cursor, tree_end)?;
                read_u32(bytes, &mut cursor, tree_end)?;
                if read_u16(bytes, &mut cursor, tree_end)? != ENTRY_TERMINATOR {
                    return Err("game_vpk_invalid".to_string());
                }
                cursor = cursor
                    .checked_add(preload)
                    .filter(|end| *end <= tree_end)
                    .ok_or_else(|| "game_vpk_invalid".to_string())?;
                if paths.len() >= GAME_VPK_MAX_ENTRIES {
                    return Err("game_vpk_invalid".to_string());
                }
                let directory = if directory == " " {
                    ""
                } else {
                    directory.as_str()
                };
                let path = if directory.is_empty() {
                    format!("{stem}.{extension}")
                } else {
                    format!("{directory}/{stem}.{extension}")
                };
                paths.push(path.to_ascii_lowercase());
            }
        }
    }
    Ok(paths)
}

/// Reads selected resources (lowercase paths) out of a game VPK: the
/// directory file `dir_path` (`.../pak01_dir.vpk`) and its numbered side
/// archives (`pak01_NNN.vpk`). Every resource is checked against the CRC in
/// the directory. Paths the archive does not have are absent from the result.
pub(crate) fn read_game_resources(
    dir_path: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    use std::io::{Read, Seek, SeekFrom};
    const MAX_GAME_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
    let invalid = || "game_vpk_invalid".to_string();
    let bytes = std::fs::read(dir_path).map_err(|_| "game_archive_unreadable".to_string())?;
    if bytes.len() < 12 || bytes.len() > GAME_VPK_MAX_DIRECTORY_BYTES {
        return Err(invalid());
    }
    let mut cursor = 0usize;
    if read_u32(&bytes, &mut cursor, bytes.len())? != VPK_SIGNATURE {
        return Err(invalid());
    }
    let version = read_u32(&bytes, &mut cursor, bytes.len())?;
    let tree_size = read_u32(&bytes, &mut cursor, bytes.len())? as usize;
    let tree_start = match version {
        1 => 12usize,
        2 => 28usize,
        _ => return Err("game_vpk_version_unsupported".to_string()),
    };
    let tree_end = tree_start
        .checked_add(tree_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(invalid)?;
    let archive_prefix = dir_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix("dir.vpk"))
        .ok_or_else(invalid)?
        .to_string();
    let mut found = BTreeMap::new();
    let mut cursor = tree_start;
    loop {
        let extension = read_cstring(&bytes, &mut cursor, tree_end)?;
        if extension.is_empty() {
            break;
        }
        loop {
            let directory = read_cstring(&bytes, &mut cursor, tree_end)?;
            if directory.is_empty() {
                break;
            }
            loop {
                let stem = read_cstring(&bytes, &mut cursor, tree_end)?;
                if stem.is_empty() {
                    break;
                }
                let crc = read_u32(&bytes, &mut cursor, tree_end)?;
                let preload_length = read_u16(&bytes, &mut cursor, tree_end)? as usize;
                let archive = read_u16(&bytes, &mut cursor, tree_end)?;
                let offset = read_u32(&bytes, &mut cursor, tree_end)? as usize;
                let length = read_u32(&bytes, &mut cursor, tree_end)? as usize;
                if read_u16(&bytes, &mut cursor, tree_end)? != ENTRY_TERMINATOR {
                    return Err(invalid());
                }
                let preload_end = cursor
                    .checked_add(preload_length)
                    .filter(|end| *end <= tree_end)
                    .ok_or_else(invalid)?;
                let preload = &bytes[cursor..preload_end];
                cursor = preload_end;
                let directory = if directory == " " {
                    ""
                } else {
                    directory.as_str()
                };
                let path = if directory.is_empty() {
                    format!("{stem}.{extension}")
                } else {
                    format!("{directory}/{stem}.{extension}")
                }
                .to_ascii_lowercase();
                if !wanted.contains(&path) {
                    continue;
                }
                if preload_length + length > MAX_GAME_RESOURCE_BYTES {
                    return Err(invalid());
                }
                let mut payload = preload.to_vec();
                if length > 0 {
                    if archive == DIRECTORY_ARCHIVE_INDEX {
                        let start = tree_end.checked_add(offset).ok_or_else(invalid)?;
                        let end = start
                            .checked_add(length)
                            .filter(|end| *end <= bytes.len())
                            .ok_or_else(invalid)?;
                        payload.extend_from_slice(&bytes[start..end]);
                    } else {
                        let side =
                            dir_path.with_file_name(format!("{archive_prefix}{archive:03}.vpk"));
                        let mut file = std::fs::File::open(side)
                            .map_err(|_| "game_archive_unreadable".to_string())?;
                        file.seek(SeekFrom::Start(offset as u64))
                            .map_err(|_| "game_archive_unreadable".to_string())?;
                        let mut chunk = vec![0u8; length];
                        file.read_exact(&mut chunk)
                            .map_err(|_| "game_archive_unreadable".to_string())?;
                        payload.extend_from_slice(&chunk);
                    }
                }
                let mut check = Crc32::new();
                check.update(&payload);
                if check.finalize() != crc {
                    return Err("game_vpk_crc_mismatch".to_string());
                }
                found.insert(path, payload);
            }
        }
    }
    Ok(found)
}

pub fn inspect(bytes: &[u8]) -> Result<VpkReport, String> {
    let resources = extract_embedded(bytes)?;
    let payload_bytes = resources.values().try_fold(0u64, |total, resource| {
        total
            .checked_add(resource.len() as u64)
            .ok_or_else(|| "vpk_invalid".to_string())
    })?;
    Ok(VpkReport {
        version: VPK_VERSION,
        entries: resources.len(),
        payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_paths_of_version_one_and_two_directories() {
        let built = build(vec![
            VpkInput {
                path: "sounds/ui/ping.vsnd_c",
                bytes: b"ping",
            },
            VpkInput {
                path: "particles/rain_fx/rain.vpcf_c",
                bytes: b"rain",
            },
        ])
        .expect("vpk");
        let mut listed = list_directory_paths(&built).expect("version 1");
        listed.sort();
        assert_eq!(
            listed,
            vec!["particles/rain_fx/rain.vpcf_c", "sounds/ui/ping.vsnd_c"]
        );
        // Same tree behind Valve's 28-byte version 2 header.
        let tree_size = u32::from_le_bytes(built[8..12].try_into().unwrap()) as usize;
        let mut v2 = Vec::new();
        v2.extend_from_slice(&built[0..4]);
        v2.extend_from_slice(&2u32.to_le_bytes());
        v2.extend_from_slice(&built[8..12]);
        v2.extend_from_slice(&[0u8; 16]);
        v2.extend_from_slice(&built[12..12 + tree_size]);
        let mut listed = list_directory_paths(&v2).expect("version 2");
        listed.sort();
        assert_eq!(listed.len(), 2);
        assert!(list_directory_paths(b"not a vpk at all").is_err());
    }

    fn fixture() -> Vec<VpkInput<'static>> {
        vec![
            VpkInput {
                path: "materials/tree_topiary.vmat_c",
                bytes: b"compiled-material",
            },
            VpkInput {
                path: "models/props_tree/tree_oak_01.vmdl_c",
                bytes: b"compiled-model",
            },
            VpkInput {
                path: "betterfy_manifest.txt",
                bytes: b"schema=1\nmod=tree-mod\n",
            },
        ]
    }

    #[test]
    fn zero_length_entries_round_trip() {
        let inputs = || {
            vec![
                VpkInput {
                    path: "sounds/physics/footsteps/common/wade5.vsnd_c",
                    bytes: b"",
                },
                VpkInput {
                    path: "materials/water/water_generic_000.vmat_c",
                    bytes: b"compiled-material",
                },
            ]
        };
        let first = build(inputs()).expect("build with an empty entry");
        assert_eq!(first, build(inputs()).expect("deterministic"));
        let resources = extract_embedded(&first).expect("reopen");
        assert_eq!(
            resources["sounds/physics/footsteps/common/wade5.vsnd_c"],
            Vec::<u8>::new()
        );
        assert_eq!(inspect(&first).expect("inspect").entries, 2);
    }

    #[test]
    fn builds_a_deterministic_embedded_vpk_and_reopens_it() {
        let first = build(fixture()).expect("build");
        let second = build(fixture()).expect("build again");
        assert_eq!(first, second);
        let report = inspect(&first).expect("inspect");
        assert_eq!(report.entries, 3);
        assert_eq!(report.payload_bytes, 53);
        let resources = extract_embedded(&first).expect("extract");
        assert_eq!(
            resources["materials/tree_topiary.vmat_c"],
            b"compiled-material"
        );
        assert_eq!(
            resources["betterfy_manifest.txt"],
            b"schema=1\nmod=tree-mod\n"
        );
    }

    #[test]
    fn rejects_traversal_collisions_and_bad_crc() {
        assert_eq!(
            build(vec![VpkInput {
                path: "../escape.vtex_c",
                bytes: b"x",
            }])
            .err()
            .as_deref(),
            Some("vpk_path_invalid")
        );
        assert_eq!(
            build(vec![
                VpkInput {
                    path: "materials/a.vtex_c",
                    bytes: b"a",
                },
                VpkInput {
                    path: "materials/a.vtex_c",
                    bytes: b"b",
                },
            ])
            .err()
            .as_deref(),
            Some("vpk_path_collision")
        );
        let mut bytes = build(fixture()).expect("build");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        assert_eq!(inspect(&bytes).err().as_deref(), Some("vpk_crc_mismatch"));
    }

    #[test]
    fn rejects_external_archive_entries() {
        let mut bytes = build(fixture()).expect("build");
        let archive_index = bytes
            .windows(2)
            .position(|window| window == DIRECTORY_ARCHIVE_INDEX.to_le_bytes())
            .expect("archive index");
        bytes[archive_index..archive_index + 2].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            inspect(&bytes).err().as_deref(),
            Some("vpk_archive_unsupported")
        );
    }
}
