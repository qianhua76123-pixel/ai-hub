use crate::pricing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRecommendation {
    pub provider_id: String,
    pub model_id: String,
    pub model_name: String,
    pub reason: String,
    pub score: f64,
    pub cost_per_m_input: f64,
    pub arena_score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClassification {
    pub task_type: String,
    pub confidence: f64,
    pub recommendations: Vec<RouteRecommendation>,
}

/// 分析 prompt 特征，判断任务类型
fn classify_prompt(prompt: &str) -> (String, f64) {
    let lower = prompt.to_lowercase();
    let len = prompt.len();

    // 代码类关键词
    let code_signals = ["代码", "code", "函数", "function", "bug", "fix", "implement", "重构",
        "refactor", "debug", "compile", "编译", "class", "struct", "api", "接口",
        "def ", "fn ", "func ", "import ", "require", "module", "组件", "component"];
    let code_score: f64 = code_signals.iter()
        .filter(|kw| lower.contains(*kw)).count() as f64;

    // 推理/数学
    let reasoning_signals = ["证明", "推理", "数学", "计算", "公式", "算法", "math",
        "prove", "reason", "logic", "逻辑", "解方程", "优化问题"];
    let reason_score: f64 = reasoning_signals.iter()
        .filter(|kw| lower.contains(*kw)).count() as f64;

    // 写作/创意
    let writing_signals = ["写", "write", "文章", "article", "翻译", "translate",
        "总结", "summary", "邮件", "email", "文案", "copy", "小说", "故事"];
    let writing_score: f64 = writing_signals.iter()
        .filter(|kw| lower.contains(*kw)).count() as f64;

    // 对话/问答
    let chat_signals = ["什么是", "怎么", "如何", "为什么", "what", "how", "why",
        "解释", "explain", "告诉我", "帮我"];
    let chat_score: f64 = chat_signals.iter()
        .filter(|kw| lower.contains(*kw)).count() as f64;

    // 长文本倾向于代码或写作
    let length_bonus = if len > 2000 { 1.0 } else { 0.0 };

    let scores = vec![
        ("code", code_score + length_bonus),
        ("reasoning", reason_score),
        ("writing", writing_score),
        ("chat", chat_score),
    ];

    let (task_type, max_score) = scores.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(t, s)| (t.to_string(), *s))
        .unwrap_or(("chat".to_string(), 0.0));

    let total: f64 = scores.iter().map(|(_, s)| s).sum();
    let confidence = if total > 0.0 { max_score / total } else { 0.25 };

    (task_type, confidence.min(0.95))
}

/// 根据任务类型推荐模型
pub fn recommend_models(prompt: &str) -> TaskClassification {
    let (task_type, confidence) = classify_prompt(prompt);
    let models = pricing::get_all_model_prices();

    let mut scored: Vec<RouteRecommendation> = models.iter()
        .filter(|m| m.input_per_m > 0.0) // 排除免费模型（通常能力有限）
        .map(|m| {
            let mut score: f64 = m.arena_score as f64;

            // 根据任务类型加权
            match task_type.as_str() {
                "code" => {
                    score += m.swe_bench as f64 * 5.0;
                    score += m.aider_polyglot as f64 * 4.0;
                    score += m.humaneval as f64 * 3.0;
                    if m.category == "flagship" || m.category == "reasoning" { score += 100.0; }
                }
                "reasoning" => {
                    if m.category == "reasoning" { score += 300.0; }
                    score += m.humaneval as f64 * 2.0;
                }
                "writing" => {
                    if m.category == "flagship" { score += 200.0; }
                    // 大上下文窗口有利于长文写作
                    if m.context_window >= 200000 { score += 100.0; }
                }
                "chat" => {
                    // 快速模型更适合简单对话
                    if m.category == "fast" || m.category == "mini" { score += 150.0; }
                    // 性价比加分
                    let avg = (m.input_per_m + m.output_per_m) / 2.0;
                    if avg < 1.0 { score += 200.0; }
                    else if avg < 5.0 { score += 100.0; }
                }
                _ => {}
            }

            let reason = match task_type.as_str() {
                "code" if m.swe_bench > 60.0 => format!("SWE-bench {:.0}%，代码能力强", m.swe_bench),
                "code" if m.aider_polyglot > 50.0 => format!("Aider {:.0}%，多语言编程优秀", m.aider_polyglot),
                "reasoning" if m.category == "reasoning" => "推理专用模型，逻辑能力最强".to_string(),
                "writing" if m.context_window >= 200000 => format!("{}K 上下文，适合长文", m.context_window / 1000),
                "chat" if m.input_per_m < 1.0 => format!("${:.2}/M，性价比高", m.input_per_m),
                _ => format!("Arena {}, 综合能力强", m.arena_score),
            };

            RouteRecommendation {
                provider_id: m.provider.clone(),
                model_id: m.model_id.clone(),
                model_name: m.model_name.clone(),
                reason,
                score,
                cost_per_m_input: m.input_per_m,
                arena_score: m.arena_score,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    scored.truncate(5);

    TaskClassification {
        task_type,
        confidence,
        recommendations: scored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_code_prompt() {
        let (task_type, confidence) = classify_prompt("implement a function to parse JSON, fix the bug in the class struct");
        assert_eq!(task_type, "code");
        assert!(confidence > 0.0);
    }

    #[test]
    fn classify_reasoning_prompt() {
        let (task_type, _) = classify_prompt("请证明这个数学定理的逻辑推理过程");
        assert_eq!(task_type, "reasoning");
    }

    #[test]
    fn classify_writing_prompt() {
        let (task_type, _) = classify_prompt("帮我写一篇关于 AI 的文章");
        assert_eq!(task_type, "writing");
    }

    #[test]
    fn classify_chat_prompt() {
        let (task_type, _) = classify_prompt("什么是机器学习？");
        assert_eq!(task_type, "chat");
    }

    #[test]
    fn classify_empty_prompt_does_not_panic() {
        let (task_type, confidence) = classify_prompt("");
        assert_eq!(task_type, "chat");
        assert!(confidence >= 0.0 && confidence <= 1.0);
    }

    #[test]
    fn recommend_models_returns_results() {
        let result = recommend_models("help me write a function");
        assert!(!result.task_type.is_empty());
        assert!(result.confidence >= 0.0);
        // Should return some recommendations (may be 0 if no pricing data loaded)
    }
}
