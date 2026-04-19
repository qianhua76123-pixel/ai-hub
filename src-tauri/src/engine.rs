use crate::db::{Database, TaskRecord};
use crate::traffic::estimate_cost;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 支持的 Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub id: String,
    pub name: String,
    pub api_format: String,     // "openai" | "anthropic"
    pub base_url: String,
    pub env_key: String,        // 从哪个环境变量读 API key
    pub default_model: String,
}

fn get_providers() -> Vec<ProviderEndpoint> {
    vec![
        ProviderEndpoint {
            id: "openai".into(), name: "OpenAI".into(), api_format: "openai".into(),
            base_url: "https://api.openai.com/v1".into(), env_key: "OPENAI_API_KEY".into(),
            default_model: "gpt-4o".into(),
        },
        ProviderEndpoint {
            id: "anthropic".into(), name: "Anthropic".into(), api_format: "anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(), env_key: "ANTHROPIC_API_KEY".into(),
            default_model: "claude-sonnet-4-6".into(),
        },
        ProviderEndpoint {
            id: "deepseek".into(), name: "DeepSeek".into(), api_format: "openai".into(),
            base_url: "https://api.deepseek.com/v1".into(), env_key: "DEEPSEEK_API_KEY".into(),
            default_model: "deepseek-chat".into(),
        },
        ProviderEndpoint {
            id: "kimi".into(), name: "Kimi".into(), api_format: "openai".into(),
            base_url: "https://api.moonshot.cn/v1".into(), env_key: "MOONSHOT_API_KEY".into(),
            default_model: "moonshot-v1-128k".into(),
        },
        ProviderEndpoint {
            id: "qwen".into(), name: "通义千问".into(), api_format: "openai".into(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(), env_key: "DASHSCOPE_API_KEY".into(),
            default_model: "qwen-max".into(),
        },
        ProviderEndpoint {
            id: "zhipu".into(), name: "智谱GLM".into(), api_format: "openai".into(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(), env_key: "ZHIPU_API_KEY".into(),
            default_model: "glm-4-plus".into(),
        },
        ProviderEndpoint {
            id: "groq".into(), name: "Groq".into(), api_format: "openai".into(),
            base_url: "https://api.groq.com/openai/v1".into(), env_key: "GROQ_API_KEY".into(),
            default_model: "llama-3.3-70b-versatile".into(),
        },
        ProviderEndpoint {
            id: "mistral".into(), name: "Mistral".into(), api_format: "openai".into(),
            base_url: "https://api.mistral.ai/v1".into(), env_key: "MISTRAL_API_KEY".into(),
            default_model: "mistral-large-latest".into(),
        },
    ]
}

/// 获取当前可用的（有 API key 的）Provider
pub fn get_available_providers() -> Vec<ProviderEndpoint> {
    get_providers()
        .into_iter()
        .filter(|p| crate::keystore::registry_get(&p.env_key).is_some())
        .collect()
}

/// 执行单个任务 — 调用 AI API
pub async fn execute_task(db: &Arc<Database>, task_id: &str) -> Result<(), String> {
    let task = db.get_task(task_id).map_err(|e| e.to_string())?
        .ok_or("任务不存在")?;

    let provider = get_providers().into_iter()
        .find(|p| p.id == task.provider_id)
        .ok_or(format!("Provider {} 不存在", task.provider_id))?;

    let api_key = crate::keystore::registry_get(&provider.env_key)
        .ok_or_else(|| format!("未找到 {} 的 API Key", provider.env_key))?;

    // 更新状态为运行中
    db.update_task_status(task_id, "running", "", "", 0, 0, 0.0, 0).ok();

    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    let model = if task.model.is_empty() { &provider.default_model } else { &task.model };

    let result = match provider.api_format.as_str() {
        "anthropic" => call_anthropic(&client, &provider.base_url, &api_key, model, &task.prompt).await,
        _ => call_openai_compat(&client, &provider.base_url, &api_key, model, &task.prompt).await,
    };

    let latency = start.elapsed().as_millis() as i64;

    match result {
        Ok((text, input_tokens, output_tokens)) => {
            let cost = estimate_cost(model, input_tokens, output_tokens);
            db.update_task_status(task_id, "completed", &text, "", input_tokens, output_tokens, cost, latency).ok();
        }
        Err(e) => {
            db.update_task_status(task_id, "failed", "", &e, 0, 0, 0.0, latency).ok();
        }
    }

    Ok(())
}

/// 调用 OpenAI 兼容 API（OpenAI/DeepSeek/Kimi/Qwen/智谱/Groq/Mistral）
async fn call_openai_compat(client: &reqwest::Client, base_url: &str, api_key: &str, model: &str, prompt: &str)
    -> Result<(String, i64, i64), String>
{
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 4096,
    });

    let resp = client.post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send().await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("API 返回 {}: {}", status, &text[..text.len().min(200)]));
    }

    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = val["choices"][0]["message"]["content"]
        .as_str().unwrap_or("").to_string();
    let input = val["usage"]["prompt_tokens"].as_i64().unwrap_or(0);
    let output = val["usage"]["completion_tokens"].as_i64().unwrap_or(0);

    Ok((content, input, output))
}

/// 调用 Anthropic API
async fn call_anthropic(client: &reqwest::Client, base_url: &str, api_key: &str, model: &str, prompt: &str)
    -> Result<(String, i64, i64), String>
{
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [{"role": "user", "content": prompt}],
    });

    let resp = client.post(format!("{}/messages", base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send().await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("API 返回 {}: {}", status, &text[..text.len().min(200)]));
    }

    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = val["content"][0]["text"]
        .as_str().unwrap_or("").to_string();
    let input = val["usage"]["input_tokens"].as_i64().unwrap_or(0);
    let output = val["usage"]["output_tokens"].as_i64().unwrap_or(0);

    Ok((content, input, output))
}

/// 多 Agent 并行：将同一个 prompt 分发到多个 Provider，并行执行
pub async fn execute_multi_agent(db: &Arc<Database>, parent_task_id: &str) -> Result<(), String> {
    let parent = db.get_task(parent_task_id).map_err(|e| e.to_string())?
        .ok_or("父任务不存在")?;

    db.update_task_status(parent_task_id, "running", "", "", 0, 0, 0.0, 0).ok();

    let available = get_available_providers();
    if available.is_empty() {
        db.update_task_status(parent_task_id, "failed", "", "没有可用的 Provider（请设置 API Key 环境变量）", 0, 0, 0.0, 0).ok();
        return Err("没有可用的 Provider".into());
    }

    // 为每个 Provider 创建子任务
    let mut subtask_ids = Vec::new();
    for provider in &available {
        let sub_id = format!("{}_{}", parent_task_id, provider.id);
        let subtask = TaskRecord {
            id: sub_id.clone(),
            title: format!("{} → {}", parent.title, provider.name),
            prompt: parent.prompt.clone(),
            task_type: "agent".into(),
            provider_id: provider.id.clone(),
            model: provider.default_model.clone(),
            status: "pending".into(),
            result: String::new(),
            input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
            error_msg: String::new(),
            parent_id: Some(parent_task_id.to_string()),
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None, completed_at: None,
        };
        db.insert_task(&subtask).ok();
        subtask_ids.push(sub_id);
    }

    // 并行执行所有子任务
    let mut handles = Vec::new();
    for sub_id in subtask_ids {
        let db_clone = db.clone();
        let id = sub_id.clone();
        handles.push(tokio::spawn(async move {
            execute_task(&db_clone, &id).await.ok();
        }));
    }

    // 等待所有完成
    for h in handles {
        h.await.ok();
    }

    // 汇总结果
    let subtasks = db.get_subtasks(parent_task_id).map_err(|e| e.to_string())?;
    let total_input: i64 = subtasks.iter().map(|s| s.input_tokens).sum();
    let total_output: i64 = subtasks.iter().map(|s| s.output_tokens).sum();
    let total_cost: f64 = subtasks.iter().map(|s| s.estimated_cost).sum();
    let total_latency = subtasks.iter().map(|s| s.latency_ms).max().unwrap_or(0);
    let completed = subtasks.iter().filter(|s| s.status == "completed").count();
    let total = subtasks.len();

    let summary = format!("{}/{} 个 Agent 完成", completed, total);
    let status = if completed > 0 { "completed" } else { "failed" };

    db.update_task_status(parent_task_id, status, &summary, "", total_input, total_output, total_cost, total_latency).ok();

    Ok(())
}

/// Verification mode: run a second model to review the first model's output
pub async fn execute_verification(db: &Arc<Database>, task_id: &str) -> Result<(), String> {
    let task = db.get_task(task_id).map_err(|e| e.to_string())?
        .ok_or("任务不存在")?;

    if task.status != "completed" || task.result.is_empty() {
        return Err("只能验证已完成且有结果的任务".into());
    }

    let available = get_available_providers();
    let verifier = available.iter()
        .find(|p| p.id != task.provider_id)
        .or_else(|| available.first())
        .ok_or("没有可用的验证 Provider")?;

    let verify_id = format!("{}_verify", task_id);
    let verify_prompt = format!(
        "请审查以下 AI 生成的回答，指出可能的错误、遗漏或改进建议。\n\n原始问题：{}\n\nAI 回答：{}\n\n请给出：\n1. 正确性评分 (1-10)\n2. 发现的问题\n3. 改进建议",
        task.prompt.chars().take(2000).collect::<String>(),
        task.result.chars().take(3000).collect::<String>(),
    );

    let verify_task = TaskRecord {
        id: verify_id.clone(),
        title: format!("验证: {}", task.title),
        prompt: verify_prompt,
        task_type: "verification".into(),
        provider_id: verifier.id.clone(),
        model: verifier.default_model.clone(),
        status: "pending".into(),
        result: String::new(),
        input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
        error_msg: String::new(),
        parent_id: Some(task_id.to_string()),
        created_at: chrono::Utc::now().timestamp_millis(),
        started_at: None, completed_at: None,
    };
    db.insert_task(&verify_task).map_err(|e| e.to_string())?;

    execute_task(db, &verify_id).await?;

    Ok(())
}
