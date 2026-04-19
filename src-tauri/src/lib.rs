mod scanner;
mod db;
mod traffic;
mod proxy;
mod switcher;
mod engine;
mod pricing;
mod router;
mod health;
mod conversations;
mod presets;
mod keystore;
mod benchmarks;
mod notify;
mod advisor;
mod news;
mod rankings;

use db::{Database, TrafficRecord, DailyUsage, TaskRecord, ConversationRecord};
use presets::ProviderPreset;
use scanner::DetectedProvider;
use switcher::ToolConfig;
use engine::ProviderEndpoint;
use health::HealthMonitor;
use std::sync::Arc;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;

struct AppState {
    db: Arc<Database>,
    proxy_port: std::sync::Mutex<u16>,
    health: Arc<HealthMonitor>,
}

// ===== 扫描 =====

#[tauri::command]
fn scan_providers() -> Vec<DetectedProvider> {
    scanner::scan_all()
}

#[tauri::command]
fn get_app_info() -> serde_json::Value {
    let rate = pricing::get_pricing_info().currency_rate_usd_to_cny;
    serde_json::json!({
        "name": "AI Hub",
        "version": "0.3.0",
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "currency_rate": rate,
    })
}

// ===== 流量 =====

#[tauri::command]
fn get_recent_traffic(state: tauri::State<'_, AppState>, limit: i64) -> Vec<TrafficRecord> {
    state.db.get_recent_traffic(limit).unwrap_or_default()
}

#[tauri::command]
fn get_daily_usage(state: tauri::State<'_, AppState>, days: i64) -> Vec<DailyUsage> {
    state.db.get_daily_usage(days).unwrap_or_default()
}

#[tauri::command]
fn get_total_stats(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let (requests, tokens, cost) = state.db.get_total_stats().unwrap_or((0, 0, 0.0));
    serde_json::json!({ "requests": requests, "tokens": tokens, "cost": cost })
}

#[tauri::command]
fn get_provider_usage(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_provider_usage_summary().unwrap_or_default()
}

#[tauri::command]
fn refresh_traffic(state: tauri::State<'_, AppState>) -> String {
    traffic::scan_all_sources(&state.db);
    "ok".to_string()
}

#[tauri::command]
fn get_usage_by_provider(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_usage_by_provider().unwrap_or_default()
}

#[tauri::command]
fn get_hourly_usage(state: tauri::State<'_, AppState>, hours: i64) -> Vec<serde_json::Value> {
    state.db.get_hourly_usage(hours).unwrap_or_default()
}

// ===== 代理 =====

#[tauri::command]
fn get_proxy_status(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let port = *state.proxy_port.lock().unwrap();
    serde_json::json!({
        "running": port > 0,
        "port": port,
        "base_url": format!("http://127.0.0.1:{}", port),
    })
}

#[tauri::command]
fn get_manageable_tools() -> Vec<ToolConfig> {
    switcher::get_manageable_tools()
}

#[tauri::command]
fn enable_proxy_for_tool(tool_id: String) -> Result<String, String> {
    switcher::enable_proxy_for(&tool_id)
}

#[tauri::command]
fn disable_proxy_for_tool(tool_id: String) -> Result<String, String> {
    switcher::disable_proxy_for(&tool_id)
}

#[tauri::command]
fn get_env_exports() -> String {
    switcher::generate_env_exports()
}

#[tauri::command]
fn install_shell_proxy() -> Result<String, String> {
    switcher::install_env_to_shell()
}

#[tauri::command]
fn uninstall_shell_proxy() -> Result<String, String> {
    switcher::uninstall_env_from_shell()
}

// ===== 模型价格 =====

#[tauri::command]
fn get_model_prices() -> Vec<pricing::ModelPrice> {
    pricing::get_all_model_prices()
}

#[tauri::command]
fn get_subscription_plans() -> Vec<pricing::SubscriptionPlan> {
    pricing::get_subscription_plans()
}

#[tauri::command]
fn get_pricing_info() -> serde_json::Value {
    let data = pricing::get_pricing_info();
    serde_json::json!({
        "last_updated": data.last_updated,
        "model_count": data.models.len(),
        "currency_rate": data.currency_rate_usd_to_cny,
        "source": "~/Library/Application Support/ai-hub/pricing.json",
    })
}

#[tauri::command]
async fn fetch_latest_pricing() -> Result<String, String> {
    benchmarks::fetch_and_update().await
}

#[tauri::command]
fn update_model_price(model_id: String, input_per_m: f64, output_per_m: f64, cache_write_per_m: f64, cache_read_per_m: f64) -> Result<String, String> {
    let mut data = pricing::get_pricing_info();
    if let Some(model) = data.models.iter_mut().find(|m| m.model_id == model_id) {
        model.input_per_m = input_per_m;
        model.output_per_m = output_per_m;
        model.cache_write_per_m = cache_write_per_m;
        model.cache_read_per_m = cache_read_per_m;
        data.last_updated = chrono::Local::now().format("%Y-%m-%d").to_string();
        pricing::save_pricing(&data)?;
        Ok(format!("已更新 {} 的价格", model_id))
    } else {
        Err(format!("未找到模型 {}", model_id))
    }
}

#[tauri::command]
fn get_cost_comparison(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let cny_rate = pricing::get_pricing_info().currency_rate_usd_to_cny;
    // 获取过去 30 天的 API 花费
    let conn_guard = state.db.get_conn();
    let cutoff = chrono::Utc::now().timestamp_millis() - (30 * 86400 * 1000);

    let monthly_api_cost: f64 = conn_guard.query_row(
        "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE timestamp >= ?1",
        rusqlite::params![cutoff],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let by_provider: Vec<serde_json::Value> = {
        let mut stmt = conn_guard.prepare(
            "SELECT provider_id, SUM(estimated_cost), SUM(input_tokens + output_tokens), COUNT(*)
             FROM traffic WHERE timestamp >= ?1 GROUP BY provider_id"
        ).unwrap();
        stmt.query_map(rusqlite::params![cutoff], |row| {
            Ok(serde_json::json!({
                "provider_id": row.get::<_, String>(0)?,
                "api_cost_usd": row.get::<_, f64>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "requests": row.get::<_, i64>(3)?,
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };

    let plans = pricing::get_subscription_plans();
    let mut comparisons = Vec::new();

    for bp in &by_provider {
        let pid = bp["provider_id"].as_str().unwrap_or("");
        let api_cost = bp["api_cost_usd"].as_f64().unwrap_or(0.0);

        // 找该 provider 的订阅计划
        let matching_plans: Vec<&pricing::SubscriptionPlan> = plans.iter()
            .filter(|p| p.provider == pid && p.price_monthly_usd > 0.0)
            .collect();

        for plan in matching_plans {
            let savings = api_cost - plan.price_monthly_usd;
            comparisons.push(serde_json::json!({
                "provider_id": pid,
                "provider_name": plan.provider_name,
                "plan_name": plan.plan_name,
                "subscription_usd": plan.price_monthly_usd,
                "subscription_cny": plan.price_monthly_cny,
                "api_cost_usd": api_cost,
                "api_cost_cny": api_cost * cny_rate,
                "savings_usd": savings,
                "savings_cny": savings * cny_rate,
                "recommendation": if savings > 0.0 { "订阅更划算" } else { "API 按量计费更省" },
            }));
        }
    }

    serde_json::json!({
        "monthly_api_cost_usd": monthly_api_cost,
        "monthly_api_cost_cny": monthly_api_cost * cny_rate,
        "by_provider": by_provider,
        "comparisons": comparisons,
    })
}

// ===== 智能路由 =====

#[tauri::command]
fn recommend_route(prompt: String) -> router::TaskClassification {
    router::recommend_models(&prompt)
}

// ===== 缓存摘要 =====

#[tauri::command]
fn get_cache_summary(state: tauri::State<'_, AppState>) -> serde_json::Value {
    state.db.get_cache_summary().unwrap_or(serde_json::json!({}))
}

#[tauri::command]
fn get_today_stats(state: tauri::State<'_, AppState>) -> serde_json::Value {
    state.db.get_today_stats().unwrap_or(serde_json::json!({ "requests": 0, "tokens": 0, "cost": 0.0 }))
}

// ===== 账户模式（订阅 vs API）=====

#[tauri::command]
fn get_account_modes(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_account_modes().unwrap_or_default()
}

#[tauri::command]
fn set_account_mode(state: tauri::State<'_, AppState>, provider_id: String, mode: String, subscription_monthly_usd: f64) -> Result<String, String> {
    if !matches!(mode.as_str(), "api" | "subscription" | "hybrid") {
        return Err("mode 必须是 api/subscription/hybrid".into());
    }
    state.db.set_account_mode(&provider_id, &mode, subscription_monthly_usd).map_err(|e| e.to_string())?;
    Ok(format!("{} 账户模式已设为 {}", provider_id, mode))
}

#[tauri::command]
fn get_cost_breakdown(state: tauri::State<'_, AppState>, days: i64) -> serde_json::Value {
    state.db.get_cost_breakdown(days).unwrap_or(serde_json::json!({}))
}

#[tauri::command]
async fn fetch_news() -> news::NewsResult {
    news::fetch_all().await
}

// ===== 订阅顾问 =====

#[tauri::command]
fn add_user_subscription(state: tauri::State<'_, AppState>, provider_id: String, provider_name: String, plan_name: String, monthly_usd: f64, category: String, billing_day: i32) -> Result<String, String> {
    let sub = advisor::UserSubscription {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id, provider_name, plan_name, monthly_usd, category, billing_day,
        started_at: chrono::Utc::now().timestamp_millis(),
    };
    state.db.insert_user_subscription(&sub).map_err(|e| e.to_string())?;
    Ok(sub.id)
}

#[tauri::command]
fn get_user_subscriptions(state: tauri::State<'_, AppState>) -> Vec<advisor::UserSubscription> {
    state.db.get_user_subscriptions().unwrap_or_default()
}

#[tauri::command]
fn delete_user_subscription(state: tauri::State<'_, AppState>, id: String) -> Result<String, String> {
    state.db.delete_user_subscription(&id).map_err(|e| e.to_string())?;
    Ok("已删除".into())
}

#[tauri::command]
fn get_subscription_recommendations(state: tauri::State<'_, AppState>) -> Vec<advisor::Recommendation> {
    let subs = state.db.get_user_subscriptions().unwrap_or_default();
    let usage = state.db.get_monthly_usage_by_provider().unwrap_or_default();
    advisor::analyze(&subs, &usage)
}

#[tauri::command]
fn get_stack_cost_estimate(state: tauri::State<'_, AppState>) -> advisor::StackCostEstimate {
    let subs = state.db.get_user_subscriptions().unwrap_or_default();
    let usage = state.db.get_monthly_usage_by_provider().unwrap_or_default();
    advisor::estimate_stack_cost(&subs, &usage)
}

// ===== 预算 =====

#[tauri::command]
fn get_budgets(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_budgets().unwrap_or_default()
}

#[tauri::command]
fn set_budget(state: tauri::State<'_, AppState>, provider_id: String, monthly_limit_usd: f64, notify_70: bool, notify_90: bool, pause_at_100: bool) -> Result<String, String> {
    state.db.set_budget(&provider_id, monthly_limit_usd, notify_70, notify_90, pause_at_100).map_err(|e| e.to_string())?;
    Ok("预算已设置".into())
}

#[tauri::command]
fn delete_budget(state: tauri::State<'_, AppState>, id: String) -> Result<String, String> {
    state.db.delete_budget(&id).map_err(|e| e.to_string())?;
    Ok("预算已删除".into())
}

#[tauri::command]
fn get_budget_status(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let spend = state.db.get_monthly_spend().unwrap_or(0.0);
    let budgets = state.db.get_budgets().unwrap_or_default();
    let global = budgets.iter().find(|b| b["provider_id"].as_str() == Some(""));
    let limit = global.and_then(|b| b["monthly_limit_usd"].as_f64()).unwrap_or(0.0);
    let percent = if limit > 0.0 { (spend / limit * 100.0).min(999.0) } else { 0.0 };
    let warning = if percent >= 100.0 { "exceeded" } else if percent >= 90.0 { "critical" } else if percent >= 70.0 { "warning" } else { "ok" };

    serde_json::json!({
        "monthly_spend_usd": spend,
        "monthly_limit_usd": limit,
        "percent": percent,
        "warning_level": warning,
    })
}

// ===== 汇率 =====

#[tauri::command]
async fn refresh_exchange_rate() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| e.to_string())?;

    // Try multiple free APIs for redundancy
    let apis = vec![
        ("https://open.er-api.com/v6/latest/USD", "rates.CNY"),
        ("https://api.frankfurter.dev/v1/latest?from=USD&to=CNY", "rates.CNY"),
    ];

    for (url, _path) in &apis {
        if let Ok(resp) = client.get(*url).send().await {
            if let Ok(text) = resp.text().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(rate) = val.get("rates").and_then(|r| r.get("CNY")).and_then(|c| c.as_f64()) {
                        if rate > 5.0 && rate < 10.0 {
                            // Update pricing data
                            let mut data = pricing::get_pricing_info();
                            let old_rate = data.currency_rate_usd_to_cny;
                            data.currency_rate_usd_to_cny = rate;
                            pricing::save_pricing(&data).ok();

                            let source = if url.contains("er-api") { "ExchangeRate-API" } else { "Frankfurter" };
                            return Ok(serde_json::json!({
                                "rate": rate,
                                "old_rate": old_rate,
                                "source": source,
                                "updated": true,
                            }));
                        }
                    }
                }
            }
        }
    }

    Err("无法获取汇率数据，请检查网络连接".into())
}

// ===== 健康监控 =====

#[tauri::command]
fn get_provider_health(state: tauri::State<'_, AppState>) -> Vec<health::ProviderHealth> {
    state.health.get_all_health()
}

#[tauri::command]
fn get_rate_limit_status(state: tauri::State<'_, AppState>) -> Vec<health::RateLimitStatus> {
    state.health.get_rate_limit_summary()
}

// ===== 项目归因 =====

#[tauri::command]
fn get_usage_by_project(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_usage_by_project().unwrap_or_default()
}

#[tauri::command]
fn tag_traffic_project(state: tauri::State<'_, AppState>, old_project: String, new_project: String) -> Result<String, String> {
    let count = state.db.batch_update_project(&old_project, &new_project).map_err(|e| e.to_string())?;
    Ok(format!("已更新 {} 条记录", count))
}

// ===== 路由决策 =====

#[tauri::command]
fn get_route_decision(state: tauri::State<'_, AppState>, task_id: String) -> Option<serde_json::Value> {
    state.db.get_route_decision(&task_id).ok().flatten()
}

// ===== ROI 计算 =====

#[tauri::command]
fn get_subscription_roi(state: tauri::State<'_, AppState>) -> serde_json::Value {
    let cny_rate = pricing::get_pricing_info().currency_rate_usd_to_cny;
    let conn = state.db.get_conn();
    let cutoff = chrono::Utc::now().timestamp_millis() - (30 * 86400 * 1000);

    let plans = pricing::get_subscription_plans();
    let mut roi_results = Vec::new();

    for plan in &plans {
        if plan.price_monthly_usd <= 0.0 { continue; }

        let api_cost: f64 = conn.query_row(
            "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE provider_id = ?1 AND timestamp >= ?2",
            rusqlite::params![plan.provider, cutoff],
            |row| row.get(0),
        ).unwrap_or(0.0);

        let request_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM traffic WHERE provider_id = ?1 AND timestamp >= ?2",
            rusqlite::params![plan.provider, cutoff],
            |row| row.get(0),
        ).unwrap_or(0);

        if request_count == 0 { continue; }

        let savings = api_cost - plan.price_monthly_usd;
        let roi_pct = if plan.price_monthly_usd > 0.0 { (savings / plan.price_monthly_usd) * 100.0 } else { 0.0 };

        let recommendation = if savings > plan.price_monthly_usd * 0.5 {
            "强烈建议保留订阅"
        } else if savings > 0.0 {
            "建议保留"
        } else if savings > -plan.price_monthly_usd * 0.3 {
            "可保留可取消"
        } else {
            "建议取消，改用 API"
        };

        // 月度费用预测：对比前15天 vs 后15天的趋势
        let mid_cutoff = chrono::Utc::now().timestamp_millis() - (15 * 86400 * 1000);
        let first_half: f64 = conn.query_row(
            "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE provider_id = ?1 AND timestamp >= ?2 AND timestamp < ?3",
            rusqlite::params![plan.provider, cutoff, mid_cutoff],
            |row| row.get(0),
        ).unwrap_or(0.0);
        let second_half: f64 = conn.query_row(
            "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE provider_id = ?1 AND timestamp >= ?2",
            rusqlite::params![plan.provider, mid_cutoff],
            |row| row.get(0),
        ).unwrap_or(0.0);

        // Extrapolate: predicted = second_half * 2 (second half represents more recent trend)
        let predicted_usd = if first_half > 0.0 && second_half > 0.0 {
            second_half * 2.0
        } else {
            api_cost // fallback to current month
        };
        let trend = if first_half > 0.0 { (second_half - first_half) / first_half * 100.0 } else { 0.0 };

        roi_results.push(serde_json::json!({
            "provider": plan.provider,
            "provider_name": plan.provider_name,
            "plan_name": plan.plan_name,
            "subscription_usd": plan.price_monthly_usd,
            "subscription_cny": plan.price_monthly_cny,
            "api_cost_usd": api_cost,
            "api_cost_cny": api_cost * cny_rate,
            "savings_usd": savings,
            "savings_cny": savings * cny_rate,
            "roi_percent": roi_pct,
            "request_count": request_count,
            "recommendation": recommendation,
            "cost_per_request": if request_count > 0 { api_cost / request_count as f64 } else { 0.0 },
            "predicted_next_month_usd": predicted_usd,
            "predicted_next_month_cny": predicted_usd * cny_rate,
            "trend_percent": trend,
        }));
    }

    serde_json::json!({ "results": roi_results })
}

// ===== 任务引擎 =====

#[tauri::command]
fn get_available_providers() -> Vec<ProviderEndpoint> {
    engine::get_available_providers()
}

#[tauri::command]
fn get_tasks(state: tauri::State<'_, AppState>, limit: i64) -> Vec<TaskRecord> {
    state.db.get_tasks(limit).unwrap_or_default()
}

#[tauri::command]
fn get_task_detail(state: tauri::State<'_, AppState>, task_id: String) -> Option<TaskRecord> {
    state.db.get_task(&task_id).ok().flatten()
}

#[tauri::command]
fn get_subtasks(state: tauri::State<'_, AppState>, parent_id: String) -> Vec<TaskRecord> {
    state.db.get_subtasks(&parent_id).unwrap_or_default()
}

#[tauri::command]
async fn create_task(
    state: tauri::State<'_, AppState>,
    title: String,
    prompt: String,
    provider_id: String,
    model: String,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let pid = provider_id.clone();
    let task = TaskRecord {
        id: id.clone(),
        title,
        prompt,
        task_type: "chat".into(),
        provider_id,
        model,
        status: "pending".into(),
        result: String::new(),
        input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
        error_msg: String::new(),
        parent_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        started_at: None, completed_at: None,
    };
    state.db.insert_task(&task).map_err(|e| e.to_string())?;

    // 记录路由决策
    let route_info = router::recommend_models(&task.prompt);
    let top_rec = route_info.recommendations.first();
    state.db.insert_route_decision(
        &id,
        &route_info.task_type,
        route_info.confidence,
        top_rec.map(|r| r.model_name.as_str()).unwrap_or(""),
        top_rec.map(|r| r.provider_id.as_str()).unwrap_or(""),
        &task.model,
        &pid,
    ).ok();

    // 速率限制检查：如果目标 Provider 即将限流，自动切换到备用
    if state.health.is_rate_limit_near(&pid) {
        if let Some(fallback_id) = state.health.get_best_fallback(&pid) {
            let fallback_provider = engine::get_available_providers().iter()
                .find(|p| p.id == fallback_id).cloned();
            if let Some(fp) = fallback_provider {
                let mut updated_task = task.clone();
                updated_task.provider_id = fp.id.clone();
                updated_task.model = fp.default_model.clone();
                state.db.insert_task(&updated_task).ok();
                println!("[Failover] {} 即将限流，切换到 {}", pid, fp.name);
            }
        }
    }

    // 异步执行
    let db = state.db.clone();
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        engine::execute_task(&db, &task_id).await.ok();
    });

    Ok(id)
}

#[tauri::command]
async fn create_multi_agent_task(
    state: tauri::State<'_, AppState>,
    title: String,
    prompt: String,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let task = TaskRecord {
        id: id.clone(),
        title,
        prompt,
        task_type: "multi_agent".into(),
        provider_id: "all".into(),
        model: String::new(),
        status: "pending".into(),
        result: String::new(),
        input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
        error_msg: String::new(),
        parent_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        started_at: None, completed_at: None,
    };
    state.db.insert_task(&task).map_err(|e| e.to_string())?;

    let db = state.db.clone();
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        engine::execute_multi_agent(&db, &task_id).await.ok();
    });

    Ok(id)
}

// ===== 对话搜索 =====

#[tauri::command]
fn search_conversations(state: tauri::State<'_, AppState>, query: String, source: String, limit: i64) -> Vec<ConversationRecord> {
    state.db.search_conversations(&query, &source, limit).unwrap_or_default()
}

#[tauri::command]
fn get_recent_conversations(state: tauri::State<'_, AppState>, limit: i64) -> Vec<ConversationRecord> {
    state.db.get_recent_conversations(limit).unwrap_or_default()
}

#[tauri::command]
fn get_conversation_sources(state: tauri::State<'_, AppState>) -> Vec<serde_json::Value> {
    state.db.get_conversation_sources().unwrap_or_default()
}

#[tauri::command]
fn refresh_conversations(state: tauri::State<'_, AppState>) -> String {
    conversations::scan_all_conversations(&state.db);
    "ok".to_string()
}

// ===== Provider 预设 =====

#[tauri::command]
fn get_provider_presets() -> Vec<ProviderPreset> {
    presets::get_all_presets()
}

#[tauri::command]
fn add_provider(preset_id: String, api_key: String) -> Result<String, String> {
    presets::save_custom_provider(&preset_id, &api_key)
}

#[tauri::command]
fn remove_provider(preset_id: String) -> Result<String, String> {
    presets::remove_custom_provider(&preset_id)
}

// ===== 导出 =====

#[tauri::command]
fn export_usage_csv(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.db.export_usage_csv().map_err(|e| e.to_string())
}

#[tauri::command]
fn export_usage_json(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.db.export_usage_json().map_err(|e| e.to_string())
}

// ===== 实时排行榜 =====

#[tauri::command]
async fn fetch_rankings(aa_api_key: Option<String>) -> rankings::RankingsResult {
    rankings::fetch_all(aa_api_key).await
}

// ===== 评测数据更新 =====

#[tauri::command]
async fn run_benchmark_update(dry_run: bool) -> Result<String, String> {
    let script_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| d.join("../../../scripts/update-benchmarks.py"))
        .unwrap_or_else(|| {
            dirs::home_dir().unwrap_or_default().join("ai-hub/scripts/update-benchmarks.py")
        });

    // Try common locations (relative to exe, no hardcoded user paths)
    let candidates = vec![
        script_path,
        dirs::home_dir().unwrap_or_default().join("ai-hub/scripts/update-benchmarks.py"),
        std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.join("scripts/update-benchmarks.py")))
            .unwrap_or_default(),
    ];

    let script = candidates.iter()
        .find(|p| p.exists())
        .ok_or("未找到 update-benchmarks.py 脚本")?;

    let mut cmd = std::process::Command::new("python3");
    cmd.arg(script);
    if dry_run {
        cmd.arg("--dry-run");
    }

    let output = cmd.output().map_err(|e| format!("执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("脚本错误:\n{}\n{}", stdout, stderr));
    }

    Ok(stdout)
}

// ===== 验证 =====

#[tauri::command]
async fn verify_task(state: tauri::State<'_, AppState>, task_id: String) -> Result<String, String> {
    let db = state.db.clone();
    engine::execute_verification(&db, &task_id).await?;
    Ok(format!("{}_verify", task_id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("FATAL: Failed to initialize database: {}", e);
            eprintln!("Please check disk space and file permissions.");
            std::process::exit(1);
        }
    };
    let db = Arc::new(database);
    let health_monitor = Arc::new(HealthMonitor::new());

    traffic::scan_all_sources(&db);

    let db_for_proxy = db.clone();
    let health_for_proxy = health_monitor.clone();
    let db_for_scan = db.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_log::Builder::new()
            .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("ai-hub".into()) }))
            .max_file_size(5_000_000) // 5MB
            .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
            .build())
        // Updater: activate when code signing is configured
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            db: db.clone(),
            proxy_port: std::sync::Mutex::new(0),
            health: health_monitor.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            scan_providers, get_app_info,
            get_recent_traffic, get_daily_usage, get_provider_usage, get_total_stats, refresh_traffic,
            get_usage_by_provider, get_hourly_usage,
            get_proxy_status, get_manageable_tools, enable_proxy_for_tool, disable_proxy_for_tool,
            get_env_exports, install_shell_proxy, uninstall_shell_proxy,
            get_model_prices, get_subscription_plans, get_cost_comparison, get_pricing_info, update_model_price, fetch_latest_pricing,
            recommend_route, refresh_exchange_rate, get_budgets, set_budget, delete_budget, get_budget_status,
            add_user_subscription, get_user_subscriptions, delete_user_subscription, get_subscription_recommendations, get_stack_cost_estimate,
            get_cache_summary, get_today_stats, fetch_news,
            get_account_modes, set_account_mode, get_cost_breakdown, get_provider_health, get_rate_limit_status, get_usage_by_project, get_subscription_roi, tag_traffic_project, get_route_decision,
            get_available_providers, get_tasks, get_task_detail, get_subtasks,
            create_task, create_multi_agent_task,
            search_conversations, get_recent_conversations, get_conversation_sources, refresh_conversations,
            get_provider_presets, add_provider, remove_provider,
            export_usage_csv, export_usage_json,
            run_benchmark_update,
            fetch_rankings,
            verify_task,
        ])
        .setup(move |app| {
            let db_proxy = db_for_proxy.clone();
            let hp = health_for_proxy.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let port = proxy::start_proxy(db_proxy, hp).await;
                if let Some(state) = app_handle.try_state::<AppState>() {
                    *state.proxy_port.lock().unwrap() = port;
                }
                // Update switcher to use actual bound port
                switcher::set_proxy_port(port);
                println!("AI Hub Proxy running on http://127.0.0.1:{}", port);
            });

            // Load saved custom providers
            presets::load_custom_providers();

            // System tray
            let show_item = MenuItemBuilder::with_id("show", "显示 AI Hub").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("AI Hub - AI 工具管理")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(win) = app.get_webview_window("main") {
                                win.show().ok();
                                win.set_focus().ok();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(win) = tray.app_handle().get_webview_window("main") {
                            win.show().ok();
                            win.set_focus().ok();
                        }
                    }
                })
                .build(app)?;

            // Auto-refresh benchmark data + exchange rate on startup
            tauri::async_runtime::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                match benchmarks::fetch_and_update().await {
                    Ok(msg) => println!("[Benchmarks] Auto-update:\n{}", msg),
                    Err(e) => eprintln!("[Benchmarks] Auto-update failed: {}", e),
                }
            });

            // Background health checks
            let health_for_checks = health_monitor.clone();
            tauri::async_runtime::spawn(async move {
                health::run_health_checks(health_for_checks).await;
            });

            // 启动时自动接入 Claude Code
            if let Err(e) = switcher::enable_claude_proxy() {
                eprintln!("Auto-enable Claude proxy: {}", e);
            } else {
                println!("Claude Code 已自动接入 AI Hub 代理");
            }

            // Initial conversation scan
            let db_for_conv = db.clone();
            conversations::scan_all_conversations(&db_for_conv);

            // 后台定时扫描日志
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    traffic::scan_all_sources(&db_for_scan);
                }
            });

            // Conversation scan - less frequent (every 5 min)
            let db_for_conv_periodic = db_for_conv.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(300));
                    conversations::scan_all_conversations(&db_for_conv_periodic);
                }
            });

            // Auto-update benchmark data on startup + every 6 hours
            std::thread::spawn(|| {
                fn run_benchmark_script() {
                    // Look for script relative to executable, then common dev locations
                    let candidates: Vec<std::path::PathBuf> = vec![
                        std::env::current_exe().ok()
                            .and_then(|p| p.parent().map(|d| d.join("../Resources/scripts/update-benchmarks.py")))
                            .unwrap_or_default(),
                        std::env::current_exe().ok()
                            .and_then(|p| p.parent().map(|d| d.join("../../../scripts/update-benchmarks.py")))
                            .unwrap_or_default(),
                    ];
                    for script in candidates {
                        if script.exists() {
                            let _ = std::process::Command::new("python3").arg(&script).output();
                            return;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(10));
                run_benchmark_script();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
                    run_benchmark_script();
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                println!("AI Hub exiting, restoring all tool configs...");
                switcher::disable_all_proxies();
                println!("All tool configs restored");
            }
        });
}
