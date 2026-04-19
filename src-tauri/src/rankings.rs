//! Real-time model rankings fetcher
//! Sources:
//! 1. LMSYS Chatbot Arena — text / code / vision ELO scores
//! 2. Artificial Analysis — quality index + speed + price composite

use serde::{Deserialize, Serialize};

// api.wulong.dev 实际支持的榜单 (验证通过):
//   text / code / vision / document / search / text-to-image / image-edit /
//   text-to-video / image-to-video / video-edit
const ARENA_TEXT_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=text";
const ARENA_CODE_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=code";
const ARENA_VISION_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=vision";
const ARENA_DOCUMENT_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=document";
const ARENA_SEARCH_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=search";
const ARENA_IMAGE_URL: &str = "https://api.wulong.dev/arena-ai-leaderboards/v1/leaderboard?name=text-to-image";
const AA_MODELS_URL: &str = "https://api.artificialanalysis.ai/v2/models";

// ── Public types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedModel {
    pub rank: i32,
    pub name: String,
    pub provider: String,
    pub score: i64,
    /// "arena_text" | "arena_code" | "arena_vision" | "artificial_analysis"
    pub source: String,
    /// Optional category label (e.g. "flagship", "reasoning", "open-source")
    pub category: String,
    pub votes: i64,
    /// ±CI if available
    pub ci: i64,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsResult {
    pub arena_text: Vec<RankedModel>,
    pub arena_code: Vec<RankedModel>,
    pub arena_vision: Vec<RankedModel>,
    pub arena_document: Vec<RankedModel>,
    pub arena_search: Vec<RankedModel>,
    pub arena_image: Vec<RankedModel>,
    pub artificial_analysis: Vec<RankedModel>,
    pub fetched_at: String,
    pub errors: Vec<String>,
}

// ── Arena deserialization ───────────────────────────────────

#[derive(Debug, Deserialize)]
struct ArenaResp {
    #[allow(dead_code)]
    meta: serde_json::Value,
    models: Vec<ArenaEntry>,
}

#[derive(Debug, Deserialize)]
struct ArenaEntry {
    rank: i32,
    model: String,
    vendor: String,
    #[serde(default, deserialize_with = "string_or_null")]
    license: String,
    score: i64,
    #[serde(default)]
    ci: i64,
    #[serde(default)]
    votes: i64,
}

/// Accept both "string" and null for optional text fields (some leaderboards return null license)
fn string_or_null<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

// ── Artificial Analysis deserialization ─────────────────────

#[derive(Debug, Deserialize)]
struct AAModel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    creator: Option<AACreator>,
    #[serde(default, alias = "intelligenceIndex")]
    intelligence_index: Option<f64>,
    #[serde(default, alias = "eloScore")]
    elo_score: Option<i64>,
    #[serde(default)]
    license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AACreator {
    #[serde(default)]
    name: String,
}

// ── Helpers ─────────────────────────────────────────────────

fn infer_category(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("o1") || lower.contains("o3") || lower.contains("o4")
        || lower.contains("r1") || lower.contains("reasoner")
        || lower.contains("thinking")
    {
        return "reasoning".into();
    }
    if lower.contains("mini") || lower.contains("nano") || lower.contains("flash")
        || lower.contains("haiku") || lower.contains("lite")
    {
        return "fast".into();
    }
    if lower.contains("opus") || lower.contains("gpt-5") || lower.contains("grok-4")
        || lower.contains("gemini-3") || lower.contains("pro")
    {
        return "flagship".into();
    }
    if lower.contains("llama") || lower.contains("qwen") || lower.contains("mistral")
        || lower.contains("deepseek") || lower.contains("yi-")
    {
        return "open-source".into();
    }
    "general".into()
}

fn normalize_provider(vendor: &str) -> String {
    let v = vendor.to_lowercase();
    if v.contains("anthropic") || v.contains("claude") { return "Anthropic".into(); }
    if v.contains("openai") || v.contains("gpt") { return "OpenAI".into(); }
    if v.contains("google") || v.contains("gemini") || v.contains("deepmind") { return "Google".into(); }
    if v.contains("xai") || v.contains("grok") { return "xAI".into(); }
    if v.contains("deepseek") { return "DeepSeek".into(); }
    if v.contains("meta") || v.contains("llama") { return "Meta".into(); }
    if v.contains("alibaba") || v.contains("qwen") { return "Alibaba".into(); }
    if v.contains("moonshot") || v.contains("kimi") { return "Moonshot".into(); }
    if v.contains("zhipu") || v.contains("glm") { return "Zhipu AI".into(); }
    if v.contains("mistral") { return "Mistral".into(); }
    vendor.to_string()
}

// ── Fetch logic ─────────────────────────────────────────────

async fn fetch_arena(client: &reqwest::Client, url: &str, source: &str) -> Result<Vec<RankedModel>, String> {
    let resp = client.get(url).send().await.map_err(|e| format!("{source}: {e}"))?;
    let arena: ArenaResp = resp.json().await.map_err(|e| format!("{source} parse: {e}"))?;

    Ok(arena.models.into_iter().map(|m| RankedModel {
        rank: m.rank,
        name: m.model.clone(),
        provider: normalize_provider(&m.vendor),
        score: m.score,
        source: source.to_string(),
        category: infer_category(&m.model),
        votes: m.votes,
        ci: m.ci,
        license: m.license,
    }).collect())
}

async fn fetch_artificial_analysis(client: &reqwest::Client, api_key: Option<&str>) -> Result<Vec<RankedModel>, String> {
    let key = api_key.unwrap_or("");
    if key.is_empty() {
        return Err("Artificial Analysis API key not configured".into());
    }

    let resp = client.get(AA_MODELS_URL)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| format!("AA: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("AA: HTTP {}", resp.status()));
    }

    let models: Vec<AAModel> = resp.json().await.map_err(|e| format!("AA parse: {e}"))?;

    let mut ranked: Vec<RankedModel> = models.into_iter()
        .filter_map(|m| {
            let score = m.elo_score.or(m.intelligence_index.map(|i| (i * 10.0) as i64))?;
            Some(RankedModel {
                rank: 0, // will be assigned below
                name: m.name.clone(),
                provider: m.creator.as_ref().map(|c| normalize_provider(&c.name)).unwrap_or_default(),
                score,
                source: "artificial_analysis".into(),
                category: infer_category(&m.name),
                votes: 0,
                ci: 0,
                license: m.license.unwrap_or_default(),
            })
        })
        .collect();

    // Sort by score descending, assign ranks
    ranked.sort_by(|a, b| b.score.cmp(&a.score));
    for (i, m) in ranked.iter_mut().enumerate() {
        m.rank = (i + 1) as i32;
    }

    Ok(ranked)
}

/// Fetch all rankings from all sources.
/// `aa_api_key` can be None / empty to skip Artificial Analysis.
pub async fn fetch_all(aa_api_key: Option<String>) -> RankingsResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let mut errors = Vec::new();

    // 并行获取所有 Arena 榜单（全部经过探测验证可用）
    let (text, code, vision, doc, search, image) = tokio::join!(
        fetch_arena(&client, ARENA_TEXT_URL, "arena_text"),
        fetch_arena(&client, ARENA_CODE_URL, "arena_code"),
        fetch_arena(&client, ARENA_VISION_URL, "arena_vision"),
        fetch_arena(&client, ARENA_DOCUMENT_URL, "arena_document"),
        fetch_arena(&client, ARENA_SEARCH_URL, "arena_search"),
        fetch_arena(&client, ARENA_IMAGE_URL, "arena_image"),
    );

    let arena_text = text.unwrap_or_else(|e| { errors.push(e); vec![] });
    let arena_code = code.unwrap_or_else(|e| { errors.push(e); vec![] });
    let arena_vision = vision.unwrap_or_else(|e| { errors.push(e); vec![] });
    let arena_document = doc.unwrap_or_else(|e| { errors.push(e); vec![] });
    let arena_search = search.unwrap_or_else(|e| { errors.push(e); vec![] });
    let arena_image = image.unwrap_or_else(|e| { errors.push(e); vec![] });

    let aa_key = aa_api_key.as_deref().filter(|k| !k.is_empty());
    let artificial_analysis = fetch_artificial_analysis(&client, aa_key).await.unwrap_or_else(|e| { errors.push(e); vec![] });

    RankingsResult {
        arena_text,
        arena_code,
        arena_vision,
        arena_document,
        arena_search,
        arena_image,
        artificial_analysis,
        fetched_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_category_works() {
        assert_eq!(infer_category("o3-pro"), "reasoning");
        assert_eq!(infer_category("Claude Haiku 4.5"), "fast");
        assert_eq!(infer_category("Claude Opus 4.6"), "flagship");
        assert_eq!(infer_category("Llama 4 70B"), "open-source");
    }

    #[test]
    fn normalize_provider_works() {
        assert_eq!(normalize_provider("Anthropic"), "Anthropic");
        assert_eq!(normalize_provider("openai"), "OpenAI");
        assert_eq!(normalize_provider("Google DeepMind"), "Google");
    }
}
