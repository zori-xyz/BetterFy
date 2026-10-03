use keyring::v1::{Entry, Error as KeyringError};
use reqwest::blocking::{Client, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const AUTH_ORIGIN: &str = "https://betterfy-auth.zori-xyz.workers.dev";
const CREDENTIAL_SERVICE: &str = "app.betterfy.desktop";
const CREDENTIAL_ACCOUNT: &str = "telegram-refresh";
const DEVICE_ACCOUNT: &str = "device-public-id";
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
    pub telegram_linked: Option<bool>,
    /// Set by the auth service for logins listed in its developer setting.
    #[serde(default)]
    pub developer: bool,
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

#[derive(Clone)]
struct PendingChallenge {
    challenge_token: String,
    device_id: String,
}

#[derive(Default)]
pub struct AuthState {
    session: Mutex<Option<ActiveSession>>,
    pending_challenge: Mutex<Option<PendingChallenge>>,
    refresh_lock: Mutex<()>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceChallengeResponse {
    challenge_token: String,
    device_id: String,
    deep_link: String,
    expires_at: i64,
    poll_after_seconds: u64,
    match_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChallengeStart {
    deep_link: String,
    expires_at: i64,
    poll_after_seconds: u64,
    match_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChallengePoll {
    state: &'static str,
    profile: Option<AuthProfile>,
}

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

fn credential_entry(account: &str) -> Result<Entry, String> {
    Entry::new(CREDENTIAL_SERVICE, account).map_err(|_| "auth_vault_unavailable".to_string())
}

fn read_refresh_credential() -> Result<Option<String>, String> {
    match credential_entry(CREDENTIAL_ACCOUNT)?.get_password() {
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
    credential_entry(CREDENTIAL_ACCOUNT)?
        .set_password(value)
        .map_err(|_| "auth_vault_unavailable".to_string())
}

fn delete_refresh_credential() -> Result<(), String> {
    match credential_entry(CREDENTIAL_ACCOUNT)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(_) => Err("auth_vault_unavailable".to_string()),
    }
}

fn read_device_id() -> Result<Option<String>, String> {
    match credential_entry(DEVICE_ACCOUNT)?.get_password() {
        Ok(value)
            if value.len() == 43
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
        {
            Ok(Some(value))
        }
        Ok(_) => Err("auth_device_invalid".to_string()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(_) => Err("auth_vault_unavailable".to_string()),
    }
}

fn store_device_id(value: &str) -> Result<(), String> {
    if value.len() != 43
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("auth_response_invalid".to_string());
    }
    credential_entry(DEVICE_ACCOUNT)?
        .set_password(value)
        .map_err(|_| "auth_vault_unavailable".to_string())
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
        .session
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = Some(ActiveSession {
        profile: response.profile,
        access_token: response.session_token,
    });
    Ok(profile)
}

fn refresh_from_vault(
    state: &AuthState,
    stale_access_token: Option<&str>,
) -> Result<Option<AuthProfile>, String> {
    let _refresh_guard = state
        .refresh_lock
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?;
    if let Some(session) = state
        .session
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .clone()
    {
        if stale_access_token.is_none_or(|token| token != session.access_token) {
            return Ok(Some(session.profile.clone()));
        }
    }
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
            .session
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
        .session
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
    let first_token = access_token(state)?;
    let first = send(&first_token)?;
    if first.status().as_u16() != 401 {
        return Ok(first);
    }
    refresh_from_vault(state, Some(&first_token))?
        .ok_or_else(|| "auth_session_expired".to_string())?;
    send(&access_token(state)?)
}

fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    value.len() <= 254
        && value.is_ascii()
        && value == value.trim()
        && !local.is_empty()
        && local.len() <= 64
        && domain.contains('.')
        && !domain.contains('@')
        && !value.contains("..")
}

fn email_request(path: &str, payload: serde_json::Value) -> Result<Response, String> {
    client()?
        .post(format!("{AUTH_ORIGIN}{path}"))
        .json(&payload)
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())
}

fn authenticated_email_request(
    state: &AuthState,
    path: &str,
    payload: serde_json::Value,
) -> Result<Response, String> {
    let send = |token: &str| {
        client()?
            .post(format!("{AUTH_ORIGIN}{path}"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(&payload)
            .send()
            .map_err(|_| "auth_service_unavailable".to_string())
    };
    let first_token = access_token(state)?;
    let first = send(&first_token)?;
    if first.status().as_u16() != 401 {
        return Ok(first);
    }
    refresh_from_vault(state, Some(&first_token))?
        .ok_or_else(|| "auth_session_expired".to_string())?;
    send(&access_token(state)?)
}

fn auth_begin_email_blocking(email: String, language: String) -> Result<(), String> {
    if !valid_email(&email) || !matches!(language.as_str(), "ru" | "en") {
        return Err("auth_email_invalid".to_string());
    }
    let response = email_request(
        "/v1/auth/email/start",
        serde_json::json!({ "email": email, "language": language }),
    )?;
    if !response.status().is_success() {
        return Err("auth_email_unavailable".to_string());
    }
    Ok(())
}

fn auth_verify_email_blocking(
    email: String,
    code: String,
    state: &AuthState,
) -> Result<AuthProfile, String> {
    if !valid_email(&email) || code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("auth_code_invalid".to_string());
    }
    let response = email_request(
        "/v1/auth/email/verify",
        serde_json::json!({ "email": email, "code": code, "clientKind": "desktop", "credentialMode": "rotating-v1" }),
    )?;
    if response.status().as_u16() == 401 {
        return Err("auth_code_invalid".to_string());
    }
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    let credentials = response
        .json::<CredentialResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    commit_credentials(state, credentials)
}

fn valid_id_username(value: &str) -> bool {
    let mut chars = value.chars();
    value.len() >= 3
        && value.len() <= 24
        && chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn valid_id_password(value: &str) -> bool {
    value.len() >= 12 && value.len() <= 128 && !value.chars().any(|ch| ch.is_control())
}

fn id_credential_response(response: Response, state: &AuthState) -> Result<AuthProfile, String> {
    match response.status().as_u16() {
        200..=299 => {}
        401 => return Err("auth_id_invalid_credentials".to_string()),
        429 => return Err("auth_rate_limited".to_string()),
        _ => return Err("auth_id_unavailable".to_string()),
    }
    let credentials = response
        .json::<CredentialResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    commit_credentials(state, credentials)
}

fn auth_id_login_blocking(
    identifier: String,
    password: String,
    state: &AuthState,
) -> Result<AuthProfile, String> {
    if !(valid_email(&identifier) || valid_id_username(&identifier))
        || !valid_id_password(&password)
    {
        return Err("auth_id_invalid_credentials".to_string());
    }
    let response = email_request(
        "/v1/auth/id/login",
        serde_json::json!({ "identifier": identifier, "password": password, "clientKind": "desktop", "credentialMode": "rotating-v1" }),
    )?;
    id_credential_response(response, state)
}

fn auth_id_register_start_blocking(
    username: String,
    email: String,
    password: String,
    language: String,
) -> Result<(), String> {
    if !valid_id_username(&username)
        || !valid_email(&email)
        || !valid_id_password(&password)
        || !matches!(language.as_str(), "ru" | "en")
    {
        return Err("auth_id_invalid_request".to_string());
    }
    let response = email_request(
        "/v1/auth/id/register/start",
        serde_json::json!({ "username": username, "email": email, "password": password, "language": language }),
    )?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        409 => Err("auth_id_username_taken".to_string()),
        429 => Err("auth_rate_limited".to_string()),
        _ => Err("auth_id_unavailable".to_string()),
    }
}

fn auth_id_register_verify_blocking(
    email: String,
    code: String,
    state: &AuthState,
) -> Result<AuthProfile, String> {
    if !valid_email(&email) || code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("auth_code_invalid".to_string());
    }
    let response = email_request(
        "/v1/auth/id/register/verify",
        serde_json::json!({ "email": email, "code": code, "clientKind": "desktop", "credentialMode": "rotating-v1" }),
    )?;
    id_credential_response(response, state)
}

fn auth_link_email_start_blocking(
    email: String,
    language: String,
    state: &AuthState,
) -> Result<(), String> {
    if !valid_email(&email) || !matches!(language.as_str(), "ru" | "en") {
        return Err("auth_email_invalid".to_string());
    }
    let response = authenticated_email_request(
        state,
        "/v1/session/email/start",
        serde_json::json!({ "email": email, "language": language }),
    )?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        409 => Err("auth_email_already_linked".to_string()),
        429 => Err("auth_rate_limited".to_string()),
        _ => Err("auth_email_unavailable".to_string()),
    }
}

fn auth_link_email_verify_blocking(
    email: String,
    code: String,
    state: &AuthState,
) -> Result<String, String> {
    if !valid_email(&email) || code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("auth_code_invalid".to_string());
    }
    let response = authenticated_email_request(
        state,
        "/v1/session/email/verify",
        serde_json::json!({ "email": email, "code": code }),
    )?;
    match response.status().as_u16() {
        200..=299 => {}
        409 => return Err("auth_email_already_linked".to_string()),
        429 => return Err("auth_rate_limited".to_string()),
        _ => return Err("auth_code_invalid".to_string()),
    }
    let result = response
        .json::<serde_json::Value>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    result["emailHint"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "auth_response_invalid".to_string())
}

fn auth_email_identity_blocking(state: &AuthState) -> Result<serde_json::Value, String> {
    let response = authenticated_request(state, reqwest::Method::GET, "/v1/session/email")?;
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    response
        .json::<serde_json::Value>()
        .map_err(|_| "auth_response_invalid".to_string())
}

fn auth_verify_code_blocking(code: String, state: &AuthState) -> Result<AuthProfile, String> {
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
    commit_credentials(state, credentials)
}

fn auth_begin_device_challenge_blocking(
    state: &AuthState,
    app_version: &str,
) -> Result<DeviceChallengeStart, String> {
    let existing_device_id = read_device_id()?;
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let response = client()?
        .post(format!("{AUTH_ORIGIN}/v1/auth/device/challenges"))
        .json(&serde_json::json!({
            "deviceId": existing_device_id.as_deref(),
            "clientKind": "desktop",
            "credentialMode": "rotating-v1",
            "platform": platform,
            "appVersion": app_version
        }))
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())?;
    if response.status().as_u16() == 429 {
        return Err("auth_rate_limited".to_string());
    }
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    let challenge = response
        .json::<DeviceChallengeResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    if challenge.challenge_token.len() != 43
        || challenge.device_id.len() != 43
        || challenge.poll_after_seconds < 1
        || challenge.poll_after_seconds > 10
        || challenge.expires_at <= 0
        || !challenge
            .challenge_token
            .chars()
            .chain(challenge.device_id.chars())
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("auth_response_invalid".to_string());
    }
    if let Some(existing) = existing_device_id {
        if existing != challenge.device_id {
            return Err("auth_response_invalid".to_string());
        }
    } else {
        store_device_id(&challenge.device_id)?;
    }
    let expected_link = format!(
        "https://t.me/BeterFyBot?start=auth_{}",
        challenge.challenge_token
    );
    if challenge.deep_link != expected_link {
        return Err("auth_response_invalid".to_string());
    }
    if challenge
        .match_code
        .as_deref()
        .is_some_and(|code| code.len() != 2 || !code.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("auth_response_invalid".to_string());
    }
    *state
        .pending_challenge
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = Some(PendingChallenge {
        challenge_token: challenge.challenge_token,
        device_id: challenge.device_id,
    });
    Ok(DeviceChallengeStart {
        deep_link: challenge.deep_link,
        expires_at: challenge.expires_at,
        poll_after_seconds: challenge.poll_after_seconds,
        match_code: challenge.match_code,
    })
}

fn auth_poll_device_challenge_blocking(state: &AuthState) -> Result<DeviceChallengePoll, String> {
    let pending = state
        .pending_challenge
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .clone()
        .ok_or_else(|| "auth_challenge_unavailable".to_string())?;
    let response = client()?
        .post(format!("{AUTH_ORIGIN}/v1/auth/device/challenges/poll"))
        .json(&serde_json::json!({
            "challengeToken": pending.challenge_token,
            "deviceId": pending.device_id
        }))
        .send()
        .map_err(|_| "auth_service_unavailable".to_string())?;
    match response.status().as_u16() {
        200 => {
            let credentials = response
                .json::<CredentialResponse>()
                .map_err(|_| "auth_response_invalid".to_string())?;
            let profile = commit_credentials(state, credentials)?;
            *state
                .pending_challenge
                .lock()
                .map_err(|_| "auth_state_unavailable".to_string())? = None;
            Ok(DeviceChallengePoll {
                state: "confirmed",
                profile: Some(profile),
            })
        }
        202 => Ok(DeviceChallengePoll {
            state: "pending",
            profile: None,
        }),
        403 => {
            *state
                .pending_challenge
                .lock()
                .map_err(|_| "auth_state_unavailable".to_string())? = None;
            Ok(DeviceChallengePoll {
                state: "denied",
                profile: None,
            })
        }
        404 | 409 | 410 => {
            *state
                .pending_challenge
                .lock()
                .map_err(|_| "auth_state_unavailable".to_string())? = None;
            Ok(DeviceChallengePoll {
                state: "expired",
                profile: None,
            })
        }
        _ => Err("auth_service_unavailable".to_string()),
    }
}

fn auth_cancel_device_challenge_blocking(state: &AuthState) -> Result<(), String> {
    *state
        .pending_challenge
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = None;
    Ok(())
}

fn auth_restore_session_blocking(state: &AuthState) -> Result<Option<AuthProfile>, String> {
    if let Some(session) = state
        .session
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?
        .clone()
    {
        return Ok(Some(session.profile));
    }
    refresh_from_vault(state, None)
}

/// Re-reads the profile (entitlement, expiry) so a purchase made in the bot
/// shows up without restarting the app.
fn auth_profile_blocking(state: &AuthState) -> Result<AuthProfile, String> {
    let response = authenticated_request(state, reqwest::Method::GET, "/v1/session/profile")?;
    if !response.status().is_success() {
        return Err("auth_service_unavailable".to_string());
    }
    let profile = response
        .json::<AuthProfile>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    if profile.user_id.is_empty() || profile.display_name.is_empty() {
        return Err("auth_response_invalid".to_string());
    }
    let mut session = state
        .session
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())?;
    match session.as_mut() {
        Some(active) if active.profile.user_id == profile.user_id => {
            active.profile = profile.clone();
            Ok(profile)
        }
        _ => Err("auth_session_unavailable".to_string()),
    }
}

fn auth_fetch_avatar_blocking(state: &AuthState) -> Result<Option<AvatarPayload>, String> {
    let response = authenticated_request(state, reqwest::Method::GET, "/v1/session/avatar")?;
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

fn auth_list_sessions_blocking(state: &AuthState) -> Result<Vec<DeviceSession>, String> {
    let response = authenticated_request(state, reqwest::Method::GET, "/v1/session/devices")?;
    if !response.status().is_success() {
        return Err("auth_sessions_unavailable".to_string());
    }
    let payload = response
        .json::<DeviceSessionsResponse>()
        .map_err(|_| "auth_response_invalid".to_string())?;
    Ok(payload.sessions)
}

fn auth_revoke_device_blocking(session_id: String, state: &AuthState) -> Result<bool, String> {
    if session_id.len() < 32
        || session_id.len() > 36
        || !session_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err("auth_session_invalid".to_string());
    }
    // The access token lives 15 minutes; a Profile screen left open longer
    // must refresh it instead of failing the revoke.
    let response = authenticated_email_request(
        state,
        "/v1/session/devices/revoke",
        serde_json::json!({ "sessionId": session_id }),
    )?;
    if !response.status().is_success() {
        return Err("auth_session_revoke_failed".to_string());
    }
    Ok(response
        .json::<serde_json::Value>()
        .ok()
        .and_then(|value| value.get("ok").and_then(|ok| ok.as_bool()))
        .unwrap_or(false))
}

fn auth_logout_blocking(state: &AuthState) -> Result<(), String> {
    let token = state
        .session
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
        .session
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = None;
    *state
        .pending_challenge
        .lock()
        .map_err(|_| "auth_state_unavailable".to_string())? = None;
    Ok(())
}

// Every auth command performs network and credential-vault I/O. Tauri runs
// synchronous commands on the main thread, which would freeze the window for
// up to the client timeout, so each one runs on a blocking worker instead.
async fn run_blocking<T, F>(app: AppHandle, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AuthState) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || task(&app.state::<AuthState>()))
        .await
        .map_err(|_| "auth_worker_failed".to_string())?
}

#[tauri::command]
pub async fn auth_begin_email(
    app: AppHandle,
    email: String,
    language: String,
) -> Result<(), String> {
    run_blocking(app, move |_| auth_begin_email_blocking(email, language)).await
}

#[tauri::command]
pub async fn auth_verify_email(
    app: AppHandle,
    email: String,
    code: String,
) -> Result<AuthProfile, String> {
    run_blocking(app, move |state| {
        auth_verify_email_blocking(email, code, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_id_login(
    app: AppHandle,
    identifier: String,
    password: String,
) -> Result<AuthProfile, String> {
    run_blocking(app, move |state| {
        auth_id_login_blocking(identifier, password, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_id_register_start(
    app: AppHandle,
    username: String,
    email: String,
    password: String,
    language: String,
) -> Result<(), String> {
    run_blocking(app, move |_| {
        auth_id_register_start_blocking(username, email, password, language)
    })
    .await
}

#[tauri::command]
pub async fn auth_id_register_verify(
    app: AppHandle,
    email: String,
    code: String,
) -> Result<AuthProfile, String> {
    run_blocking(app, move |state| {
        auth_id_register_verify_blocking(email, code, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_link_email_start(
    app: AppHandle,
    email: String,
    language: String,
) -> Result<(), String> {
    run_blocking(app, move |state| {
        auth_link_email_start_blocking(email, language, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_link_email_verify(
    app: AppHandle,
    email: String,
    code: String,
) -> Result<String, String> {
    run_blocking(app, move |state| {
        auth_link_email_verify_blocking(email, code, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_email_identity(app: AppHandle) -> Result<serde_json::Value, String> {
    run_blocking(app, auth_email_identity_blocking).await
}

#[tauri::command]
pub async fn auth_verify_code(app: AppHandle, code: String) -> Result<AuthProfile, String> {
    run_blocking(app, move |state| auth_verify_code_blocking(code, state)).await
}

#[tauri::command]
pub async fn auth_begin_device_challenge(app: AppHandle) -> Result<DeviceChallengeStart, String> {
    let version = app.package_info().version.to_string();
    run_blocking(app, move |state| {
        auth_begin_device_challenge_blocking(state, &version)
    })
    .await
}

#[tauri::command]
pub async fn auth_poll_device_challenge(app: AppHandle) -> Result<DeviceChallengePoll, String> {
    run_blocking(app, auth_poll_device_challenge_blocking).await
}

#[tauri::command]
pub async fn auth_cancel_device_challenge(app: AppHandle) -> Result<(), String> {
    run_blocking(app, auth_cancel_device_challenge_blocking).await
}

#[tauri::command]
pub async fn auth_restore_session(app: AppHandle) -> Result<Option<AuthProfile>, String> {
    run_blocking(app, auth_restore_session_blocking).await
}

#[tauri::command]
pub async fn auth_profile(app: AppHandle) -> Result<AuthProfile, String> {
    run_blocking(app, auth_profile_blocking).await
}

/// Whether the signed-in account carries the developer mark. The profile is
/// re-read from the auth service each time, so removing a login from the
/// Worker setting takes effect without a new sign-in. Signed out, offline, or
/// any service error counts as not a developer.
pub async fn developer_access(app: AppHandle) -> bool {
    run_blocking(app, auth_profile_blocking)
        .await
        .map(|profile| profile.developer)
        .unwrap_or(false)
}

#[tauri::command]
pub async fn auth_fetch_avatar(app: AppHandle) -> Result<Option<AvatarPayload>, String> {
    run_blocking(app, auth_fetch_avatar_blocking).await
}

#[tauri::command]
pub async fn auth_list_sessions(app: AppHandle) -> Result<Vec<DeviceSession>, String> {
    run_blocking(app, auth_list_sessions_blocking).await
}

#[tauri::command]
pub async fn auth_revoke_device(app: AppHandle, session_id: String) -> Result<bool, String> {
    run_blocking(app, move |state| {
        auth_revoke_device_blocking(session_id, state)
    })
    .await
}

#[tauri::command]
pub async fn auth_logout(app: AppHandle) -> Result<(), String> {
    run_blocking(app, auth_logout_blocking).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_restore_reuses_the_in_memory_session() {
        let state = AuthState::default();
        let profile = AuthProfile {
            user_id: "bf-user".into(),
            display_name: "Tester".into(),
            username: None,
            access_tier: "early-access".into(),
            access_expires_at: None,
            access_plan: None,
            access_recurring: None,
            session_id: None,
            avatar_available: None,
            telegram_linked: None,
            developer: false,
        };
        *state.session.lock().expect("session lock") = Some(ActiveSession {
            profile: profile.clone(),
            access_token: "current-access-token".into(),
        });
        for stale_token in [None, Some("old-access-token")] {
            let restored = refresh_from_vault(&state, stale_token)
                .expect("reuse session without reading the vault")
                .expect("active profile");
            assert_eq!(restored.user_id, profile.user_id);
        }
    }

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
            telegram_linked: Some(true),
            developer: false,
        };
        let serialized = serde_json::to_string(&profile).expect("profile json");
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("refresh"));
    }

    #[test]
    fn public_challenge_contract_hides_the_device_binding() {
        let challenge = DeviceChallengeStart {
            deep_link: "https://t.me/BeterFyBot?start=auth_opaque".into(),
            expires_at: 123,
            poll_after_seconds: 2,
            match_code: Some("42".into()),
        };
        let serialized = serde_json::to_string(&challenge).expect("challenge json");
        assert!(!serialized.contains("deviceId"));
        assert!(!serialized.contains("challengeToken"));
        assert!(!serialized.contains("refreshToken"));
        assert!(serialized.contains("\"matchCode\":\"42\""));
    }
}
