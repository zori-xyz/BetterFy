//! Minify's "User Styles": a pinned file of optional style snippets
//! (`Minify/mods/User Styles/_styling.css`) from which the player picks.
//!
//! Minify asks the player to copy `_styling.css` to `styling.css` and delete
//! the snippets they do not want. BetterFy installs the same snippets, chosen
//! with switches: the choice is part of the package ID
//! (`minify.user-styles.pick-<hex>`, one bit per snippet in file order), so a
//! build stays deterministic, its plan binds the choice, and restore works as
//! for any other package. The snippets are plain CSS appended to the game's
//! compiled styles by `panorama.rs`, exactly like every other `styling.css`.

pub(crate) const PACKAGE_ID: &str = "minify.user-styles";
pub(crate) const STYLES_PATH: &str = "Minify/mods/User Styles/_styling.css";
const VARIANT_INFIX: &str = ".pick-";
/// The pinned file has 17 snippets; the mask has room for a few more.
const MAX_SNIPPETS: usize = 24;

/// One snippet: its own text, with the style headers it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snippet {
    pub css: String,
}

fn is_separator(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("/* ---")
        && line.ends_with("*/")
        && line[2..line.len() - 2].trim().chars().all(|c| c == '-')
}

fn header_of(line: &str) -> Option<&str> {
    let inner = line.trim().strip_prefix("/*")?.strip_suffix("*/")?.trim();
    (inner.starts_with("g:") || inner.starts_with("c:")).then_some(inner)
}

/// Splits the pinned file into its snippets, in file order. A snippet with
/// no style header of its own (the file has one: "removes the last match
/// details") belongs to the style the next snippet starts with, which is the
/// page its selectors are on.
pub(crate) fn snippets(text: &str) -> Result<Vec<Snippet>, String> {
    let mut parts: Vec<String> = vec![String::new()];
    for line in text.lines() {
        if is_separator(line) {
            parts.push(String::new());
        } else {
            let part = parts.last_mut().expect("one part");
            part.push_str(line);
            part.push('\n');
        }
    }
    let parts = parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > MAX_SNIPPETS {
        return Err("user_styles_invalid".to_string());
    }
    let first_header = |part: &str| part.lines().find_map(header_of).map(str::to_string);
    let mut snippets = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let starts_with_header = part
            .lines()
            .find(|line| !line.trim().is_empty())
            .and_then(header_of)
            .is_some();
        let css = if starts_with_header {
            part.clone()
        } else {
            let header = parts[index + 1..]
                .iter()
                .find_map(|next| first_header(next))
                .ok_or_else(|| "user_styles_invalid".to_string())?;
            format!("/* {header} */\n{part}")
        };
        snippets.push(Snippet { css });
    }
    Ok(snippets)
}

/// `minify.user-styles.pick-<hex>`: the base ID and the chosen snippets.
pub(crate) fn parse_variant(id: &str) -> Option<(&str, u32)> {
    let (base, hex) = id.split_once(VARIANT_INFIX)?;
    if base != PACKAGE_ID
        || hex.is_empty()
        || hex.len() > 6
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || hex.starts_with('0')
    {
        return None;
    }
    let mask = u32::from_str_radix(hex, 16).ok()?;
    (mask != 0 && mask < (1 << MAX_SNIPPETS)).then_some((base, mask))
}

pub(crate) fn variant_id(mask: u32) -> String {
    format!("{PACKAGE_ID}{VARIANT_INFIX}{mask:x}")
}

/// The chosen snippets joined into one `styling.css`. Every bit must name a
/// snippet of the file.
pub(crate) fn select(text: &str, mask: u32) -> Result<String, String> {
    let snippets = snippets(text)?;
    if mask == 0 || mask >> snippets.len() != 0 {
        return Err("user_styles_selection_invalid".to_string());
    }
    Ok(snippets
        .iter()
        .enumerate()
        .filter(|(index, _)| mask & (1 << index) != 0)
        .map(|(_, snippet)| snippet.css.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED: &str = include_str!("../tests/fixtures/minify_user_styles_styling.css");

    #[test]
    fn the_pinned_file_has_seventeen_snippets_that_each_parse() {
        let snippets = snippets(PINNED).expect("splits");
        assert_eq!(snippets.len(), 17);
        for (index, snippet) in snippets.iter().enumerate() {
            let sections = crate::panorama::split_styling(&snippet.css)
                .unwrap_or_else(|error| panic!("snippet {index}: {error}"));
            assert!(!sections.is_empty(), "snippet {index} names no style");
        }
        // The one snippet without a header of its own goes to the home page.
        assert!(snippets[15]
            .css
            .starts_with("/* g:panorama/styles/dashboard_page_home */"));
        assert!(snippets[15].css.contains("#MissingDataContainer"));
    }

    #[test]
    fn a_selection_is_exactly_the_chosen_snippets() {
        let one = select(PINNED, 1).expect("first");
        assert!(one.contains("#PortraitGroup") && !one.contains("#SearchResultsStatToggles"));
        let both = select(PINNED, 0b11).expect("first two");
        assert!(both.contains("#PortraitGroup") && both.contains("#SearchResultsStatToggles"));
        assert!(select(PINNED, 0).is_err());
        assert!(select(PINNED, 1 << 17).is_err());
        let all = select(PINNED, (1 << 17) - 1).expect("all");
        let sections = crate::panorama::split_styling(&all).expect("sections");
        assert!(sections.contains_key("game:panorama/styles/hud/hud_reborn.vcss_c"));
        assert!(sections.contains_key("core:panorama/styles/popups/popup_manager.vcss_c"));
    }

    #[test]
    fn variant_ids_round_trip_and_refuse_anything_else() {
        assert_eq!(variant_id(0x1a3), "minify.user-styles.pick-1a3");
        assert_eq!(
            parse_variant("minify.user-styles.pick-1a3"),
            Some((PACKAGE_ID, 0x1a3))
        );
        for bad in [
            "minify.user-styles",
            "minify.user-styles.pick-",
            "minify.user-styles.pick-0",
            "minify.user-styles.pick-01",
            "minify.user-styles.pick-1A",
            "minify.user-styles.pick-zz",
            "minify.user-styles.pick-1000000",
            "minify.tree-mod.pick-1",
        ] {
            assert_eq!(parse_variant(bad), None, "{bad}");
        }
    }
}
