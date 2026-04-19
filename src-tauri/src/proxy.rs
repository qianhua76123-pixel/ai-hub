use crate::db::{Database, TrafficRecord};
use crate::health::HealthMonitor;
use crate::traffic::estimate_cost;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use futures::StreamExt;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, AllowOrigin};

#[derive(Clone)]
pub struct ProxyState {
    pub db: Arc<Database>,
    pub client: reqwest::Client,
    pub health: Arc<HealthMonitor>,
}

/// 根据请求路径判断目标提供商和真实 base URL
fn resolve_target(path: &str, headers: &HeaderMap) -> Option<(&'static str, &'static str, String)> {
    let routes: &[(&str, &str, &str, &str)] = &[
        ("/openai/",      "openai",      "OpenAI",      "https://api.openai.com"),
        ("/anthropic/",   "anthropic",   "Anthropic",   "https://api.anthropic.com"),
        ("/gemini/",      "google",      "Gemini",      "https://generativelanguage.googleapis.com"),
        ("/deepseek/",    "deepseek",    "DeepSeek",    "https://api.deepseek.com"),
        ("/kimi/",        "kimi",        "Kimi",        "https://api.moonshot.cn"),
        ("/qwen/",        "qwen",        "通义千问",     "https://dashscope.aliyuncs.com"),
        ("/zhipu/",       "zhipu",       "智谱GLM",     "https://open.bigmodel.cn"),
        ("/mistral/",     "mistral",     "Mistral",     "https://api.mistral.ai"),
        ("/groq/",        "groq",        "Groq",        "https://api.groq.com"),
        ("/siliconflow/", "siliconflow", "SiliconFlow", "https://api.siliconflow.cn"),
    ];

    for &(prefix, id, name, base) in routes {
        if let Some(rest) = path.strip_prefix(prefix.trim_end_matches('/')) {
            return Some((id, name, format!("{}{}", base, rest)));
        }
    }

    // /v1/... 兼容模式：根据 header 判断
    if path.starts_with("/v1/") {
        let auth = headers.get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if auth.contains("sk-ant-") || headers.contains_key("x-api-key") {
            return Some(("anthropic", "Anthropic", format!("https://api.anthropic.com{}", path)));
        }
        return Some(("openai", "OpenAI", format!("https://api.openai.com{}", path)));
    }

    None
}

/// 从非 streaming 响应 body 中提取 usage
fn extract_usage_from_body(body: &str, provider_id: &str) -> (String, i64, i64) {
    let val: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return ("unknown".to_string(), 0, 0),
    };

    let model = val.get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    match provider_id {
        "anthropic" => {
            let usage = val.get("usage");
            let input = usage.and_then(|u| u.get("input_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            let output = usage.and_then(|u| u.get("output_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            (model, input, output)
        }
        _ => {
            let usage = val.get("usage");
            let input = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            let output = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            (model, input, output)
        }
    }
}

/// 从 SSE streaming chunks 中提取 usage（最后一个 chunk 通常包含 usage）
fn extract_usage_from_sse_chunks(chunks: &str, provider_id: &str) -> (String, i64, i64) {
    let mut model = "unknown".to_string();
    let mut input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;

    // SSE format: "data: {...}\n\n"
    for line in chunks.lines() {
        let data = if let Some(d) = line.strip_prefix("data: ") { d } else { continue };
        if data == "[DONE]" { continue; }

        let val: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Extract model from first chunk
        if let Some(m) = val.get("model").and_then(|m| m.as_str()) {
            if m != "unknown" { model = m.to_string(); }
        }

        // Anthropic streaming: message_start has usage.input_tokens, message_delta has usage.output_tokens
        if provider_id == "anthropic" {
            if let Some(msg) = val.get("message") {
                if let Some(usage) = msg.get("usage") {
                    if let Some(t) = usage.get("input_tokens").and_then(|t| t.as_i64()) { input_tokens = t; }
                    if let Some(t) = usage.get("output_tokens").and_then(|t| t.as_i64()) { output_tokens = t; }
                }
                if let Some(m) = msg.get("model").and_then(|m| m.as_str()) { model = m.to_string(); }
            }
            if let Some(usage) = val.get("usage") {
                if let Some(t) = usage.get("input_tokens").and_then(|t| t.as_i64()) { if t > 0 { input_tokens = t; } }
                if let Some(t) = usage.get("output_tokens").and_then(|t| t.as_i64()) { if t > 0 { output_tokens = t; } }
            }
        } else {
            // OpenAI-compatible: usage in final chunk
            if let Some(usage) = val.get("usage") {
                if let Some(t) = usage.get("prompt_tokens").and_then(|t| t.as_i64()) { input_tokens = t; }
                if let Some(t) = usage.get("completion_tokens").and_then(|t| t.as_i64()) { output_tokens = t; }
            }
        }
    }

    (model, input_tokens, output_tokens)
}

/// Check if response is streaming (SSE)
fn is_streaming_response(headers: &reqwest::header::HeaderMap) -> bool {
    headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream") || ct.contains("application/x-ndjson"))
        .unwrap_or(false)
}

/// Extract project/branch metadata from request headers and body
fn extract_project_info(headers: &HeaderMap, body_bytes: &[u8]) -> (String, String, String) {
    let project = headers.get("x-ai-hub-project")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            if !body_bytes.is_empty() {
                if let Ok(body_val) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    if let Some(cwd) = body_val.get("metadata").and_then(|m| m.get("cwd")).and_then(|c| c.as_str()) {
                        return Some(cwd.rsplit('/').next().unwrap_or(cwd).to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_default();

    let git_branch = headers.get("x-ai-hub-branch")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let working_dir = headers.get("x-ai-hub-workdir")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    (project, git_branch, working_dir)
}

/// Map a provider to its fallback route prefix for auto-retry
fn get_fallback_route(original_provider: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // (provider_id, provider_name, base_url)
    // Only fallback between providers that use the same API format
    match original_provider {
        "openai" => Some(("anthropic", "Anthropic", "https://api.anthropic.com")),
        "anthropic" => Some(("openai", "OpenAI", "https://api.openai.com")),
        "deepseek" => Some(("openai", "OpenAI", "https://api.openai.com")),
        _ => None,
    }
}

fn should_retry(status: u16) -> bool {
    status == 429 || status >= 500
}

fn make_error_response(status: u16, msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"error":"{}"}}"#, msg.replace('"', "\\\"").chars().take(500).collect::<String>())))
        .unwrap_or_else(|_| Response::new(Body::from(r#"{"error":"internal"}"#)))
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let full_path = format!("{}{}", path, query);
    let headers = req.headers().clone();
    let method = req.method().clone();

    // 健康检查
    if path == "/health" || path == "/" {
        return Response::builder()
            .status(200)
            .body(Body::from(r#"{"status":"ok","service":"ai-hub-proxy"}"#))
            .unwrap_or_else(|_| Response::new(Body::from("{}")));
    }

    // 解析目标
    let (provider_id, provider_name, target_url) = match resolve_target(&full_path, &headers) {
        Some(t) => t,
        None => return make_error_response(404, "Unknown route. Use /openai/v1/..., /anthropic/v1/..., etc."),
    };

    // Budget check — block if over 100% and pause enabled
    if let Ok(budgets) = state.db.get_budgets() {
        let spend = state.db.get_monthly_spend().unwrap_or(0.0);
        for b in &budgets {
            let limit = b["monthly_limit_usd"].as_f64().unwrap_or(0.0);
            if limit > 0.0 && spend >= limit && b["pause_at_100"].as_bool() == Some(true) {
                return make_error_response(429, "AI Hub: Monthly budget exceeded. Proxy paused.");
            }
        }
    }

    // Circuit breaker check
    if !state.health.should_allow_request(provider_id) {
        if let Some(_fallback_id) = state.health.get_best_fallback(provider_id) {
            eprintln!("[CircuitBreaker] {} is open", provider_id);
        }
        return make_error_response(503, &format!("Provider {} is temporarily unavailable", provider_name));
    }

    let start = std::time::Instant::now();

    // 读取请求 body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return make_error_response(400, "Failed to read request body"),
    };

    // Extract project info before forwarding
    let (project, git_branch, working_dir) = extract_project_info(&headers, &body_bytes);

    // Helper to build a forward request (closure can be called multiple times for retries)
    let build_request = |target: &str, provider: &str| {
        let mut req = state.client.request(method.clone(), target);
        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if matches!(key_str.as_str(), "host" | "connection" | "content-length" | "transfer-encoding") { continue; }
            if key_str.starts_with("x-ai-hub-") { continue; }
            // For fallback: rewrite auth header if provider mismatch and env key available
            if provider != provider_id && (key_str == "authorization" || key_str == "x-api-key") {
                // Try to use the fallback provider's own key from registry
                let env_key = match provider {
                    "openai" => "OPENAI_API_KEY",
                    "anthropic" => "ANTHROPIC_API_KEY",
                    "deepseek" => "DEEPSEEK_API_KEY",
                    _ => { req = req.header(key.clone(), value.clone()); continue; }
                };
                if let Some(api_key) = crate::keystore::registry_get(env_key) {
                    if provider == "anthropic" {
                        req = req.header("x-api-key", api_key).header("anthropic-version", "2023-06-01");
                    } else {
                        req = req.header("Authorization", format!("Bearer {}", api_key));
                    }
                    continue;
                }
            }
            req = req.header(key.clone(), value.clone());
        }
        if !body_bytes.is_empty() {
            req = req.body(body_bytes.clone());
        }
        req
    };

    // 发送请求
    let forward_req = build_request(&target_url, provider_id);
    let response = match forward_req.send().await {
        Ok(r) => r,
        Err(e) => {
            let latency = start.elapsed().as_millis() as i64;
            state.health.record_request(provider_id, provider_name, latency, true, None, None);
            let record = TrafficRecord {
                id: format!("proxy_{}_{}_{}", provider_id, chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4().simple()),
                timestamp: chrono::Utc::now().timestamp_millis(),
                provider_id: provider_id.to_string(),
                model: "unknown".to_string(),
                endpoint: path,
                input_tokens: 0, output_tokens: 0,
                latency_ms: latency,
                status: "error".to_string(),
                estimated_cost: 0.0,
                source: format!("AI Hub Proxy → {}", provider_name),
                project, git_branch, working_dir,
            };
            state.db.insert_traffic(&record).ok();
            return make_error_response(502, &format!("Proxy error: {}", e));
        }
    };

    let mut resp_status = response.status();
    let mut resp_headers = response.headers().clone();
    let mut current_response = response;
    let mut fallback_used: Option<String> = None;

    // Auto-fallback: if error and fallback available, retry once
    if should_retry(resp_status.as_u16()) {
        if let Some((fb_id, fb_name, fb_base)) = get_fallback_route(provider_id) {
            // Only use fallback if it's healthy and has an API key
            let has_key = crate::keystore::registry_get(match fb_id {
                "openai" => "OPENAI_API_KEY",
                "anthropic" => "ANTHROPIC_API_KEY",
                "deepseek" => "DEEPSEEK_API_KEY",
                _ => "",
            }).is_some();
            if has_key && state.health.should_allow_request(fb_id) {
                let fb_target = format!("{}{}", fb_base, path.trim_start_matches(&format!("/{}", fb_id)).trim_start_matches(&format!("/{}", provider_id)));
                eprintln!("[Fallback] {} returned {}, retrying via {}", provider_id, resp_status.as_u16(), fb_name);
                if let Ok(fb_resp) = build_request(&fb_target, fb_id).send().await {
                    resp_status = fb_resp.status();
                    resp_headers = fb_resp.headers().clone();
                    current_response = fb_resp;
                    fallback_used = Some(fb_id.to_string());
                }
            }
        }
    }
    let response = current_response;

    // 提取 rate-limit headers
    let rl_remaining = resp_headers.get("x-ratelimit-remaining-tokens")
        .or_else(|| resp_headers.get("x-ratelimit-remaining-requests"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());
    let rl_reset = resp_headers.get("x-ratelimit-reset-tokens")
        .or_else(|| resp_headers.get("retry-after"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    let is_streaming = is_streaming_response(&resp_headers);

    if is_streaming {
        // ===== STREAMING RESPONSE (SSE) =====
        // Stream chunks directly to client while collecting for usage extraction
        let db = state.db.clone();
        let health = state.health.clone();
        let provider_id_owned = provider_id.to_string();
        let provider_name_owned = provider_name.to_string();
        let path_owned = path.clone();

        let mut upstream = response.bytes_stream();

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(64);

        // Spawn background task to forward chunks and collect usage
        tokio::spawn(async move {
            let mut collected = String::new();
            let start_time = start;

            while let Some(chunk_result) = upstream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Collect for usage extraction (only last ~8KB to save memory)
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        if collected.len() < 256_000 {
                            collected.push_str(&chunk_str);
                        } else {
                            // Keep only the tail for final usage extraction
                            let drain_len = collected.len().saturating_sub(128_000);
                            collected.drain(..drain_len);
                            collected.push_str(&chunk_str);
                        }

                        if tx.send(Ok(chunk)).await.is_err() { break; }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))).await;
                        break;
                    }
                }
            }
            drop(tx);

            // Extract usage from collected SSE data
            let latency = start_time.elapsed().as_millis() as i64;
            let (model, input_tokens, output_tokens) = extract_usage_from_sse_chunks(&collected, &provider_id_owned);
            let cost = estimate_cost(&model, input_tokens, output_tokens);

            let record_status = if resp_status.is_success() { "success" }
                else if resp_status.as_u16() == 429 { "rate_limited" }
                else { "error" };

            health.record_request(&provider_id_owned, &provider_name_owned, latency, !resp_status.is_success(), rl_remaining, rl_reset);

            let record = TrafficRecord {
                id: format!("proxy_{}_{}", provider_id_owned, chrono::Utc::now().timestamp_millis()),
                timestamp: chrono::Utc::now().timestamp_millis(),
                provider_id: provider_id_owned,
                model,
                endpoint: path_owned,
                input_tokens, output_tokens,
                latency_ms: latency,
                status: record_status.to_string(),
                estimated_cost: cost,
                source: format!("AI Hub Proxy → {}", provider_name_owned),
                project, git_branch, working_dir,
            };
            db.insert_traffic(&record).ok();
        });

        // Build streaming response
        let body = Body::from_stream(tokio_stream::wrappers::ReceiverStream::new(rx));
        let mut builder = Response::builder().status(resp_status);
        for (key, value) in resp_headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str == "transfer-encoding" || key_str == "content-length" { continue; }
            builder = builder.header(key.clone(), value.clone());
        }
        builder.body(body).unwrap_or_else(|_| Response::new(Body::empty()))

    } else {
        // ===== NON-STREAMING RESPONSE =====
        let resp_bytes = response.bytes().await.unwrap_or_default();
        let latency = start.elapsed().as_millis() as i64;

        state.health.record_request(provider_id, provider_name, latency, !resp_status.is_success(), rl_remaining, rl_reset);

        let resp_str = String::from_utf8_lossy(&resp_bytes);
        let (model, input_tokens, output_tokens) = extract_usage_from_body(&resp_str, provider_id);
        let cost = estimate_cost(&model, input_tokens, output_tokens);

        let record_status = if resp_status.is_success() { "success" }
            else if resp_status.as_u16() == 429 { "rate_limited" }
            else { "error" };

        let record = TrafficRecord {
            id: format!("proxy_{}_{}_{}", provider_id, chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4().simple()),
            timestamp: chrono::Utc::now().timestamp_millis(),
            provider_id: provider_id.to_string(),
            model,
            endpoint: path,
            input_tokens, output_tokens,
            latency_ms: latency,
            status: record_status.to_string(),
            estimated_cost: cost,
            source: match &fallback_used {
                Some(fb) => format!("AI Hub Proxy → {} (fallback from {})", fb, provider_id),
                None => format!("AI Hub Proxy → {}", provider_name),
            },
            project, git_branch, working_dir,
        };
        state.db.insert_traffic(&record).ok();

        // Budget alert check (fires notifications at 70/90/100%)
        if let (Ok(budgets), Ok(spend)) = (state.db.get_budgets(), state.db.get_monthly_spend()) {
            let cny_rate = crate::pricing::get_pricing_info().currency_rate_usd_to_cny;
            crate::notify::check_budget_and_notify(spend, &budgets, cny_rate);
        }

        let mut builder = Response::builder().status(resp_status);
        for (key, value) in resp_headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str == "transfer-encoding" || key_str == "content-length" { continue; }
            builder = builder.header(key.clone(), value.clone());
        }
        builder.body(Body::from(resp_bytes)).unwrap_or_else(|_| Response::new(Body::empty()))
    }
}

/// 启动代理服务器，支持端口冲突自动切换
pub async fn start_proxy(db: Arc<Database>, health: Arc<HealthMonitor>) -> u16 {
    let state = ProxyState {
        db,
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_default(),
        health,
    };

    // CORS: 只允许本地来源
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.as_bytes().starts_with(b"http://localhost")
                || origin.as_bytes().starts_with(b"http://127.0.0.1")
                || origin.as_bytes().starts_with(b"tauri://")
        }))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .fallback(any(proxy_handler))
        .layer(cors)
        .with_state(state);

    // Try ports: 23456, 23457, 23458, ..., 23465
    for port in 23456..=23465 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => {
                println!("AI Hub Proxy listening on http://127.0.0.1:{}", port);
                tokio::spawn(async move {
                    if let Err(e) = axum::serve(listener, app).await {
                        eprintln!("Proxy server error: {}", e);
                    }
                });
                return port;
            }
            Err(e) => {
                eprintln!("Port {} unavailable ({}), trying next...", port, e);
            }
        }
    }

    eprintln!("WARNING: Failed to bind any proxy port (23456-23465)");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_openai_route() {
        let headers = HeaderMap::new();
        let result = resolve_target("/openai/v1/chat/completions", &headers);
        assert!(result.is_some());
        let (id, _, url) = result.unwrap();
        assert_eq!(id, "openai");
        assert!(url.contains("api.openai.com/v1/chat/completions"));
    }

    #[test]
    fn resolve_anthropic_route() {
        let headers = HeaderMap::new();
        let result = resolve_target("/anthropic/v1/messages", &headers);
        let (id, _, url) = result.unwrap();
        assert_eq!(id, "anthropic");
        assert!(url.contains("api.anthropic.com/v1/messages"));
    }

    #[test]
    fn resolve_v1_with_anthropic_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "sk-ant-test123".parse().unwrap());
        let result = resolve_target("/v1/messages", &headers);
        let (id, _, _) = result.unwrap();
        assert_eq!(id, "anthropic");
    }

    #[test]
    fn resolve_v1_defaults_to_openai() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer sk-test".parse().unwrap());
        let result = resolve_target("/v1/chat/completions", &headers);
        let (id, _, _) = result.unwrap();
        assert_eq!(id, "openai");
    }

    #[test]
    fn resolve_unknown_returns_none() {
        let headers = HeaderMap::new();
        assert!(resolve_target("/unknown/endpoint", &headers).is_none());
    }

    #[test]
    fn extract_openai_usage() {
        let body = r#"{"model":"gpt-4o","usage":{"prompt_tokens":100,"completion_tokens":50}}"#;
        let (model, input, output) = extract_usage_from_body(body, "openai");
        assert_eq!(model, "gpt-4o");
        assert_eq!(input, 100);
        assert_eq!(output, 50);
    }

    #[test]
    fn extract_anthropic_usage() {
        let body = r#"{"model":"claude-sonnet-4-6","usage":{"input_tokens":200,"output_tokens":80}}"#;
        let (model, input, output) = extract_usage_from_body(body, "anthropic");
        assert_eq!(model, "claude-sonnet-4-6");
        assert_eq!(input, 200);
        assert_eq!(output, 80);
    }

    #[test]
    fn extract_sse_openai_usage() {
        let chunks = r#"data: {"model":"gpt-4o","choices":[{"delta":{"content":"hello"}}]}

data: {"model":"gpt-4o","usage":{"prompt_tokens":150,"completion_tokens":30}}

data: [DONE]
"#;
        let (model, input, output) = extract_usage_from_sse_chunks(chunks, "openai");
        assert_eq!(model, "gpt-4o");
        assert_eq!(input, 150);
        assert_eq!(output, 30);
    }

    #[test]
    fn extract_sse_anthropic_usage() {
        let chunks = r#"data: {"type":"message_start","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":500,"output_tokens":0}}}

data: {"type":"content_block_delta","delta":{"text":"hi"}}

data: {"type":"message_delta","usage":{"output_tokens":120}}
"#;
        let (model, input, output) = extract_usage_from_sse_chunks(chunks, "anthropic");
        assert_eq!(model, "claude-sonnet-4-6");
        assert_eq!(input, 500);
        assert_eq!(output, 120);
    }

    #[test]
    fn extract_usage_invalid_json() {
        let (model, input, output) = extract_usage_from_body("not json", "openai");
        assert_eq!(model, "unknown");
        assert_eq!(input, 0);
        assert_eq!(output, 0);
    }

    #[test]
    fn is_streaming_detects_sse() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "text/event-stream".parse().unwrap());
        assert!(is_streaming_response(&headers));
    }

    #[test]
    fn is_streaming_detects_non_streaming() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(!is_streaming_response(&headers));
    }
}
