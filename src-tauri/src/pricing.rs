use serde::{Deserialize, Serialize};
// Pricing cache uses RwLock (see PRICING_CACHE below)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    pub provider: String,
    pub provider_name: String,
    pub model_id: String,
    pub model_name: String,
    pub input_per_m: f64,
    pub output_per_m: f64,
    #[serde(default)]
    pub cache_write_per_m: f64,
    #[serde(default)]
    pub cache_read_per_m: f64,
    #[serde(default)]
    pub context_window: i64,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub arena_score: i64,
    #[serde(default)]
    pub swe_bench: f64,
    #[serde(default)]
    pub aider_polyglot: f64,
    #[serde(default)]
    pub humaneval: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub last_updated: String,
    #[serde(default = "default_rate")]
    pub currency_rate_usd_to_cny: f64,
    #[serde(default)]
    pub pricing_sources: std::collections::HashMap<String, String>,
    pub models: Vec<ModelPrice>,
}

fn default_rate() -> f64 { 7.2 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub provider: String,
    pub provider_name: String,
    pub plan_name: String,
    pub price_monthly_usd: f64,
    pub price_monthly_cny: f64,
    pub includes: String,
    pub api_equivalent_note: String,
}

fn get_pricing_path() -> std::path::PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
        .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
        .join("ai-hub/pricing.json")
}

static PRICING_CACHE: std::sync::RwLock<Option<PricingData>> = std::sync::RwLock::new(None);

/// 从外部 JSON 文件加载价格表（with cache）
fn load_pricing() -> PricingData {
    // Check cache first
    if let Ok(guard) = PRICING_CACHE.read() {
        if let Some(ref cached) = *guard {
            return cached.clone();
        }
    }
    // Load from disk
    let data = load_pricing_from_disk();
    // Store in cache
    if let Ok(mut guard) = PRICING_CACHE.write() {
        *guard = Some(data.clone());
    }
    data
}

fn load_pricing_from_disk() -> PricingData {
    let path = get_pricing_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<PricingData>(&content) {
            return data;
        }
    }
    PricingData {
        last_updated: "builtin".to_string(),
        currency_rate_usd_to_cny: 7.2,
        pricing_sources: std::collections::HashMap::new(),
        models: vec![],
    }
}

/// Invalidate pricing cache (call after save_pricing)
pub fn invalidate_cache() {
    if let Ok(mut guard) = PRICING_CACHE.write() {
        *guard = None;
    }
}

pub fn get_all_model_prices() -> Vec<ModelPrice> {
    load_pricing().models
}

pub fn get_pricing_info() -> PricingData {
    load_pricing()
}

/// 更新外部 pricing.json（从前端传入新数据）
pub fn save_pricing(data: &PricingData) -> Result<(), String> {
    let path = get_pricing_path();
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    invalidate_cache();
    Ok(())
}

pub fn get_subscription_plans() -> Vec<SubscriptionPlan> {
    vec![
        SubscriptionPlan {
            provider: "anthropic".into(), provider_name: "Anthropic".into(),
            plan_name: "Claude Pro".into(),
            price_monthly_usd: 20.0, price_monthly_cny: 144.0,
            includes: "Opus/Sonnet/Haiku 无限使用（有速率限制）".into(),
            api_equivalent_note: "Pro 订阅不按 token 计费，高频使用时远比 API 划算".into(),
        },
        SubscriptionPlan {
            provider: "anthropic".into(), provider_name: "Anthropic".into(),
            plan_name: "Claude Max (5x)".into(),
            price_monthly_usd: 100.0, price_monthly_cny: 720.0,
            includes: "5 倍 Pro 用量上限".into(),
            api_equivalent_note: "重度 Opus 用户首选".into(),
        },
        SubscriptionPlan {
            provider: "anthropic".into(), provider_name: "Anthropic".into(),
            plan_name: "Claude Max (20x)".into(),
            price_monthly_usd: 200.0, price_monthly_cny: 1440.0,
            includes: "20 倍 Pro 用量上限".into(),
            api_equivalent_note: "极限使用场景".into(),
        },
        SubscriptionPlan {
            provider: "openai".into(), provider_name: "OpenAI".into(),
            plan_name: "ChatGPT Plus".into(),
            price_monthly_usd: 20.0, price_monthly_cny: 144.0,
            includes: "GPT-4o / o3 / o4-mini 等，有速率限制".into(),
            api_equivalent_note: "日均 <50 条可能 API 更便宜".into(),
        },
        SubscriptionPlan {
            provider: "openai".into(), provider_name: "OpenAI".into(),
            plan_name: "ChatGPT Pro".into(),
            price_monthly_usd: 200.0, price_monthly_cny: 1440.0,
            includes: "无限 GPT-4o / o3 pro，含 Codex CLI 额度".into(),
            api_equivalent_note: "重度推理用户可节省大量 API 费用".into(),
        },
        SubscriptionPlan {
            provider: "cursor".into(), provider_name: "Cursor".into(),
            plan_name: "Cursor Pro".into(),
            price_monthly_usd: 20.0, price_monthly_cny: 144.0,
            includes: "500 次快速请求/月 + 无限慢速请求".into(),
            api_equivalent_note: "自带模型调用，不额外收 API 费".into(),
        },
        SubscriptionPlan {
            provider: "copilot".into(), provider_name: "GitHub".into(),
            plan_name: "GitHub Copilot".into(),
            price_monthly_usd: 10.0, price_monthly_cny: 72.0,
            includes: "代码补全 + Chat + Agent".into(),
            api_equivalent_note: "纯代码补全场景最经济".into(),
        },
        SubscriptionPlan {
            provider: "google".into(), provider_name: "Google".into(),
            plan_name: "Gemini Advanced".into(),
            price_monthly_usd: 19.99, price_monthly_cny: 144.0,
            includes: "Gemini 2.5 Pro + 2TB Google One 存储".into(),
            api_equivalent_note: "含存储套餐，轻度使用划算".into(),
        },
    ]
}

/// 精确计算费用 — 从外部 pricing.json 查价格
pub fn calculate_cost_precise(model: &str, input_tokens: i64, cache_write_tokens: i64, cache_read_tokens: i64, output_tokens: i64) -> f64 {
    let pricing = load_pricing();
    let model_lower = model.to_lowercase();

    let price = pricing.models.iter()
        .find(|p| {
            let pid = p.model_id.to_lowercase();
            // 精确匹配
            model_lower == pid
            // 或包含匹配
            || model_lower.contains(&pid)
            || pid.contains(&model_lower)
        })
        .or_else(|| {
            // 模糊匹配
            pricing.models.iter().find(|p| {
                (model_lower.contains("opus") && p.model_id.contains("opus"))
                    || (model_lower.contains("sonnet") && p.model_id.contains("sonnet"))
                    || (model_lower.contains("haiku") && p.model_id.contains("haiku"))
                    || (model_lower.contains("gpt-4o") && !model_lower.contains("mini") && p.model_id == "gpt-4o")
                    || (model_lower.contains("gpt-4o-mini") && p.model_id == "gpt-4o-mini")
                    || (model_lower.contains("deepseek-chat") && p.model_id == "deepseek-chat")
                    || (model_lower.contains("deepseek-reasoner") && p.model_id == "deepseek-reasoner")
            })
        });

    match price {
        Some(p) => {
            let cw_price = if p.cache_write_per_m > 0.0 { p.cache_write_per_m } else { p.input_per_m };
            let cr_price = if p.cache_read_per_m > 0.0 { p.cache_read_per_m } else { p.input_per_m };

            let input_cost = input_tokens as f64 * p.input_per_m / 1_000_000.0;
            let cache_w_cost = cache_write_tokens as f64 * cw_price / 1_000_000.0;
            let cache_r_cost = cache_read_tokens as f64 * cr_price / 1_000_000.0;
            let output_cost = output_tokens as f64 * p.output_per_m / 1_000_000.0;

            input_cost + cache_w_cost + cache_r_cost + output_cost
        }
        None => {
            // 未知模型：保守估计
            let total_input = input_tokens + cache_write_tokens + cache_read_tokens;
            total_input as f64 * 1.0 / 1_000_000.0 + output_tokens as f64 * 3.0 / 1_000_000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_cache_returns_data() {
        let info = get_pricing_info();
        assert!(info.currency_rate_usd_to_cny > 0.0);
    }

    #[test]
    fn subscription_plans_not_empty() {
        let plans = get_subscription_plans();
        assert!(!plans.is_empty());
        assert!(plans.iter().any(|p| p.provider == "anthropic"));
    }

    #[test]
    fn calculate_cost_precise_unknown_model() {
        let cost = calculate_cost_precise("totally-unknown", 1_000_000, 0, 0, 0);
        assert!(cost > 0.0);
    }

    #[test]
    fn invalidate_cache_reloads() {
        let _ = get_pricing_info();
        invalidate_cache();
        let info = get_pricing_info();
        assert!(info.currency_rate_usd_to_cny > 0.0);
    }
}
