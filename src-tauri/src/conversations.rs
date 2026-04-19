use crate::db::{Database, ConversationRecord};
use std::sync::Arc;

/// Scan Claude Code JSONL files for conversations
pub fn scan_claude_conversations(db: &Arc<Database>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let projects_dir = home.join(".claude/projects");
    if !projects_dir.exists() {
        return;
    }

    for entry in walkdir::WalkDir::new(&projects_dir)
        .max_depth(5)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            let mut conv_title = String::new();
            let mut conv_parts: Vec<String> = Vec::new();
            let mut conv_timestamp: i64 = 0;
            let mut conv_model = String::new();
            let mut conv_tokens: i64 = 0;
            let mut msg_count = 0;

            for line in content.lines() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    let role = val.get("message")
                        .and_then(|m| m.get("role"))
                        .and_then(|r| r.as_str())
                        .unwrap_or("");

                    let ts = val.get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);

                    if ts > conv_timestamp {
                        conv_timestamp = ts;
                    }

                    if role == "user" {
                        let text = extract_text_content(&val["message"]);
                        if !text.is_empty() {
                            if conv_title.is_empty() && text.len() > 2 {
                                conv_title = text.chars().take(80).collect::<String>();
                            }
                            conv_parts.push(format!("[用户] {}", text.chars().take(500).collect::<String>()));
                            msg_count += 1;
                        }
                    } else if role == "assistant" {
                        let text = extract_text_content(&val["message"]);
                        if !text.is_empty() {
                            conv_parts.push(format!("[助手] {}", text.chars().take(500).collect::<String>()));
                        }
                        if let Some(model) = val["message"].get("model").and_then(|m| m.as_str()) {
                            conv_model = model.to_string();
                        }
                        if let Some(usage) = val["message"].get("usage") {
                            let input = usage.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                            let output = usage.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                            conv_tokens += input + output;
                        }
                    }
                }
            }

            // Skip internal/automated model calls (haiku is used internally by Claude Code, not user-initiated)
            if conv_model.contains("haiku") { continue; }

            if msg_count > 0 && conv_timestamp > 0 {
                let file_id = path.to_string_lossy()
                    .replace('/', "_").replace('\\', "_").replace('.', "_");
                let conv_id = format!("claude_conv_{}", file_id);
                let content_text = conv_parts.join("\n---\n");

                let record = ConversationRecord {
                    id: conv_id,
                    source: "Claude Code".to_string(),
                    tool: "claude-code".to_string(),
                    title: if conv_title.is_empty() { "无标题对话".to_string() } else { conv_title },
                    content: content_text.chars().take(10000).collect(),
                    timestamp: conv_timestamp,
                    tokens: conv_tokens,
                    model: conv_model,
                };

                db.insert_conversation(&record).ok();
            }
        }
    }
}

/// Scan Cursor conversations
pub fn scan_cursor_conversations(db: &Arc<Database>) {
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

    let mut stmt = match conn.prepare(
        "SELECT conversationId, title, model, mode, updatedAt FROM conversation_summaries ORDER BY updatedAt DESC LIMIT 500"
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
            let (conv_id, title, model, mode, updated_at) = row;
            let record = ConversationRecord {
                id: format!("cursor_conv_{}", conv_id),
                source: "Cursor".to_string(),
                tool: "cursor".to_string(),
                title: title.unwrap_or_else(|| "Cursor 对话".to_string()),
                content: format!("模式: {}", mode.unwrap_or_else(|| "chat".to_string())),
                timestamp: updated_at,
                tokens: 0,
                model: model.unwrap_or_default(),
            };
            db.insert_conversation(&record).ok();
        }
    }
}

/// Scan Codex conversations
pub fn scan_codex_conversations(db: &Arc<Database>) {
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

    let mut stmt = match conn.prepare(
        "SELECT ts, feedback_log_body FROM logs
         WHERE feedback_log_body LIKE '%user_message%'
         ORDER BY ts DESC LIMIT 500"
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
            let preview = body.chars().take(200).collect::<String>();
            let record = ConversationRecord {
                id: format!("codex_conv_{}", ts),
                source: "Codex CLI".to_string(),
                tool: "codex".to_string(),
                title: preview.chars().take(80).collect(),
                content: body.chars().take(5000).collect(),
                timestamp: ts * 1000,
                tokens: 0,
                model: String::new(),
            };
            db.insert_conversation(&record).ok();
        }
    }
}

/// Extract text from message content (handles both string and array formats)
fn extract_text_content(message: &serde_json::Value) -> String {
    if let Some(content) = message.get("content") {
        if let Some(s) = content.as_str() {
            return s.to_string();
        }
        if let Some(arr) = content.as_array() {
            let mut text = String::new();
            for item in arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() { text.push('\n'); }
                        text.push_str(t);
                    }
                }
            }
            return text;
        }
    }
    String::new()
}

/// Scan all conversation sources
pub fn scan_all_conversations(db: &Arc<Database>) {
    scan_claude_conversations(db);
    scan_cursor_conversations(db);
    scan_codex_conversations(db);
}
