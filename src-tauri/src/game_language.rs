use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameLanguage {
    Russian,
    Koreana,
    Schinese,
    #[default]
    Dutch,
}

impl GameLanguage {
    pub const ALL: [Self; 4] = [Self::Russian, Self::Koreana, Self::Schinese, Self::Dutch];

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Russian => "russian",
            Self::Koreana => "koreana",
            Self::Schinese => "schinese",
            Self::Dutch => "dutch",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_allowlisted_language_names() {
        for language in [
            GameLanguage::Russian,
            GameLanguage::Koreana,
            GameLanguage::Schinese,
            GameLanguage::Dutch,
        ] {
            assert_eq!(
                serde_json::from_str::<GameLanguage>(&format!("\"{}\"", language.suffix()))
                    .expect("supported language"),
                language
            );
        }
        for value in ["betterfy", "english", "../../dota", "russian/"] {
            assert!(serde_json::from_str::<GameLanguage>(&format!("\"{value}\"")).is_err());
        }
    }
}
