use crc32fast::Hasher as Crc32;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const VPK_SIGNATURE: u32 = 0x55aa_1234;
const VPK_VERSION: u32 = 1;
const DIRECTORY_ARCHIVE_INDEX: u16 = 0x7fff;
const ENTRY_TERMINATOR: u16 = 0xffff;
const MAX_VPK_BYTES: usize = 256 * 1024 * 1024;
const MAX_VPK_ENTRIES: usize = 4096;
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

fn validate_path(value: &str) -> Result<(), String> {
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
    if input.bytes.is_empty() || input.bytes.len() > MAX_VPK_BYTES {
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

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "enabled with the pinned Tree Mod package intake")
)]
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

pub fn inspect(bytes: &[u8]) -> Result<VpkReport, String> {
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
    let mut entries = 0usize;
    let mut payload_bytes = 0u64;
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
                payload_bytes = payload_bytes
                    .checked_add((preload_length + length) as u64)
                    .ok_or_else(|| "vpk_invalid".to_string())?;
            }
        }
    }
    if cursor != tree_end || entries == 0 {
        return Err("vpk_invalid".to_string());
    }
    Ok(VpkReport {
        version,
        entries,
        payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn builds_a_deterministic_embedded_vpk_and_reopens_it() {
        let first = build(fixture()).expect("build");
        let second = build(fixture()).expect("build again");
        assert_eq!(first, second);
        let report = inspect(&first).expect("inspect");
        assert_eq!(report.entries, 3);
        assert_eq!(report.payload_bytes, 53);
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
