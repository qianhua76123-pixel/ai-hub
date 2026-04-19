//! Auto-detect subscription vs API mode per provider based on environment signals.
//! Called once on startup (or when user clicks "auto-detect").
//!
//! Detection logic:
//!   Claude Code:
//!     - OAuth token at ~/.claude/.credentials.json → SUBSCRIPTION (Pro / Max)
//!     - ANTHROPIC_API_KEY in env/keychain → API
//!     - Both → HYBRID (API falls back when subscription rate-limited)
//!     - Neither → unknown, leave alone
//!   Codex (OpenAI CLI):
//!     - OPENAI_API_KEY → API
//!     - ~/.codex/auth.json (ChatGPT login) → SUBSCRIPTION
//!   Cursor / Copilot: always SUBSCRIPTION (IDE bundled)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedMode {
    pub provider_id: String,
    pub provider_name: String,
    pub detected_mode: String,  // "api" | "subscription" | "hybrid" | "unknown"
    pub reason: String,         // Why we picked this mode
    pub suggested_monthly_usd: f64,  // Best-guess default plan price
    pub confidence: String,     // "high" | "medium" | "low"
}

pub fn detect_all() -> Vec<DetectedMode> {
    let home = dirs::home_dir();
    let mut results = Vec::new();

    // === Anthropic / Claude Code ===
    results.push(detect_anthropic(&home));

    // === OpenAI / Codex ===
    results.push(detect_openai(&home));

    // === Cursor ===
    if let Some(ref h) = home {
        if h.join(".cursor").exists() || h.join("Library/Application Support/Cursor").exists() {
            results.push(DetectedMode {
                provider_id: "cursor".into(),
                provider_name: "Cursor".into(),
                detected_mode: "subscription".into(),
                reason: "Cursor 使用内置订阅模型（非 API）".into(),
                suggested_monthly_usd: 20.0, // Cursor Pro
                confidence: "high".into(),
            });
        }
    }

    // === GitHub Copilot ===
    if let Some(ref h) = home {
        if h.join(".config/github-copilot").exists() || h.join("Library/Application Support/GitHub Copilot").exists() {
            results.push(DetectedMode {
                provider_id: "copilot".into(),
                provider_name: "GitHub Copilot".into(),
                detected_mode: "subscription".into(),
                reason: "Copilot 是订阅制，无公开 API".into(),
                suggested_monthly_usd: 10.0, // Copilot Pro
                confidence: "high".into(),
            });
        }
    }

    results
}

fn detect_anthropic(home: &Option<std::path::PathBuf>) -> DetectedMode {
    // Claude Code 的 OAuth token 在 macOS Keychain 的 "Claude Safe Storage" 服务下
    let has_oauth = check_keychain_exists("Claude Safe Storage")
        || home.as_ref()
            .map(|h| h.join(".claude/.credentials.json").exists()
                  || h.join(".claude/oauth.json").exists()
                  || has_anthropic_oauth_in_settings(h))
            .unwrap_or(false)
        // 如果 .claude 目录存在且有 projects 子目录（说明用过 Claude Code），高概率是订阅
        || home.as_ref().map(|h| h.join(".claude/projects").exists()).unwrap_or(false);

    let has_api_key = crate::keystore::registry_get("ANTHROPIC_API_KEY").is_some()
        || std::env::var("ANTHROPIC_API_KEY").is_ok();

    match (has_oauth, has_api_key) {
        (true, false) => DetectedMode {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic (Claude)".into(),
            detected_mode: "subscription".into(),
            reason: "检测到 Claude Code OAuth 凭据，未设置 ANTHROPIC_API_KEY → 订阅模式".into(),
            suggested_monthly_usd: 20.0,  // Claude Pro 默认，用户可改 Max
            confidence: "high".into(),
        },
        (true, true) => DetectedMode {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic (Claude)".into(),
            detected_mode: "hybrid".into(),
            reason: "同时存在 OAuth 和 API Key → 混合模式（订阅为主，超限走 API）".into(),
            suggested_monthly_usd: 20.0,
            confidence: "medium".into(),
        },
        (false, true) => DetectedMode {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic (Claude)".into(),
            detected_mode: "api".into(),
            reason: "仅配置了 ANTHROPIC_API_KEY，无 OAuth 凭据 → API 模式".into(),
            suggested_monthly_usd: 0.0,
            confidence: "high".into(),
        },
        (false, false) => DetectedMode {
            provider_id: "anthropic".into(),
            provider_name: "Anthropic (Claude)".into(),
            detected_mode: "unknown".into(),
            reason: "未检测到 Claude 凭据（可能未登录）".into(),
            suggested_monthly_usd: 0.0,
            confidence: "low".into(),
        },
    }
}

/// 检查 macOS Keychain 某个 service 是否有条目
#[cfg(target_os = "macos")]
fn check_keychain_exists(service: &str) -> bool {
    std::process::Command::new("security")
        .args(["find-generic-password", "-s", service])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn check_keychain_exists(_service: &str) -> bool { false }

fn has_anthropic_oauth_in_settings(home: &std::path::PathBuf) -> bool {
    // Claude Code settings.json 里如果有 account 字段或 authType=oauth 也算登录
    let settings = home.join(".claude/settings.json");
    if let Ok(content) = std::fs::read_to_string(&settings) {
        return content.contains("\"account\"")
            || content.contains("\"authType\":\"oauth\"")
            || content.contains("\"oauth\"");
    }
    false
}

fn detect_openai(home: &Option<std::path::PathBuf>) -> DetectedMode {
    let has_oauth = home.as_ref()
        .map(|h| h.join(".codex/auth.json").exists()
              || h.join(".codex/oauth.json").exists()
              || h.join(".config/openai/auth.json").exists())
        .unwrap_or(false);

    let has_api_key = crate::keystore::registry_get("OPENAI_API_KEY").is_some()
        || std::env::var("OPENAI_API_KEY").is_ok();

    match (has_oauth, has_api_key) {
        (true, false) => DetectedMode {
            provider_id: "openai".into(),
            provider_name: "OpenAI (ChatGPT / Codex)".into(),
            detected_mode: "subscription".into(),
            reason: "检测到 ChatGPT 登录凭据 → 订阅模式".into(),
            suggested_monthly_usd: 20.0,
            confidence: "high".into(),
        },
        (true, true) => DetectedMode {
            provider_id: "openai".into(),
            provider_name: "OpenAI (ChatGPT / Codex)".into(),
            detected_mode: "hybrid".into(),
            reason: "同时存在登录凭据和 API Key → 混合模式".into(),
            suggested_monthly_usd: 20.0,
            confidence: "medium".into(),
        },
        (false, true) => DetectedMode {
            provider_id: "openai".into(),
            provider_name: "OpenAI (ChatGPT / Codex)".into(),
            detected_mode: "api".into(),
            reason: "仅配置了 OPENAI_API_KEY → API 模式".into(),
            suggested_monthly_usd: 0.0,
            confidence: "high".into(),
        },
        (false, false) => DetectedMode {
            provider_id: "openai".into(),
            provider_name: "OpenAI (ChatGPT / Codex)".into(),
            detected_mode: "unknown".into(),
            reason: "未检测到 OpenAI 凭据".into(),
            suggested_monthly_usd: 0.0,
            confidence: "low".into(),
        },
    }
}

/// Apply detection results to DB (sets account_modes + backfills traffic.cost_mode)
pub fn apply_detection(db: &crate::db::Database, results: &[DetectedMode]) -> Result<usize, String> {
    let mut updated = 0;
    for r in results {
        if r.detected_mode == "unknown" { continue; }
        db.set_account_mode(&r.provider_id, &r.detected_mode, r.suggested_monthly_usd)
            .map_err(|e| e.to_string())?;
        updated += 1;
    }
    Ok(updated)
}
