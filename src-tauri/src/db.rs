use chrono::Datelike;
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRecord {
    pub id: String,
    pub timestamp: i64,
    pub provider_id: String,
    pub model: String,
    pub endpoint: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub latency_ms: i64,
    pub status: String,
    pub estimated_cost: f64,
    pub source: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub git_branch: String,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub cache_creation_tokens: i64,  // 新写入缓存的 token（一次性付费，~1.25x input 价格）
    #[serde(default)]
    pub cache_read_tokens: i64,      // 从缓存读取的 token（折扣价，~10% input 价格）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub day: String,
    pub tokens: i64,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
            .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
            .join("ai-hub");
        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("ai-hub.db");
        let conn = Connection::open(db_path)?;

        // Schema versioning
        conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL DEFAULT 0)")?;
        let current_version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0)
        ).unwrap_or(0);

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS traffic (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL DEFAULT '',
                endpoint TEXT NOT NULL DEFAULT '',
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'success',
                estimated_cost REAL NOT NULL DEFAULT 0.0,
                source TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_traffic_timestamp ON traffic(timestamp);
            CREATE INDEX IF NOT EXISTS idx_traffic_provider ON traffic(provider_id);


            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                prompt TEXT NOT NULL DEFAULT '',
                task_type TEXT NOT NULL DEFAULT 'chat',
                provider_id TEXT NOT NULL DEFAULT '',
                model TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                result TEXT NOT NULL DEFAULT '',
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                estimated_cost REAL NOT NULL DEFAULT 0.0,
                latency_ms INTEGER NOT NULL DEFAULT 0,
                error_msg TEXT NOT NULL DEFAULT '',
                parent_id TEXT,
                created_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL DEFAULT '',
                tool TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL,
                tokens INTEGER NOT NULL DEFAULT 0,
                model TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_conv_timestamp ON conversations(timestamp);
            CREATE INDEX IF NOT EXISTS idx_conv_source ON conversations(source);

            CREATE TABLE IF NOT EXISTS subscriptions (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                plan_name TEXT NOT NULL DEFAULT '',
                price_per_month REAL NOT NULL DEFAULT 0.0,
                billing_cycle TEXT NOT NULL DEFAULT 'monthly',
                next_billing_date TEXT,
                detected_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS route_decisions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL DEFAULT '',
                prompt_type TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 0.0,
                recommended_model TEXT NOT NULL DEFAULT '',
                recommended_provider TEXT NOT NULL DEFAULT '',
                actual_model TEXT NOT NULL DEFAULT '',
                actual_provider TEXT NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_route_task ON route_decisions(task_id);
            "
        )?;

        // v3: budgets table
        if current_version < 3 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS budgets (
                    id TEXT PRIMARY KEY,
                    provider_id TEXT DEFAULT '',
                    monthly_limit_usd REAL NOT NULL DEFAULT 0.0,
                    notify_70 INTEGER DEFAULT 1,
                    notify_90 INTEGER DEFAULT 1,
                    pause_at_100 INTEGER DEFAULT 0
                );"
            );
        }

        // v5: add cache token columns to traffic
        if current_version < 5 {
            let _ = conn.execute("ALTER TABLE traffic ADD COLUMN cache_creation_tokens INTEGER DEFAULT 0", []);
            let _ = conn.execute("ALTER TABLE traffic ADD COLUMN cache_read_tokens INTEGER DEFAULT 0", []);
        }

        // v6: account mode per provider (api/subscription/hybrid)
        if current_version < 6 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS account_modes (
                    provider_id TEXT PRIMARY KEY,
                    mode TEXT NOT NULL DEFAULT 'api',  -- api | subscription | hybrid
                    subscription_monthly_usd REAL DEFAULT 0.0,
                    updated_at INTEGER NOT NULL DEFAULT 0
                );"
            );
            let _ = conn.execute("ALTER TABLE traffic ADD COLUMN cost_mode TEXT DEFAULT 'api'", []);
        }

        // v4: user subscriptions table (for advisor)
        if current_version < 4 {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS user_subscriptions (
                    id TEXT PRIMARY KEY,
                    provider_id TEXT NOT NULL DEFAULT '',
                    provider_name TEXT NOT NULL DEFAULT '',
                    plan_name TEXT NOT NULL DEFAULT '',
                    monthly_usd REAL NOT NULL DEFAULT 0.0,
                    category TEXT NOT NULL DEFAULT 'chat',
                    billing_day INTEGER DEFAULT 1,
                    started_at INTEGER NOT NULL DEFAULT 0
                );"
            );
        }

        // Track schema version
        if current_version < 6 {
            let _ = conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (6)", []);
        }

        // 安全添加新列（忽略已存在错误）
        let _ = conn.execute("ALTER TABLE traffic ADD COLUMN project TEXT DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE traffic ADD COLUMN git_branch TEXT DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE traffic ADD COLUMN working_dir TEXT DEFAULT ''", []);

        // FTS5 for conversation search (<200ms)
        let _ = conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS conversations_fts USING fts5(title, content, content=conversations, content_rowid=rowid);
             CREATE TRIGGER IF NOT EXISTS conversations_ai AFTER INSERT ON conversations BEGIN
               INSERT INTO conversations_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
             END;
             CREATE TRIGGER IF NOT EXISTS conversations_ad AFTER DELETE ON conversations BEGIN
               INSERT INTO conversations_fts(conversations_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
             END;"
        );

        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn get_usage_by_provider(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT provider_id, model, COUNT(*) as cnt,
                    SUM(input_tokens) as input, SUM(output_tokens) as output,
                    COALESCE(SUM(cache_creation_tokens), 0) as cache_write,
                    COALESCE(SUM(cache_read_tokens), 0) as cache_read,
                    ROUND(SUM(estimated_cost), 4) as cost,
                    MIN(timestamp) as first_use, MAX(timestamp) as last_use
             FROM traffic GROUP BY provider_id, model ORDER BY cost DESC"
        )?;
        let records = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "provider_id": row.get::<_, String>(0)?,
                "model": row.get::<_, String>(1)?,
                "requests": row.get::<_, i64>(2)?,
                "input_tokens": row.get::<_, i64>(3)?,
                "output_tokens": row.get::<_, i64>(4)?,
                "cache_write_tokens": row.get::<_, i64>(5)?,
                "cache_read_tokens": row.get::<_, i64>(6)?,
                "cost": row.get::<_, f64>(7)?,
                "first_use": row.get::<_, i64>(8)?,
                "last_use": row.get::<_, i64>(9)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    /// 今日真实统计（请求数/token/费用），全量 DB 汇总
    /// Token 口径: 真实新消耗 = input + cache_write + output (不含 cache_read，避免重复上下文累加)
    /// cache_read 单独字段返回，前端可显示"缓存复用量"
    pub fn get_today_stats(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let today_start = chrono::Local::now().date_naive().and_hms_opt(0,0,0)
            .and_then(|n| n.and_local_timezone(chrono::Local).single())
            .map(|dt| dt.timestamp_millis()).unwrap_or(0);
        let (requests, tokens, cache_reused, cost) = conn.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(input_tokens + cache_creation_tokens + output_tokens), 0),
                    COALESCE(SUM(cache_read_tokens), 0),
                    COALESCE(SUM(estimated_cost), 0.0)
             FROM traffic WHERE timestamp >= ?1",
            params![today_start],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, f64>(3)?)),
        ).unwrap_or((0, 0, 0, 0.0));
        Ok(serde_json::json!({
            "requests": requests,
            "tokens": tokens,           // 真实新消耗
            "cache_reused": cache_reused, // 复用缓存 token（不重复计费）
            "cost": cost,
        }))
    }

    /// 汇总缓存节省统计
    pub fn get_cache_summary(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let today_start = chrono::Local::now().date_naive().and_hms_opt(0,0,0)
            .and_then(|n| n.and_local_timezone(chrono::Local).single())
            .map(|dt| dt.timestamp_millis()).unwrap_or(0);
        let month_start = {
            let now = chrono::Local::now();
            chrono::NaiveDate::from_ymd_opt(chrono::Datelike::year(&now), chrono::Datelike::month(&now), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .and_then(|n| n.and_local_timezone(chrono::Local).single())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0)
        };
        let row = conn.query_row(
            "SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(cache_creation_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(output_tokens), 0)
             FROM traffic WHERE timestamp >= ?1",
            params![today_start],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)),
        ).unwrap_or((0, 0, 0, 0));

        let month_row = conn.query_row(
            "SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(cache_creation_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(output_tokens), 0)
             FROM traffic WHERE timestamp >= ?1",
            params![month_start],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)),
        ).unwrap_or((0, 0, 0, 0));

        Ok(serde_json::json!({
            "today": {
                "input": row.0, "cache_write": row.1, "cache_read": row.2, "output": row.3
            },
            "month": {
                "input": month_row.0, "cache_write": month_row.1, "cache_read": month_row.2, "output": month_row.3
            },
        }))
    }

    pub fn get_hourly_usage(&self, hours: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now().timestamp_millis() - (hours * 3600 * 1000);
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d %H:00', timestamp/1000, 'unixepoch', 'localtime') as hour,
                    provider_id,
                    COUNT(*) as cnt,
                    SUM(input_tokens + COALESCE(cache_creation_tokens,0) + output_tokens) as tokens,
                    ROUND(SUM(estimated_cost), 4) as cost
             FROM traffic WHERE timestamp >= ?1
             GROUP BY hour, provider_id ORDER BY hour ASC"
        )?;
        let records = stmt.query_map(params![cutoff], |row| {
            Ok(serde_json::json!({
                "hour": row.get::<_, String>(0)?,
                "provider_id": row.get::<_, String>(1)?,
                "requests": row.get::<_, i64>(2)?,
                "tokens": row.get::<_, i64>(3)?,
                "cost": row.get::<_, f64>(4)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn get_usage_by_project(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(project,''), '未分类') as proj,
                    COUNT(*) as cnt, SUM(input_tokens + COALESCE(cache_creation_tokens,0) + output_tokens) as tokens,
                    ROUND(SUM(estimated_cost), 4) as cost
             FROM traffic GROUP BY proj ORDER BY cost DESC"
        )?;
        let records = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "project": row.get::<_, String>(0)?,
                "requests": row.get::<_, i64>(1)?,
                "tokens": row.get::<_, i64>(2)?,
                "cost": row.get::<_, f64>(3)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn get_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    pub fn insert_traffic(&self, record: &TrafficRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Look up account mode for this provider
        let cost_mode: String = conn.query_row(
            "SELECT mode FROM account_modes WHERE provider_id = ?1",
            params![record.provider_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| "api".to_string());

        conn.execute(
            "INSERT OR IGNORE INTO traffic (id, timestamp, provider_id, model, endpoint, input_tokens, output_tokens, latency_ms, status, estimated_cost, source, project, git_branch, working_dir, cache_creation_tokens, cache_read_tokens, cost_mode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                record.id,
                record.timestamp,
                record.provider_id,
                record.model,
                record.endpoint,
                record.input_tokens,
                record.output_tokens,
                record.latency_ms,
                record.status,
                record.estimated_cost,
                record.source,
                record.project,
                record.git_branch,
                record.working_dir,
                record.cache_creation_tokens,
                record.cache_read_tokens,
                cost_mode,
            ],
        )?;
        Ok(())
    }

    // ===== Account Modes =====

    pub fn get_account_modes(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT provider_id, mode, subscription_monthly_usd FROM account_modes")?;
        let rows = stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "provider_id": r.get::<_, String>(0)?,
                "mode": r.get::<_, String>(1)?,
                "subscription_monthly_usd": r.get::<_, f64>(2)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    pub fn set_account_mode(&self, provider_id: &str, mode: &str, subscription_usd: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO account_modes (provider_id, mode, subscription_monthly_usd, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![provider_id, mode, subscription_usd, chrono::Utc::now().timestamp_millis()],
        )?;
        // Retroactively update existing traffic records for this provider
        conn.execute(
            "UPDATE traffic SET cost_mode = ?2 WHERE provider_id = ?1",
            params![provider_id, mode],
        )?;
        Ok(())
    }

    /// 费用拆分：按模式汇总（api = 真实付费，subscription = 虚拟等价，hybrid = 超限部分）
    pub fn get_cost_breakdown(&self, days: i64) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now().timestamp_millis() - (days * 86400 * 1000);

        let mut api_cost = 0.0_f64;
        let mut sub_virtual_cost = 0.0_f64;
        let mut hybrid_cost = 0.0_f64;
        let mut api_requests = 0_i64;
        let mut sub_requests = 0_i64;

        let mut stmt = conn.prepare(
            "SELECT COALESCE(cost_mode, 'api') as mode, COUNT(*), COALESCE(SUM(estimated_cost), 0)
             FROM traffic WHERE timestamp >= ?1 GROUP BY mode"
        )?;
        let rows = stmt.query_map(params![cutoff], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?))
        })?;
        for row in rows.flatten() {
            match row.0.as_str() {
                "api" => { api_requests = row.1; api_cost = row.2; }
                "subscription" => { sub_requests = row.1; sub_virtual_cost = row.2; }
                "hybrid" => { hybrid_cost = row.2; }
                _ => {}
            }
        }

        // Sum subscription monthly fees user declared
        let sub_monthly_fee: f64 = conn.query_row(
            "SELECT COALESCE(SUM(subscription_monthly_usd), 0) FROM account_modes WHERE mode='subscription' OR mode='hybrid'",
            [], |r| r.get(0)
        ).unwrap_or(0.0);

        let savings = sub_virtual_cost - sub_monthly_fee;

        Ok(serde_json::json!({
            "api_cost_usd": api_cost,
            "api_requests": api_requests,
            "subscription_virtual_cost_usd": sub_virtual_cost,
            "subscription_requests": sub_requests,
            "subscription_monthly_fee_usd": sub_monthly_fee,
            "subscription_savings_usd": savings,
            "hybrid_cost_usd": hybrid_cost,
            "total_actual_usd": api_cost + sub_monthly_fee + hybrid_cost,
            "total_virtual_equivalent_usd": api_cost + sub_virtual_cost + hybrid_cost,
        }))
    }

    pub fn get_recent_traffic(&self, limit: i64) -> Result<Vec<TrafficRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, provider_id, model, endpoint, input_tokens, output_tokens, latency_ms, status, estimated_cost, source, COALESCE(project,''), COALESCE(git_branch,''), COALESCE(working_dir,''), COALESCE(cache_creation_tokens,0), COALESCE(cache_read_tokens,0)
             FROM traffic ORDER BY timestamp DESC LIMIT ?1"
        )?;

        let records = stmt.query_map(params![limit], |row| {
            Ok(TrafficRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                provider_id: row.get(2)?,
                model: row.get(3)?,
                endpoint: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                latency_ms: row.get(7)?,
                status: row.get(8)?,
                estimated_cost: row.get(9)?,
                source: row.get(10)?,
                project: row.get(11)?,
                git_branch: row.get(12)?,
                working_dir: row.get(13)?,
                cache_creation_tokens: row.get(14)?,
                cache_read_tokens: row.get(15)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(records)
    }

    pub fn get_daily_usage(&self, days: i64) -> Result<Vec<DailyUsage>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now().timestamp_millis() - (days * 86400 * 1000);

        // 统一口径: 新消耗 token = input + cache_write + output (cache_read 不计入，避免重复)
        let mut stmt = conn.prepare(
            "SELECT date(timestamp / 1000, 'unixepoch', 'localtime') as day,
                    SUM(input_tokens + COALESCE(cache_creation_tokens,0) + output_tokens) as tokens
             FROM traffic
             WHERE timestamp >= ?1
             GROUP BY day
             ORDER BY day ASC"
        )?;

        let records = stmt.query_map(params![cutoff], |row| {
            let day_str: String = row.get(0)?;
            // 将日期转换为星期几
            let tokens: i64 = row.get(1)?;
            Ok(DailyUsage {
                day: day_str,
                tokens,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(records)
    }

    pub fn get_provider_usage_summary(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let today_start = {
            let now = chrono::Local::now();
            now.date_naive().and_hms_opt(0, 0, 0).unwrap()
                .and_local_timezone(chrono::Local).unwrap()
                .timestamp_millis()
        };

        let mut stmt = conn.prepare(
            "SELECT provider_id,
                    COUNT(*) as request_count,
                    SUM(input_tokens + COALESCE(cache_creation_tokens,0) + output_tokens) as total_tokens,
                    SUM(estimated_cost) as total_cost
             FROM traffic
             WHERE timestamp >= ?1
             GROUP BY provider_id"
        )?;

        let records = stmt.query_map(params![today_start], |row| {
            let provider_id: String = row.get(0)?;
            let request_count: i64 = row.get(1)?;
            let total_tokens: i64 = row.get(2)?;
            let total_cost: f64 = row.get(3)?;
            Ok(serde_json::json!({
                "provider_id": provider_id,
                "request_count": request_count,
                "total_tokens": total_tokens,
                "total_cost": total_cost,
            }))
        })?.filter_map(|r| r.ok()).collect();

        Ok(records)
    }

    pub fn get_total_stats(&self) -> Result<(i64, i64, f64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(input_tokens + COALESCE(cache_creation_tokens,0) + output_tokens), 0), COALESCE(SUM(estimated_cost), 0.0) FROM traffic"
        )?;
        let result = stmt.query_row([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?))
        })?;
        Ok(result)
    }

    // ===== Task CRUD =====

    pub fn insert_task(&self, task: &TaskRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tasks (id, title, prompt, task_type, provider_id, model, status, result, input_tokens, output_tokens, estimated_cost, latency_ms, error_msg, parent_id, created_at, started_at, completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                task.id, task.title, task.prompt, task.task_type, task.provider_id, task.model,
                task.status, task.result, task.input_tokens, task.output_tokens, task.estimated_cost,
                task.latency_ms, task.error_msg, task.parent_id, task.created_at, task.started_at, task.completed_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_task_status(&self, id: &str, status: &str, result: &str, error_msg: &str,
        input_tokens: i64, output_tokens: i64, cost: f64, latency_ms: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE tasks SET status=?2, result=?3, error_msg=?4, input_tokens=?5, output_tokens=?6, estimated_cost=?7, latency_ms=?8, completed_at=?9, started_at=COALESCE(started_at,?9) WHERE id=?1",
            params![id, status, result, error_msg, input_tokens, output_tokens, cost, latency_ms, now],
        )?;
        Ok(())
    }

    pub fn get_tasks(&self, limit: i64) -> Result<Vec<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, task_type, provider_id, model, status, result, input_tokens, output_tokens, estimated_cost, latency_ms, error_msg, parent_id, created_at, started_at, completed_at
             FROM tasks ORDER BY created_at DESC LIMIT ?1"
        )?;
        let records = stmt.query_map(params![limit], |row| {
            Ok(TaskRecord {
                id: row.get(0)?, title: row.get(1)?, prompt: row.get(2)?, task_type: row.get(3)?,
                provider_id: row.get(4)?, model: row.get(5)?, status: row.get(6)?, result: row.get(7)?,
                input_tokens: row.get(8)?, output_tokens: row.get(9)?, estimated_cost: row.get(10)?,
                latency_ms: row.get(11)?, error_msg: row.get(12)?, parent_id: row.get(13)?,
                created_at: row.get(14)?, started_at: row.get(15)?, completed_at: row.get(16)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, task_type, provider_id, model, status, result, input_tokens, output_tokens, estimated_cost, latency_ms, error_msg, parent_id, created_at, started_at, completed_at
             FROM tasks WHERE id=?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(TaskRecord {
                id: row.get(0)?, title: row.get(1)?, prompt: row.get(2)?, task_type: row.get(3)?,
                provider_id: row.get(4)?, model: row.get(5)?, status: row.get(6)?, result: row.get(7)?,
                input_tokens: row.get(8)?, output_tokens: row.get(9)?, estimated_cost: row.get(10)?,
                latency_ms: row.get(11)?, error_msg: row.get(12)?, parent_id: row.get(13)?,
                created_at: row.get(14)?, started_at: row.get(15)?, completed_at: row.get(16)?,
            })
        })?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    // ===== Conversation CRUD =====

    pub fn insert_conversation(&self, conv: &ConversationRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO conversations (id, source, tool, title, content, timestamp, tokens, model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![conv.id, conv.source, conv.tool, conv.title, conv.content, conv.timestamp, conv.tokens, conv.model],
        )?;
        Ok(())
    }

    pub fn search_conversations(&self, query: &str, source_filter: &str, limit: i64) -> Result<Vec<ConversationRecord>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        // Try FTS5 first for fast search, fallback to LIKE
        // Sanitize for FTS5: wrap as phrase literal, escape internal double-quotes
        let fts_query = format!("\"{}\"", query.replace("\"", "\"\""));
        if source_filter.is_empty() || source_filter == "all" {
            let result = conn.prepare(
                "SELECT c.id, c.source, c.tool, c.title, c.content, c.timestamp, c.tokens, c.model
                 FROM conversations c JOIN conversations_fts f ON c.rowid = f.rowid
                 WHERE conversations_fts MATCH ?1
                 ORDER BY c.timestamp DESC LIMIT ?2"
            ).and_then(|mut stmt| {
                let records = stmt.query_map(params![fts_query, limit], |row| {
                    Ok(ConversationRecord {
                        id: row.get(0)?, source: row.get(1)?, tool: row.get(2)?,
                        title: row.get(3)?, content: row.get(4)?, timestamp: row.get(5)?,
                        tokens: row.get(6)?, model: row.get(7)?,
                    })
                })?.filter_map(|r| r.ok()).collect::<Vec<_>>();
                Ok(records)
            });
            if let Ok(records) = result { return Ok(records); }
            // Fallback to LIKE
            let mut stmt = conn.prepare(
                "SELECT id, source, tool, title, content, timestamp, tokens, model
                 FROM conversations WHERE (title LIKE ?1 OR content LIKE ?1)
                 ORDER BY timestamp DESC LIMIT ?2"
            )?;
            let records = stmt.query_map(params![pattern, limit], |row| {
                Ok(ConversationRecord {
                    id: row.get(0)?, source: row.get(1)?, tool: row.get(2)?,
                    title: row.get(3)?, content: row.get(4)?, timestamp: row.get(5)?,
                    tokens: row.get(6)?, model: row.get(7)?,
                })
            })?.filter_map(|r| r.ok()).collect();
            Ok(records)
        } else {
            let result = conn.prepare(
                "SELECT c.id, c.source, c.tool, c.title, c.content, c.timestamp, c.tokens, c.model
                 FROM conversations c JOIN conversations_fts f ON c.rowid = f.rowid
                 WHERE conversations_fts MATCH ?1 AND c.source = ?3
                 ORDER BY c.timestamp DESC LIMIT ?2"
            ).and_then(|mut stmt| {
                let records = stmt.query_map(params![fts_query, limit, source_filter], |row| {
                    Ok(ConversationRecord {
                        id: row.get(0)?, source: row.get(1)?, tool: row.get(2)?,
                        title: row.get(3)?, content: row.get(4)?, timestamp: row.get(5)?,
                        tokens: row.get(6)?, model: row.get(7)?,
                    })
                })?.filter_map(|r| r.ok()).collect::<Vec<_>>();
                Ok(records)
            });
            if let Ok(records) = result { return Ok(records); }
            // Fallback to LIKE
            let mut stmt = conn.prepare(
                "SELECT id, source, tool, title, content, timestamp, tokens, model
                 FROM conversations WHERE (title LIKE ?1 OR content LIKE ?1) AND source = ?3
                 ORDER BY timestamp DESC LIMIT ?2"
            )?;
            let records = stmt.query_map(params![pattern, limit, source_filter], |row| {
                Ok(ConversationRecord {
                    id: row.get(0)?, source: row.get(1)?, tool: row.get(2)?,
                    title: row.get(3)?, content: row.get(4)?, timestamp: row.get(5)?,
                    tokens: row.get(6)?, model: row.get(7)?,
                })
            })?.filter_map(|r| r.ok()).collect();
            Ok(records)
        }
    }

    pub fn get_recent_conversations(&self, limit: i64) -> Result<Vec<ConversationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, source, tool, title, content, timestamp, tokens, model
             FROM conversations ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let records = stmt.query_map(params![limit], |row| {
            Ok(ConversationRecord {
                id: row.get(0)?, source: row.get(1)?, tool: row.get(2)?,
                title: row.get(3)?, content: row.get(4)?, timestamp: row.get(5)?,
                tokens: row.get(6)?, model: row.get(7)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn get_conversation_sources(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source, COUNT(*) as cnt FROM conversations GROUP BY source ORDER BY cnt DESC"
        )?;
        let records = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "source": row.get::<_, String>(0)?,
                "count": row.get::<_, i64>(1)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    // ===== Export =====

    pub fn export_usage_csv(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(project,''), '未分类') as proj, provider_id, model,
                    COUNT(*) as cnt, SUM(input_tokens) as input, SUM(output_tokens) as output,
                    ROUND(SUM(estimated_cost), 4) as cost
             FROM traffic GROUP BY proj, provider_id, model ORDER BY cost DESC"
        )?;
        let mut csv = String::from("项目,Provider,模型,请求数,输入Token,输出Token,费用USD,费用CNY\n");
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, f64>(6)?,
            ))
        })?.filter_map(|r| r.ok());
        for (proj, provider, model, cnt, input, output, cost) in rows {
            csv.push_str(&format!("{},{},{},{},{},{},{:.4},{:.2}\n",
                proj, provider, model, cnt, input, output, cost, cost * 7.2));
        }
        Ok(csv)
    }

    pub fn export_usage_json(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(project,''), '未分类') as proj, provider_id, model,
                    COUNT(*) as cnt, SUM(input_tokens) as input, SUM(output_tokens) as output,
                    ROUND(SUM(estimated_cost), 4) as cost
             FROM traffic GROUP BY proj, provider_id, model ORDER BY cost DESC"
        )?;
        let records: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "project": row.get::<_, String>(0)?,
                "provider": row.get::<_, String>(1)?,
                "model": row.get::<_, String>(2)?,
                "requests": row.get::<_, i64>(3)?,
                "input_tokens": row.get::<_, i64>(4)?,
                "output_tokens": row.get::<_, i64>(5)?,
                "cost_usd": row.get::<_, f64>(6)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "exported_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "data": records,
        })).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    // ===== Route Decisions =====

    pub fn insert_route_decision(&self, task_id: &str, prompt_type: &str, confidence: f64,
        recommended_model: &str, recommended_provider: &str, actual_model: &str, actual_provider: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = format!("route_{}_{}", task_id, chrono::Utc::now().timestamp_millis());
        conn.execute(
            "INSERT OR REPLACE INTO route_decisions (id, task_id, prompt_type, confidence, recommended_model, recommended_provider, actual_model, actual_provider, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, task_id, prompt_type, confidence, recommended_model, recommended_provider, actual_model, actual_provider, chrono::Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }

    pub fn get_route_decision(&self, task_id: &str) -> Result<Option<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT prompt_type, confidence, recommended_model, recommended_provider, actual_model, actual_provider
             FROM route_decisions WHERE task_id = ?1 LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![task_id], |row| {
            Ok(serde_json::json!({
                "prompt_type": row.get::<_, String>(0)?,
                "confidence": row.get::<_, f64>(1)?,
                "recommended_model": row.get::<_, String>(2)?,
                "recommended_provider": row.get::<_, String>(3)?,
                "actual_model": row.get::<_, String>(4)?,
                "actual_provider": row.get::<_, String>(5)?,
            }))
        })?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    // ===== Project Tagging =====

    #[allow(dead_code)]
    pub fn update_traffic_project(&self, traffic_id: &str, project: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE traffic SET project = ?2 WHERE id = ?1", params![traffic_id, project])?;
        Ok(())
    }

    pub fn batch_update_project(&self, old_project: &str, new_project: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "UPDATE traffic SET project = ?2 WHERE project = ?1",
            params![old_project, new_project],
        )?;
        Ok(count)
    }

    // ===== Budget =====

    pub fn set_budget(&self, provider_id: &str, limit_usd: f64, notify_70: bool, notify_90: bool, pause_100: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = if provider_id.is_empty() { "global" } else { provider_id };
        conn.execute(
            "INSERT OR REPLACE INTO budgets (id, provider_id, monthly_limit_usd, notify_70, notify_90, pause_at_100) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, provider_id, limit_usd, notify_70 as i32, notify_90 as i32, pause_100 as i32],
        )?;
        Ok(())
    }

    pub fn get_budgets(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, provider_id, monthly_limit_usd, notify_70, notify_90, pause_at_100 FROM budgets")?;
        let records = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "provider_id": row.get::<_, String>(1)?,
                "monthly_limit_usd": row.get::<_, f64>(2)?,
                "notify_70": row.get::<_, i32>(3)? == 1,
                "notify_90": row.get::<_, i32>(4)? == 1,
                "pause_at_100": row.get::<_, i32>(5)? == 1,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn delete_budget(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM budgets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_monthly_spend(&self) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let start_of_month = {
            let now = chrono::Local::now();
            chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .and_then(|n| n.and_local_timezone(chrono::Local).single())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0)
        };
        conn.query_row(
            "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE timestamp >= ?1",
            params![start_of_month],
            |row| row.get(0),
        )
    }

    // ===== User Subscriptions (for advisor) =====

    pub fn insert_user_subscription(&self, sub: &crate::advisor::UserSubscription) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO user_subscriptions (id, provider_id, provider_name, plan_name, monthly_usd, category, billing_day, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![sub.id, sub.provider_id, sub.provider_name, sub.plan_name, sub.monthly_usd, sub.category, sub.billing_day, sub.started_at],
        )?;
        Ok(())
    }

    pub fn get_user_subscriptions(&self) -> Result<Vec<crate::advisor::UserSubscription>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, provider_name, plan_name, monthly_usd, category, billing_day, started_at FROM user_subscriptions"
        )?;
        let records = stmt.query_map([], |row| {
            Ok(crate::advisor::UserSubscription {
                id: row.get(0)?, provider_id: row.get(1)?, provider_name: row.get(2)?,
                plan_name: row.get(3)?, monthly_usd: row.get(4)?, category: row.get(5)?,
                billing_day: row.get(6)?, started_at: row.get(7)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }

    pub fn delete_user_subscription(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM user_subscriptions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_monthly_usage_by_provider(&self) -> Result<std::collections::HashMap<String, (i64, f64)>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now().timestamp_millis() - (30 * 86400 * 1000);
        let mut stmt = conn.prepare(
            "SELECT provider_id, COUNT(*), COALESCE(SUM(estimated_cost), 0.0) FROM traffic WHERE timestamp >= ?1 GROUP BY provider_id"
        )?;
        let mut map = std::collections::HashMap::new();
        let iter = stmt.query_map(params![cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?))
        })?;
        for r in iter.flatten() {
            map.insert(r.0, (r.1, r.2));
        }
        Ok(map)
    }

    pub fn get_subtasks(&self, parent_id: &str) -> Result<Vec<TaskRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, prompt, task_type, provider_id, model, status, result, input_tokens, output_tokens, estimated_cost, latency_ms, error_msg, parent_id, created_at, started_at, completed_at
             FROM tasks WHERE parent_id=?1 ORDER BY created_at ASC"
        )?;
        let records = stmt.query_map(params![parent_id], |row| {
            Ok(TaskRecord {
                id: row.get(0)?, title: row.get(1)?, prompt: row.get(2)?, task_type: row.get(3)?,
                provider_id: row.get(4)?, model: row.get(5)?, status: row.get(6)?, result: row.get(7)?,
                input_tokens: row.get(8)?, output_tokens: row.get(9)?, estimated_cost: row.get(10)?,
                latency_ms: row.get(11)?, error_msg: row.get(12)?, parent_id: row.get(13)?,
                created_at: row.get(14)?, started_at: row.get(15)?, completed_at: row.get(16)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(records)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub task_type: String,
    pub provider_id: String,
    pub model: String,
    pub status: String,
    pub result: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub estimated_cost: f64,
    pub latency_ms: i64,
    pub error_msg: String,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: String,
    pub source: String,
    pub tool: String,
    pub title: String,
    pub content: String,
    pub timestamp: i64,
    pub tokens: i64,
    pub model: String,
}

#[cfg(test)]
impl Database {
    /// Create an in-memory database for testing
    pub fn new_test() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        // Run same schema creation as production
        conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL DEFAULT 0)")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS traffic (
                id TEXT PRIMARY KEY, timestamp INTEGER NOT NULL,
                provider_id TEXT NOT NULL, model TEXT NOT NULL DEFAULT '',
                endpoint TEXT NOT NULL DEFAULT '',
                input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'success',
                estimated_cost REAL NOT NULL DEFAULT 0.0, source TEXT NOT NULL DEFAULT '',
                project TEXT DEFAULT '', git_branch TEXT DEFAULT '', working_dir TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, prompt TEXT NOT NULL DEFAULT '',
                task_type TEXT NOT NULL DEFAULT 'chat', provider_id TEXT NOT NULL DEFAULT '',
                model TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'pending',
                result TEXT NOT NULL DEFAULT '', input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0, estimated_cost REAL NOT NULL DEFAULT 0.0,
                latency_ms INTEGER NOT NULL DEFAULT 0, error_msg TEXT NOT NULL DEFAULT '',
                parent_id TEXT, created_at INTEGER NOT NULL, started_at INTEGER, completed_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS route_decisions (
                id TEXT PRIMARY KEY, task_id TEXT NOT NULL DEFAULT '',
                prompt_type TEXT NOT NULL DEFAULT '', confidence REAL NOT NULL DEFAULT 0.0,
                recommended_model TEXT NOT NULL DEFAULT '', recommended_provider TEXT NOT NULL DEFAULT '',
                actual_model TEXT NOT NULL DEFAULT '', actual_provider TEXT NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL
            );"
        )?;
        Ok(Database { conn: Mutex::new(conn) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: &str, provider: &str, model: &str, cost: f64) -> TrafficRecord {
        TrafficRecord {
            id: id.to_string(), timestamp: chrono::Utc::now().timestamp_millis(),
            provider_id: provider.to_string(), model: model.to_string(),
            endpoint: "/v1/chat".to_string(),
            input_tokens: 1000, output_tokens: 500, latency_ms: 200,
            status: "success".to_string(), estimated_cost: cost,
            source: "test".to_string(),
            project: "test-project".to_string(),
            git_branch: "main".to_string(),
            working_dir: "/tmp".to_string(),
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        }
    }

    #[test]
    fn insert_and_get_traffic() {
        let db = Database::new_test().unwrap();
        let record = make_record("t1", "openai", "gpt-4o", 0.05);
        db.insert_traffic(&record).unwrap();

        let recent = db.get_recent_traffic(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "t1");
        assert_eq!(recent[0].model, "gpt-4o");
        assert_eq!(recent[0].project, "test-project");
    }

    #[test]
    fn insert_ignore_duplicate() {
        let db = Database::new_test().unwrap();
        let record = make_record("dup1", "openai", "gpt-4o", 0.05);
        db.insert_traffic(&record).unwrap();
        // Second insert with same ID should be ignored
        db.insert_traffic(&record).unwrap();
        let recent = db.get_recent_traffic(10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn get_total_stats() {
        let db = Database::new_test().unwrap();
        db.insert_traffic(&make_record("s1", "openai", "gpt-4o", 0.10)).unwrap();
        db.insert_traffic(&make_record("s2", "anthropic", "claude", 0.20)).unwrap();

        let (requests, _tokens, cost) = db.get_total_stats().unwrap();
        assert_eq!(requests, 2);
        assert!((cost - 0.30).abs() < 0.001);
    }

    #[test]
    fn get_usage_by_project() {
        let db = Database::new_test().unwrap();
        db.insert_traffic(&make_record("p1", "openai", "gpt-4o", 0.10)).unwrap();
        db.insert_traffic(&make_record("p2", "openai", "gpt-4o", 0.20)).unwrap();

        let projects = db.get_usage_by_project().unwrap();
        assert!(!projects.is_empty());
        let proj = &projects[0];
        assert_eq!(proj["project"], "test-project");
    }

    #[test]
    fn batch_update_project() {
        let db = Database::new_test().unwrap();
        db.insert_traffic(&make_record("bu1", "openai", "gpt-4o", 0.10)).unwrap();
        let count = db.batch_update_project("test-project", "renamed-project").unwrap();
        assert_eq!(count, 1);

        let projects = db.get_usage_by_project().unwrap();
        assert_eq!(projects[0]["project"], "renamed-project");
    }

    #[test]
    fn insert_and_get_task() {
        let db = Database::new_test().unwrap();
        let task = TaskRecord {
            id: "task1".into(), title: "Test".into(), prompt: "hello".into(),
            task_type: "chat".into(), provider_id: "openai".into(), model: "gpt-4o".into(),
            status: "pending".into(), result: String::new(),
            input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
            error_msg: String::new(), parent_id: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None, completed_at: None,
        };
        db.insert_task(&task).unwrap();

        let tasks = db.get_tasks(10).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Test");
    }

    #[test]
    fn route_decision_crud() {
        let db = Database::new_test().unwrap();
        db.insert_route_decision("task1", "code", 0.85, "claude-sonnet", "anthropic", "gpt-4o", "openai").unwrap();

        let decision = db.get_route_decision("task1").unwrap();
        assert!(decision.is_some());
        let d = decision.unwrap();
        assert_eq!(d["prompt_type"], "code");
        assert_eq!(d["recommended_model"], "claude-sonnet");
        assert_eq!(d["actual_model"], "gpt-4o");
    }

    #[test]
    fn update_task_status() {
        let db = Database::new_test().unwrap();
        let task = TaskRecord {
            id: "task2".into(), title: "Status Test".into(), prompt: "test".into(),
            task_type: "chat".into(), provider_id: "openai".into(), model: "gpt-4o".into(),
            status: "pending".into(), result: String::new(),
            input_tokens: 0, output_tokens: 0, estimated_cost: 0.0, latency_ms: 0,
            error_msg: String::new(), parent_id: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None, completed_at: None,
        };
        db.insert_task(&task).unwrap();
        db.update_task_status("task2", "completed", "result here", "", 100, 50, 0.01, 500).unwrap();

        let t = db.get_task("task2").unwrap().unwrap();
        assert_eq!(t.status, "completed");
        assert_eq!(t.result, "result here");
        assert_eq!(t.input_tokens, 100);
    }
}
