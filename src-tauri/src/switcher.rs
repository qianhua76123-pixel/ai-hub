use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};

static PROXY_PORT: AtomicU16 = AtomicU16::new(23456);

pub fn set_proxy_port(port: u16) {
    PROXY_PORT.store(port, Ordering::Relaxed);
}

fn proxy_base() -> String {
    format!("http://127.0.0.1:{}", PROXY_PORT.load(Ordering::Relaxed))
}

fn port_str() -> String {
    format!("127.0.0.1:{}", PROXY_PORT.load(Ordering::Relaxed))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tool_id: String,
    pub tool_name: String,
    pub config_path: String,
    pub is_redirected: bool,
    pub original_base_url: Option<String>,
}

/// 获取所有可管理的工具
pub fn get_manageable_tools() -> Vec<ToolConfig> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let mut tools = Vec::new();

    // Claude Code
    let claude_settings = home.join(".claude/settings.json");
    if claude_settings.exists() || home.join(".claude").exists() {
        tools.push(ToolConfig {
            tool_id: "claude_code".into(), tool_name: "Claude Code".into(),
            config_path: claude_settings.to_string_lossy().to_string(),
            is_redirected: check_file_contains(&claude_settings, &port_str()),
            original_base_url: Some("https://api.anthropic.com".into()),
        });
    }

    // Codex CLI
    let codex_dir = home.join(".codex");
    if codex_dir.exists() {
        tools.push(ToolConfig {
            tool_id: "codex".into(), tool_name: "Codex CLI".into(),
            config_path: codex_dir.join("config.toml").to_string_lossy().to_string(),
            is_redirected: check_registry_redirected("OPENAI_BASE_URL"),
            original_base_url: Some("https://api.openai.com/v1".into()),
        });
    }

    // Cursor
    let cursor_dir = home.join(".cursor");
    if cursor_dir.exists() {
        let cursor_settings = home.join(".cursor/User/settings.json");
        tools.push(ToolConfig {
            tool_id: "cursor".into(), tool_name: "Cursor".into(),
            config_path: cursor_settings.to_string_lossy().to_string(),
            is_redirected: check_file_contains(&cursor_settings, &port_str()),
            original_base_url: Some("https://api.openai.com/v1".into()),
        });
    }

    // Aider
    if which_exists("aider") || home.join(".aider").exists() {
        tools.push(ToolConfig {
            tool_id: "aider".into(), tool_name: "Aider".into(),
            config_path: "环境变量 OPENAI_API_BASE".into(),
            is_redirected: check_registry_redirected("OPENAI_API_BASE"),
            original_base_url: Some("https://api.openai.com/v1".into()),
        });
    }

    // Continue
    let continue_config = home.join(".continue/config.json");
    if continue_config.exists() {
        tools.push(ToolConfig {
            tool_id: "continue".into(), tool_name: "Continue".into(),
            config_path: continue_config.to_string_lossy().to_string(),
            is_redirected: check_file_contains(&continue_config, &port_str()),
            original_base_url: Some("https://api.openai.com/v1".into()),
        });
    }

    // Gemini CLI
    if which_exists("gemini") || std::env::var("GEMINI_API_KEY").is_ok() {
        tools.push(ToolConfig {
            tool_id: "gemini_cli".into(), tool_name: "Gemini CLI".into(),
            config_path: "环境变量 GEMINI_API_BASE".into(),
            is_redirected: check_registry_redirected("GEMINI_API_BASE"),
            original_base_url: Some("https://generativelanguage.googleapis.com".into()),
        });
    }

    tools
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which").arg(cmd).output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn check_file_contains(path: &PathBuf, needle: &str) -> bool {
    std::fs::read_to_string(path).map(|c| c.contains(needle)).unwrap_or(false)
}

fn check_registry_redirected(env_key: &str) -> bool {
    crate::keystore::registry_get(env_key)
        .map(|v| v.contains(&port_str()))
        .unwrap_or(false)
}

// ===== Enable/Disable per tool =====

pub fn enable_proxy_for(tool_id: &str) -> Result<String, String> {
    match tool_id {
        "claude_code" => enable_claude_proxy(),
        "codex" => enable_env_proxy("OPENAI_BASE_URL", "openai/v1", "Codex CLI"),
        "cursor" => enable_cursor_proxy(),
        "aider" => enable_env_proxy("OPENAI_API_BASE", "openai/v1", "Aider"),
        "continue" => enable_continue_proxy(),
        "gemini_cli" => enable_env_proxy("GEMINI_API_BASE", "gemini", "Gemini CLI"),
        _ => Err(format!("不支持的工具: {}", tool_id)),
    }
}

pub fn disable_proxy_for(tool_id: &str) -> Result<String, String> {
    match tool_id {
        "claude_code" => disable_claude_proxy(),
        "codex" => disable_env_proxy("OPENAI_BASE_URL", "Codex CLI"),
        "cursor" => disable_cursor_proxy(),
        "aider" => disable_env_proxy("OPENAI_API_BASE", "Aider"),
        "continue" => disable_continue_proxy(),
        "gemini_cli" => disable_env_proxy("GEMINI_API_BASE", "Gemini CLI"),
        _ => Err(format!("不支持的工具: {}", tool_id)),
    }
}

/// Disable all currently redirected tools (called on app exit)
pub fn disable_all_proxies() {
    for tool in get_manageable_tools() {
        if tool.is_redirected {
            let _ = disable_proxy_for(&tool.tool_id);
        }
    }
}

// ===== Tool-specific implementations =====

/// Claude Code — modify ~/.claude/settings.json
pub fn enable_claude_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".claude/settings.json");
    std::fs::create_dir_all(home.join(".claude")).ok();

    let mut settings: serde_json::Value = if path.exists() {
        let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&c).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let env = settings.as_object_mut().unwrap()
        .entry("env").or_insert(serde_json::json!({}));
    env.as_object_mut().unwrap()
        .insert("ANTHROPIC_BASE_URL".into(), serde_json::json!(format!("{}/anthropic", proxy_base())));

    atomic_write_json(&path, &settings)?;
    Ok("Claude Code 已接入代理".into())
}

pub fn disable_claude_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".claude/settings.json");
    if !path.exists() { return Ok("无需操作".into()); }

    let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut settings: serde_json::Value = serde_json::from_str(&c).unwrap_or(serde_json::json!({}));

    if let Some(env) = settings.get_mut("env").and_then(|e| e.as_object_mut()) {
        env.remove("ANTHROPIC_BASE_URL");
    }
    atomic_write_json(&path, &settings)?;
    Ok("Claude Code 代理已关闭".into())
}

/// Cursor — modify ~/.cursor/User/settings.json
fn enable_cursor_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".cursor/User/settings.json");
    std::fs::create_dir_all(home.join(".cursor/User")).ok();

    let mut settings: serde_json::Value = if path.exists() {
        let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&c).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    settings.as_object_mut().unwrap()
        .insert("openai.baseUrl".into(), serde_json::json!(format!("{}/openai/v1", proxy_base())));

    atomic_write_json(&path, &settings)?;
    Ok("Cursor 已接入代理".into())
}

fn disable_cursor_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".cursor/User/settings.json");
    if !path.exists() { return Ok("无需操作".into()); }

    let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut settings: serde_json::Value = serde_json::from_str(&c).unwrap_or(serde_json::json!({}));
    settings.as_object_mut().unwrap().remove("openai.baseUrl");
    atomic_write_json(&path, &settings)?;
    Ok("Cursor 代理已关闭".into())
}

/// Continue — modify ~/.continue/config.json
fn enable_continue_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".continue/config.json");
    if !path.exists() { return Err("未找到 Continue 配置文件".into()); }

    let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: serde_json::Value = serde_json::from_str(&c).unwrap_or(serde_json::json!({}));

    // Set apiBase for all models in the config
    if let Some(models) = config.get_mut("models").and_then(|m| m.as_array_mut()) {
        for model in models.iter_mut() {
            model.as_object_mut().unwrap()
                .insert("apiBase".into(), serde_json::json!(format!("{}/openai/v1", proxy_base())));
        }
    }
    atomic_write_json(&path, &config)?;
    Ok("Continue 已接入代理".into())
}

fn disable_continue_proxy() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let path = home.join(".continue/config.json");
    if !path.exists() { return Ok("无需操作".into()); }

    let c = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: serde_json::Value = serde_json::from_str(&c).unwrap_or(serde_json::json!({}));

    if let Some(models) = config.get_mut("models").and_then(|m| m.as_array_mut()) {
        for model in models.iter_mut() {
            model.as_object_mut().unwrap().remove("apiBase");
        }
    }
    atomic_write_json(&path, &config)?;
    Ok("Continue 代理已关闭".into())
}

/// Generic env-var based proxy (Codex, Aider, Gemini CLI)
fn enable_env_proxy(env_key: &str, route: &str, name: &str) -> Result<String, String> {
    crate::keystore::registry_set(env_key, &format!("{}/{}", proxy_base(), route));
    Ok(format!("{} 已接入代理 ({})", name, env_key))
}

fn disable_env_proxy(env_key: &str, name: &str) -> Result<String, String> {
    crate::keystore::registry_remove(env_key);
    Ok(format!("{} 代理已关闭", name))
}

// ===== Utilities =====

fn atomic_write_json(path: &PathBuf, value: &serde_json::Value) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, &content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 生成 shell 环境变量导出命令
pub fn generate_env_exports() -> String {
    format!(
        r#"# AI Hub Proxy - 将以下内容添加到 ~/.zshrc 或 ~/.bashrc
# 所有 AI 工具的请求将通过 AI Hub 代理，自动记录用量

# OpenAI / Codex
export OPENAI_BASE_URL="{proxy}/openai/v1"
export OPENAI_API_BASE="{proxy}/openai/v1"

# Anthropic / Claude
export ANTHROPIC_BASE_URL="{proxy}/anthropic"

# Google Gemini
export GEMINI_API_BASE="{proxy}/gemini"

# DeepSeek
export DEEPSEEK_BASE_URL="{proxy}/deepseek"

# Kimi / Moonshot
export MOONSHOT_BASE_URL="{proxy}/kimi"

# 通义千问
export DASHSCOPE_BASE_URL="{proxy}/qwen"

# 智谱 GLM
export ZHIPU_BASE_URL="{proxy}/zhipu"

# Mistral
export MISTRAL_BASE_URL="{proxy}/mistral"

# Groq
export GROQ_BASE_URL="{proxy}/groq"

# SiliconFlow
export SILICONFLOW_BASE_URL="{proxy}/siliconflow"
"#,
        proxy = proxy_base()
    )
}

/// 自动写入 .zshrc
pub fn install_env_to_shell() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let shell_rc = if home.join(".zshrc").exists() { home.join(".zshrc") } else { home.join(".bashrc") };
    let content = std::fs::read_to_string(&shell_rc).unwrap_or_default();
    if content.contains("AI Hub Proxy") { return Ok("已安装".into()); }

    let exports = generate_env_exports();
    let addition = format!("\n# >>> AI Hub Proxy >>>\n{}# <<< AI Hub Proxy <<<\n", exports);
    std::fs::write(&shell_rc, format!("{}{}", content, addition)).map_err(|e| e.to_string())?;

    Ok(format!("已写入 {}", shell_rc.display()))
}

/// 从 .zshrc 卸载
pub fn uninstall_env_from_shell() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取主目录")?;
    let shell_rc = if home.join(".zshrc").exists() { home.join(".zshrc") } else { home.join(".bashrc") };
    let content = std::fs::read_to_string(&shell_rc).unwrap_or_default();
    if !content.contains("AI Hub Proxy") { return Ok("未安装".into()); }

    let start = "# >>> AI Hub Proxy >>>";
    let end = "# <<< AI Hub Proxy <<<";
    if let (Some(s), Some(e)) = (content.find(start), content.find(end)) {
        let end_pos = e + end.len();
        let mut new_content = content[..s].to_string();
        if end_pos < content.len() { new_content.push_str(&content[end_pos..]); }
        std::fs::write(&shell_rc, new_content.trim_end()).map_err(|e| e.to_string())?;
    }
    Ok("已移除".into())
}
