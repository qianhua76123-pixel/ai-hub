//! AI 新闻聚合模块
//! 数据源（全部无需鉴权）:
//!   - Hacker News Firebase API: 热门 AI 话题
//!   - AI Weekly RSS / Reddit r/LocalLLaMA top posts
//!   - Provider blog RSS (Anthropic/OpenAI/Google)
//!
//! 调用约束：
//!   - 每次 fetch 上限 30 条，本地 DB 缓存 30 分钟
//!   - 失败单源不影响整体，errors 返回给前端

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub source: String,       // "HackerNews" | "Anthropic" | "OpenAI" | "Reddit" ...
    pub summary: String,      // 前 300 字符
    pub timestamp: i64,       // ms
    pub score: i64,           // HN points / Reddit upvotes
    pub category: String,     // "release" | "benchmark" | "pricing" | "tool" | "discussion"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResult {
    pub items: Vec<NewsItem>,
    pub fetched_at: String,
    pub errors: Vec<String>,
}

const HN_SEARCH_URL: &str = "https://hn.algolia.com/api/v1/search";
// Reddit 对非浏览器请求返回 "Blocked" HTML，不再使用
// 改用 HuggingFace 博客 RSS + HN 扩大关键词覆盖
const HF_BLOG_RSS: &str = "https://huggingface.co/blog/feed.xml";

// ── Hacker News Algolia search ────────────────────────────────

#[derive(Debug, Deserialize)]
struct HNResp {
    hits: Vec<HNHit>,
}

#[derive(Debug, Deserialize)]
struct HNHit {
    #[serde(rename = "objectID")]
    object_id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default, rename = "story_text")]
    story_text: Option<String>,
    #[serde(default)]
    points: Option<i64>,
    #[serde(default)]
    created_at_i: Option<i64>,
}

async fn fetch_hn(client: &reqwest::Client) -> Result<Vec<NewsItem>, String> {
    // 扩大关键词覆盖，覆盖 Reddit 原本的讨论面
    let queries = [
        "Claude", "GPT-5", "GPT-6", "Gemini", "Grok",
        "LLM", "open source model", "DeepSeek", "Qwen",
        "MCP", "AI agent", "Cursor", "Claude Code",
    ];
    let mut all_items = Vec::new();

    // 分数门槛降低到 50（更多条目）
    for q in queries.iter() {
        let url = format!("{}?query={}&tags=story&numericFilters=points%3E50&hitsPerPage=3", HN_SEARCH_URL, urlencoding_minimal(q));
        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(hn) = resp.json::<HNResp>().await {
                for hit in hn.hits {
                    let title = hit.title.unwrap_or_default();
                    if title.is_empty() { continue; }
                    let url = hit.url.clone().unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={}", hit.object_id));
                    let summary = hit.story_text.unwrap_or_default().chars().take(300).collect::<String>();
                    let ts = hit.created_at_i.unwrap_or(0) * 1000;
                    let category = categorize(&title);
                    all_items.push(NewsItem {
                        id: format!("hn_{}", hit.object_id),
                        title,
                        url,
                        source: "HackerNews".into(),
                        summary,
                        timestamp: ts,
                        score: hit.points.unwrap_or(0),
                        category,
                    });
                }
            }
        }
    }

    all_items.sort_by(|a, b| b.score.cmp(&a.score));
    all_items.dedup_by(|a, b| a.title == b.title);
    all_items.truncate(30);
    Ok(all_items)
}

fn urlencoding_minimal(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_string() }
        else { format!("%{:02X}", c as u32) }
    }).collect()
}

/// Fetch HuggingFace blog RSS (reliable, high-quality AI posts)
async fn fetch_hf_blog(client: &reqwest::Client) -> Result<Vec<NewsItem>, String> {
    let resp = client.get(HF_BLOG_RSS)
        .header("User-Agent", "Mozilla/5.0 (compatible; ai-hub/0.3)")
        .send().await.map_err(|e| format!("HF: {}", e))?;
    let text = resp.text().await.map_err(|e| format!("HF text: {}", e))?;

    // Minimal RSS parser — extract <item>, <title>, <link>, <pubDate>, <description>
    let mut items = Vec::new();
    let parts: Vec<&str> = text.split("<item>").skip(1).take(20).collect();
    for (i, part) in parts.iter().enumerate() {
        let title = extract_tag(part, "title").unwrap_or_default();
        let link = extract_tag(part, "link").unwrap_or_default();
        let date = extract_tag(part, "pubDate").unwrap_or_default();
        let desc = extract_tag(part, "description").unwrap_or_default();
        if title.is_empty() { continue; }

        // Parse pubDate (RFC 2822)
        let ts = chrono::DateTime::parse_from_rfc2822(&date)
            .map(|dt| dt.timestamp_millis()).unwrap_or(0);

        // Strip CDATA and HTML
        let clean_desc = desc
            .replace("<![CDATA[", "").replace("]]>", "")
            .split('<').next().unwrap_or("").trim()
            .chars().take(300).collect::<String>();

        items.push(NewsItem {
            id: format!("hf_{}", i),
            title: clean_title(&title),
            url: link.trim().to_string(),
            source: "HuggingFace".into(),
            summary: clean_desc,
            timestamp: ts,
            score: 0,
            category: categorize(&title),
        });
    }
    Ok(items)
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(xml[start..start + end].to_string())
}

fn clean_title(s: &str) -> String {
    s.replace("<![CDATA[", "").replace("]]>", "").trim().to_string()
}

// ── Category inference ────────────────────────────────────────

fn categorize(title: &str) -> String {
    let t = title.to_lowercase();
    if t.contains("release") || t.contains("announce") || t.contains("launch") || t.contains("发布") {
        return "release".into();
    }
    if t.contains("benchmark") || t.contains("leaderboard") || t.contains("beats") || t.contains("sota") {
        return "benchmark".into();
    }
    if t.contains("pricing") || t.contains("price") || t.contains("cost") || t.contains("free") {
        return "pricing".into();
    }
    if t.contains("mcp") || t.contains("tool") || t.contains("agent") || t.contains("cli") {
        return "tool".into();
    }
    "discussion".into()
}

// ── Main fetch ────────────────────────────────────────────────

pub async fn fetch_all() -> NewsResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let mut errors = Vec::new();

    let (hn, hf) = tokio::join!(
        fetch_hn(&client),
        fetch_hf_blog(&client),
    );

    let mut items = Vec::new();
    items.extend(hn.unwrap_or_else(|e| { errors.push(e); vec![] }));
    items.extend(hf.unwrap_or_else(|e| { errors.push(e); vec![] }));

    // Sort by timestamp DESC
    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    items.truncate(40);

    NewsResult {
        items,
        fetched_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize_release() {
        assert_eq!(categorize("Anthropic announces Claude 5"), "release");
        assert_eq!(categorize("OpenAI launches new tier"), "release");
    }

    #[test]
    fn categorize_benchmark() {
        assert_eq!(categorize("GPT-5 beats Claude on SWE-bench"), "benchmark");
    }

    #[test]
    fn categorize_default_is_discussion() {
        assert_eq!(categorize("Anyone else using this?"), "discussion");
    }
}
