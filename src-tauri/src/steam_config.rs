use crate::game_language::GameLanguage;
use serde::Serialize;
use sha2::{Digest, Sha256};

const DOTA_CONFIG_PATH: [&str; 7] = [
    "UserLocalConfigStore",
    "Software",
    "Valve",
    "Steam",
    "apps",
    "570",
    "LaunchOptions",
];

/// Steam's own files nest a handful of levels; a deeper file is not one of
/// them, and the bound keeps the recursive parser off the end of the stack.
const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptionPlan {
    pub changed: bool,
    pub before_sha256: String,
    pub after_sha256: String,
    pub updated_contents: String,
}

#[derive(Clone, Debug)]
struct TextValue {
    decoded: String,
    content_start: usize,
    content_end: usize,
}

#[derive(Clone, Debug)]
struct ObjectValue {
    entries: Vec<Entry>,
    close_line_start: usize,
}

#[derive(Clone, Debug)]
enum Value {
    Text(TextValue),
    Object(ObjectValue),
}

#[derive(Clone, Debug)]
struct Entry {
    key: String,
    value: Value,
}

#[derive(Clone, Debug)]
enum TokenKind {
    Text(TextValue),
    Open,
    Close,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    start: usize,
}

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn line_start(contents: &str, position: usize) -> usize {
    contents[..position]
        .rfind('\n')
        .map_or(0, |newline| newline + 1)
}

fn decode_quoted(contents: &str, start: usize, end: usize) -> Result<String, String> {
    let bytes = contents.as_bytes();
    let mut decoded = Vec::with_capacity(end.saturating_sub(start));
    let mut index = start;
    while index < end {
        if bytes[index] == b'\\' {
            index += 1;
            if index >= end {
                return Err("steam_config_invalid".to_string());
            }
            match bytes[index] {
                b'\\' | b'"' => decoded.push(bytes[index]),
                b'n' => decoded.push(b'\n'),
                b't' => decoded.push(b'\t'),
                other => {
                    decoded.push(b'\\');
                    decoded.push(other);
                }
            }
        } else {
            decoded.push(bytes[index]);
        }
        index += 1;
    }
    String::from_utf8(decoded).map_err(|_| "steam_config_invalid".to_string())
}

fn tokenize(contents: &str) -> Result<Vec<Token>, String> {
    let bytes = contents.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\r' | b'\n' => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'{' => {
                tokens.push(Token {
                    kind: TokenKind::Open,
                    start: index,
                });
                index += 1;
            }
            b'}' => {
                tokens.push(Token {
                    kind: TokenKind::Close,
                    start: index,
                });
                index += 1;
            }
            b'"' => {
                let token_start = index;
                let content_start = index + 1;
                index += 1;
                let mut escaped = false;
                while index < bytes.len() {
                    if !escaped && bytes[index] == b'"' {
                        break;
                    }
                    escaped = !escaped && bytes[index] == b'\\';
                    index += 1;
                }
                if index >= bytes.len() {
                    return Err("steam_config_invalid".to_string());
                }
                let content_end = index;
                let decoded = decode_quoted(contents, content_start, content_end)?;
                tokens.push(Token {
                    kind: TokenKind::Text(TextValue {
                        decoded,
                        content_start,
                        content_end,
                    }),
                    start: token_start,
                });
                index += 1;
            }
            _ => return Err("steam_config_invalid".to_string()),
        }
    }
    Ok(tokens)
}

fn parse_entries(
    contents: &str,
    tokens: &[Token],
    mut index: usize,
    depth: usize,
) -> Result<(ObjectValue, usize), String> {
    let nested = depth > 0;
    if depth > MAX_DEPTH {
        return Err("steam_config_invalid".to_string());
    }
    let mut entries = Vec::new();
    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::Close) {
            if !nested {
                return Err("steam_config_invalid".to_string());
            }
            return Ok((
                ObjectValue {
                    entries,
                    close_line_start: line_start(contents, tokens[index].start),
                },
                index + 1,
            ));
        }

        let TokenKind::Text(key) = &tokens[index].kind else {
            return Err("steam_config_invalid".to_string());
        };
        index += 1;
        let Some(value_token) = tokens.get(index) else {
            return Err("steam_config_invalid".to_string());
        };
        let value = match &value_token.kind {
            TokenKind::Text(value) => {
                index += 1;
                Value::Text(value.clone())
            }
            TokenKind::Open => {
                let (object, next) = parse_entries(contents, tokens, index + 1, depth + 1)?;
                index = next;
                Value::Object(object)
            }
            TokenKind::Close => return Err("steam_config_invalid".to_string()),
        };
        entries.push(Entry {
            key: key.decoded.clone(),
            value,
        });
    }

    if nested {
        return Err("steam_config_invalid".to_string());
    }
    Ok((
        ObjectValue {
            entries,
            close_line_start: contents.len(),
        },
        index,
    ))
}

fn parse(contents: &str) -> Result<ObjectValue, String> {
    let tokens = tokenize(contents)?;
    let (root, consumed) = parse_entries(contents, &tokens, 0, 0)?;
    if consumed != tokens.len() {
        return Err("steam_config_invalid".to_string());
    }
    Ok(root)
}

fn find_value<'a>(object: &'a ObjectValue, path: &[&str]) -> Option<&'a Value> {
    let (head, tail) = path.split_first()?;
    let entry = object
        .entries
        .iter()
        .find(|entry| entry.key.eq_ignore_ascii_case(head))?;
    if tail.is_empty() {
        return Some(&entry.value);
    }
    let Value::Object(child) = &entry.value else {
        return None;
    };
    find_value(child, tail)
}

fn command_token_spans(value: &str) -> Result<Vec<(String, usize, usize)>, String> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut current = Vec::new();
    let mut start = None;
    let mut quoted = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                start.get_or_insert(index);
                quoted = !quoted;
            }
            b' ' | b'\t' if !quoted => {
                if !current.is_empty() {
                    tokens.push((
                        String::from_utf8(std::mem::take(&mut current))
                            .map_err(|_| "launch_options_invalid".to_string())?,
                        start
                            .take()
                            .ok_or_else(|| "launch_options_invalid".to_string())?,
                        index,
                    ));
                }
            }
            byte => {
                start.get_or_insert(index);
                current.push(byte);
            }
        }
        index += 1;
    }
    if quoted {
        return Err("launch_options_invalid".to_string());
    }
    if !current.is_empty() {
        tokens.push((
            String::from_utf8(current).map_err(|_| "launch_options_invalid".to_string())?,
            start.ok_or_else(|| "launch_options_invalid".to_string())?,
            value.len(),
        ));
    }
    Ok(tokens)
}

fn add_managed_argument(existing: &str, language: GameLanguage) -> Result<String, String> {
    let tokens = command_token_spans(existing)?;
    let mut managed_language: Option<(usize, usize, &str)> = None;
    for (index, (token, _, _)) in tokens.iter().enumerate() {
        if token.eq_ignore_ascii_case("-language") {
            let Some((existing_language, start, end)) = tokens.get(index + 1) else {
                return Err("launch_options_invalid".to_string());
            };
            if managed_language.is_some() {
                return Err("launch_options_invalid".to_string());
            }
            managed_language = Some((*start, *end, existing_language));
        }
    }
    if let Some((start, end, existing_language)) = managed_language {
        if existing_language.eq_ignore_ascii_case(language.suffix()) {
            return Ok(existing.to_string());
        }
        return Ok(format!(
            "{}{}{}",
            &existing[..start],
            language.suffix(),
            &existing[end..]
        ));
    }

    let separator = if existing.is_empty()
        || existing
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_whitespace)
    {
        ""
    } else {
        " "
    };
    Ok(format!(
        "{existing}{separator}-language {}",
        language.suffix()
    ))
}

fn encode_vdf(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn plan_managed_launch_option(contents: &str) -> Result<LaunchOptionPlan, String> {
    plan_launch_option_for_language(contents, GameLanguage::Dutch)
}

pub fn plan_launch_option_for_language(
    contents: &str,
    language: GameLanguage,
) -> Result<LaunchOptionPlan, String> {
    let root = parse(contents)?;
    let app_path = &DOTA_CONFIG_PATH[..DOTA_CONFIG_PATH.len() - 1];
    let app = find_value(&root, app_path).ok_or_else(|| "dota_config_missing".to_string())?;
    let Value::Object(app) = app else {
        return Err("steam_config_invalid".to_string());
    };

    let updated_contents = match find_value(&root, &DOTA_CONFIG_PATH) {
        Some(Value::Text(value)) => {
            let updated = add_managed_argument(&value.decoded, language)?;
            if updated == value.decoded {
                contents.to_string()
            } else {
                let mut result = String::with_capacity(contents.len() + 32);
                result.push_str(&contents[..value.content_start]);
                result.push_str(&encode_vdf(&updated));
                result.push_str(&contents[value.content_end..]);
                result
            }
        }
        Some(Value::Object(_)) => return Err("steam_config_invalid".to_string()),
        None => {
            let close_line = &contents[app.close_line_start..];
            let indentation = close_line
                .chars()
                .take_while(|character| matches!(character, ' ' | '\t'))
                .collect::<String>();
            let insertion = format!(
                "{indentation}\t\"LaunchOptions\"\t\t\"-language {}\"\n",
                language.suffix()
            );
            let mut result = String::with_capacity(contents.len() + insertion.len());
            result.push_str(&contents[..app.close_line_start]);
            result.push_str(&insertion);
            result.push_str(&contents[app.close_line_start..]);
            result
        }
    };

    Ok(LaunchOptionPlan {
        changed: updated_contents != contents,
        before_sha256: sha256(contents),
        after_sha256: sha256(&updated_contents),
        updated_contents,
    })
}

/// The decoded Dota 2 `LaunchOptions` value, or `None` when the entry is absent.
pub fn launch_options_value(contents: &str) -> Result<Option<String>, String> {
    let root = parse(contents)?;
    match find_value(&root, &DOTA_CONFIG_PATH) {
        Some(Value::Text(value)) => Ok(Some(value.decoded.clone())),
        Some(Value::Object(_)) => Err("steam_config_invalid".to_string()),
        None => Ok(None),
    }
}

/// The language Steam has set for Dota 2 in `appmanifest_570.acf`: the
/// player's choice under `UserConfig`, else the one installed under
/// `MountedConfig`. `None` when the manifest records neither.
pub fn app_manifest_language(contents: &str) -> Result<Option<String>, String> {
    let invalid = || "steam_manifest_invalid".to_string();
    let root = parse(contents).map_err(|_| invalid())?;
    match find_value(&root, &["AppState", "appid"]) {
        Some(Value::Text(value)) if value.decoded == "570" => {}
        _ => return Err(invalid()),
    }
    for section in ["UserConfig", "MountedConfig"] {
        match find_value(&root, &["AppState", section, "language"]) {
            Some(Value::Text(value)) if !value.decoded.trim().is_empty() => {
                return Ok(Some(value.decoded.trim().to_string()));
            }
            Some(Value::Object(_)) => return Err(invalid()),
            _ => {}
        }
    }
    Ok(None)
}

/// The `-language` argument in a launch-option value: its language and the
/// byte span of the whole `-language <value>` pair.
fn language_argument(value: &str) -> Result<Option<(String, usize, usize, usize)>, String> {
    let tokens = command_token_spans(value)?;
    let mut found = None;
    for (index, (token, start, _)) in tokens.iter().enumerate() {
        if token.eq_ignore_ascii_case("-language") {
            let Some((language, language_start, end)) = tokens.get(index + 1) else {
                return Err("launch_options_invalid".to_string());
            };
            if found.is_some() {
                return Err("launch_options_invalid".to_string());
            }
            found = Some((language.clone(), *start, *language_start, *end));
        }
    }
    Ok(found)
}

/// An absent entry and an empty value launch Dota the same way.
fn same_launch_options(left: Option<&str>, right: Option<&str>) -> bool {
    left.unwrap_or("").trim() == right.unwrap_or("").trim()
}

/// Result of undoing BetterFy's own `-language` change in the current file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchOptionRestore {
    /// The file with BetterFy's change undone, or `None` when nothing needs
    /// to be written.
    pub updated_contents: Option<String>,
    /// The `LaunchOptions` value the file holds afterwards.
    pub restored_value: Option<String>,
    /// The value differs from the one before installation because someone
    /// changed it after BetterFy did; BetterFy leaves those changes alone.
    pub kept_user_change: bool,
}

/// Undoes only the `-language` argument BetterFy set, leaving the rest of the
/// file as Steam last wrote it. Steam rewrites `localconfig.vdf` on every
/// start (play time, window state and more), so the file as a whole cannot be
/// returned to its bytes from before installation without discarding those
/// changes.
///
/// `before` is the value before installation and `applied` the language
/// BetterFy set. When the current value no longer carries `-language
/// <applied>`, someone already changed it and it is left as it is.
pub fn plan_language_restore(
    contents: &str,
    before: Option<&str>,
    applied: GameLanguage,
) -> Result<LaunchOptionRestore, String> {
    let root = parse(contents)?;
    let current = match find_value(&root, &DOTA_CONFIG_PATH) {
        Some(Value::Text(value)) => value.clone(),
        Some(Value::Object(_)) => return Err("steam_config_invalid".to_string()),
        None => {
            return Ok(LaunchOptionRestore {
                updated_contents: None,
                restored_value: None,
                kept_user_change: !same_launch_options(before, None),
            })
        }
    };
    let unchanged = |value: &str| LaunchOptionRestore {
        updated_contents: None,
        restored_value: Some(value.to_string()),
        kept_user_change: !same_launch_options(before, Some(value)),
    };
    let Some((language, pair_start, language_start, end)) = language_argument(&current.decoded)?
    else {
        return Ok(unchanged(&current.decoded));
    };
    if !language.eq_ignore_ascii_case(applied.suffix()) {
        return Ok(unchanged(&current.decoded));
    }
    let before_language = match before {
        Some(value) => language_argument(value)?.map(|(language, ..)| language),
        None => None,
    };
    let value = &current.decoded;
    let restored = match &before_language {
        Some(previous) => format!("{}{}{}", &value[..language_start], previous, &value[end..]),
        None => {
            // Remove the pair and one separating space, the way it was added.
            let (start, end) =
                if pair_start > 0 && value.as_bytes()[pair_start - 1].is_ascii_whitespace() {
                    (pair_start - 1, end)
                } else if value
                    .as_bytes()
                    .get(end)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    (pair_start, end + 1)
                } else {
                    (pair_start, end)
                };
            format!("{}{}", &value[..start], &value[end..])
        }
    };
    if before.is_none() && restored.trim().is_empty() {
        // BetterFy created the entry; remove its line if it holds nothing else.
        let line = line_start(contents, current.content_start);
        let line_end = contents[current.content_end..]
            .find('\n')
            .map_or(contents.len(), |offset| current.content_end + offset + 1);
        let text = contents[line..line_end].trim();
        let quoted_value = format!(
            "\"{}\"",
            &contents[current.content_start..current.content_end]
        );
        if let Some(rest) = text
            .get(..15)
            .filter(|key| key.eq_ignore_ascii_case("\"LaunchOptions\""))
            .map(|_| text[15..].trim())
        {
            if rest == quoted_value {
                let updated = format!("{}{}", &contents[..line], &contents[line_end..]);
                return Ok(LaunchOptionRestore {
                    updated_contents: Some(updated),
                    restored_value: None,
                    kept_user_change: false,
                });
            }
        }
    }
    let updated = format!(
        "{}{}{}",
        &contents[..current.content_start],
        encode_vdf(&restored),
        &contents[current.content_end..]
    );
    Ok(LaunchOptionRestore {
        kept_user_change: !same_launch_options(before, Some(&restored)),
        updated_contents: Some(updated),
        restored_value: Some(restored),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_config(launch_options: Option<&str>) -> String {
        let options = launch_options.map_or_else(String::new, |value| {
            format!("\t\t\t\t\t\t\t\"LaunchOptions\"\t\t\"{value}\"\n")
        });
        format!(
            "\"UserLocalConfigStore\"\n{{\n\t\"Software\"\n\t{{\n\t\t\"Valve\"\n\t\t{{\n\t\t\t\"Steam\"\n\t\t\t{{\n\t\t\t\t\"apps\"\n\t\t\t\t{{\n\t\t\t\t\t\"570\"\n\t\t\t\t\t{{\n{options}\t\t\t\t\t}}\n\t\t\t\t}}\n\t\t\t}}\n\t\t}}\n\t}}\n}}\n"
        )
    }

    #[test]
    fn language_restore_undoes_only_betterfys_argument() {
        // Appended to existing options.
        let added = local_config(Some("-novid -language dutch"));
        let plan =
            plan_language_restore(&added, Some("-novid"), GameLanguage::Dutch).expect("plan");
        assert_eq!(
            plan.updated_contents.as_deref(),
            Some(local_config(Some("-novid")).as_str())
        );
        assert!(!plan.kept_user_change);
        // Replaced a previous language.
        let switched = local_config(Some("-novid -language russian +exec a.cfg"));
        let plan = plan_language_restore(
            &switched,
            Some("-novid -language dutch +exec a.cfg"),
            GameLanguage::Russian,
        )
        .expect("plan");
        assert_eq!(
            plan.updated_contents.as_deref(),
            Some(local_config(Some("-novid -language dutch +exec a.cfg")).as_str())
        );
        // An entry BetterFy created is removed again, byte for byte.
        let without = local_config(None);
        let created = plan_launch_option_for_language(&without, GameLanguage::Dutch)
            .expect("insert")
            .updated_contents;
        let plan = plan_language_restore(&created, None, GameLanguage::Dutch).expect("plan");
        assert_eq!(plan.updated_contents.as_deref(), Some(without.as_str()));
        assert_eq!(plan.restored_value, None);
        assert!(!plan.kept_user_change);
        // An argument the player already changed is left alone.
        let other = local_config(Some("-novid -language koreana"));
        let plan =
            plan_language_restore(&other, Some("-novid"), GameLanguage::Dutch).expect("plan");
        assert_eq!(plan.updated_contents, None);
        assert!(plan.kept_user_change);
        let gone = local_config(Some("-novid"));
        let plan = plan_language_restore(&gone, Some("-novid"), GameLanguage::Dutch).expect("plan");
        assert_eq!(plan.updated_contents, None);
        assert!(!plan.kept_user_change);
    }

    #[test]
    fn every_language_is_applied_and_restored_exactly() {
        let before = "-novid -language russian +exec a.cfg";
        let foreign = local_config(Some(before));
        let without = local_config(None);
        for language in GameLanguage::ALL {
            let created = plan_launch_option_for_language(&without, language)
                .expect("insert")
                .updated_contents;
            assert!(created.contains(&format!("\"-language {}\"", language.suffix())));
            let plan = plan_language_restore(&created, None, language).expect("restore");
            assert_eq!(plan.updated_contents.as_deref(), Some(without.as_str()));
            assert!(!plan.kept_user_change);
            if language == GameLanguage::Russian {
                continue;
            }
            let switched = plan_launch_option_for_language(&foreign, language)
                .expect("switch")
                .updated_contents;
            assert!(switched.contains(&format!(
                "-novid -language {} +exec a.cfg",
                language.suffix()
            )));
            let plan = plan_language_restore(&switched, Some(before), language).expect("restore");
            assert_eq!(plan.updated_contents.as_deref(), Some(foreign.as_str()));
            assert!(!plan.kept_user_change);
        }
        // Minify's old `-language minify` is foreign to BetterFy: replaced on
        // apply, given back on restore.
        let minify = local_config(Some("-language minify -novid"));
        let applied = plan_launch_option_for_language(&minify, GameLanguage::Betterfy)
            .expect("apply")
            .updated_contents;
        assert!(applied.contains("\"-language betterfy -novid\""));
        let plan = plan_language_restore(
            &applied,
            Some("-language minify -novid"),
            GameLanguage::Betterfy,
        )
        .expect("restore");
        assert_eq!(plan.updated_contents.as_deref(), Some(minify.as_str()));
    }

    #[test]
    fn rejects_nesting_deeper_than_steam_writes() {
        let nested = |depth: usize| format!("{}{}", "\"a\" { ".repeat(depth), "} ".repeat(depth));
        assert!(parse(&nested(MAX_DEPTH)).is_ok());
        assert_eq!(
            parse(&nested(MAX_DEPTH + 1)).err().as_deref(),
            Some("steam_config_invalid")
        );
    }

    #[test]
    fn reads_the_dota_language_from_the_app_manifest() {
        let manifest = "\"AppState\"\n{\n\t\"appid\"\t\t\"570\"\n\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"russian\"\n\t}\n\t\"MountedConfig\"\n\t{\n\t\t\"language\"\t\t\"english\"\n\t}\n}\n";
        assert_eq!(
            app_manifest_language(manifest),
            Ok(Some("russian".to_string()))
        );
        assert_eq!(
            app_manifest_language("\"AppState\" { \"appid\" \"570\" }"),
            Ok(None)
        );
        assert_eq!(
            app_manifest_language("\"AppState\" { \"appid\" \"570\"").err(),
            Some("steam_manifest_invalid".to_string())
        );
        assert_eq!(
            app_manifest_language("\"UserLocalConfigStore\" { }").err(),
            Some("steam_manifest_invalid".to_string())
        );
    }

    #[test]
    fn appends_owned_argument_without_reformatting_the_file() {
        let input = local_config(Some("-novid +exec autoexec.cfg"));
        let plan = plan_managed_launch_option(&input).expect("plan");
        let expected = input.replacen(
            "-novid +exec autoexec.cfg",
            "-novid +exec autoexec.cfg -language dutch",
            1,
        );
        assert!(plan.changed);
        assert_eq!(plan.updated_contents, expected);
        assert_ne!(plan.before_sha256, plan.after_sha256);
    }

    #[test]
    fn inserts_launch_options_when_the_dota_entry_has_none() {
        let input = local_config(None);
        let plan = plan_managed_launch_option(&input).expect("plan");
        assert!(plan.changed);
        assert!(plan
            .updated_contents
            .contains("\"LaunchOptions\"\t\t\"-language dutch\""));
        parse(&plan.updated_contents).expect("updated VDF remains valid");
    }

    #[test]
    fn accepts_an_already_managed_language_without_a_write() {
        let input = local_config(Some("-novid -language DUTCH"));
        let plan = plan_managed_launch_option(&input).expect("plan");
        assert!(!plan.changed);
        assert_eq!(plan.updated_contents, input);
        assert_eq!(plan.before_sha256, plan.after_sha256);
    }

    #[test]
    fn selected_language_is_preserved_and_replaces_only_the_language_value() {
        let input = local_config(Some("-novid"));
        for language in GameLanguage::ALL {
            let plan = plan_launch_option_for_language(&input, language).expect("language plan");
            assert!(plan
                .updated_contents
                .contains(&format!("-novid -language {}", language.suffix())));
            let repeat = plan_launch_option_for_language(&plan.updated_contents, language)
                .expect("same language");
            assert!(!repeat.changed);
            for other in GameLanguage::ALL {
                if other != language {
                    let switched = plan_launch_option_for_language(&plan.updated_contents, other)
                        .expect("replace selected language");
                    assert!(switched.changed);
                    assert!(switched
                        .updated_contents
                        .contains(&format!("-novid -language {}", other.suffix())));
                }
            }
        }
    }

    #[test]
    fn replaces_a_foreign_language_without_touching_other_options() {
        let input = local_config(Some("-novid -language russian +exec autoexec.cfg"));
        let plan = plan_managed_launch_option(&input).expect("replace language");
        assert!(plan
            .updated_contents
            .contains("-novid -language dutch +exec autoexec.cfg"));
    }

    #[test]
    fn rejects_mixed_or_duplicate_language_arguments() {
        let mixed = local_config(Some("-language dutch -language russian"));
        assert_eq!(
            plan_managed_launch_option(&mixed).err().as_deref(),
            Some("launch_options_invalid")
        );
        let duplicate = local_config(Some("-language dutch -language DUTCH"));
        assert_eq!(
            plan_managed_launch_option(&duplicate).err().as_deref(),
            Some("launch_options_invalid")
        );
    }

    #[test]
    fn rejects_unbalanced_quotes_and_vdf_objects() {
        assert_eq!(
            plan_managed_launch_option("\"UserLocalConfigStore\" { \"broken")
                .err()
                .as_deref(),
            Some("steam_config_invalid")
        );
        assert_eq!(
            plan_managed_launch_option("\"UserLocalConfigStore\" {")
                .err()
                .as_deref(),
            Some("steam_config_invalid")
        );
    }

    #[test]
    fn rejects_missing_dota_configuration() {
        let input = "\"UserLocalConfigStore\" { \"Software\" { } }";
        assert_eq!(
            plan_managed_launch_option(input).err().as_deref(),
            Some("dota_config_missing")
        );
    }
}
