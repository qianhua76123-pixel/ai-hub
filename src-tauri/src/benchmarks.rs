//! Automated benchmark data fetcher
//! Sources:
//! 1. Arena AI (LMSYS) — text + code ELO scores (api.wulong.dev, no auth)
//! 2. OpenRouter — model pricing (openrouter.ai/api/v1/models, no auth)
//! 3. ExchangeRate-API — USD/CNY exchange rate (open.er-api.com, no auth)

use crate::pricing;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ARENA_TEXT_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=text";
const ARENA_CODE_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=code";
const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const EXCHANGE_RATE_URL: &str = "https://open.er-api.com/v6/latest/USD";

#[derive(Debug, Serialize, Deserialize)]
struct ArenaResponse {
    meta: serde_json::Value,
    models: Vec<ArenaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArenaModel {
    rank: i32,
    model: String,
    vendor: String,
    #[serde(default)]
    license: String,
    score: i64,
    #[serde(default)]
    ci: i64,
    #[serde(default)]
    votes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModel {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    context_length: i64,
    #[serde(default)]
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterPricing {
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    completion: String,
}

/// Mapping from arena model IDs to our normalized model IDs
fn normalize_arena_id(arena_id: &str) -> Option<&'static str> {
    let id = arena_id.to_lowercase();
    // Claude
    if id.contains("claude-opus-4-6") && !id.contains("thinking") { return Some("claude-opus-4-6"); }
    if id.contains("claude-sonnet-4-6") { return Some("claude-sonnet-4-6"); }
    if id.contains("claude-haiku-4.5") || id.contains("claude-haiku-4-5") { return Some("claude-haiku-4-5"); }
    // OpenAI
    if id.contains("gpt-5.4") && id.contains("high") { return Some("gpt-5.4"); }
    if id.contains("gpt-5.4") && !id.contains("mini") && !id.contains("nano") && !id.contains("pro") && !id.contains("high") { return Some("gpt-5.4"); }
    if id.contains("gpt-4.1") && !id.contains("mini") && !id.contains("nano") { return Some("gpt-4.1"); }
    if id.contains("gpt-4.1-mini") || id.contains("gpt-4.1 mini") { return Some("gpt-4.1-mini"); }
    if (id.starts_with("o3") || id.contains("/o3")) && !id.contains("mini") && !id.contains("pro") { return Some("o3"); }
    if id.contains("o4-mini") { return Some("o4-mini"); }
    // Google
    if id.contains("gemini-3.1-pro") { return Some("gemini-3.1-pro"); }
    if id.contains("gemini-2.5-pro") { return Some("gemini-2.5-pro"); }
    if id.contains("gemini-2.5-flash") { return Some("gemini-2.5-flash"); }
    if id.contains("gemini-3-pro") { return Some("gemini-3-pro"); }
    if id.contains("gemini-3-flash") { return Some("gemini-3-flash"); }
    // xAI
    if id.contains("grok-4") && !id.contains("fast") && !id.contains("reasoning") && !id.contains("multi") { return Some("grok-4"); }
    // DeepSeek
    if id.contains("deepseek") && (id.contains("v3") || id.contains("chat")) { return Some("deepseek-chat"); }
    if id.contains("deepseek") && (id.contains("r1") || id.contains("reasoner")) { return Some("deepseek-reasoner"); }
    // Chinese models
    if id.contains("qwen3.5") || id.contains("qwen-3.5") { return Some("qwen-3.5-max"); }
    if id.contains("glm-5") { return Some("glm-5.1"); }
    if id.contains("kimi") || id.contains("k2.5") { return Some("kimi-k2.5"); }

    None
}

/// Mapping from OpenRouter model IDs to our IDs
fn normalize_openrouter_id(or_id: &str) -> Option<&'static str> {
    let id = or_id.to_lowercase();
    if id.contains("claude-opus-4.6") && !id.contains("fast") { return Some("claude-opus-4-6"); }
    if id.contains("claude-sonnet-4.6") { return Some("claude-sonnet-4-6"); }
    if id.contains("claude-haiku-4.5") { return Some("claude-haiku-4-5"); }
    if id == "openai/gpt-5.4" { return Some("gpt-5.4"); }
    if id.contains("gpt-4.1") && !id.contains("mini") && !id.contains("nano") { return Some("gpt-4.1"); }
    if id.contains("gpt-4.1-mini") { return Some("gpt-4.1-mini"); }
    if id == "openai/o3" { return Some("o3"); }
    if id.contains("o4-mini") && !id.contains("deep") { return Some("o4-mini"); }
    if id.contains("gemini-3.1-pro") { return Some("gemini-3.1-pro"); }
    if id.contains("gemini-2.5-pro") && !id.contains("exp") { return Some("gemini-2.5-pro"); }
    if id.contains("gemini-2.5-flash") && !id.contains("lite") { return Some("gemini-2.5-flash"); }
    if id.contains("grok-4.20") && !id.contains("fast") && !id.contains("multi") && !id.contains("reasoning") { return Some("grok-4"); }
    if id.contains("deepseek/deepseek-chat") || id.contains("deepseek-v3") { return Some("deepseek-chat"); }
    if id.contains("deepseek/deepseek-r1") || id.contains("deepseek-reasoner") { return Some("deepseek-reasoner"); }
    None
}

/// Fetch all benchmark data and update pricing.json
pub async fn fetch_and_update() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut data = pricing::get_pricing_info();
    let mut updates: Vec<String> = Vec::new();

    // Build lookup by model_id
    let mut model_map: HashMap<String, usize> = HashMap::new();
    for (i, m) in data.models.iter().enumerate() {
        model_map.insert(m.model_id.clone(), i);
    }

    // 1. Arena Text ELO
    if let Ok(resp) = client.get(ARENA_TEXT_URL).send().await {
        if let Ok(arena) = resp.json::<ArenaResponse>().await {
            let mut count = 0;
            for am in &arena.models {
                if let Some(our_id) = normalize_arena_id(&am.model) {
                    if let Some(&idx) = model_map.get(our_id) {
                        data.models[idx].arena_score = am.score;
                        count += 1;
                    }
                }
            }
            let fetched = arena.meta.get("fetched_at").and_then(|v| v.as_str()).unwrap_or("?");
            updates.push(format!("Arena Text: {} 个模型已更新 (source: {})", count, fetched));
        }
    }

    // 2. Arena Code ELO — store in a new field or as supplementary data
    let mut code_scores: HashMap<String, i64> = HashMap::new();
    if let Ok(resp) = client.get(ARENA_CODE_URL).send().await {
        if let Ok(arena) = resp.json::<ArenaResponse>().await {
            for am in &arena.models {
                if let Some(our_id) = normalize_arena_id(&am.model) {
                    code_scores.insert(our_id.to_string(), am.score);
                }
            }
            updates.push(format!("Arena Code: {} 个编程评分已获取", code_scores.len()));
        }
    }

    // 3. OpenRouter Pricing — update prices from real market data
    if let Ok(resp) = client.get(OPENROUTER_MODELS_URL).send().await {
        if let Ok(or_data) = resp.json::<OpenRouterResponse>().await {
            let mut price_count = 0;
            for orm in &or_data.data {
                if let Some(our_id) = normalize_openrouter_id(&orm.id) {
                    if let Some(&idx) = model_map.get(our_id) {
                        if let Some(ref p) = orm.pricing {
                            let input = p.prompt.parse::<f64>().unwrap_or(0.0) * 1_000_000.0;
                            let output = p.completion.parse::<f64>().unwrap_or(0.0) * 1_000_000.0;
                            if input > 0.0 || output > 0.0 {
                                data.models[idx].input_per_m = input;
                                data.models[idx].output_per_m = output;
                                price_count += 1;
                            }
                        }
                        if orm.context_length > 0 {
                            data.models[idx].context_window = orm.context_length;
                        }
                    }
                }
            }
            updates.push(format!("OpenRouter: {} 个模型价格已更新 ({}+ 来源)", price_count, or_data.data.len()));
        }
    }

    // 4. Exchange rate
    if let Ok(resp) = client.get(EXCHANGE_RATE_URL).send().await {
        if let Ok(val) = resp.json::<serde_json::Value>().await {
            if let Some(rate) = val.get("rates").and_then(|r| r.get("CNY")).and_then(|c| c.as_f64()) {
                if rate > 5.0 && rate < 10.0 {
                    data.currency_rate_usd_to_cny = rate;
                    updates.push(format!("汇率: 1 USD = {:.4} CNY", rate));
                }
            }
        }
    }

    // Store code_scores in notes for display
    for model in data.models.iter_mut() {
        if let Some(&code_elo) = code_scores.get(model.model_id.as_str()) {
            // Encode code ELO into note field for frontend to parse
            let existing_note = model.note.split(" | code_elo:").next().unwrap_or(&model.note).to_string();
            model.note = format!("{} | code_elo:{}", existing_note, code_elo);
        }
    }

    data.last_updated = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    pricing::save_pricing(&data).map_err(|e| e.to_string())?;

    if updates.is_empty() {
        Err("未能获取任何数据，请检查网络连接".into())
    } else {
        Ok(updates.join("\n"))
    }
}
