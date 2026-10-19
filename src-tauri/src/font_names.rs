//! Renaming an OpenType or TrueType font, for Minify's "Custom Fonts".
//!
//! Dota's Panorama picks its fonts by family name (Radiance, RadianceM,
//! Reaver) and weight. Minify replaces each of the game's font files with a
//! copy of the player's font whose family, full and PostScript names are
//! those of the file it replaces (`REPLACE_MAP` of its `script.py`), and
//! clears the embedding restriction. BetterFy does the same, but rebuilds the
//! font properly: the `name` table is written anew (every Windows and Mac
//! record of names 1, 4, 6 and 16 replaced, missing Windows ones added,
//! records sorted), every table is laid out again on a four-byte boundary,
//! and the table checksums and `head.checkSumAdjustment` are recomputed.
//! Font collections and compressed web fonts are refused.

const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;
const MAX_TABLES: usize = 64;

/// The game's font files and the names each must carry: file, family, full
/// name, PostScript name. From Minify's `Custom Fonts/script.py`.
pub(crate) const REPLACE_MAP: [(&str, &str, &str, &str); 29] = [
    (
        "radiance-black.otf",
        "Radiance Black",
        "Radiance Black",
        "Radiance-Black",
    ),
    (
        "radiance-blackitalic.otf",
        "Radiance Black",
        "Radiance Black Italic",
        "Radiance-BlackItalic",
    ),
    (
        "radiance-bold.otf",
        "Radiance",
        "Radiance Bold",
        "Radiance-Bold",
    ),
    (
        "radiance-bolditalic.otf",
        "Radiance",
        "Radiance Bold Italic",
        "Radiance-BoldItalic",
    ),
    (
        "radiance-light.otf",
        "Radiance",
        "Radiance Light",
        "Radiance-Light",
    ),
    (
        "radiance-lightitalic.otf",
        "Radiance",
        "Radiance Light Italic",
        "Radiance-LightItalic",
    ),
    (
        "radiance-regular.otf",
        "Radiance",
        "Radiance Regular",
        "Radiance-Regular",
    ),
    (
        "radiance-regularitalic.otf",
        "Radiance",
        "Radiance Regular Italic",
        "Radiance-RegularItalic",
    ),
    (
        "radiance-semibold.otf",
        "Radiance Semibold",
        "Radiance Semibold",
        "Radiance-Semibold",
    ),
    (
        "radiance-semibolditalic.otf",
        "Radiance Semibold",
        "Radiance Semibold Italic",
        "Radiance-SemiboldItalic",
    ),
    (
        "radiance-ultralight.otf",
        "Radiance UltraLight",
        "Radiance UltraLight",
        "Radiance-UltraLight",
    ),
    (
        "radiance-ultralightitalic.otf",
        "Radiance",
        "Radiance UltraLight Italic",
        "Radiance-UltraLightItalic",
    ),
    (
        "radiancem-black.otf",
        "RadianceM Black",
        "RadianceM Black",
        "RadianceM-Black",
    ),
    (
        "radiancem-blackitalic.otf",
        "RadianceM Black",
        "RadianceM Black Italic",
        "RadianceM-BlackItalic",
    ),
    (
        "radiancem-bold.otf",
        "RadianceM",
        "RadianceM Bold",
        "RadianceM-Bold",
    ),
    (
        "radiancem-bolditalic.otf",
        "RadianceM",
        "RadianceM Bold Italic",
        "RadianceM-BoldItalic",
    ),
    (
        "radiancem-italic.otf",
        "RadianceM",
        "RadianceM Italic",
        "RadianceM-Italic",
    ),
    (
        "radiancem-light.otf",
        "RadianceM",
        "RadianceM Light",
        "RadianceM-Light",
    ),
    (
        "radiancem-lightitalic.otf",
        "RadianceM",
        "RadianceM Light Italic",
        "RadianceM-LightItalic",
    ),
    (
        "radiancem-regular.otf",
        "RadianceM",
        "RadianceM Regular",
        "RadianceM-Regular",
    ),
    (
        "radiancem-regularitalic.otf",
        "RadianceM",
        "RadianceM Regular Italic",
        "RadianceM-RegularItalic",
    ),
    (
        "radiancem-semibold.otf",
        "RadianceM Semibold",
        "RadianceM Semibold",
        "RadianceM-Semibold",
    ),
    (
        "radiancem-semibolditalic.otf",
        "RadianceM Semibold",
        "RadianceM Semibold Italic",
        "RadianceM-SemiboldItalic",
    ),
    (
        "radiancem-ultralight.otf",
        "RadianceM UltraLight",
        "RadianceM UltraLight",
        "RadianceM-UltraLight",
    ),
    (
        "reaver-black.otf",
        "Reaver Black",
        "Reaver Black",
        "Reaver-Black",
    ),
    ("reaver-bold.otf", "Reaver", "Reaver Bold", "Reaver-Bold"),
    (
        "reaver-light.otf",
        "Reaver Light",
        "Reaver Light",
        "Reaver-Light",
    ),
    (
        "reaver-regular.otf",
        "Reaver",
        "Reaver Regular",
        "Reaver-Regular",
    ),
    (
        "reaver-semibold.otf",
        "Reaver SemiBold",
        "Reaver SemiBold",
        "Reaver-SemiBold",
    ),
];

fn invalid() -> String {
    "custom_font_invalid".to_string()
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes
        .get(at..at + 2)
        .map(|slice| u16::from_be_bytes([slice[0], slice[1]]))
        .ok_or_else(invalid)
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .map(|slice| u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
        .ok_or_else(invalid)
}

struct Table {
    tag: [u8; 4],
    data: Vec<u8>,
}

fn read_tables(bytes: &[u8]) -> Result<(u32, Vec<Table>), String> {
    if bytes.len() < 12 || bytes.len() > MAX_FONT_BYTES {
        return Err(invalid());
    }
    let version = u32_at(bytes, 0)?;
    if !matches!(version, 0x0001_0000 | 0x4F54_544F | 0x7472_7565) {
        return Err(invalid());
    }
    let count = u16_at(bytes, 4)? as usize;
    if count == 0 || count > MAX_TABLES {
        return Err(invalid());
    }
    let mut tables = Vec::with_capacity(count);
    let mut tags = std::collections::BTreeSet::new();
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = bytes
            .get(record..record + 4)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_| invalid())?;
        let offset = u32_at(bytes, record + 8)? as usize;
        let length = u32_at(bytes, record + 12)? as usize;
        let data = bytes
            .get(offset..offset.checked_add(length).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        if !tags.insert(tag) {
            return Err(invalid());
        }
        tables.push(Table {
            tag,
            data: data.to_vec(),
        });
    }
    for required in [b"name", b"head", b"cmap"] {
        if !tags.contains(required) {
            return Err(invalid());
        }
    }
    Ok((version, tables))
}

/// A name record: platform, encoding, language, name ID and its bytes.
type NameRecord = (u16, u16, u16, u16, Vec<u8>);

fn read_names(name: &[u8]) -> Result<Vec<NameRecord>, String> {
    let format = u16_at(name, 0)?;
    if format > 1 {
        return Err(invalid());
    }
    let count = u16_at(name, 2)? as usize;
    let storage = u16_at(name, 4)? as usize;
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let at = 6 + index * 12;
        let platform = u16_at(name, at)?;
        let encoding = u16_at(name, at + 2)?;
        let language = u16_at(name, at + 4)?;
        let id = u16_at(name, at + 6)?;
        let length = u16_at(name, at + 8)? as usize;
        let offset = u16_at(name, at + 10)? as usize;
        // Format 1 language-tag records are dropped with the tags.
        if format == 1 && language >= 0x8000 {
            continue;
        }
        let start = storage + offset;
        let text = name.get(start..start + length).ok_or_else(invalid)?;
        records.push((platform, encoding, language, id, text.to_vec()));
    }
    Ok(records)
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

/// The family name of a font (name 16 if present, else 1), for display.
pub(crate) fn family_name(bytes: &[u8]) -> Result<String, String> {
    let (_, tables) = read_tables(bytes)?;
    let name = tables
        .iter()
        .find(|table| &table.tag == b"name")
        .ok_or_else(invalid)?;
    let records = read_names(&name.data)?;
    let windows = |id: u16| {
        records
            .iter()
            .find(|record| record.0 == 3 && record.3 == id)
            .and_then(|record| {
                let units = record
                    .4
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                    .collect::<Vec<_>>();
                String::from_utf16(&units).ok()
            })
    };
    windows(16)
        .or_else(|| windows(1))
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(invalid)
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// The font with family, full and PostScript names replaced and embedding
/// unrestricted, rebuilt as a valid font file.
pub(crate) fn rename(
    bytes: &[u8],
    family: &str,
    full: &str,
    postscript: &str,
) -> Result<Vec<u8>, String> {
    let (version, mut tables) = read_tables(bytes)?;
    let name_index = tables
        .iter()
        .position(|table| &table.tag == b"name")
        .ok_or_else(invalid)?;
    let mut records = read_names(&tables[name_index].data)?;
    let replacement = |id: u16| match id {
        1 | 16 => Some(family),
        4 => Some(full),
        6 => Some(postscript),
        _ => None,
    };
    for record in &mut records {
        if let Some(text) = replacement(record.3) {
            match (record.0, record.1) {
                (3, _) | (0, _) => record.4 = utf16(text),
                (1, 0) => record.4 = text.as_bytes().to_vec(),
                _ => {}
            }
        }
    }
    for id in [1u16, 4, 6] {
        if !records.iter().any(|record| record.0 == 3 && record.3 == id) {
            records.push((3, 1, 0x0409, id, utf16(replacement(id).expect("named"))));
        }
    }
    records.sort_by_key(|record| (record.0, record.1, record.2, record.3));
    let mut storage = Vec::new();
    let mut table = Vec::new();
    table.extend_from_slice(&0u16.to_be_bytes());
    table.extend_from_slice(&(records.len() as u16).to_be_bytes());
    table.extend_from_slice(&((6 + records.len() * 12) as u16).to_be_bytes());
    for (platform, encoding, language, id, text) in &records {
        if storage.len() + text.len() > u16::MAX as usize || text.len() > u16::MAX as usize {
            return Err(invalid());
        }
        for value in [
            *platform,
            *encoding,
            *language,
            *id,
            text.len() as u16,
            storage.len() as u16,
        ] {
            table.extend_from_slice(&value.to_be_bytes());
        }
        storage.extend_from_slice(text);
    }
    table.extend_from_slice(&storage);
    tables[name_index].data = table;
    // Embedding: installable (fsType 0), as Minify sets it.
    if let Some(os2) = tables.iter_mut().find(|table| &table.tag == b"OS/2") {
        if os2.data.len() >= 10 {
            os2.data[8..10].copy_from_slice(&0u16.to_be_bytes());
        }
    }
    let head_index = tables
        .iter()
        .position(|table| &table.tag == b"head")
        .ok_or_else(invalid)?;
    if tables[head_index].data.len() < 12 {
        return Err(invalid());
    }
    tables[head_index].data[8..12].copy_from_slice(&0u32.to_be_bytes());
    tables.sort_by_key(|table| table.tag);

    let count = tables.len();
    let mut power = 1usize;
    let mut selector = 0u16;
    while power * 2 <= count {
        power *= 2;
        selector += 1;
    }
    let search_range = (power * 16) as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&(count as u16).to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&selector.to_be_bytes());
    out.extend_from_slice(&((count * 16) as u16 - search_range).to_be_bytes());
    let directory = out.len();
    out.resize(directory + count * 16, 0);
    let mut head_offset = 0;
    for (index, table) in tables.iter().enumerate() {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        let offset = out.len();
        if &table.tag == b"head" {
            head_offset = offset;
        }
        out.extend_from_slice(&table.data);
        let record = directory + index * 16;
        out[record..record + 4].copy_from_slice(&table.tag);
        out[record + 4..record + 8].copy_from_slice(&checksum(&table.data).to_be_bytes());
        out[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        out[record + 12..record + 16].copy_from_slice(&(table.data.len() as u32).to_be_bytes());
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    if out.len() > MAX_FONT_BYTES {
        return Err(invalid());
    }
    Ok(out)
}

/// A minimal font for tests: `head`, `cmap`, `OS/2` and a `name` table
/// naming it "Original".
#[cfg(test)]
pub(crate) fn test_font() -> Vec<u8> {
    tests::font()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn font() -> Vec<u8> {
        let mut name = Vec::new();
        let records: [(u16, u16, u16, u16, Vec<u8>); 3] = [
            (1, 0, 0, 1, b"Original".to_vec()),
            (3, 1, 0x409, 1, utf16("Original")),
            (3, 1, 0x409, 2, utf16("Regular")),
        ];
        name.extend_from_slice(&0u16.to_be_bytes());
        name.extend_from_slice(&3u16.to_be_bytes());
        name.extend_from_slice(&(6u16 + 36).to_be_bytes());
        let mut storage = Vec::new();
        for (p, e, l, id, text) in &records {
            for value in [*p, *e, *l, *id, text.len() as u16, storage.len() as u16] {
                name.extend_from_slice(&value.to_be_bytes());
            }
            storage.extend_from_slice(text);
        }
        name.extend_from_slice(&storage);
        let mut head = vec![0u8; 54];
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        let mut os2 = vec![0u8; 78];
        os2[8..10].copy_from_slice(&2u16.to_be_bytes());
        let cmap = vec![0u8; 4];
        let tables: [(&[u8; 4], Vec<u8>); 4] = [
            (b"OS/2", os2),
            (b"cmap", cmap),
            (b"head", head),
            (b"name", name),
        ];
        let mut out = Vec::new();
        out.extend_from_slice(&0x4F54_544Fu32.to_be_bytes());
        out.extend_from_slice(&4u16.to_be_bytes());
        out.extend_from_slice(&[0, 64, 0, 2, 0, 0]);
        let directory = out.len();
        out.resize(directory + 64, 0);
        for (index, (tag, data)) in tables.iter().enumerate() {
            while out.len() % 4 != 0 {
                out.push(0);
            }
            let offset = out.len() as u32;
            out.extend_from_slice(data);
            let record = directory + index * 16;
            out[record..record + 4].copy_from_slice(*tag);
            out[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
            out[record + 12..record + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        }
        out
    }

    #[test]
    fn a_renamed_font_carries_the_new_names_and_valid_checksums() {
        let original = font();
        assert_eq!(family_name(&original).expect("family"), "Original");
        let renamed =
            rename(&original, "Radiance", "Radiance Bold", "Radiance-Bold").expect("renames");
        assert_eq!(family_name(&renamed).expect("family"), "Radiance");
        let (_, tables) = read_tables(&renamed).expect("reads back");
        let name = tables.iter().find(|table| &table.tag == b"name").unwrap();
        let records = read_names(&name.data).unwrap();
        let mac = records.iter().find(|r| r.0 == 1 && r.3 == 1).unwrap();
        assert_eq!(mac.4, b"Radiance");
        for (id, text) in [
            (4u16, "Radiance Bold"),
            (6, "Radiance-Bold"),
            (2, "Regular"),
        ] {
            let record = records.iter().find(|r| r.0 == 3 && r.3 == id).unwrap();
            assert_eq!(record.4, utf16(text), "name {id}");
        }
        let os2 = tables.iter().find(|table| &table.tag == b"OS/2").unwrap();
        assert_eq!(&os2.data[8..10], &[0, 0]);
        // Whole-file checksum with the adjustment is the magic number.
        assert_eq!(checksum(&renamed), 0xB1B0_AFBA);
        // Each table's recorded checksum matches its data.
        let count = u16_at(&renamed, 4).unwrap() as usize;
        for index in 0..count {
            let record = 12 + index * 16;
            let offset = u32_at(&renamed, record + 8).unwrap() as usize;
            let length = u32_at(&renamed, record + 12).unwrap() as usize;
            let mut data = renamed[offset..offset + length].to_vec();
            if &renamed[record..record + 4] == b"head" {
                data[8..12].copy_from_slice(&[0; 4]);
            }
            assert_eq!(u32_at(&renamed, record + 4).unwrap(), checksum(&data));
            assert_eq!(offset % 4, 0);
        }
        // Deterministic.
        assert_eq!(
            renamed,
            rename(&original, "Radiance", "Radiance Bold", "Radiance-Bold").unwrap()
        );
    }

    #[test]
    fn collections_web_fonts_and_damaged_fonts_are_refused() {
        let mut collection = font();
        collection[..4].copy_from_slice(b"ttcf");
        assert!(rename(&collection, "a", "b", "c").is_err());
        let mut web = font();
        web[..4].copy_from_slice(b"wOFF");
        assert!(rename(&web, "a", "b", "c").is_err());
        let mut damaged = font();
        let at = 12 + 3 * 16 + 12;
        damaged[at..at + 4].copy_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        assert!(rename(&damaged, "a", "b", "c").is_err());
        assert!(rename(&font()[..30], "a", "b", "c").is_err());
    }

    #[test]
    fn the_map_covers_every_radiance_and_reaver_weight_once() {
        let names = REPLACE_MAP
            .iter()
            .map(|(file, ..)| *file)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), 29);
        for (file, ..) in REPLACE_MAP {
            assert!(crate::game_files::validate_path(&format!("panorama/fonts/{file}")).is_ok());
        }
    }
}
