use keyring::v1::{Entry, Error as KeyringError};
use reqwest::blocking::{Client, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;

const AUTH_ORIGIN: &str = "https://betterfy-auth.zori-xyz.workers.dev";
const CREDENTIAL_SERVICE: &str = "app.betterfy.desktop";
const CREDENTIAL_ACCOUNT: &str = "telegram-refresh";
const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProfile {
    pub user_id: String,
    pub display_name: String,
    pub username: Option<String>,
    pub access_tier: String,
    pub access_expires_at: Option<i64>,
    pub access_plan: Option<String>,
    pub access_recurring: Option<bool>,
    pub session_id: Option<String>,
    pub avatar_available: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialResponse {
    #[serde(flatten)]
    profile: AuthProfile,
    session_token: String,
    refresh_token: String,
}

#[derive(Clone)]
struct ActiveSession {
    profile: AuthProfile,
    access_token: String,
}

#[derive(Default)]
pub struct AuthState(Mutex<Option<ActiveSession>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarPayload {
    content_type: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSession {
    session_id: String,
    client_kind: String,
    created_at: i64,
    last_used_at: i64,
    expires_at: i64,
    current: bool,
}

#[derive(Deserialize)]
struct DeviceSessionsResponse {
    sessions: Vec<DeviceSession>,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("BetterFy Desktop/0.1")
        .build()
        .map_err(|_| "auth_client_unavailable".to_string())
}

fn credential_entry() -> Result<Entry, String> {
    Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT)
        .map_err(|_| "auth_vault_unavailable".to_string())
}

fn read_refresh_credential() -> Result<Option<String>, String> {
    match credential_entry()?.get_password() {
        Ok(value)
            if value.len() == 43
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
        {
            Ok(Some(value))
        }
        Ok(_) => Err("auth_vault_invalid".to_string()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(_) => Err("auth_vault_unavailable".to_string()),
    }
}

fn store_refresh_credential(value: &str) -> Result<(), String> {
    if value.len() != 43
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("auth_response_invalid".to_string());
    }
    credential_entry()?
        .set_password(value)
        .map_err(|_| "auth_vault_unavailable".to_string())
}

fn delete_refresh_credential() -> Result<(), String> {
    match credential_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(_) => Err("auth_vault_unavailable".to_string()),
    }
}

fn validate_credential_response(response: &CredentialResponse) -> Result<(), String> {
    if response.profile.user_id.is_empty()
        || response.profile.display_name.is_empty()
        || response.session_token.len() != 43
        || response.refresh_token.len() != 43
    {
        return Err("auth_response_invalid".to_string());
    }
    Ok(())
}

fn commit_credentials(
    state: &AuthState,
    response: CredentialResponse,
) -> Result<AuthProfile, String> {
    validate_credential_response(&response)?;
    store_refresh_credential(&response.refresh_token)?;
    let profile = response.profile.clone();
    *state
        .0
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = Some(ActiveSession {
        profile: response.profile,
        access_token: response.session_token,
    });
    Ok(profile)
}

fn refresh_from_vault(state: &AuthState) -> Result<Option<AuthProfile>, String> {
    let Some(refresh_token) = read_refresh_credential()? else {
        return Ok(None);
    };
    let response = client()?
        .post(format!("{AUTH_ORIGIN}/v1/auth/refresh"))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())?;
    if response.status().as_u16() == 401 {
        let _ = delete_refresh_credential();
        *state
            .0
            .lock()
            .map_err(|_| "auth_state_unavailable".to_string())? = None;
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    let credentials = response
        .json::<CredentialResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    commit_credentials(state, credentials).map(Some)
}

fn access_token(state: &AuthState) -> Result<String, String> {
    state
        .0
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .as_ref()
        .map(|session| session.access_token.clone())
        .ok_or_else(|| "auth_session_unavailable".to_string())
}

fn authenticated_request(
    state: &AuthState,
    method: reqwest::Method,
    path: &str,
) -> Result<Response, String> {
    let send = |token: &str| {
        client()?
            .request(method.clone(), format!("{AUTH_ORIGIN}{path}"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .map_err(|_| "auth_service_unavailable".to_string())
    };
    let first = send(&access_token(state)?)?;
    if first.status().as_u16() != 401 {
        return Ok(first);
    }
    refresh_from_vault(state)?.ok_or_else(|| "auth_session_expired".to_string())?;
    send(&access_token(state)?)
}

#[tauri::command]
pub fn auth_verify_code(
    code: String,
    state: tauri::State<'_, AuthState>,
) -> Result<AuthProfile, String> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("auth_code_invalid".to_string());
    }
    let response = client()?
        .post(format!("{AUTH_ORIGIN}/v1/auth/telegram/code"))
        .json(&serde_json::json!({
            "code": code,
            "clientKind": "desktop",
            "credentialMode": "rotating-v1"
        }))
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())?;
    if response.status().as_u16() == 401 {
        return Err("auth_code_invalid".to_string());
    }
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    let credentials = response
        .json::<CredentialResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    commit_credentials(&state, credentials)
}

#[tauri::command]
pub fn auth_restore_session(
    state: tauri::State<'_, AuthState>,
) -> Result<Option<AuthProfile>, String> {
    if let Some(session) = state
        .0
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .clone()
    {
        return Ok(Some(session.profile));
    }
    refresh_from_vault(&state)
}

#[tauri::command]
pub fn auth_fetch_avatar(
    state: tauri::State<'_, AuthState>,
) -> Result<Option<AvatarPayload>, String> {
    let response = authenticated_request(&state, reqwest::Method::GET, "/v1/session/avatar")?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err("auth_avatar_unavailable".to_string());
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| matches!(*value, "image/jpeg" | "image/png" | "image/webp"))
        .ok_or_else(|| "auth_avatar_invalid".to_string())?
        .to_string();
    let bytes = response
        .bytes()
        .map_err(|_| "auth_avatar_unavailable".to_string())?;
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err("auth_avatar_invalid".to_string());
    }
    Ok(Some(AvatarPayload {
        content_type,
        bytes: bytes.to_vec(),
    }))
}

#[tauri::command]
pub fn auth_list_sessions(
    state: tauri::State<'_, AuthState>,
) -> Result<Vec<DeviceSession>, String> {
    let response = authenticated_request(&state, reqwest::Method::GET, "/v1/session/devices")?;
    if !response.status().is_success() {
        return Err("auth_sessions_unavailable".to_string());
    }
    let payload = response
        .json::<DeviceSessionsResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    Ok(payload.sessions)
}

#[tauri::command]
pub fn auth_revoke_device(
    session_id: String,
    state: tauri::State<'_, AuthState>,
) -> Result<bool, String> {
    if session_id.len() < 32
        || session_id.len() > 36
        || !session_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err("auth_session_invalid".to_string());
    }
    let token = access_token(&state)?;
    let response = client()?
        .post(format!("{AUTH_ORIGIN}/v1/session/devices/revoke"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({ "sessionId": session_id }))
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())?;
    if !response.status().is_success() {
        return Err("auth_session_revoke_failed".to_string());
    }
    Ok(response
        .json::<serde_json::Value>()
        .ok()
        .and_then(|value| value.get("ok").and_then(|ok| ok.as_bool()))
        .unwrap_or(false))
}

#[tauri::command]
pub fn auth_logout(state: tauri::State<'_, AuthState>) -> Result<(), String> {
    let token = state
        .0
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .as_ref()
        .map(|session| session.access_token.clone());
    if let Some(token) = token {
        let _ = client()?
            .post(format!("{AUTH_ORIGIN}/v1/session/logout"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send();
    }
    delete_refresh_credential()?;
    *state
        .0
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_profile_serialization_never_contains_tokens() {
        let profile = AuthProfile {
            user_id: "bf-user".into(),
            display_name: "Tester".into(),
            username: Some("tester".into()),
            access_tier: "early-access".into(),
            access_expires_at: None,
            access_plan: None,
            access_recurring: Some(false),
            session_id: Some("11111111-1111-4111-8111-111111111111".into()),
            avatar_available: Some(true),
        };
        let serialized = serde_json::to_string(&profile).expect("profile json");
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("refresh"));
    }
}
