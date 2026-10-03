//! Expansion of Minify `blacklist.txt` files against the player's own game.
//!
//! A line is one of:
//! - `# ...` or blank: ignored;
//! - `>>dir/path`: every resource under that directory;
//! - `**regex`: every resource whose path the regular expression matches
//!   anywhere, as Minify does with ripgrep over the game's file list;
//! - anything else: that exact resource path.
//!
//! Only paths that exist in the game's own archives are kept, and only for
//! extensions the package has a blank placeholder for. Everything else is
//! counted and reported, never guessed.

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_LINES: usize = 20_000;
const MAX_PATTERN_BYTES: usize = 512;
const MAX_REGEX_SIZE: usize = 1 << 20;
/// Upper bound on resources one blacklist may blank.
pub(crate) const MAX_EXPANDED: usize = 60_000;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Expansion {
    /// Game path -> placeholder extension.
    pub paths: BTreeMap<String, String>,
    /// Exact lines naming a resource the current game does not have.
    pub missing_paths: usize,
    /// Directory or regex lines that matched nothing.
    pub unmatched_rules: usize,
    /// Matched resources skipped because no placeholder exists for their type.
    pub unsupported_types: usize,
}

fn extension(path: &str) -> Option<&str> {
    path.rsplit('/')
        .next()?
        .rsplit_once('.')
        .map(|(_, extension)| extension)
}

/// Expands `text` against `game_paths` (lowercase, `/`-separated).
pub(crate) fn expand(
    text: &str,
    game_paths: &BTreeSet<String>,
    blanks: &BTreeSet<String>,
) -> Result<Expansion, String> {
    let mut expansion = Expansion::default();
    let mut matched = BTreeSet::new();
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() > MAX_LINES {
        return Err("blacklist_too_large".to_string());
    }
    for raw in lines {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.len() > MAX_PATTERN_BYTES {
            return Err("blacklist_rule_invalid".to_string());
        }
        if let Some(directory) = line.strip_prefix(">>") {
            let prefix = format!(
                "{}/",
                directory.trim().trim_matches('/').to_ascii_lowercase()
            );
            if prefix == "/" {
                return Err("blacklist_rule_invalid".to_string());
            }
            let before = matched.len();
            matched.extend(
                game_paths
                    .range(prefix.clone()..)
                    .take_while(|path| path.starts_with(&prefix))
                    .cloned(),
            );
            if matched.len() == before {
                expansion.unmatched_rules += 1;
            }
        } else if let Some(pattern) = line.strip_prefix("**") {
            let regex = RegexBuilder::new(pattern)
                .case_insensitive(true)
                .size_limit(MAX_REGEX_SIZE)
                .build()
                .map_err(|_| "blacklist_rule_invalid".to_string())?;
            let before = matched.len();
            matched.extend(
                game_paths
                    .iter()
                    .filter(|path| regex.is_match(path))
                    .cloned(),
            );
            if matched.len() == before {
                expansion.unmatched_rules += 1;
            }
        } else {
            let path = line.replace('\\', "/").to_ascii_lowercase();
            if game_paths.contains(&path) {
                matched.insert(path);
            } else {
                expansion.missing_paths += 1;
            }
        }
        if matched.len() > MAX_EXPANDED {
            return Err("blacklist_too_large".to_string());
        }
    }
    for path in matched {
        match extension(&path) {
            Some(extension) if blanks.contains(extension) => {
                let extension = extension.to_string();
                expansion.paths.insert(path, extension);
            }
            _ => expansion.unsupported_types += 1,
        }
    }
    Ok(expansion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> BTreeSet<String> {
        [
            "particles/rain_fx/rain.vpcf_c",
            "particles/rain_fx/drop/drop.vpcf_c",
            "particles/rain_fxs/other.vpcf_c",
            "sounds/ui/ping_attack.vsnd_c",
            "sounds/ui/click.vsnd_c",
            "sounds/vo/axe/taunt_01.vsnd_c",
            "scripts/ping.vjs_c",
            "materials/nature/card_grass_01.vmat_c",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn blanks() -> BTreeSet<String> {
        ["vpcf_c", "vsnd_c", "vmat_c"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn expands_directories_regexes_and_exact_paths() {
        let text = "# comment\n\n>>particles/rain_fx\n**sounds/ui/.*ping.*\\.vsnd_c\nmaterials/nature/card_grass_01.vmat_c\r\nmissing/thing.vpcf_c\n";
        let expansion = expand(text, &game(), &blanks()).expect("expands");
        assert_eq!(
            expansion.paths.keys().cloned().collect::<Vec<_>>(),
            vec![
                "materials/nature/card_grass_01.vmat_c",
                "particles/rain_fx/drop/drop.vpcf_c",
                "particles/rain_fx/rain.vpcf_c",
                "sounds/ui/ping_attack.vsnd_c",
            ]
        );
        assert_eq!(expansion.paths["sounds/ui/ping_attack.vsnd_c"], "vsnd_c");
        assert_eq!(expansion.missing_paths, 1);
        assert_eq!(expansion.unmatched_rules, 0);
    }

    #[test]
    fn directory_rules_do_not_match_sibling_prefixes() {
        let expansion = expand(">>particles/rain_fx", &game(), &blanks()).expect("expands");
        assert!(!expansion
            .paths
            .contains_key("particles/rain_fxs/other.vpcf_c"));
    }

    #[test]
    fn regex_matches_anywhere_and_types_without_a_blank_are_skipped() {
        let expansion = expand("**ping", &game(), &blanks()).expect("expands");
        assert_eq!(expansion.paths.len(), 1);
        assert_eq!(
            expansion.unsupported_types, 1,
            "the vjs_c script is never blanked"
        );
    }

    #[test]
    fn rules_that_match_nothing_are_counted() {
        let expansion = expand(">>nothing/here\n**^nope$", &game(), &blanks()).expect("expands");
        assert!(expansion.paths.is_empty());
        assert_eq!(expansion.unmatched_rules, 2);
    }

    #[test]
    fn invalid_rules_are_rejected() {
        assert!(expand("**(unclosed", &game(), &blanks()).is_err());
        assert!(expand(">>/", &game(), &blanks()).is_err());
        assert!(expand(&"x".repeat(MAX_PATTERN_BYTES + 1), &game(), &blanks()).is_err());
    }
}
