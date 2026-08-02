use serde::Serialize;
use sha2::{Digest, Sha256};

const MANAGED_LANGUAGE: &str = "dutch";
const DOTA_CONFIG_PATH: [&str; 7] = [
    "UserLocalConfigStore",
    "Software",
    "Valve",
    "Steam",
    "apps",
    "570",
    "LaunchOptions",
];

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
    nested: bool,
) -> Result<(ObjectValue, usize), String> {
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
                let (object, next) = parse_entries(contents, tokens, index + 1, true)?;
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
    let (root, consumed) = parse_entries(contents, &tokens, 0, false)?;
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

fn command_tokens(value: &str) -> Result<Vec<String>, String> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut current = Vec::new();
    let mut quoted = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => quoted = !quoted,
            b' ' | b'\t' if !quoted => {
                if !current.is_empty() {
                    tokens.push(
                        String::from_utf8(std::mem::take(&mut current))
                            .map_err(|_| "launch_options_invalid".to_string())?,
                    );
                }
            }
            byte => current.push(byte),
        }
        index += 1;
    }
    if quoted {
        return Err("launch_options_invalid".to_string());
    }
    if !current.is_empty() {
        tokens.push(String::from_utf8(current).map_err(|_| "launch_options_invalid".to_string())?);
    }
    Ok(tokens)
}

fn add_managed_argument(existing: &str) -> Result<String, String> {
    let tokens = command_tokens(existing)?;
    let mut managed_language_found = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.eq_ignore_ascii_case("-language") {
            let Some(language) = tokens.get(index + 1) else {
                return Err("launch_options_invalid".to_string());
            };
            if language.eq_ignore_ascii_case(MANAGED_LANGUAGE) {
                if managed_language_found {
                    return Err("launch_options_invalid".to_string());
                }
                managed_language_found = true;
            } else {
                return Err("launch_option_conflict".to_string());
            }
        }
    }
    if managed_language_found {
        return Ok(existing.to_string());
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
    Ok(format!("{existing}{separator}-language {MANAGED_LANGUAGE}"))
}

fn encode_vdf(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn plan_managed_launch_option(contents: &str) -> Result<LaunchOptionPlan, String> {
    let root = parse(contents)?;
    let app_path = &DOTA_CONFIG_PATH[..DOTA_CONFIG_PATH.len() - 1];
    let app = find_value(&root, app_path).ok_or_else(|| "dota_config_missing".to_string())?;
    let Value::Object(app) = app else {
        return Err("steam_config_invalid".to_string());
    };

    let updated_contents = match find_value(&root, &DOTA_CONFIG_PATH) {
        Some(Value::Text(value)) => {
            let updated = add_managed_argument(&value.decoded)?;
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
            let insertion =
                format!("{indentation}\t\"LaunchOptions\"\t\t\"-language {MANAGED_LANGUAGE}\"\n");
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
    fn rejects_a_foreign_language_instead_of_overwriting_it() {
        let input = local_config(Some("-novid -language russian"));
        assert_eq!(
            plan_managed_launch_option(&input).err().as_deref(),
            Some("launch_option_conflict")
        );
    }

    #[test]
    fn rejects_mixed_or_duplicate_language_arguments() {
        let mixed = local_config(Some("-language dutch -language russian"));
        assert_eq!(
            plan_managed_launch_option(&mixed).err().as_deref(),
            Some("launch_option_conflict")
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
