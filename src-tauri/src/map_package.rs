//! Whole-map VPK analyzer (map terrains).
//!
//! A "standard" terrain of the Dota2PornFx catalog is not a skin: it is a
//! complete Valve map package, `maps/dota.vpk`, that the game reads in place of
//! its own when it sits in a language folder. It is a single VPK (version 1 or
//! 2) whose entries are all embedded, and it is installed byte for byte, never
//! merged into BetterFy's `pakNN_dir.vpk`. The shared-path logic of the skin
//! analyzer does not apply: a map package does not override files of Valve's
//! `pak01_dir.vpk`, it is a separate package with its own `maps/` namespace.
//! What it replaces is the map package itself, and that is a whole-file
//! replacement.
//!
//! This module only reads and judges bytes. It checks that the file is a
//! well-formed VPK (header, directory, CRC of every entry, section sizes that
//! add up to the file length) and that its contents are what a map package
//! carries: lower-case ASCII paths under `maps/`, compiled map data only, no
//! scripts and no executables. The result is a typed [`MapReport`]; the audit
//! pins one per archive and every build must reproduce it.
//!
//! What it cannot do: it does not know what a map entry means to the game.
//! "No script-typed file inside" is a fact about file names. An entity lump
//! (`vents_c`) can name scripts that the game itself ships; that is not looked
//! at here and nothing in this report says the map is harmless in game.

use crate::vpk;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The largest map package BetterFy accepts. The largest audited one is about
/// 51 MiB; the deployment reads the file into memory whole.
pub(crate) const MAX_MAP_BYTES: usize = 128 * 1024 * 1024;
/// Where a map package is installed, relative to the language folder.
pub(crate) const MAP_RELATIVE_PATH: &str = "maps/dota.vpk";
/// The entry every Dota map package carries.
pub(crate) const MAIN_MAP_ENTRY: &str = "maps/dota.vmap_c";
const MAP_ROOT: &str = "maps";
const MAX_PATH_BYTES: usize = 260;
const MAX_ENTRIES: usize = 65_536;
/// The file types the audited map packages carry, all compiled map data.
/// Anything else (scripts, executables, other resources) is refused.
pub(crate) const ALLOWED_EXTENSIONS: [&str; 12] = [
    "gnv", "trm", "vents_c", "vgrass", "vhcg", "vmap_c", "vmdl_c", "vrman_c", "vtex_c", "vvis_c",
    "vwnod_c", "vwrld_c",
];
/// Extensions that are scripts or programs. Listed so the refusal can say
/// which kind of problem it is; everything outside the allowlist is refused
/// either way.
const SCRIPT_EXTENSIONS: [&str; 14] = [
    "vjs_c", "vjs", "js", "lua", "vts_c", "vts", "vpulse_c", "vcss_c", "vxml_c", "vsvg_c", "exe",
    "dll", "bat", "ps1",
];

/// Why a map package was refused: a stable code and, when there is one, the
/// path that caused it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MapRejection {
    pub code: &'static str,
    pub path: Option<String>,
}

impl MapRejection {
    fn new(code: &'static str) -> Self {
        Self { code, path: None }
    }

    fn at(code: &'static str, path: &str) -> Self {
        Self {
            code,
            path: Some(path.to_string()),
        }
    }
}

impl From<MapRejection> for String {
    fn from(value: MapRejection) -> Self {
        value.code.to_string()
    }
}

/// What the analyzer found in a map package.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapReport {
    /// VPK header version (1 or 2).
    pub vpk_version: u32,
    /// Size of the whole file.
    pub total_bytes: u64,
    /// Number of entries in the directory.
    pub entries: usize,
    /// Sum of the entry sizes.
    pub payload_bytes: u64,
    /// Entries with no bytes.
    pub zero_length: usize,
    /// Entry count per file extension.
    pub extensions: BTreeMap<String, usize>,
    /// Entry count per folder directly under `maps/` (`(top)` for files that
    /// sit in `maps/` itself).
    pub subfolders: BTreeMap<String, usize>,
    /// SHA-256 over the sorted (path, payload hash) pairs: the identity of the
    /// contents, independent of how the VPK was laid out.
    pub identity: String,
    /// Whether the package carries `maps/dota.vmap_c`.
    pub main_map: bool,
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at.checked_add(4)?)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
}

/// The VPK extension of a path: what follows the last dot of the file name.
fn extension_of(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    name.rsplit_once('.').map(|(_, extension)| extension)
}

fn path_is_sane(path: &str) -> bool {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || vpk::validate_path(path).is_err() {
        return false;
    }
    let Some((stem_path, _)) = path.rsplit_once('.') else {
        return false;
    };
    // No empty, dot or space-padded segment, and no stem that ends in a space
    // or a dot: such names are ambiguous on Windows even though a VPK entry is
    // never written to disk as a file of its own.
    path.split('/').all(|segment| {
        !segment.is_empty() && segment != "." && segment != ".." && !segment.starts_with(' ')
    }) && !stem_path.ends_with([' ', '.', '/'])
        && stem_path
            .split('/')
            .all(|segment| !segment.ends_with([' ', '.']))
}

/// Reads a map package and judges it. Refuses with the first problem found,
/// in a fixed order, so the same bytes always give the same answer.
pub(crate) fn analyze(bytes: &[u8]) -> Result<MapReport, MapRejection> {
    if bytes.len() < 28 || bytes.len() > MAX_MAP_BYTES {
        return Err(MapRejection::new("map_package_invalid"));
    }
    if read_u32(bytes, 0) != Some(0x55aa_1234) {
        return Err(MapRejection::new("map_package_invalid"));
    }
    let version = read_u32(bytes, 4).ok_or_else(|| MapRejection::new("map_package_invalid"))?;
    let tree_size = read_u32(bytes, 8).ok_or_else(|| MapRejection::new("map_package_invalid"))?;
    // Version 2 adds four section sizes; together with the header and the
    // directory they must account for every byte of the file, so nothing is
    // hidden after the data. Version 1 has only the directory and the data.
    let data_section = match version {
        1 => bytes
            .len()
            .checked_sub(12usize.saturating_add(tree_size as usize))
            .ok_or_else(|| MapRejection::new("map_package_invalid"))?,
        2 => {
            let sizes = [12, 16, 20, 24].map(|at| {
                read_u32(bytes, at).ok_or_else(|| MapRejection::new("map_package_invalid"))
            });
            let [data, archive_md5, other_md5, signature] = sizes;
            let (data, archive_md5, other_md5, signature) =
                (data?, archive_md5?, other_md5?, signature?);
            // An external archive part would live in a file next to this one;
            // a signature section would need a key. Neither is accepted.
            if archive_md5 != 0 || signature != 0 {
                return Err(MapRejection::new("map_package_unsupported"));
            }
            let total = 28u64
                + u64::from(tree_size)
                + u64::from(data)
                + u64::from(archive_md5)
                + u64::from(other_md5)
                + u64::from(signature);
            if total != bytes.len() as u64 {
                return Err(MapRejection::new("map_package_invalid"));
            }
            data as usize
        }
        _ => return Err(MapRejection::new("map_package_unsupported")),
    };
    let directory = vpk::read_embedded_directory(bytes).map_err(|code| {
        if code == "vpk_archive_unsupported" || code == "vpk_version_unsupported" {
            MapRejection::new("map_package_unsupported")
        } else {
            MapRejection::new("map_package_invalid")
        }
    })?;
    if directory.is_empty() || directory.len() > MAX_ENTRIES {
        return Err(MapRejection::new("map_package_invalid"));
    }

    let mut seen = BTreeSet::new();
    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    let mut subfolders: BTreeMap<String, usize> = BTreeMap::new();
    let mut hashed: Vec<(String, [u8; 32])> = Vec::with_capacity(directory.len());
    let mut payload_bytes = 0u64;
    let mut zero_length = 0usize;
    let mut main_map = false;
    for entry in &directory {
        let path = entry.path.as_str();
        if !path_is_sane(path) {
            return Err(MapRejection::at("map_entry_path_invalid", path));
        }
        if path.split('/').next() != Some(MAP_ROOT) || !path.contains('/') {
            return Err(MapRejection::at("map_entry_outside_maps", path));
        }
        if !seen.insert(path.to_string()) {
            return Err(MapRejection::at("map_entry_duplicate", path));
        }
        let extension = extension_of(path).unwrap_or("");
        if SCRIPT_EXTENSIONS.contains(&extension) {
            return Err(MapRejection::at("map_entry_script", path));
        }
        if !ALLOWED_EXTENSIONS.contains(&extension) {
            return Err(MapRejection::at("map_entry_type_blocked", path));
        }
        *extensions.entry(extension.to_string()).or_default() += 1;
        let rest = &path[MAP_ROOT.len() + 1..];
        let folder = rest.split_once('/').map_or("(top)", |(folder, _)| folder);
        *subfolders.entry(folder.to_string()).or_default() += 1;
        if path == MAIN_MAP_ENTRY {
            main_map = true;
        }
        let payload = entry.payload();
        if payload.is_empty() {
            zero_length += 1;
        }
        payload_bytes += payload.len() as u64;
        hashed.push((path.to_string(), Sha256::digest(&payload).into()));
    }
    // Every byte of the data section belongs to an entry: nothing rides along
    // unreferenced.
    if payload_bytes != data_section as u64 {
        return Err(MapRejection::new("map_package_invalid"));
    }
    if !main_map {
        return Err(MapRejection::at("map_main_missing", MAIN_MAP_ENTRY));
    }
    hashed.sort_by(|left, right| left.0.cmp(&right.0));
    let mut identity = Sha256::new();
    for (path, hash) in &hashed {
        identity.update((path.len() as u64).to_le_bytes());
        identity.update(path.as_bytes());
        identity.update(hash);
    }
    Ok(MapReport {
        vpk_version: version,
        total_bytes: bytes.len() as u64,
        entries: directory.len(),
        payload_bytes,
        zero_length,
        extensions,
        subfolders,
        identity: format!("{:x}", identity.finalize()),
        main_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_entries() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("maps/dota.vmap_c", b"map".as_slice()),
            ("maps/dota.vhcg", b"hcg".as_slice()),
            ("maps/dota/world_visibility.vvis_c", b"vis".as_slice()),
            ("maps/dota/models/tree_01.vmdl_c", b"tree".as_slice()),
        ]
    }

    #[test]
    fn a_well_formed_package_is_reported_in_both_header_versions() {
        for version in [1u32, 2] {
            let bytes = vpk::build_unchecked(version, &map_entries());
            let report = analyze(&bytes).expect("valid map package");
            assert_eq!(report.vpk_version, version);
            assert_eq!(report.entries, 4);
            assert_eq!(report.payload_bytes, 3 + 3 + 3 + 4);
            assert_eq!(report.total_bytes, bytes.len() as u64);
            assert_eq!(report.zero_length, 0);
            assert!(report.main_map);
            assert_eq!(report.extensions["vmdl_c"], 1);
            assert_eq!(report.subfolders["dota"], 2);
            assert_eq!(report.subfolders["(top)"], 2);
            assert_eq!(report.identity.len(), 64);
        }
    }

    #[test]
    fn the_identity_ignores_layout_and_follows_contents() {
        let one = analyze(&vpk::build_unchecked(1, &map_entries())).expect("one");
        let two = analyze(&vpk::build_unchecked(2, &map_entries())).expect("two");
        assert_eq!(one.identity, two.identity);
        let mut changed = map_entries();
        changed[3].1 = b"TREE";
        let other = analyze(&vpk::build_unchecked(1, &changed)).expect("other");
        assert_ne!(one.identity, other.identity);
    }

    #[test]
    fn scripts_programs_and_unknown_types_are_refused_with_their_path() {
        for (path, code) in [
            ("maps/dota/scripts/run.vjs_c", "map_entry_script"),
            ("maps/dota/evil.exe", "map_entry_script"),
            ("maps/dota/init.lua", "map_entry_script"),
            ("maps/dota/panel.vxml_c", "map_entry_script"),
            ("maps/dota/particle.vpcf_c", "map_entry_type_blocked"),
            ("maps/dota/readme.txt", "map_entry_type_blocked"),
        ] {
            let mut entries = map_entries();
            entries.push((path, b"x"));
            let rejection = analyze(&vpk::build_unchecked(2, &entries)).expect_err(path);
            assert_eq!(rejection.code, code, "{path}");
            assert_eq!(rejection.path.as_deref(), Some(path));
        }
    }

    #[test]
    fn paths_outside_maps_and_unsafe_names_are_refused() {
        for (path, code) in [
            ("models/heroes/axe/axe.vmdl_c", "map_entry_outside_maps"),
            ("dota.vmap_c", "map_entry_outside_maps"),
            ("maps/../escape.vmdl_c", "map_entry_path_invalid"),
            ("maps/Dota/Upper.vmdl_c", "map_entry_path_invalid"),
            ("maps/dota/a:b.vmdl_c", "map_entry_path_invalid"),
            ("maps/dota/a\\b.vmdl_c", "map_entry_path_invalid"),
            ("maps/dota//b.vmdl_c", "map_entry_path_invalid"),
            ("maps/dota/trailing .vmdl_c", "map_entry_path_invalid"),
        ] {
            let mut entries = map_entries();
            entries.push((path, b"x"));
            let rejection = analyze(&vpk::build_unchecked(2, &entries)).expect_err(path);
            assert_eq!(rejection.code, code, "{path}");
        }
    }

    #[test]
    fn a_package_without_the_main_map_is_refused() {
        let bytes = vpk::build_unchecked(2, &[("maps/dota/models/tree.vmdl_c", b"x".as_slice())]);
        assert_eq!(
            analyze(&bytes).expect_err("no main map").code,
            "map_main_missing"
        );
    }

    #[test]
    fn malformed_bytes_never_pass() {
        let good = vpk::build_unchecked(2, &map_entries());
        // Not a VPK, too short, wrong signature.
        assert!(analyze(b"not a vpk at all, not a vpk at all!!").is_err());
        assert!(analyze(&good[..20]).is_err());
        let mut bad_signature = good.clone();
        bad_signature[0] ^= 0xff;
        assert!(analyze(&bad_signature).is_err());
        // Bytes appended after the sections are not part of the package.
        let mut trailing = good.clone();
        trailing.extend_from_slice(b"hidden payload");
        assert_eq!(
            analyze(&trailing).expect_err("trailing").code,
            "map_package_invalid"
        );
        // A flipped payload byte fails the entry's CRC.
        let mut corrupt = good.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0x01;
        assert!(analyze(&corrupt).is_err());
        // Unknown header versions and sections BetterFy cannot read.
        let mut version_three = good.clone();
        version_three[4..8].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            analyze(&version_three).expect_err("version").code,
            "map_package_unsupported"
        );
        let mut with_signature = good.clone();
        with_signature[24..28].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            analyze(&with_signature).expect_err("signature").code,
            "map_package_unsupported"
        );
        let mut with_archive_md5 = good;
        with_archive_md5[16..20].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            analyze(&with_archive_md5).expect_err("md5").code,
            "map_package_unsupported"
        );
    }

    #[test]
    fn a_package_that_needs_a_side_archive_is_refused() {
        // Archive index 0 in the first entry instead of the embedded marker.
        let mut bytes = vpk::build_unchecked(1, &map_entries());
        // An entry is crc (4), preload length (2, zero here), archive index (2).
        let position = bytes
            .windows(4)
            .position(|quad| quad == [0, 0, 0xff, 0x7f])
            .expect("embedded marker")
            + 2;
        bytes[position..position + 2].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            analyze(&bytes).expect_err("side archive").code,
            "map_package_unsupported"
        );
    }

    #[test]
    fn unreferenced_data_is_refused() {
        // Version 2 header that claims more data than the entries use.
        let mut bytes = vpk::build_unchecked(2, &map_entries());
        let data = read_u32(&bytes, 12).expect("data size") + 5;
        bytes[12..16].copy_from_slice(&data.to_le_bytes());
        bytes.extend_from_slice(b"extra");
        assert_eq!(
            analyze(&bytes).expect_err("unreferenced").code,
            "map_package_invalid"
        );
    }
}
