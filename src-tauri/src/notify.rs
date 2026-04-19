//! Desktop notifications + budget alert state tracking
//! Uses native macOS osascript (no Tauri AppHandle needed)

use std::sync::atomic::{AtomicU8, Ordering};

// Tracks which thresholds have already fired this month
// bit 0 = 70%, bit 1 = 90%, bit 2 = 100%
static ALERT_STATE: AtomicU8 = AtomicU8::new(0);

// Store the current month to reset when month changes
static CURRENT_MONTH: std::sync::Mutex<u32> = std::sync::Mutex::new(0);

/// Send a desktop notification (cross-platform via system command)
pub fn send_notification(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"display notification "{}" with title "{}" sound name "Glass""#,
            body.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .arg(title)
            .arg(body)
            .spawn();
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (title, body);
    }
}

/// Reset alert state when a new month begins
fn check_month_reset() {
    let now = chrono::Local::now();
    let current = chrono::Datelike::year(&now) as u32 * 100 + chrono::Datelike::month(&now);
    let mut stored = CURRENT_MONTH.lock().unwrap();
    if *stored != current {
        *stored = current;
        ALERT_STATE.store(0, Ordering::Relaxed);
    }
}

/// Check budget and fire notifications at 70%/90%/100% thresholds
/// Called after each traffic record is saved
pub fn check_budget_and_notify(monthly_spend: f64, budgets: &[serde_json::Value], cny_rate: f64) {
    check_month_reset();
    let state = ALERT_STATE.load(Ordering::Relaxed);

    for b in budgets {
        let limit = b["monthly_limit_usd"].as_f64().unwrap_or(0.0);
        if limit <= 0.0 { continue; }
        let percent = (monthly_spend / limit) * 100.0;

        // 100% threshold
        if percent >= 100.0 && (state & 0b100) == 0 && b["notify_90"].as_bool() == Some(true) {
            send_notification(
                "AI Hub — 预算已超",
                &format!("本月 AI 花费已达 ¥{:.2} ({:.0}%)，超出预算 ¥{:.0}",
                    monthly_spend * cny_rate, percent, limit * cny_rate),
            );
            ALERT_STATE.fetch_or(0b100, Ordering::Relaxed);
            continue; // Don't also fire 70/90 notifications
        }
        // 90% threshold
        if percent >= 90.0 && (state & 0b010) == 0 && b["notify_90"].as_bool() == Some(true) {
            send_notification(
                "AI Hub — 预算预警",
                &format!("本月 AI 花费已达 {:.0}% (¥{:.2} / ¥{:.0})，接近月度预算",
                    percent, monthly_spend * cny_rate, limit * cny_rate),
            );
            ALERT_STATE.fetch_or(0b010, Ordering::Relaxed);
            continue;
        }
        // 70% threshold
        if percent >= 70.0 && (state & 0b001) == 0 && b["notify_70"].as_bool() == Some(true) {
            send_notification(
                "AI Hub — 预算提醒",
                &format!("本月 AI 花费已达 {:.0}% (¥{:.2} / ¥{:.0})",
                    percent, monthly_spend * cny_rate, limit * cny_rate),
            );
            ALERT_STATE.fetch_or(0b001, Ordering::Relaxed);
        }
    }
}
