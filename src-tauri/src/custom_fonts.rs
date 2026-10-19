//! Minify's "Custom Fonts": the player's own font in place of Dota's
//! Radiance, RadianceM and Reaver.
//!
//! The player picks a `.ttf` or `.otf` file. BetterFy checks it, keeps it in
//! the content store and names it in the package ID
//! (`minify.custom-fonts.font-<sha256>-<bytes>`), so a build is bound to that
//! exact file and restore works like for any package. On install, every one of
//! the game's 29 Radiance/RadianceM/Reaver files is replaced by a copy of the
//! font renamed for it (`font_names.rs`, Minify's `REPLACE_MAP`), through the
//! `game_files` transaction, which backs up the game's fonts and puts them
//! back on restore.

use crate::content_store;
use crate::font_names;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const PACKAGE_ID: &str = "minify.custom-fonts";
pub(crate) const CATALOG_ID: &str = "minify-custom-fonts";
const VARIANT_INFIX: &str = ".font-";
/// 29 copies of the font are written; a large font would take a lot of room.
pub(crate) const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomFontInfo {
    /// The package ID that installs this font.
    pub package_id: String,
    /// The font's own family name, for the interface.
    pub family: String,
    pub bytes: usize,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn variant_id(sha256: &str, bytes: usize) -> String {
    format!("{PACKAGE_ID}{VARIANT_INFIX}{sha256}-{bytes}")
}

/// `minify.custom-fonts.font-<sha256>-<bytes>`: the font file it names.
pub(crate) fn parse_variant(id: &str) -> Option<(String, usize)> {
    let rest = id.strip_prefix(PACKAGE_ID)?.strip_prefix(VARIANT_INFIX)?;
    let (sha, bytes) = rest.split_once('-')?;
    let valid_sha = sha.len() == 64
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    let bytes = bytes
        .parse::<usize>()
        .ok()
        .filter(|size| *size > 0 && *size <= MAX_SOURCE_BYTES)?;
    (valid_sha && !rest.contains('+') && bytes.to_string() == rest[65..])
        .then(|| (sha.to_string(), bytes))
}

/// Checks a font the player picked and keeps it. The font must be a single
/// OpenType or TrueType font that can be renamed for every replaced file.
pub(crate) fn store(app_data_root: &Path, font_base64: &str) -> Result<CustomFontInfo, String> {
    if font_base64.len() > MAX_SOURCE_BYTES * 4 / 3 + 8 {
        return Err("custom_font_too_large".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(font_base64)
        .map_err(|_| "custom_font_invalid".to_string())?;
    if bytes.is_empty() {
        return Err("custom_font_invalid".to_string());
    }
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("custom_font_too_large".to_string());
    }
    let family = font_names::family_name(&bytes)?;
    for (_, family, full, postscript) in font_names::REPLACE_MAP {
        font_names::rename(&bytes, family, full, postscript)?;
    }
    let sha = sha256(&bytes);
    content_store::store_pinned_resource(app_data_root, bytes.len(), &sha, &bytes)?;
    Ok(CustomFontInfo {
        package_id: variant_id(&sha, bytes.len()),
        family,
        bytes: bytes.len(),
    })
}

/// The 29 renamed copies, as paths below `game/dota`.
pub(crate) fn load(
    app_data_root: &Path,
    sha256: &str,
    bytes: usize,
) -> Result<BTreeMap<String, Option<Vec<u8>>>, String> {
    let source = content_store::read_pinned_resource(app_data_root, bytes, sha256)
        .map_err(|_| "custom_font_missing".to_string())?;
    font_names::REPLACE_MAP
        .iter()
        .map(|(file, family, full, postscript)| {
            font_names::rename(&source, family, full, postscript)
                .map(|renamed| (format!("panorama/fonts/{file}"), Some(renamed)))
        })
        .collect()
}

/// The text the font puts into the build's VPK, binding the plan to it.
pub(crate) fn marker(sha256: &str, bytes: usize) -> (String, Vec<u8>) {
    (
        format!("betterfy/{CATALOG_ID}.txt"),
        format!("{PACKAGE_ID}\nsource {bytes} {sha256}\n").into_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_lists_the_package() {
        let index: serde_json::Value =
            serde_json::from_str(include_str!("../extras/index.json")).expect("index");
        assert!(index
            .as_array()
            .expect("array")
            .iter()
            .any(|entry| entry["id"] == PACKAGE_ID && entry["catalogId"] == CATALOG_ID));
    }

    #[test]
    fn variant_ids_name_exactly_one_file() {
        let sha = "a".repeat(64);
        let id = variant_id(&sha, 1234);
        assert_eq!(id, format!("minify.custom-fonts.font-{sha}-1234"));
        assert_eq!(parse_variant(&id), Some((sha.clone(), 1234)));
        for bad in [
            "minify.custom-fonts".to_string(),
            format!("minify.custom-fonts.font-{}-1234", "a".repeat(63)),
            format!("minify.custom-fonts.font-{}-1234", "A".repeat(64)),
            format!("minify.custom-fonts.font-{sha}-0"),
            format!("minify.custom-fonts.font-{sha}-01234"),
            format!("minify.custom-fonts.font-{sha}-+1234"),
            format!("minify.custom-fonts.font-{sha}-{}", MAX_SOURCE_BYTES + 1),
            format!("minify.tree-mod.font-{sha}-1234"),
        ] {
            assert_eq!(parse_variant(&bad), None, "{bad}");
        }
        assert!(id.len() <= 128, "fits the package ID bound");
    }

    #[test]
    fn a_picked_file_that_is_not_a_font_is_refused() {
        let root =
            std::env::temp_dir().join(format!("betterfy-custom-font-{}", std::process::id()));
        let not_a_font = base64::engine::general_purpose::STANDARD.encode(b"hello, world");
        assert_eq!(
            store(&root, &not_a_font).err().as_deref(),
            Some("custom_font_invalid")
        );
        assert_eq!(
            store(&root, "@@@").err().as_deref(),
            Some("custom_font_invalid")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
