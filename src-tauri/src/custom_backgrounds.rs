//! Minify's "Custom Backgrounds": the main menu background, either one of
//! the game's own loading screens or the player's own video.
//!
//! Minify's mod edits the dashboard layouts (`xml.json`: a `Movie` panel that
//! plays `panorama/videos/background.webm`, the 3D background turned off) and
//! styles the dashboard with `background-image: <&bg_img_style>`. Its default
//! value names a TI 2025 loading screen of the game; with a video it is
//! `none`. A picture of the player's own needs Valve's texture compiler, which
//! BetterFy does not run, so the picture choice is one of the game's own
//! loading screens. The choice is part of the package ID:
//!
//! * `minify.custom-backgrounds.ls-<name>`: a loading screen from
//!   [`LOADING_SCREENS`];
//! * `minify.custom-backgrounds.video-<sha256>-<bytes>`: a WebM the player
//!   picked, checked by the Panorama video proof and kept in the content
//!   store, installed as `panorama/videos/background.webm`.

use crate::content_store;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) const PACKAGE_ID: &str = "minify.custom-backgrounds";
pub(crate) const VIDEO_PATH: &str = "panorama/videos/background.webm";
const PLACEHOLDER: &str = "<&bg_img_style>";
const SCREEN_INFIX: &str = ".ls-";
const VIDEO_INFIX: &str = ".video-";
pub(crate) const MAX_VIDEO_BYTES: usize = 64 * 1024 * 1024;

/// Loading screens of the game's own events, by folder name below
/// `panorama/images/loadingscreens/` (each has a `loadingscreen.vtex_c` in
/// Valve's file list of 2026-10-17).
pub(crate) const LOADING_SCREENS: [&str; 28] = [
    "international_2025_ls_1",
    "international_2025_ls_2",
    "international_2025_ls_3",
    "international_2025_ls_4",
    "international_2025_ls_5",
    "international_2024_ls_1",
    "international_2024_ls_2",
    "international_2024_ls_3",
    "international_2024_ls_4",
    "international_2024_ls_5",
    "international_2023_ls_1",
    "international_2023_ls_2",
    "international_2023_ls_3",
    "international_2023_ls_4",
    "international_2023_ls_5",
    "international_2023_ls_6",
    "compendium2021_1_ls",
    "compendium2021_2_ls",
    "compendium2021_3_ls",
    "compendium2021_4_ls",
    "compendium2021_5_ls",
    "aghs_2021_roshan_loadingscreen",
    "diretide_2022_candy_loading_screen",
    "diretide_2022_greevil_loading_screen",
    "frostivus_2018_loadingscreen",
    "fall_major_2016_loadscreen_1",
    "fall_major_2016_loadscreen_2",
    "fall_major_2016_loadscreen_3",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Background {
    LoadingScreen(&'static str),
    Video { sha256: String, bytes: usize },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomVideoInfo {
    pub package_id: String,
    pub bytes: usize,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn variant_id(background: &Background) -> String {
    match background {
        Background::LoadingScreen(name) => format!("{PACKAGE_ID}{SCREEN_INFIX}{name}"),
        Background::Video { sha256, bytes } => format!("{PACKAGE_ID}{VIDEO_INFIX}{sha256}-{bytes}"),
    }
}

pub(crate) fn parse_variant(id: &str) -> Option<Background> {
    let rest = id.strip_prefix(PACKAGE_ID)?;
    if let Some(name) = rest.strip_prefix(SCREEN_INFIX) {
        return LOADING_SCREENS
            .iter()
            .find(|screen| **screen == name)
            .map(|screen| Background::LoadingScreen(screen));
    }
    let (sha, bytes) = rest.strip_prefix(VIDEO_INFIX)?.split_once('-')?;
    let valid_sha = sha.len() == 64
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    let size = bytes
        .parse::<usize>()
        .ok()
        .filter(|size| *size > 0 && *size <= MAX_VIDEO_BYTES && size.to_string() == bytes)?;
    valid_sha.then(|| Background::Video {
        sha256: sha.to_string(),
        bytes: size,
    })
}

/// The `background-image` value for the dashboard's style.
pub(crate) fn style_value(background: &Background) -> String {
    match background {
        Background::LoadingScreen(name) => {
            format!("url(\"s2r://panorama/images/loadingscreens/{name}/loadingscreen.vtex\")")
        }
        Background::Video { .. } => "none".to_string(),
    }
}

/// Minify's `styling.css` with its placeholder filled in.
pub(crate) fn fill_style(text: &str, background: &Background) -> Result<String, String> {
    if !text.contains(PLACEHOLDER) {
        return Err("panorama_styling_invalid".to_string());
    }
    Ok(text.replace(PLACEHOLDER, &style_value(background)))
}

/// Checks a video the player picked and keeps it.
pub(crate) fn store_video(
    app_data_root: &Path,
    video_base64: &str,
) -> Result<CustomVideoInfo, String> {
    if video_base64.len() > MAX_VIDEO_BYTES * 4 / 3 + 8 {
        return Err("custom_video_too_large".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(video_base64)
        .map_err(|_| "custom_video_invalid".to_string())?;
    if bytes.is_empty() {
        return Err("custom_video_invalid".to_string());
    }
    if bytes.len() > MAX_VIDEO_BYTES {
        return Err("custom_video_too_large".to_string());
    }
    crate::panorama_decor::check_webm(&bytes).map_err(|_| "custom_video_invalid".to_string())?;
    let sha = sha256(&bytes);
    content_store::store_pinned_resource(app_data_root, bytes.len(), &sha, &bytes)?;
    Ok(CustomVideoInfo {
        package_id: variant_id(&Background::Video {
            sha256: sha,
            bytes: bytes.len(),
        }),
        bytes: bytes.len(),
    })
}

/// The stored video, checked again.
pub(crate) fn load_video(
    app_data_root: &Path,
    sha256: &str,
    bytes: usize,
) -> Result<Vec<u8>, String> {
    let video = content_store::read_pinned_resource(app_data_root, bytes, sha256)
        .map_err(|_| "custom_video_missing".to_string())?;
    crate::panorama_decor::check_webm(&video).map_err(|_| "custom_video_invalid".to_string())?;
    Ok(video)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_name_a_listed_screen_or_one_video() {
        let screen = Background::LoadingScreen("international_2025_ls_3");
        let id = variant_id(&screen);
        assert_eq!(id, "minify.custom-backgrounds.ls-international_2025_ls_3");
        assert_eq!(parse_variant(&id), Some(screen));
        let video = Background::Video {
            sha256: "b".repeat(64),
            bytes: 1000,
        };
        assert_eq!(parse_variant(&variant_id(&video)), Some(video));
        for bad in [
            "minify.custom-backgrounds".to_string(),
            "minify.custom-backgrounds.ls-some_other_screen".to_string(),
            "minify.custom-backgrounds.ls-../../x".to_string(),
            format!("minify.custom-backgrounds.video-{}-0", "b".repeat(64)),
            format!("minify.custom-backgrounds.video-{}-01", "b".repeat(64)),
            format!("minify.custom-backgrounds.video-{}-1", "b".repeat(63)),
        ] {
            assert_eq!(parse_variant(&bad), None, "{bad}");
        }
        for name in LOADING_SCREENS {
            assert!(variant_id(&Background::LoadingScreen(name)).len() <= 128);
        }
    }

    #[test]
    fn the_style_placeholder_is_filled() {
        let css = "a { background-image: <&bg_img_style>; }";
        assert_eq!(
            fill_style(css, &Background::LoadingScreen("international_2024_ls_1")).unwrap(),
            "a { background-image: url(\"s2r://panorama/images/loadingscreens/international_2024_ls_1/loadingscreen.vtex\"); }"
        );
        assert_eq!(
            fill_style(
                css,
                &Background::Video {
                    sha256: "c".repeat(64),
                    bytes: 1
                }
            )
            .unwrap(),
            "a { background-image: none; }"
        );
        assert!(fill_style(
            "a {}",
            &Background::LoadingScreen("international_2024_ls_1")
        )
        .is_err());
    }

    #[test]
    fn a_picked_file_that_is_not_a_webm_is_refused() {
        let root =
            std::env::temp_dir().join(format!("betterfy-custom-video-{}", std::process::id()));
        let not_video = base64::engine::general_purpose::STANDARD.encode(b"RIFF....AVI ");
        assert_eq!(
            store_video(&root, &not_video).err().as_deref(),
            Some("custom_video_invalid")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
