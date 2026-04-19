use crate::db::{Database, TrafficRecord};
use crate::pricing;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Track last-scanned file sizes to enable incremental scanning
static SCAN_STATE: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// 价格表（每百万 token，美元）
struct ModelPricing {
    input: f64,
    output: f64,
}

fn get_pricing(model: &str) -> ModelPricing {
    match model {
        // OpenAI
        m if m.contains("gpt-5.4") && !m.contains("mini") => ModelPricing { input: 2.5, output: 10.0 },
        m if m.contains("gpt-5.4-mini") => ModelPricing { input: 0.15, output: 0.6 },
        m if m.contains("gpt-4o") && !m.contains("mini") => ModelPricing { input: 2.5, output: 10.0 },
        m if m.contains("gpt-4o-mini") => ModelPricing { input: 0.15, output: 0.6 },
        m if m.contains("gpt-4-turbo") => ModelPricing { input: 10.0, output: 30.0 },
        m if m.contains("o1") && !m.contains("mini") => ModelPricing { input: 15.0, output: 60.0 },
        m if m.contains("o3") && !m.contains("mini") => ModelPricing { input: 10.0, output: 40.0 },
        m if m.contains("o3-mini") || m.contains("o4-mini") || m.contains("o1-mini") => ModelPricing { input: 1.1, output: 4.4 },
        // Anthropic
        m if m.contains("opus") => ModelPricing { input: 15.0, output: 75.0 },
        m if m.contains("sonnet") => ModelPricing { input: 3.0, output: 15.0 },
        m if m.contains("haiku") => ModelPricing { input: 0.25, output: 1.25 },
        // Gemini
        m if m.contains("gemini-2.5-pro") => ModelPricing { input: 1.25, output: 10.0 },
        m if m.contains("gemini-2.5-flash") => ModelPricing { input: 0.15, output: 0.6 },
        m if m.contains("gemini-2.0") => ModelPricing { input: 0.1, output: 0.4 },
        // DeepSeek
        m if m.contains("deepseek") => ModelPricing { input: 0.27, output: 1.1 },
        // Mistral
        m if m.contains("mistral-large") => ModelPricing { input: 2.0, output: 6.0 },
        // Kimi / Moonshot
        m if m.contains("kimi") || m.contains("moonshot") => ModelPricing { input: 0.7, output: 0.7 },
        // 通义千问
        m if m.contains("qwen") => ModelPricing { input: 0.3, output: 0.6 },
        // 智谱
        m if m.contains("glm") => ModelPricing { input: 0.5, output: 0.5 },
        _ => ModelPricing { input: 1.0, output: 3.0 },
    }
}

pub fn estimate_cost(model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
    let pricing = get_pricing(model);
    (input_tokens as f64 * pricing.input + output_tokens as f64 * pricing.output) / 1_000_000.0
}

// ==================== Claude Code ====================

pub fn scan_claude_code_logs(db: &Arc<Database>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let projects_dir = home.join(".claude/projects");
    if !projects_dir.exists() {
        return;
    }

    let mut state_guard = SCAN_STATE.lock().unwrap();
    let scan_state = state_guard.get_or_insert_with(HashMap::new);

    for entry in walkdir::WalkDir::new(&projects_dir)
        .max_depth(5)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();

        // Skip if file size hasn't changed (incremental scanning)
        let current_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let last_size = scan_state.get(&path_str).copied().unwrap_or(0);
        if current_size == last_size && current_size > 0 {
            continue;
        }
        scan_state.insert(path_str, current_size);

        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if !line.contains("\"usage\"") || !line.contains("\"assistant\"") {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    parse_claude_log_entry(&val, db);
                }
            }
        }
    }
}

fn parse_claude_log_entry(val: &serde_json::Value, db: &Arc<Database>) {
    let message = match val.get("message") {
        Some(m) => m,
        None => return,
    };

    let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("");
    if role != "assistant" {
        return;
    }

    let stop_reason = message.get("stop_reason");
    if stop_reason.is_none() || stop_reason == Some(&serde_json::Value::Null) {
        return;
    }

    let usage = match message.get("usage") {
        Some(u) => u,
        None => return,
    };

    let model = message.get("model").and_then(|m| m.as_str()).unwrap_or("").to_string();
    if model.is_empty() || model.contains("haiku") {
        return; // Skip empty model and internal haiku calls
    }

    let input_tokens = usage.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
    let cache_creation = usage.get("cache_creation_input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
    let cache_read = usage.get("cache_read_input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
    let total_input = input_tokens + cache_creation + cache_read;
    let output_tokens = usage.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);

    if total_input == 0 && output_tokens == 0 {
        return;
    }

    let timestamp = val.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);

    if timestamp == 0 {
        return;
    }

    // 精确计算：区分 cache write / cache read / 正常 input
    let cost = pricing::calculate_cost_precise(&model, input_tokens, cache_creation, cache_read, output_tokens);
    let msg_id = message.get("id").and_then(|i| i.as_str()).unwrap_or("");
    let id = format!("claude_{}_{}", msg_id, timestamp);
    let stop = stop_reason.and_then(|s| s.as_str()).unwrap_or("end_turn");

    // Extract project from the JSONL file path: ~/.claude/projects/{project_path}/...
    let cwd = val.get("cwd").and_then(|c| c.as_str()).unwrap_or("");
    let project_name = if !cwd.is_empty() {
        cwd.rsplit('/').next().unwrap_or("").to_string()
    } else {
        String::new()
    };

    // Try to detect git branch for the working directory
    let branch = if !cwd.is_empty() {
        std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(cwd)
            .output()
            .ok()
            .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let record = TrafficRecord {
        id,
        timestamp,
        provider_id: "anthropic".to_string(),
        model,
        endpoint: format!("claude-code ({})", stop),
        input_tokens: total_input,
        output_tokens,
        latency_ms: 0,
        status: "success".to_string(),
        estimated_cost: cost,
        source: "Claude Code".to_string(),
        project: project_name,
        git_branch: branch,
        working_dir: cwd.to_string(),
    };

    db.insert_traffic(&record).ok();
}

// ==================== Codex CLI ====================

pub fn scan_codex_logs(db: &Arc<Database>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let codex_db = home.join(".codex/logs_1.sqlite");
    if !codex_db.exists() {
        return;
    }

    let conn = match rusqlite::Connection::open_with_flags(
        &codex_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return,
    };

    // 解析 "post sampling token usage" 日志行
    // 格式：...model=gpt-5.4}:run_turn: post sampling token usage turn_id=xxx total_usage_tokens=13874 ...
    let re_usage = Regex::new(
        r"model=([^\s\}]+)\}.*post sampling token usage turn_id=(\S+) total_usage_tokens=(\d+)"
    ).unwrap();

    let mut stmt = match conn.prepare(
        "SELECT ts, feedback_log_body FROM logs
         WHERE feedback_log_body LIKE '%post sampling token usage%'
         ORDER BY ts DESC"
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows = stmt.query_map([], |row| {
        let ts: i64 = row.get(0)?;
        let body: String = row.get(1)?;
        Ok((ts, body))
    });

    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (ts, body) = row;
            if let Some(caps) = re_usage.captures(&body) {
                let model = caps.get(1).unwrap().as_str().to_string();
                let turn_id = caps.get(2).unwrap().as_str().to_string();
                let total_tokens: i64 = caps.get(3).unwrap().as_str().parse().unwrap_or(0);

                if total_tokens == 0 {
                    continue;
                }

                // Codex ts 是秒级
                let timestamp = ts * 1000;
                // 粗略拆分：60% input, 40% output
                let input_tokens = (total_tokens as f64 * 0.6) as i64;
                let output_tokens = total_tokens - input_tokens;
                let cost = estimate_cost(&model, input_tokens, output_tokens);

                let id = format!("codex_{}_{}", turn_id, ts);

                let record = TrafficRecord {
                    id,
                    timestamp,
                    provider_id: "openai".to_string(),
                    model,
                    endpoint: "codex-cli".to_string(),
                    input_tokens,
                    output_tokens,
                    latency_ms: 0,
                    status: "success".to_string(),
                    estimated_cost: cost,
                    source: "Codex CLI".to_string(),
                    project: String::new(),
                    git_branch: String::new(),
                    working_dir: String::new(),
                };

                db.insert_traffic(&record).ok();
            }
        }
    }

    // 补充：从模型调用日志提取更多记录
    let re_model = Regex::new(
        r"model=([^\s\}]+)\}.*:run_turn:run_sampling_request"
    ).unwrap();

    let mut stmt2 = match conn.prepare(
        "SELECT ts, feedback_log_body FROM logs
         WHERE feedback_log_body LIKE '%run_sampling_request%'
         AND feedback_log_body LIKE '%input_tokens%'
         ORDER BY ts DESC"
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows2 = stmt2.query_map([], |row| {
        let ts: i64 = row.get(0)?;
        let body: String = row.get(1)?;
        Ok((ts, body))
    });

    let re_input = Regex::new(r"input_tokens[=:](\d+)").unwrap();
    let re_output = Regex::new(r"(?:output_tokens|completion_tokens)[=:](\d+)").unwrap();

    if let Ok(rows2) = rows2 {
        for row in rows2.flatten() {
            let (ts, body) = row;
            if let Some(model_cap) = re_model.captures(&body) {
                let model = model_cap.get(1).unwrap().as_str().to_string();
                let input = re_input.captures(&body)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<i64>().ok())
                    .unwrap_or(0);
                let output = re_output.captures(&body)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<i64>().ok())
                    .unwrap_or(0);

                if input == 0 && output == 0 {
                    continue;
                }

                let timestamp = ts * 1000;
                let cost = estimate_cost(&model, input, output);
                let id = format!("codex_detail_{}_{}", &body[..20.min(body.len())].replace(' ', ""), ts);

                let record = TrafficRecord {
                    id,
                    timestamp,
                    provider_id: "openai".to_string(),
                    model,
                    endpoint: "codex-cli".to_string(),
                    input_tokens: input,
                    output_tokens: output,
                    latency_ms: 0,
                    status: "success".to_string(),
                    estimated_cost: cost,
                    source: "Codex CLI".to_string(),
                    project: String::new(),
                    git_branch: String::new(),
                    working_dir: String::new(),
                };

                db.insert_traffic(&record).ok();
            }
        }
    }
}

// ==================== Cursor ====================

pub fn scan_cursor_logs(db: &Arc<Database>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let cursor_db = home.join(".cursor/ai-tracking/ai-code-tracking.db");
    if !cursor_db.exists() {
        return;
    }

    let conn = match rusqlite::Connection::open_with_flags(
        &cursor_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return,
    };

    // conversation_summaries: 每个对话记录
    let mut stmt = match conn.prepare(
        "SELECT conversationId, title, model, mode, updatedAt FROM conversation_summaries ORDER BY updatedAt DESC"
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows = stmt.query_map([], |row| {
        let conv_id: String = row.get(0)?;
        let title: Option<String> = row.get(1)?;
        let model: Option<String> = row.get(2)?;
        let mode: Option<String> = row.get(3)?;
        let updated_at: i64 = row.get(4)?;
        Ok((conv_id, title, model, mode, updated_at))
    });

    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (conv_id, _title, model, mode, updated_at) = row;
            let model_str = model.unwrap_or_else(|| "cursor-unknown".to_string());
            let mode_str = mode.unwrap_or_else(|| "chat".to_string());

            let id = format!("cursor_conv_{}", conv_id);
            // Cursor 不提供 token 数据，记录存在性
            let record = TrafficRecord {
                id,
                timestamp: updated_at,
                provider_id: "cursor".to_string(),
                model: model_str,
                endpoint: format!("cursor ({})", mode_str),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                status: "success".to_string(),
                estimated_cost: 0.0,
                source: "Cursor".to_string(),
                project: String::new(),
                git_branch: String::new(),
                working_dir: String::new(),
            };

            db.insert_traffic(&record).ok();
        }
    }

    // ai_code_hashes: AI 生成代码记录
    let mut stmt2 = match conn.prepare(
        "SELECT hash, model, fileName, fileExtension, createdAt FROM ai_code_hashes ORDER BY createdAt DESC"
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows2 = stmt2.query_map([], |row| {
        let hash: String = row.get(0)?;
        let model: Option<String> = row.get(1)?;
        let file_name: Option<String> = row.get(2)?;
        let _ext: Option<String> = row.get(3)?;
        let created_at: i64 = row.get(4)?;
        Ok((hash, model, file_name, created_at))
    });

    if let Ok(rows2) = rows2 {
        for row in rows2.flatten() {
            let (hash, model, file_name, created_at) = row;
            let model_str = model.unwrap_or_else(|| "cursor-unknown".to_string());
            let file_str = file_name.unwrap_or_default();

            let id = format!("cursor_code_{}", hash);
            let record = TrafficRecord {
                id,
                timestamp: created_at,
                provider_id: "cursor".to_string(),
                model: model_str,
                endpoint: format!("cursor-codegen ({})", file_str),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                status: "success".to_string(),
                estimated_cost: 0.0,
                source: "Cursor".to_string(),
                project: String::new(),
                git_branch: String::new(),
                working_dir: String::new(),
            };

            db.insert_traffic(&record).ok();
        }
    }
}

// ==================== 运行所有扫描器 ====================

pub fn scan_all_sources(db: &Arc<Database>) {
    scan_claude_code_logs(db);
    scan_codex_logs(db);
    scan_cursor_logs(db);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_cost_claude_sonnet() {
        let cost = estimate_cost("claude-sonnet-4-6", 1_000_000, 0);
        assert!((cost - 3.0).abs() < 0.01, "Sonnet input should be ~$3/M");
    }

    #[test]
    fn estimate_cost_gpt4o() {
        let cost = estimate_cost("gpt-4o", 1_000_000, 0);
        assert!((cost - 2.5).abs() < 0.01, "GPT-4o input should be ~$2.5/M");
    }

    #[test]
    fn estimate_cost_deepseek() {
        let cost = estimate_cost("deepseek-chat", 1_000_000, 0);
        assert!((cost - 0.27).abs() < 0.01, "DeepSeek input should be ~$0.27/M");
    }

    #[test]
    fn estimate_cost_zero_tokens() {
        let cost = estimate_cost("gpt-4o", 0, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn estimate_cost_unknown_model_has_default() {
        let cost = estimate_cost("totally-unknown-model", 1_000_000, 0);
        assert!(cost > 0.0, "Unknown model should use default pricing");
    }
}
