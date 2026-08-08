use crate::build_engine::OperationDiagnosticCounts;
use crate::content_store::ContentDiagnosticCounts;
use crate::runtime_control::RuntimeState;
use crate::steam_accounts::SteamProfileDiagnosticCounts;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticState {
    Ready,
    Attention,
    Blocked,
    Unsupported,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    code: &'static str,
    state: DiagnosticState,
    detail: &'static str,
    value: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnosticReport {
    schema_version: u32,
    app_version: &'static str,
    platform: &'static str,
    generated_at_ms: u64,
    overall: DiagnosticState,
    checks: Vec<DiagnosticCheck>,
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn severity(state: DiagnosticState) -> u8 {
    match state {
        DiagnosticState::Ready => 0,
        DiagnosticState::Attention => 1,
        DiagnosticState::Blocked => 2,
        DiagnosticState::Unsupported => 3,
    }
}

fn check(
    code: &'static str,
    state: DiagnosticState,
    detail: &'static str,
    value: usize,
) -> DiagnosticCheck {
    DiagnosticCheck {
        code,
        state,
        detail,
        value: value.min(u32::MAX as usize) as u32,
    }
}

pub fn assemble_report(
    platform_supported: bool,
    game_verified: bool,
    runtime: Result<RuntimeState, String>,
    steam_profiles: Result<SteamProfileDiagnosticCounts, String>,
    staging: Result<OperationDiagnosticCounts, String>,
    content: Result<ContentDiagnosticCounts, String>,
) -> SystemDiagnosticReport {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "other"
    };
    let mut checks = vec![if platform_supported {
        check("platform", DiagnosticState::Ready, "windows_supported", 1)
    } else {
        check(
            "platform",
            DiagnosticState::Unsupported,
            "development_only",
            0,
        )
    }];

    checks.push(if game_verified {
        check("game", DiagnosticState::Ready, "game_verified", 1)
    } else {
        check("game", DiagnosticState::Blocked, "game_missing", 0)
    });

    checks.push(match runtime {
        Ok(state) if state.patch_ready => {
            check("runtime", DiagnosticState::Ready, "runtime_ready", 0)
        }
        Ok(state) => check(
            "runtime",
            DiagnosticState::Attention,
            if state.dota_running {
                "dota_running"
            } else {
                "steam_running"
            },
            usize::from(state.steam_running) + usize::from(state.dota_running),
        ),
        Err(_) => check(
            "runtime",
            DiagnosticState::Attention,
            "runtime_unavailable",
            0,
        ),
    });

    checks.push(match steam_profiles {
        Ok(counts) if counts.total == 0 => check(
            "steam_profiles",
            DiagnosticState::Attention,
            "profiles_missing",
            0,
        ),
        Ok(counts) if counts.conflicts > 0 || counts.invalid > 0 => check(
            "steam_profiles",
            DiagnosticState::Attention,
            "profiles_need_attention",
            counts.total,
        ),
        Ok(counts) => check(
            "steam_profiles",
            DiagnosticState::Ready,
            if counts.already_managed > 0 {
                "profiles_managed"
            } else {
                "profiles_ready"
            },
            counts.total,
        ),
        Err(_) => check(
            "steam_profiles",
            DiagnosticState::Attention,
            "profiles_unavailable",
            0,
        ),
    });

    checks.push(match staging {
        Ok(counts) if counts.recoverable > 0 => check(
            "staging",
            DiagnosticState::Attention,
            "staging_recovery_available",
            counts.recoverable,
        ),
        Ok(counts) => check(
            "staging",
            DiagnosticState::Ready,
            if counts.total > 0 {
                "staging_history_ready"
            } else {
                "staging_clean"
            },
            counts.total,
        ),
        Err(_) => check(
            "staging",
            DiagnosticState::Attention,
            "staging_unavailable",
            0,
        ),
    });

    checks.push(match content {
        Ok(counts) if counts.verified_packages > 0 => check(
            "content",
            DiagnosticState::Ready,
            "content_store_ready",
            counts.verified_packages,
        ),
        Ok(_) => check("content", DiagnosticState::Ready, "content_store_empty", 0),
        Err(_) => check(
            "content",
            DiagnosticState::Attention,
            "content_store_invalid",
            0,
        ),
    });

    let overall = checks
        .iter()
        .map(|item| item.state)
        .max_by_key(|state| severity(*state))
        .unwrap_or(DiagnosticState::Attention);

    SystemDiagnosticReport {
        schema_version: REPORT_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION"),
        platform,
        generated_at_ms: timestamp_ms(),
        overall,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_runtime() -> RuntimeState {
        RuntimeState {
            platform_supported: true,
            steam_running: false,
            dota_running: false,
            patch_ready: true,
        }
    }

    #[test]
    fn missing_game_blocks_an_otherwise_ready_report() {
        let report = assemble_report(
            true,
            false,
            Ok(ready_runtime()),
            Ok(SteamProfileDiagnosticCounts {
                total: 1,
                ready: 1,
                already_managed: 0,
                conflicts: 0,
                invalid: 0,
            }),
            Ok(OperationDiagnosticCounts {
                total: 0,
                recoverable: 0,
            }),
            Ok(ContentDiagnosticCounts {
                verified_packages: 0,
            }),
        );
        assert_eq!(report.overall, DiagnosticState::Blocked);
        assert_eq!(report.checks.len(), 6);
    }

    #[test]
    fn recovery_and_profile_conflicts_require_attention() {
        let report = assemble_report(
            true,
            true,
            Ok(ready_runtime()),
            Ok(SteamProfileDiagnosticCounts {
                total: 2,
                ready: 1,
                already_managed: 0,
                conflicts: 1,
                invalid: 0,
            }),
            Ok(OperationDiagnosticCounts {
                total: 2,
                recoverable: 1,
            }),
            Ok(ContentDiagnosticCounts {
                verified_packages: 1,
            }),
        );
        assert_eq!(report.overall, DiagnosticState::Attention);
    }

    #[test]
    fn serialized_report_contains_only_the_public_diagnostic_contract() {
        let report = assemble_report(
            true,
            true,
            Ok(ready_runtime()),
            Ok(SteamProfileDiagnosticCounts {
                total: 1,
                ready: 1,
                already_managed: 0,
                conflicts: 0,
                invalid: 0,
            }),
            Ok(OperationDiagnosticCounts {
                total: 0,
                recoverable: 0,
            }),
            Ok(ContentDiagnosticCounts {
                verified_packages: 0,
            }),
        );
        let value = serde_json::to_value(report).expect("diagnostic report serializes");
        let object = value.as_object().expect("report is an object");
        let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "appVersion",
                "checks",
                "generatedAtMs",
                "overall",
                "platform",
                "schemaVersion",
            ]
        );
        let serialized = value.to_string().to_ascii_lowercase();
        for forbidden in ["path", "steamid", "account", "telegram", "token"] {
            assert!(!serialized.contains(forbidden));
        }
    }
}
