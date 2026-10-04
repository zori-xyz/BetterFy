use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::steam_config::app_manifest_language;

/// Steam keeps a few KiB per app here; anything far larger is not a manifest.
const MAX_APP_MANIFEST_BYTES: u64 = 256 * 1024;

/// A language slot: BetterFy writes its VPK to `game/dota_<suffix>` and
/// starts Dota with `-language <suffix>`. The names are Valve's Steam API
/// language codes for the languages Dota 2 ships, plus `betterfy`, which
/// stands in for English: English is Dota's base language and has no
/// `dota_english` folder to write into.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameLanguage {
    Betterfy,
    Brazilian,
    Bulgarian,
    Czech,
    Danish,
    // Journals written before the language choice existed carry no language;
    // they were all Dutch.
    #[default]
    Dutch,
    Finnish,
    French,
    German,
    Greek,
    Hungarian,
    Italian,
    Japanese,
    Koreana,
    Latam,
    Norwegian,
    Polish,
    Portuguese,
    Romanian,
    Russian,
    Schinese,
    Spanish,
    Swedish,
    Tchinese,
    Thai,
    Turkish,
    Ukrainian,
    Vietnamese,
}

impl GameLanguage {
    pub const ALL: [Self; 28] = [
        Self::Betterfy,
        Self::Brazilian,
        Self::Bulgarian,
        Self::Czech,
        Self::Danish,
        Self::Dutch,
        Self::Finnish,
        Self::French,
        Self::German,
        Self::Greek,
        Self::Hungarian,
        Self::Italian,
        Self::Japanese,
        Self::Koreana,
        Self::Latam,
        Self::Norwegian,
        Self::Polish,
        Self::Portuguese,
        Self::Romanian,
        Self::Russian,
        Self::Schinese,
        Self::Spanish,
        Self::Swedish,
        Self::Tchinese,
        Self::Thai,
        Self::Turkish,
        Self::Ukrainian,
        Self::Vietnamese,
    ];

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Betterfy => "betterfy",
            Self::Brazilian => "brazilian",
            Self::Bulgarian => "bulgarian",
            Self::Czech => "czech",
            Self::Danish => "danish",
            Self::Dutch => "dutch",
            Self::Finnish => "finnish",
            Self::French => "french",
            Self::German => "german",
            Self::Greek => "greek",
            Self::Hungarian => "hungarian",
            Self::Italian => "italian",
            Self::Japanese => "japanese",
            Self::Koreana => "koreana",
            Self::Latam => "latam",
            Self::Norwegian => "norwegian",
            Self::Polish => "polish",
            Self::Portuguese => "portuguese",
            Self::Romanian => "romanian",
            Self::Russian => "russian",
            Self::Schinese => "schinese",
            Self::Spanish => "spanish",
            Self::Swedish => "swedish",
            Self::Tchinese => "tchinese",
            Self::Thai => "thai",
            Self::Turkish => "turkish",
            Self::Ukrainian => "ukrainian",
            Self::Vietnamese => "vietnamese",
        }
    }

    /// The slot for a language name as Steam writes it. English maps to the
    /// BetterFy slot; `betterfy` itself is not a Steam language.
    pub fn from_steam_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("english") {
            return Some(Self::Betterfy);
        }
        Self::ALL.into_iter().find(|language| {
            *language != Self::Betterfy && language.suffix().eq_ignore_ascii_case(name)
        })
    }
}

/// The language Steam has set for Dota 2. `language` is `None` when BetterFy
/// has no slot for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLanguage {
    pub steam_language: String,
    pub language: Option<GameLanguage>,
}

fn invalid() -> String {
    "steam_manifest_invalid".to_string()
}

/// Reads the language from the text of `appmanifest_570.acf`.
pub fn detect_from_manifest(contents: &str) -> Result<Option<DetectedLanguage>, String> {
    let Some(name) = app_manifest_language(contents)? else {
        return Ok(None);
    };
    if name.len() > 32
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(invalid());
    }
    Ok(Some(DetectedLanguage {
        language: GameLanguage::from_steam_name(&name),
        steam_language: name,
    }))
}

/// Reads the language Steam set for the Dota installation at `game_root`, a
/// validated `steamapps/common/dota 2 beta` folder, from its library's
/// `steamapps/appmanifest_570.acf`. `None` when there is no such manifest or
/// it records no language. Read-only.
pub fn detect_for_game_root(game_root: &Path) -> Result<Option<DetectedLanguage>, String> {
    let named = |path: &Path, name: &str| {
        path.file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(name))
    };
    let Some(common) = game_root.parent().filter(|path| named(path, "common")) else {
        return Ok(None);
    };
    let Some(steamapps) = common.parent().filter(|path| named(path, "steamapps")) else {
        return Ok(None);
    };
    let manifest = steamapps.join("appmanifest_570.acf");
    let metadata = match fs::symlink_metadata(&manifest) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(invalid()),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_APP_MANIFEST_BYTES
    {
        return Err(invalid());
    }
    // Bounded again while reading, in case Steam rewrites the file meanwhile.
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(&manifest)
        .and_then(|file| {
            file.take(MAX_APP_MANIFEST_BYTES + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > MAX_APP_MANIFEST_BYTES {
        return Err(invalid());
    }
    detect_from_manifest(&String::from_utf8(bytes).map_err(|_| invalid())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // The layout Steam writes for app 570, trimmed to the parts that matter
    // here plus the nested sections a parser must walk past.
    fn manifest(user: Option<&str>, mounted: Option<&str>) -> String {
        let section = |name: &str, value: Option<&str>, extra: &str| {
            value.map_or_else(String::new, |value| {
                format!("\t\"{name}\"\n\t{{\n\t\t\"language\"\t\t\"{value}\"\n{extra}\t}}\n")
            })
        };
        [
            "\"AppState\"\n{\n",
            "\t\"appid\"\t\t\"570\"\n",
            "\t\"LauncherPath\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\\\\steam.exe\"\n",
            "\t\"name\"\t\t\"Dota 2\"\n",
            "\t\"installdir\"\t\t\"dota 2 beta\"\n",
            "\t\"buildid\"\t\t\"20512345\"\n",
            "\t\"InstalledDepots\"\n\t{\n",
            "\t\t\"373301\"\n\t\t{\n\t\t\t\"manifest\"\t\t\"1\"\n\t\t\t\"size\"\t\t\"2\"\n\t\t}\n",
            "\t\t\"381451\"\n\t\t{\n\t\t\t\"manifest\"\t\t\"3\"\n\t\t\t\"size\"\t\t\"4\"\n\t\t\t\"dlcappid\"\t\t\"381450\"\n\t\t}\n",
            "\t}\n",
            "\t\"SharedDepots\"\n\t{\n\t\t\"228990\"\t\t\"228980\"\n\t}\n",
            &section("UserConfig", user, "\t\t\"BetaKey\"\t\t\"public\"\n"),
            &section("MountedConfig", mounted, ""),
            "}\n",
        ]
        .concat()
    }

    #[test]
    fn accepts_only_allowlisted_language_names() {
        for language in GameLanguage::ALL {
            assert_eq!(
                serde_json::from_str::<GameLanguage>(&format!("\"{}\"", language.suffix()))
                    .expect("supported language"),
                language
            );
            assert_eq!(
                serde_json::to_string(&language).expect("serialize"),
                format!("\"{}\"", language.suffix())
            );
            assert!(language
                .suffix()
                .bytes()
                .all(|byte| byte.is_ascii_lowercase()));
        }
        let mut suffixes = GameLanguage::ALL.map(GameLanguage::suffix).to_vec();
        suffixes.sort_unstable();
        suffixes.dedup();
        assert_eq!(suffixes.len(), GameLanguage::ALL.len());
        // English has no `dota_english` folder: only `betterfy` stands for it.
        for value in [
            "english",
            "minify",
            "arabic",
            "Russian",
            "../../dota",
            "russian/",
            "",
        ] {
            assert!(serde_json::from_str::<GameLanguage>(&format!("\"{value}\"")).is_err());
        }
        assert_eq!(GameLanguage::default(), GameLanguage::Dutch);
    }

    #[test]
    fn steam_names_map_to_their_slots() {
        assert_eq!(
            GameLanguage::from_steam_name("english"),
            Some(GameLanguage::Betterfy)
        );
        assert_eq!(GameLanguage::from_steam_name("betterfy"), None);
        assert_eq!(GameLanguage::from_steam_name("arabic"), None);
        for language in GameLanguage::ALL {
            if language != GameLanguage::Betterfy {
                assert_eq!(
                    GameLanguage::from_steam_name(language.suffix()),
                    Some(language)
                );
            }
        }
    }

    #[test]
    fn detects_the_language_steam_set_for_dota() {
        let english = detect_from_manifest(&manifest(Some("english"), Some("english")))
            .expect("parse")
            .expect("language");
        assert_eq!(english.steam_language, "english");
        assert_eq!(english.language, Some(GameLanguage::Betterfy));

        let russian = detect_from_manifest(&manifest(Some("russian"), Some("russian")))
            .expect("parse")
            .expect("language");
        assert_eq!(russian.language, Some(GameLanguage::Russian));

        // The player's choice wins over the language still mounted while a
        // switch downloads.
        let switching = detect_from_manifest(&manifest(Some("tchinese"), Some("russian")))
            .expect("parse")
            .expect("language");
        assert_eq!(switching.language, Some(GameLanguage::Tchinese));
        let mounted_only = detect_from_manifest(&manifest(None, Some("latam")))
            .expect("parse")
            .expect("language");
        assert_eq!(mounted_only.language, Some(GameLanguage::Latam));

        let unknown = detect_from_manifest(&manifest(Some("arabic"), None))
            .expect("parse")
            .expect("language");
        assert_eq!(unknown.steam_language, "arabic");
        assert_eq!(unknown.language, None);

        assert_eq!(detect_from_manifest(&manifest(None, None)), Ok(None));
    }

    #[test]
    fn malformed_manifests_are_errors() {
        let invalid = Err("steam_manifest_invalid".to_string());
        assert_eq!(detect_from_manifest("\"AppState\" { \"appid\""), invalid);
        assert_eq!(detect_from_manifest("\"AppState\" {"), invalid);
        assert_eq!(detect_from_manifest("AppState { }"), invalid);
        assert_eq!(
            detect_from_manifest(&manifest(Some("russian"), None).replace("\"570\"", "\"440\"")),
            invalid
        );
        assert_eq!(
            detect_from_manifest(&manifest(Some("../russian"), None)),
            invalid
        );
        assert_eq!(
            detect_from_manifest(
                "\"AppState\" { \"appid\" \"570\" \"UserConfig\" { \"language\" { } } }"
            ),
            invalid
        );
    }

    fn unique_temp(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("betterfy-language-{name}-{nonce}"))
    }

    #[test]
    fn reads_the_manifest_of_the_library_holding_dota() {
        let library = unique_temp("library");
        let steamapps = library.join("steamapps");
        let game = steamapps.join("common").join("dota 2 beta");
        fs::create_dir_all(&game).expect("game dir");
        assert_eq!(detect_for_game_root(&game), Ok(None));

        let file = steamapps.join("appmanifest_570.acf");
        fs::write(&file, manifest(Some("russian"), Some("russian"))).expect("manifest");
        assert_eq!(
            detect_for_game_root(&game)
                .expect("read")
                .and_then(|found| found.language),
            Some(GameLanguage::Russian)
        );

        fs::write(&file, "x".repeat(MAX_APP_MANIFEST_BYTES as usize + 1)).expect("oversized");
        assert_eq!(
            detect_for_game_root(&game).err().as_deref(),
            Some("steam_manifest_invalid")
        );
        fs::write(&file, b"").expect("empty");
        assert_eq!(
            detect_for_game_root(&game).err().as_deref(),
            Some("steam_manifest_invalid")
        );

        // A folder outside `steamapps/common` has no library manifest.
        let loose = library.join("dota 2 beta");
        fs::create_dir_all(&loose).expect("loose dir");
        assert_eq!(detect_for_game_root(&loose), Ok(None));

        #[cfg(unix)]
        {
            let real = library.join("elsewhere.acf");
            fs::write(&real, manifest(Some("russian"), None)).expect("real manifest");
            fs::remove_file(&file).expect("remove");
            std::os::unix::fs::symlink(&real, &file).expect("symlink");
            assert_eq!(
                detect_for_game_root(&game).err().as_deref(),
                Some("steam_manifest_invalid")
            );
        }
        let _ = fs::remove_dir_all(&library);
    }
}
