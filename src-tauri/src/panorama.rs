//! Compiled Panorama style resources (`.vcss_c`) without Valve's compiler.
//!
//! A Source 2 resource is a 16-byte header (file size, header version 12,
//! resource version, block table offset relative to its own field, block
//! count) followed by a table of `tag, relative offset, size` entries and the
//! 16-byte-aligned blocks. In a Panorama style the `DATA` block holds a CRC,
//! an image table and then the plain CSS text.
//!
//! Minify's `styling.css` extends a game style with extra rules. BetterFy does
//! the same in place: the game's own resource is read, the extra CSS is
//! appended to the text in `DATA`, and every other block (dependencies, source
//! map, image table, CRC) is kept byte for byte. Appending keeps every source
//! map offset valid because only the tail changes. New `url(...)` images are
//! not supported: their reference IDs are produced by Valve's compiler.

use std::collections::BTreeMap;

const HEADER_VERSION: u16 = 12;
const MAX_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_BLOCKS: usize = 32;
const MAX_STYLE_ADDITION_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    pub tag: [u8; 4],
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resource {
    pub version: u16,
    pub blocks: Vec<Block>,
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
        .ok_or_else(|| "panorama_resource_invalid".to_string())
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
        .ok_or_else(|| "panorama_resource_invalid".to_string())
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Resource, String> {
    let invalid = || "panorama_resource_invalid".to_string();
    if bytes.len() < 16 || bytes.len() > MAX_RESOURCE_BYTES {
        return Err(invalid());
    }
    if u32_at(bytes, 0)? as usize != bytes.len() || u16_at(bytes, 4)? != HEADER_VERSION {
        return Err(invalid());
    }
    let version = u16_at(bytes, 6)?;
    let table = 8usize
        .checked_add(u32_at(bytes, 8)? as usize)
        .ok_or_else(invalid)?;
    let count = u32_at(bytes, 12)? as usize;
    if count == 0 || count > MAX_BLOCKS {
        return Err(invalid());
    }
    let mut blocks = Vec::with_capacity(count);
    for index in 0..count {
        let entry = table.checked_add(12 * index).ok_or_else(invalid)?;
        let tag: [u8; 4] = bytes
            .get(entry..entry + 4)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_| invalid())?;
        let start = (entry + 4)
            .checked_add(u32_at(bytes, entry + 4)? as usize)
            .ok_or_else(invalid)?;
        let size = u32_at(bytes, entry + 8)? as usize;
        let end = start.checked_add(size).ok_or_else(invalid)?;
        let block = bytes.get(start..end).ok_or_else(invalid)?;
        blocks.push(Block {
            tag,
            bytes: block.to_vec(),
        });
    }
    Ok(Resource { version, blocks })
}

/// Lays the blocks out after the table, each aligned to 16 bytes.
pub(crate) fn build(resource: &Resource) -> Result<Vec<u8>, String> {
    if resource.blocks.is_empty() || resource.blocks.len() > MAX_BLOCKS {
        return Err("panorama_resource_invalid".to_string());
    }
    let count = resource.blocks.len();
    let mut output = vec![0u8; 16 + 12 * count];
    let mut placed = Vec::with_capacity(count);
    for block in &resource.blocks {
        output.resize(output.len().next_multiple_of(16), 0);
        placed.push((output.len(), block.bytes.len()));
        output.extend_from_slice(&block.bytes);
    }
    if output.len() > MAX_RESOURCE_BYTES {
        return Err("panorama_resource_invalid".to_string());
    }
    let total = output.len() as u32;
    output[0..4].copy_from_slice(&total.to_le_bytes());
    output[4..6].copy_from_slice(&HEADER_VERSION.to_le_bytes());
    output[6..8].copy_from_slice(&resource.version.to_le_bytes());
    output[8..12].copy_from_slice(&8u32.to_le_bytes());
    output[12..16].copy_from_slice(&(count as u32).to_le_bytes());
    for (index, (block, (offset, size))) in resource.blocks.iter().zip(placed).enumerate() {
        let entry = 16 + 12 * index;
        output[entry..entry + 4].copy_from_slice(&block.tag);
        let relative = (offset - (entry + 4)) as u32;
        output[entry + 4..entry + 8].copy_from_slice(&relative.to_le_bytes());
        output[entry + 8..entry + 12].copy_from_slice(&(size as u32).to_le_bytes());
    }
    Ok(output)
}

/// Offset of the CSS text inside a Panorama `DATA` block: after the CRC, the
/// image count and the image table (names, sizes and, from resource version
/// 3, a CRC per image).
fn style_text_offset(data: &[u8], version: u16) -> Result<usize, String> {
    let invalid = || "panorama_resource_invalid".to_string();
    let images = u16_at(data, 4)? as usize;
    let mut cursor = 6usize;
    for _ in 0..images {
        let name_end = data
            .get(cursor..)
            .and_then(|rest| rest.iter().position(|byte| *byte == 0))
            .ok_or_else(invalid)?;
        cursor += name_end + 1 + 4;
        if version >= 3 {
            cursor += 4;
        }
        if cursor > data.len() {
            return Err(invalid());
        }
    }
    Ok(cursor)
}

/// Returns the CSS text of a compiled style.
pub(crate) fn style_text(bytes: &[u8]) -> Result<String, String> {
    let resource = parse(bytes)?;
    let data = resource
        .blocks
        .iter()
        .find(|block| &block.tag == b"DATA")
        .ok_or_else(|| "panorama_resource_invalid".to_string())?;
    let offset = style_text_offset(&data.bytes, resource.version)?;
    String::from_utf8(data.bytes[offset..].to_vec())
        .map_err(|_| "panorama_resource_invalid".to_string())
}

/// Appends `addition` to the CSS of the compiled style `bytes`.
pub(crate) fn append_style(bytes: &[u8], addition: &str) -> Result<Vec<u8>, String> {
    if addition.len() > MAX_STYLE_ADDITION_BYTES || addition.contains('\0') {
        return Err("panorama_style_invalid".to_string());
    }
    let mut resource = parse(bytes)?;
    let version = resource.version;
    let data = resource
        .blocks
        .iter_mut()
        .find(|block| &block.tag == b"DATA")
        .ok_or_else(|| "panorama_resource_invalid".to_string())?;
    let offset = style_text_offset(&data.bytes, version)?;
    std::str::from_utf8(&data.bytes[offset..])
        .map_err(|_| "panorama_resource_invalid".to_string())?;
    if data.bytes.len() > offset && data.bytes.last() != Some(&b'\n') {
        data.bytes.push(b'\n');
    }
    data.bytes.extend_from_slice(addition.trim().as_bytes());
    data.bytes.push(b'\n');
    build(&resource)
}

/// Splits a Minify `styling.css` into its sections: `/* g:panorama/styles/x */`
/// (game) or `/* c:... */` (core) headers, each followed by the CSS for that
/// style. Returns `archive:path.vcss_c` -> CSS.
pub(crate) fn split_styling(text: &str) -> Result<BTreeMap<String, String>, String> {
    let invalid = || "panorama_styling_invalid".to_string();
    let mut sections: BTreeMap<String, String> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        let header = trimmed
            .strip_prefix("/*")
            .and_then(|rest| rest.strip_suffix("*/"))
            .map(str::trim)
            .and_then(|inner| {
                inner
                    .strip_prefix("g:")
                    .map(|path| ("game", path))
                    .or_else(|| inner.strip_prefix("c:").map(|path| ("core", path)))
            });
        if let Some((archive, path)) = header {
            let path = path.trim().trim_end_matches(".css").to_ascii_lowercase();
            if !path.starts_with("panorama/")
                || path.split('/').any(|part| part.is_empty() || part == "..")
                || !path.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'/' | b'_' | b'-')
                })
            {
                return Err(invalid());
            }
            let key = format!("{archive}:{path}.vcss_c");
            sections.entry(key.clone()).or_default();
            current = Some(key);
            continue;
        }
        match &current {
            Some(key) => {
                let section = sections.get_mut(key).ok_or_else(invalid)?;
                section.push_str(line);
                section.push('\n');
            }
            None if trimmed.is_empty() => {}
            None => return Err(invalid()),
        }
    }
    sections.retain(|_, css| !css.trim().is_empty());
    if sections.is_empty() {
        return Err(invalid());
    }
    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(css: &str, images: &[(&str, u16, u16)]) -> Vec<u8> {
        let mut data = 0x1234_5678u32.to_le_bytes().to_vec();
        data.extend_from_slice(&(images.len() as u16).to_le_bytes());
        for (name, width, height) in images {
            data.extend_from_slice(name.as_bytes());
            data.push(0);
            data.extend_from_slice(&width.to_le_bytes());
            data.extend_from_slice(&height.to_le_bytes());
            data.extend_from_slice(&7u32.to_le_bytes());
        }
        data.extend_from_slice(css.as_bytes());
        build(&Resource {
            version: 3,
            blocks: vec![
                Block {
                    tag: *b"RED2",
                    bytes: vec![1, 2, 3, 4, 5],
                },
                Block {
                    tag: *b"DATA",
                    bytes: data,
                },
                Block {
                    tag: *b"SrMa",
                    bytes: vec![9; 21],
                },
            ],
        })
        .expect("resource")
    }

    #[test]
    fn round_trips_and_aligns_blocks() {
        let bytes = style(".A{x: 1px;}", &[("panorama/images/a_png.vtex", 32, 16)]);
        assert_eq!(
            bytes.len() as u32,
            u32::from_le_bytes(bytes[0..4].try_into().unwrap())
        );
        let parsed = parse(&bytes).expect("parses");
        assert_eq!(build(&parsed).expect("rebuilds"), bytes);
        assert_eq!(style_text(&bytes).expect("text"), ".A{x: 1px;}");
    }

    #[test]
    fn appending_keeps_every_other_block_and_the_image_table() {
        let original = style(".A{x: 1px;}", &[("panorama/images/a_png.vtex", 32, 16)]);
        let patched = append_style(&original, "\n#B { visibility: collapse; }\n").expect("patched");
        let before = parse(&original).unwrap();
        let after = parse(&patched).unwrap();
        assert_eq!(before.version, after.version);
        assert_eq!(before.blocks[0], after.blocks[0]);
        assert_eq!(before.blocks[2], after.blocks[2]);
        assert!(after.blocks[1].bytes.starts_with(&before.blocks[1].bytes));
        assert_eq!(
            style_text(&patched).unwrap(),
            ".A{x: 1px;}\n#B { visibility: collapse; }\n"
        );
    }

    #[test]
    fn rejects_damaged_resources() {
        let mut bytes = style(".A{}", &[]);
        bytes[0] ^= 1;
        assert!(parse(&bytes).is_err());
        assert!(parse(b"short").is_err());
        assert!(append_style(&style(".A{}", &[]), "a\0b").is_err());
    }

    #[test]
    fn splits_minify_styling_sections() {
        let text = "/* g:panorama/styles/chat */\nDOTAChat { y: -145px; }\n\n/* c:panorama/styles/tooltips/tooltip_base */\n#X { visibility: collapse; }\n/* g:panorama/styles/chat */\n#Y {}\n";
        let sections = split_styling(text).expect("splits");
        assert_eq!(sections.len(), 2);
        assert!(sections["game:panorama/styles/chat.vcss_c"].contains("DOTAChat"));
        assert!(sections["game:panorama/styles/chat.vcss_c"].contains("#Y"));
        assert!(sections.contains_key("core:panorama/styles/tooltips/tooltip_base.vcss_c"));
        assert!(split_styling("loose { }").is_err());
        assert!(split_styling("/* g:../escape */\nx{}").is_err());
    }
}
