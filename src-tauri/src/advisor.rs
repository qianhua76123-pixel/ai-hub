//! Subscription overlap detector + savings advisor
//! Core differentiation feature: analyzes user's subscriptions + usage,
//! detects overlapping/redundant subscriptions, and recommends savings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSubscription {
    pub id: String,
    pub provider_id: String,   // "anthropic" | "openai" | "cursor" | "copilot" | ...
    pub provider_name: String,
    pub plan_name: String,     // "Claude Pro", "ChatGPT Plus", "Cursor Pro"
    pub monthly_usd: f64,
    pub category: String,      // "chat" | "coding_ide" | "coding_cli" | "image" | ...
    pub billing_day: i32,      // 1-28
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub kind: String,            // "overlap" | "underused" | "cheaper_alternative" | "bundle"
    pub severity: String,        // "high" | "medium" | "low"
    pub title: String,
    pub description: String,
    pub monthly_savings_usd: f64,
    pub affected_subscriptions: Vec<String>, // subscription IDs
    pub action: String,          // "cancel" | "downgrade" | "switch_to"
    pub suggested_replacement: Option<String>,
}

/// Analyze user subscriptions and usage, return prioritized recommendations
pub fn analyze(
    subscriptions: &[UserSubscription],
    monthly_usage_by_provider: &std::collections::HashMap<String, (i64, f64)>, // (requests, api_cost)
) -> Vec<Recommendation> {
    let mut recs = Vec::new();

    // === Rule 1: Duplicate IDE coding assistants ===
    let ide_subs: Vec<&UserSubscription> = subscriptions.iter()
        .filter(|s| s.category == "coding_ide")
        .collect();
    if ide_subs.len() > 1 {
        let total_cost: f64 = ide_subs.iter().map(|s| s.monthly_usd).sum();
        let cheapest = ide_subs.iter().min_by(|a, b| a.monthly_usd.partial_cmp(&b.monthly_usd).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        let savings = total_cost - cheapest.monthly_usd;
        recs.push(Recommendation {
            kind: "overlap".into(),
            severity: "high".into(),
            title: format!("IDE 编程助手重叠 ({} 个)", ide_subs.len()),
            description: format!(
                "你同时订阅了 {}。这些工具功能高度重叠，90% 场景下只用到一个。建议保留最常用的一个。",
                ide_subs.iter().map(|s| s.plan_name.as_str()).collect::<Vec<_>>().join("、")
            ),
            monthly_savings_usd: savings,
            affected_subscriptions: ide_subs.iter().filter(|s| s.id != cheapest.id).map(|s| s.id.clone()).collect(),
            action: "cancel".into(),
            suggested_replacement: Some(cheapest.plan_name.clone()),
        });
    }

    // === Rule 2: Duplicate chat subscriptions ===
    let chat_subs: Vec<&UserSubscription> = subscriptions.iter()
        .filter(|s| s.category == "chat")
        .collect();
    if chat_subs.len() > 1 {
        let total: f64 = chat_subs.iter().map(|s| s.monthly_usd).sum();
        let cheapest = chat_subs.iter().min_by(|a, b| a.monthly_usd.partial_cmp(&b.monthly_usd).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        recs.push(Recommendation {
            kind: "overlap".into(),
            severity: "high".into(),
            title: format!("聊天订阅重叠 ({} 个)", chat_subs.len()),
            description: format!(
                "ChatGPT Plus / Claude Pro / Gemini Advanced 等聊天订阅功能相近，同时订阅多个的 ROI 很低。",
            ),
            monthly_savings_usd: total - cheapest.monthly_usd,
            affected_subscriptions: chat_subs.iter().filter(|s| s.id != cheapest.id).map(|s| s.id.clone()).collect(),
            action: "cancel".into(),
            suggested_replacement: None,
        });
    }

    // === Rule 3: Underused subscription (< 5% usage vs API equivalent) ===
    for sub in subscriptions {
        let (_requests, api_cost) = monthly_usage_by_provider.get(&sub.provider_id).copied().unwrap_or((0, 0.0));
        if sub.monthly_usd > 0.0 && api_cost < sub.monthly_usd * 0.05 {
            // Used less than 5% of what the subscription is worth
            recs.push(Recommendation {
                kind: "underused".into(),
                severity: "medium".into(),
                title: format!("{} 使用率极低", sub.plan_name),
                description: format!(
                    "过去 30 天你使用 {} 对应 API 价值仅 ${:.2}，远低于订阅费 ${:.0}。建议改用按需 API 调用。",
                    sub.provider_name, api_cost, sub.monthly_usd
                ),
                monthly_savings_usd: sub.monthly_usd - api_cost,
                affected_subscriptions: vec![sub.id.clone()],
                action: "cancel".into(),
                suggested_replacement: Some(format!("{} API 按量计费", sub.provider_name)),
            });
        }
    }

    // === Rule 4: Expensive subscription with cheaper alternatives ===
    for sub in subscriptions {
        if sub.provider_id == "openai" && sub.monthly_usd >= 20.0 {
            // ChatGPT Plus user — maybe GPT-4.1 Mini via API is enough
            let (_req, api_cost) = monthly_usage_by_provider.get("openai").copied().unwrap_or((0, 0.0));
            if api_cost < 10.0 {
                recs.push(Recommendation {
                    kind: "cheaper_alternative".into(),
                    severity: "low".into(),
                    title: format!("{} 可改用 Claude Sonnet", sub.plan_name),
                    description: "基于你的使用量，Claude Sonnet 4.6 ($3/$15 per M) 比 ChatGPT Plus 更经济，且代码能力更强。".into(),
                    monthly_savings_usd: sub.monthly_usd * 0.5,
                    affected_subscriptions: vec![sub.id.clone()],
                    action: "switch_to".into(),
                    suggested_replacement: Some("Claude Sonnet 4.6 API".into()),
                });
            }
        }
    }

    // Sort by severity then savings
    recs.sort_by(|a, b| {
        let sev = |s: &str| match s { "high" => 0, "medium" => 1, _ => 2 };
        sev(&a.severity).cmp(&sev(&b.severity))
            .then(b.monthly_savings_usd.partial_cmp(&a.monthly_savings_usd).unwrap_or(std::cmp::Ordering::Equal))
    });

    recs
}

/// Calculate AI tool stack total cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackCostEstimate {
    pub total_monthly_usd: f64,
    pub total_yearly_usd: f64,
    pub subscription_count: i32,
    pub breakdown: Vec<StackCostItem>,
    pub savings_if_optimized_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackCostItem {
    pub plan_name: String,
    pub provider_name: String,
    pub monthly_usd: f64,
    pub yearly_usd: f64,
    pub percent_of_total: f64,
}

pub fn estimate_stack_cost(
    subscriptions: &[UserSubscription],
    monthly_usage_by_provider: &std::collections::HashMap<String, (i64, f64)>,
) -> StackCostEstimate {
    let total_monthly: f64 = subscriptions.iter().map(|s| s.monthly_usd).sum();
    let breakdown: Vec<StackCostItem> = subscriptions.iter().map(|s| StackCostItem {
        plan_name: s.plan_name.clone(),
        provider_name: s.provider_name.clone(),
        monthly_usd: s.monthly_usd,
        yearly_usd: s.monthly_usd * 12.0,
        percent_of_total: if total_monthly > 0.0 { (s.monthly_usd / total_monthly) * 100.0 } else { 0.0 },
    }).collect();

    // Estimate savings based on running advisor rules
    let recs = analyze(subscriptions, monthly_usage_by_provider);
    let savings: f64 = recs.iter().map(|r| r.monthly_savings_usd).sum();

    StackCostEstimate {
        total_monthly_usd: total_monthly,
        total_yearly_usd: total_monthly * 12.0,
        subscription_count: subscriptions.len() as i32,
        breakdown,
        savings_if_optimized_usd: savings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_sub(id: &str, provider: &str, plan: &str, usd: f64, cat: &str) -> UserSubscription {
        UserSubscription {
            id: id.into(), provider_id: provider.into(), provider_name: provider.into(),
            plan_name: plan.into(), monthly_usd: usd, category: cat.into(),
            billing_day: 1, started_at: 0,
        }
    }

    #[test]
    fn detects_ide_overlap() {
        let subs = vec![
            make_sub("1", "cursor", "Cursor Pro", 20.0, "coding_ide"),
            make_sub("2", "copilot", "GitHub Copilot", 10.0, "coding_ide"),
        ];
        let recs = analyze(&subs, &HashMap::new());
        assert!(recs.iter().any(|r| r.kind == "overlap" && r.monthly_savings_usd > 0.0));
    }

    #[test]
    fn detects_underused() {
        let subs = vec![make_sub("1", "anthropic", "Claude Pro", 20.0, "chat")];
        let mut usage = HashMap::new();
        usage.insert("anthropic".to_string(), (5, 0.30)); // $0.30 api cost vs $20 sub
        let recs = analyze(&subs, &usage);
        assert!(recs.iter().any(|r| r.kind == "underused"));
    }

    #[test]
    fn no_overlap_single_subscription() {
        let subs = vec![make_sub("1", "anthropic", "Claude Pro", 20.0, "chat")];
        let mut usage = HashMap::new();
        usage.insert("anthropic".to_string(), (1000, 25.0));
        let recs = analyze(&subs, &usage);
        // Should not flag overlap or underused
        assert!(!recs.iter().any(|r| r.kind == "overlap"));
    }

    #[test]
    fn estimate_stack_cost_sums_correctly() {
        let subs = vec![
            make_sub("1", "anthropic", "Claude Pro", 20.0, "chat"),
            make_sub("2", "openai", "ChatGPT Plus", 20.0, "chat"),
            make_sub("3", "cursor", "Cursor Pro", 20.0, "coding_ide"),
        ];
        let est = estimate_stack_cost(&subs, &HashMap::new());
        assert_eq!(est.total_monthly_usd, 60.0);
        assert_eq!(est.total_yearly_usd, 720.0);
        assert_eq!(est.subscription_count, 3);
        assert!(est.savings_if_optimized_usd > 0.0); // Should detect chat overlap
    }
}
