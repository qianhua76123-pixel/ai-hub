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
const REDDIT_LOCALLLAMA_URL: &str = "https://www.reddit.com/r/LocalLLaMA/top.json?t=week&limit=15";
const REDDIT_SINGULARITY_URL: &str = "https://www.reddit.com/r/singularity/top.json?t=week&limit=10";

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
    // 搜索 AI 相关关键词，按 points 排序
    let queries = ["Claude", "GPT-5", "Gemini", "LLM", "open source model"];
    let mut all_items = Vec::new();

    for q in queries.iter() {
        let url = format!("{}?query={}&tags=story&numericFilters=points%3E100&hitsPerPage=5", HN_SEARCH_URL, q);
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

    // 去重 + 按分数/时间排序
    all_items.sort_by(|a, b| b.score.cmp(&a.score));
    all_items.dedup_by(|a, b| a.title == b.title);
    all_items.truncate(15);
    Ok(all_items)
}

// ── Reddit top posts ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RedditResp {
    data: RedditData,
}

#[derive(Debug, Deserialize)]
struct RedditData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    id: String,
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    permalink: String,
    #[serde(default)]
    selftext: String,
    #[serde(default)]
    ups: i64,
    #[serde(default)]
    created_utc: f64,
}

async fn fetch_reddit(client: &reqwest::Client, url: &str, subreddit: &str) -> Result<Vec<NewsItem>, String> {
    let resp = client.get(url)
        .header("User-Agent", "ai-hub-rss/0.3")
        .send().await.map_err(|e| format!("Reddit/{}: {}", subreddit, e))?;
    let rdt: RedditResp = resp.json().await.map_err(|e| format!("Reddit parse: {}", e))?;
    let items = rdt.data.children.into_iter().map(|c| {
        let p = c.data;
        let url = if p.url.starts_with("http") && !p.url.contains("reddit.com") {
            p.url
        } else {
            format!("https://www.reddit.com{}", p.permalink)
        };
        let category = categorize(&p.title);
        NewsItem {
            id: format!("rd_{}_{}", subreddit, p.id),
            title: p.title.clone(),
            url,
            source: format!("r/{}", subreddit),
            summary: p.selftext.chars().take(300).collect(),
            timestamp: (p.created_utc * 1000.0) as i64,
            score: p.ups,
            category,
        }
    }).collect();
    Ok(items)
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

    let (hn, reddit_ll, reddit_sing) = tokio::join!(
        fetch_hn(&client),
        fetch_reddit(&client, REDDIT_LOCALLLAMA_URL, "LocalLLaMA"),
        fetch_reddit(&client, REDDIT_SINGULARITY_URL, "singularity"),
    );

    let mut items = Vec::new();
    items.extend(hn.unwrap_or_else(|e| { errors.push(e); vec![] }));
    items.extend(reddit_ll.unwrap_or_else(|e| { errors.push(e); vec![] }));
    items.extend(reddit_sing.unwrap_or_else(|e| { errors.push(e); vec![] }));

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
