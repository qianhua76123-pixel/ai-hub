use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedProvider {
    pub id: String,
    pub name: String,
    pub status: String,
    pub detection_method: String,
    pub detection_detail: String,
    pub color: String,
    pub plan: Option<String>,
}

fn scan_env_vars() -> Vec<DetectedProvider> {
    let env_map: Vec<(&str, &str, &str, &str)> = vec![
        // 国际
        ("OPENAI_API_KEY", "openai", "OpenAI", "#10a37f"),
        ("ANTHROPIC_API_KEY", "anthropic", "Anthropic", "#d97706"),
        ("GOOGLE_API_KEY", "google", "Google AI", "#4285f4"),
        ("GEMINI_API_KEY", "gemini", "Gemini", "#4285f4"),
        ("MISTRAL_API_KEY", "mistral", "Mistral", "#ff7000"),
        ("COHERE_API_KEY", "cohere", "Cohere", "#39594d"),
        ("GROQ_API_KEY", "groq", "Groq", "#f55036"),
        ("XAI_API_KEY", "xai", "xAI / Grok", "#1d9bf0"),
        ("TOGETHER_API_KEY", "together", "Together AI", "#0066ff"),
        ("FIREWORKS_API_KEY", "fireworks", "Fireworks AI", "#ff6b35"),
        ("PERPLEXITY_API_KEY", "perplexity", "Perplexity", "#20808d"),
        // 中国 AI
        ("DEEPSEEK_API_KEY", "deepseek", "DeepSeek (深度求索)", "#4d6bfe"),
        ("MOONSHOT_API_KEY", "kimi", "Kimi (月之暗面)", "#6c5ce7"),
        ("KIMI_API_KEY", "kimi2", "Kimi", "#6c5ce7"),
        ("DASHSCOPE_API_KEY", "qwen", "通义千问 (Qwen)", "#ff6a00"),
        ("QWEN_API_KEY", "qwen2", "通义千问", "#ff6a00"),
        ("ZHIPU_API_KEY", "zhipu", "智谱 (GLM)", "#0052d9"),
        ("GLM_API_KEY", "glm", "智谱 GLM", "#0052d9"),
        ("BAIDU_API_KEY", "ernie", "文心一言 (ERNIE)", "#2932e1"),
        ("ERNIE_API_KEY", "ernie2", "文心一言", "#2932e1"),
        ("BAICHUAN_API_KEY", "baichuan", "百川智能", "#00b4d8"),
        ("MINIMAX_API_KEY", "minimax", "MiniMax", "#ff2d55"),
        ("STEPFUN_API_KEY", "stepfun", "阶跃星辰", "#7b68ee"),
        ("LINGYI_API_KEY", "lingyi", "零一万物 (Yi)", "#00c853"),
        ("YI_API_KEY", "yi", "Yi (零一万物)", "#00c853"),
        ("SILICONFLOW_API_KEY", "siliconflow", "SiliconFlow (硅基流动)", "#ff6b6b"),
        ("VOLCENGINE_API_KEY", "doubao", "豆包 (火山引擎)", "#fe2c55"),
    ];

    let mut results = Vec::new();
    for (env_key, id, name, color) in env_map {
        if let Some(val) = crate::keystore::registry_get(env_key) {
            if !val.is_empty() {
                let masked = if val.len() > 8 {
                    format!("{}...{}", &val[..4], &val[val.len() - 4..])
                } else {
                    "***".to_string()
                };
                results.push(DetectedProvider {
                    id: id.to_string(),
                    name: name.to_string(),
                    status: "connected".to_string(),
                    detection_method: "env_var".to_string(),
                    detection_detail: format!("Found {} ({})", env_key, masked),
                    color: color.to_string(),
                    plan: None,
                });
            }
        }
    }
    results
}

fn scan_config_files() -> Vec<DetectedProvider> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let config_paths: Vec<(PathBuf, &str, &str, &str)> = vec![
        (home.join(".claude"), "claude", "Claude Code", "#d97706"),
        (home.join(".cursor"), "cursor", "Cursor", "#00d4aa"),
        (home.join(".codex"), "codex", "Codex CLI (OpenAI)", "#10a37f"),
        (
            home.join(".config/github-copilot"),
            "copilot",
            "GitHub Copilot",
            "#6e40c9",
        ),
        (
            home.join(".config/gcloud"),
            "gcloud",
            "Google Cloud / Vertex AI",
            "#4285f4",
        ),
        (home.join(".openai"), "openai_config", "OpenAI", "#10a37f"),
        (home.join(".gemini"), "gemini_cli", "Gemini CLI", "#4285f4"),
        (home.join(".config/gemini"), "gemini_config", "Gemini", "#4285f4"),
        (home.join(".aider"), "aider", "Aider", "#00bfff"),
        (home.join(".continue"), "continue_ai", "Continue", "#ff4785"),
        (home.join(".cline"), "cline", "Cline", "#ff6b35"),
    ];

    let mut results = Vec::new();
    for (path, id, name, color) in config_paths {
        if path.exists() {
            results.push(DetectedProvider {
                id: id.to_string(),
                name: name.to_string(),
                status: "connected".to_string(),
                detection_method: "config_file".to_string(),
                detection_detail: format!("Found config at {}", path.display()),
                color: color.to_string(),
                plan: None,
            });
        }
    }
    results
}

fn scan_ide_plugins() -> Vec<DetectedProvider> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let mut results = Vec::new();
    let vscode_ext = home.join(".vscode/extensions");
    if vscode_ext.exists() {
        if let Ok(entries) = std::fs::read_dir(&vscode_ext) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains("github.copilot") {
                    results.push(DetectedProvider {
                        id: "copilot_vscode".to_string(),
                        name: "GitHub Copilot (VS Code)".to_string(),
                        status: "connected".to_string(),
                        detection_method: "ide_plugin".to_string(),
                        detection_detail: "Found VS Code Copilot extension".to_string(),
                        color: "#6e40c9".to_string(),
                        plan: None,
                    });
                }
                if name.contains("continue") {
                    results.push(DetectedProvider {
                        id: "continue_vscode".to_string(),
                        name: "Continue (VS Code)".to_string(),
                        status: "connected".to_string(),
                        detection_method: "ide_plugin".to_string(),
                        detection_detail: "Found VS Code Continue extension".to_string(),
                        color: "#ff4785".to_string(),
                        plan: None,
                    });
                }
            }
        }
    }
    results
}

fn scan_dotenv_files() -> Vec<DetectedProvider> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let search_dirs = vec![
        home.join("Projects"),
        home.join("projects"),
        home.join("Developer"),
        home.join("dev"),
        home.join("code"),
        home.join("workspace"),
        home.join("Desktop"),
    ];

    let ai_keys = vec![
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GOOGLE_API_KEY",
        "GEMINI_API_KEY",
    ];

    let mut found_keys: HashMap<String, String> = HashMap::new();

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(3)
            .into_iter()
            .flatten()
        {
            if entry.file_name() == ".env" || entry.file_name() == ".env.local" {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        for key in &ai_keys {
                            if line.starts_with(key) && line.contains('=') {
                                found_keys
                                    .entry(key.to_string())
                                    .or_insert_with(|| entry.path().display().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    found_keys
        .into_iter()
        .map(|(key, path)| DetectedProvider {
            id: format!("dotenv_{}", key.to_lowercase()),
            name: format!("{} (from .env)", key),
            status: "connected".to_string(),
            detection_method: "dotenv_file".to_string(),
            detection_detail: format!("Found {} in {}", key, path),
            color: "#94a3b8".to_string(),
            plan: None,
        })
        .collect()
}

pub fn scan_all() -> Vec<DetectedProvider> {
    let mut all = Vec::new();
    all.extend(scan_env_vars());
    all.extend(scan_config_files());
    all.extend(scan_ide_plugins());
    all.extend(scan_dotenv_files());

    // Deduplicate by base provider id
    let mut seen = HashMap::new();
    let mut deduped = Vec::new();
    for p in all {
        let base_id = p.id.replace("_config", "").replace("_vscode", "");
        if !seen.contains_key(&base_id) {
            seen.insert(base_id, true);
            deduped.push(p);
        }
    }
    deduped
}
