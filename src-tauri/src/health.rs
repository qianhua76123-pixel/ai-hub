use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub provider_name: String,
    pub status: String,
    pub latency_ms: i64,
    pub error_rate: f64,
    pub last_check: i64,
    pub consecutive_errors: i32,
    pub rate_limit_remaining: Option<i64>,
    pub rate_limit_reset: Option<i64>,
    pub circuit_state: CircuitState,
    pub circuit_opened_at: Option<i64>,
    pub total_requests: i64,
    pub total_errors: i64,
    pub last_success: Option<i64>,
}

pub struct HealthMonitor {
    state: Mutex<HashMap<String, ProviderHealth>>,
    request_log: Mutex<HashMap<String, Vec<i64>>>, // sliding window: recent request timestamps
    circuit_cooldown_ms: i64,
}

impl HealthMonitor {
    pub fn new() -> Self {
        HealthMonitor {
            state: Mutex::new(HashMap::new()),
            request_log: Mutex::new(HashMap::new()),
            circuit_cooldown_ms: 30_000,
        }
    }

    pub fn record_request(&self, provider_id: &str, provider_name: &str, latency_ms: i64, is_error: bool, rate_limit_remaining: Option<i64>, rate_limit_reset: Option<i64>) {
        let mut state = self.state.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        // Track request timestamps (keep last 5 minutes)
        {
            let mut log = self.request_log.lock().unwrap();
            let timestamps = log.entry(provider_id.to_string()).or_insert_with(Vec::new);
            timestamps.push(now);
            let cutoff = now - 5 * 60 * 1000;
            timestamps.retain(|&t| t > cutoff);
        }
        let entry = state.entry(provider_id.to_string()).or_insert(ProviderHealth {
            provider_id: provider_id.to_string(),
            provider_name: provider_name.to_string(),
            status: "healthy".to_string(),
            latency_ms: 0,
            error_rate: 0.0,
            last_check: 0,
            consecutive_errors: 0,
            rate_limit_remaining: None,
            rate_limit_reset: None,
            circuit_state: CircuitState::Closed,
            circuit_opened_at: None,
            total_requests: 0,
            total_errors: 0,
            last_success: None,
        });

        entry.latency_ms = latency_ms;
        entry.last_check = now;
        entry.rate_limit_remaining = rate_limit_remaining;
        entry.rate_limit_reset = rate_limit_reset;
        entry.total_requests += 1;

        if is_error {
            entry.consecutive_errors += 1;
            entry.total_errors += 1;
            entry.error_rate = (entry.error_rate * 0.8) + 0.2;

            if entry.consecutive_errors >= 5 && entry.circuit_state != CircuitState::Open {
                entry.circuit_state = CircuitState::Open;
                entry.circuit_opened_at = Some(now);
                entry.status = "down".to_string();
            } else if entry.consecutive_errors >= 2 || entry.error_rate > 0.3 || latency_ms > 30000 {
                entry.status = "degraded".to_string();
            }

            if entry.circuit_state == CircuitState::HalfOpen {
                entry.circuit_state = CircuitState::Open;
                entry.circuit_opened_at = Some(now);
            }
        } else {
            entry.consecutive_errors = 0;
            entry.error_rate = entry.error_rate * 0.9;
            entry.last_success = Some(now);

            if entry.circuit_state == CircuitState::HalfOpen {
                entry.circuit_state = CircuitState::Closed;
                entry.circuit_opened_at = None;
                entry.status = "healthy".to_string();
            } else {
                entry.status = if entry.error_rate > 0.1 { "degraded" } else { "healthy" }.to_string();
            }
        }
    }

    pub fn should_allow_request(&self, provider_id: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        if let Some(entry) = state.get_mut(provider_id) {
            match entry.circuit_state {
                CircuitState::Closed => true,
                CircuitState::Open => {
                    if let Some(opened_at) = entry.circuit_opened_at {
                        if now - opened_at > self.circuit_cooldown_ms {
                            entry.circuit_state = CircuitState::HalfOpen;
                            return true;
                        }
                    }
                    false
                }
                CircuitState::HalfOpen => true,
            }
        } else {
            true
        }
    }

    pub fn get_all_health(&self) -> Vec<ProviderHealth> {
        let state = self.state.lock().unwrap();
        state.values().cloned().collect()
    }

    pub fn get_best_fallback(&self, exclude: &str) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.values()
            .filter(|h| h.provider_id != exclude && h.status == "healthy" && h.circuit_state == CircuitState::Closed)
            .min_by_key(|h| h.latency_ms)
            .map(|h| h.provider_id.clone())
    }

    pub fn is_rate_limit_near(&self, provider_id: &str) -> bool {
        let state = self.state.lock().unwrap();
        if let Some(h) = state.get(provider_id) {
            if let Some(remaining) = h.rate_limit_remaining {
                return remaining < 5;
            }
        }
        false
    }

    pub fn get_rate_limit_summary(&self) -> Vec<RateLimitStatus> {
        let state = self.state.lock().unwrap();
        state.values().map(|h| {
            let warning = if let Some(remaining) = h.rate_limit_remaining {
                if remaining < 3 { "critical" }
                else if remaining < 10 { "warning" }
                else { "ok" }
            } else {
                "unknown"
            };

            let estimated_minutes_left = h.rate_limit_remaining.map(|r| {
                if r <= 0 { return 0.0; }
                let log = self.request_log.lock().unwrap();
                if let Some(timestamps) = log.get(&h.provider_id) {
                    if timestamps.len() >= 2 {
                        let rpm = timestamps.len() as f64 / 5.0;
                        if rpm > 0.0 { return r as f64 / rpm; }
                    }
                }
                r as f64 / 2.0
            });

            RateLimitStatus {
                provider_id: h.provider_id.clone(),
                provider_name: h.provider_name.clone(),
                status: h.status.clone(),
                latency_ms: h.latency_ms,
                error_rate: h.error_rate,
                rate_limit_remaining: h.rate_limit_remaining,
                rate_limit_reset: h.rate_limit_reset,
                warning_level: warning.to_string(),
                estimated_minutes_left,
                last_check: h.last_check,
                circuit_state: format!("{:?}", h.circuit_state),
                total_requests: h.total_requests,
                total_errors: h.total_errors,
            }
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub provider_id: String,
    pub provider_name: String,
    pub status: String,
    pub latency_ms: i64,
    pub error_rate: f64,
    pub rate_limit_remaining: Option<i64>,
    pub rate_limit_reset: Option<i64>,
    pub warning_level: String,
    pub estimated_minutes_left: Option<f64>,
    pub last_check: i64,
    pub circuit_state: String,
    pub total_requests: i64,
    pub total_errors: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_monitor_is_empty() {
        let m = HealthMonitor::new();
        assert!(m.get_all_health().is_empty());
    }

    #[test]
    fn record_successful_request() {
        let m = HealthMonitor::new();
        m.record_request("openai", "OpenAI", 150, false, Some(100), None);
        let health = m.get_all_health();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].status, "healthy");
        assert_eq!(health[0].latency_ms, 150);
    }

    #[test]
    fn consecutive_errors_degrade_status() {
        let m = HealthMonitor::new();
        m.record_request("openai", "OpenAI", 100, true, None, None);
        m.record_request("openai", "OpenAI", 100, true, None, None);
        let health = m.get_all_health();
        assert_eq!(health[0].status, "degraded");
    }

    #[test]
    fn five_errors_mark_down() {
        let m = HealthMonitor::new();
        for _ in 0..5 {
            m.record_request("openai", "OpenAI", 100, true, None, None);
        }
        let health = m.get_all_health();
        assert_eq!(health[0].status, "down");
    }

    #[test]
    fn rate_limit_near_detection() {
        let m = HealthMonitor::new();
        m.record_request("openai", "OpenAI", 100, false, Some(3), None);
        assert!(m.is_rate_limit_near("openai"));
        m.record_request("openai", "OpenAI", 100, false, Some(50), None);
        assert!(!m.is_rate_limit_near("openai"));
    }

    #[test]
    fn best_fallback_excludes_current() {
        let m = HealthMonitor::new();
        m.record_request("openai", "OpenAI", 100, false, None, None);
        m.record_request("anthropic", "Anthropic", 80, false, None, None);
        let fallback = m.get_best_fallback("openai");
        assert_eq!(fallback, Some("anthropic".to_string()));
    }

    #[test]
    fn should_allow_blocks_when_circuit_open() {
        let m = HealthMonitor::new();
        for _ in 0..5 {
            m.record_request("openai", "OpenAI", 100, true, None, None);
        }
        assert!(!m.should_allow_request("openai"));
    }
}

/// Periodically ping provider health endpoints
pub async fn run_health_checks(monitor: std::sync::Arc<HealthMonitor>) {
    let endpoints = vec![
        ("openai", "OpenAI", "https://api.openai.com/v1/models"),
        ("anthropic", "Anthropic", "https://api.anthropic.com/v1/messages"),
        ("deepseek", "DeepSeek", "https://api.deepseek.com/v1/models"),
        ("groq", "Groq", "https://api.groq.com/openai/v1/models"),
        ("mistral", "Mistral", "https://api.mistral.ai/v1/models"),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    loop {
        for (id, name, url) in &endpoints {
            let start = std::time::Instant::now();
            let result = client.head(*url).send().await;
            let latency = start.elapsed().as_millis() as i64;
            // Don't count 401/403 as errors (just means no auth, but service is up)
            let is_real_error = result.is_err() || result.as_ref().map(|r| r.status().as_u16() >= 500).unwrap_or(true);
            monitor.record_request(id, name, latency, is_real_error, None, None);
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
