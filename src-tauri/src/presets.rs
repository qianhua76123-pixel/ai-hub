use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub category: String,
    pub api_format: String,
    pub base_url: String,
    pub env_key: String,
    pub default_model: String,
    pub models: Vec<String>,
    pub color: String,
    pub description: String,
    pub doc_url: String,
}

pub fn get_all_presets() -> Vec<ProviderPreset> {
    vec![
        // ===== International =====
        ProviderPreset {
            id: "openai".into(), name: "OpenAI".into(), category: "international".into(),
            api_format: "openai".into(), base_url: "https://api.openai.com/v1".into(),
            env_key: "OPENAI_API_KEY".into(), default_model: "gpt-4o".into(),
            models: vec!["gpt-4o".into(), "gpt-4o-mini".into(), "o3".into(), "o3-mini".into(), "o4-mini".into()],
            color: "#10a37f".into(), description: "GPT-4o, o3, o4-mini".into(),
            doc_url: "https://platform.openai.com/docs".into(),
        },
        ProviderPreset {
            id: "anthropic".into(), name: "Anthropic".into(), category: "international".into(),
            api_format: "anthropic".into(), base_url: "https://api.anthropic.com/v1".into(),
            env_key: "ANTHROPIC_API_KEY".into(), default_model: "claude-sonnet-4-6".into(),
            models: vec!["claude-opus-4-6".into(), "claude-sonnet-4-6".into(), "claude-haiku-4-5-20251001".into()],
            color: "#d97706".into(), description: "Claude Opus, Sonnet, Haiku".into(),
            doc_url: "https://docs.anthropic.com".into(),
        },
        ProviderPreset {
            id: "google".into(), name: "Google AI".into(), category: "international".into(),
            api_format: "openai".into(), base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            env_key: "GOOGLE_API_KEY".into(), default_model: "gemini-2.5-pro".into(),
            models: vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into(), "gemini-2.0-flash".into()],
            color: "#4285f4".into(), description: "Gemini 2.5 Pro/Flash".into(),
            doc_url: "https://ai.google.dev/docs".into(),
        },
        ProviderPreset {
            id: "mistral".into(), name: "Mistral AI".into(), category: "international".into(),
            api_format: "openai".into(), base_url: "https://api.mistral.ai/v1".into(),
            env_key: "MISTRAL_API_KEY".into(), default_model: "mistral-large-latest".into(),
            models: vec!["mistral-large-latest".into(), "mistral-medium-latest".into(), "codestral-latest".into()],
            color: "#ff7000".into(), description: "Mistral Large, Codestral".into(),
            doc_url: "https://docs.mistral.ai".into(),
        },
        ProviderPreset {
            id: "cohere".into(), name: "Cohere".into(), category: "international".into(),
            api_format: "openai".into(), base_url: "https://api.cohere.com/v2".into(),
            env_key: "COHERE_API_KEY".into(), default_model: "command-r-plus".into(),
            models: vec!["command-r-plus".into(), "command-r".into()],
            color: "#39594d".into(), description: "Command R+, R".into(),
            doc_url: "https://docs.cohere.com".into(),
        },
        ProviderPreset {
            id: "xai".into(), name: "xAI".into(), category: "international".into(),
            api_format: "openai".into(), base_url: "https://api.x.ai/v1".into(),
            env_key: "XAI_API_KEY".into(), default_model: "grok-3".into(),
            models: vec!["grok-3".into(), "grok-3-mini".into(), "grok-2".into()],
            color: "#1d9bf0".into(), description: "Grok 3, Grok 2".into(),
            doc_url: "https://docs.x.ai".into(),
        },
        // ===== China =====
        ProviderPreset {
            id: "deepseek".into(), name: "DeepSeek".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.deepseek.com/v1".into(),
            env_key: "DEEPSEEK_API_KEY".into(), default_model: "deepseek-chat".into(),
            models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
            color: "#4d6bfe".into(), description: "DeepSeek V3, R1".into(),
            doc_url: "https://api-docs.deepseek.com".into(),
        },
        ProviderPreset {
            id: "kimi".into(), name: "Kimi (月之暗面)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.moonshot.cn/v1".into(),
            env_key: "MOONSHOT_API_KEY".into(), default_model: "moonshot-v1-128k".into(),
            models: vec!["moonshot-v1-128k".into(), "moonshot-v1-32k".into(), "moonshot-v1-8k".into()],
            color: "#6c5ce7".into(), description: "Moonshot 128K 长上下文".into(),
            doc_url: "https://platform.moonshot.cn/docs".into(),
        },
        ProviderPreset {
            id: "qwen".into(), name: "通义千问 (Qwen)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            env_key: "DASHSCOPE_API_KEY".into(), default_model: "qwen-max".into(),
            models: vec!["qwen-max".into(), "qwen-plus".into(), "qwen-turbo".into(), "qwen-long".into()],
            color: "#ff6a00".into(), description: "Qwen Max/Plus/Turbo".into(),
            doc_url: "https://help.aliyun.com/zh/dashscope".into(),
        },
        ProviderPreset {
            id: "zhipu".into(), name: "智谱 (GLM)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            env_key: "ZHIPU_API_KEY".into(), default_model: "glm-4-plus".into(),
            models: vec!["glm-4-plus".into(), "glm-4".into(), "glm-4-flash".into()],
            color: "#0052d9".into(), description: "GLM-4 Plus/Flash".into(),
            doc_url: "https://open.bigmodel.cn/dev/api".into(),
        },
        ProviderPreset {
            id: "ernie".into(), name: "文心一言 (ERNIE)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop".into(),
            env_key: "BAIDU_API_KEY".into(), default_model: "ernie-4.0".into(),
            models: vec!["ernie-4.0".into(), "ernie-3.5".into(), "ernie-speed".into()],
            color: "#2932e1".into(), description: "ERNIE 4.0/3.5".into(),
            doc_url: "https://cloud.baidu.com/doc/WENXINWORKSHOP".into(),
        },
        ProviderPreset {
            id: "baichuan".into(), name: "百川智能".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.baichuan-ai.com/v1".into(),
            env_key: "BAICHUAN_API_KEY".into(), default_model: "Baichuan4".into(),
            models: vec!["Baichuan4".into(), "Baichuan3-Turbo".into()],
            color: "#00b4d8".into(), description: "百川 4".into(),
            doc_url: "https://platform.baichuan-ai.com/docs".into(),
        },
        ProviderPreset {
            id: "minimax".into(), name: "MiniMax".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.minimax.chat/v1".into(),
            env_key: "MINIMAX_API_KEY".into(), default_model: "abab6.5s-chat".into(),
            models: vec!["abab6.5s-chat".into(), "abab6-chat".into()],
            color: "#ff2d55".into(), description: "ABAB 6.5".into(),
            doc_url: "https://www.minimaxi.com/document".into(),
        },
        ProviderPreset {
            id: "stepfun".into(), name: "阶跃星辰".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.stepfun.com/v1".into(),
            env_key: "STEPFUN_API_KEY".into(), default_model: "step-2-16k".into(),
            models: vec!["step-2-16k".into(), "step-1-256k".into()],
            color: "#7b68ee".into(), description: "Step 2/1".into(),
            doc_url: "https://platform.stepfun.com/docs".into(),
        },
        ProviderPreset {
            id: "yi".into(), name: "零一万物 (Yi)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.lingyiwanwu.com/v1".into(),
            env_key: "YI_API_KEY".into(), default_model: "yi-large".into(),
            models: vec!["yi-large".into(), "yi-medium".into(), "yi-spark".into()],
            color: "#00c853".into(), description: "Yi Large/Medium".into(),
            doc_url: "https://platform.lingyiwanwu.com/docs".into(),
        },
        ProviderPreset {
            id: "doubao".into(), name: "豆包 (火山引擎)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://ark.cn-beijing.volces.com/api/v3".into(),
            env_key: "VOLCENGINE_API_KEY".into(), default_model: "doubao-pro-128k".into(),
            models: vec!["doubao-pro-128k".into(), "doubao-pro-32k".into()],
            color: "#fe2c55".into(), description: "豆包 Pro/Lite".into(),
            doc_url: "https://www.volcengine.com/docs/82379".into(),
        },
        ProviderPreset {
            id: "hunyuan".into(), name: "混元 (腾讯)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://api.hunyuan.cloud.tencent.com/v1".into(),
            env_key: "HUNYUAN_API_KEY".into(), default_model: "hunyuan-pro".into(),
            models: vec!["hunyuan-pro".into(), "hunyuan-standard".into()],
            color: "#006eff".into(), description: "混元 Pro/Standard".into(),
            doc_url: "https://cloud.tencent.com/document/product/1729".into(),
        },
        ProviderPreset {
            id: "spark".into(), name: "星火 (讯飞)".into(), category: "china".into(),
            api_format: "openai".into(), base_url: "https://spark-api-open.xf-yun.com/v1".into(),
            env_key: "SPARK_API_KEY".into(), default_model: "4.0Ultra".into(),
            models: vec!["4.0Ultra".into(), "generalv3.5".into()],
            color: "#1677ff".into(), description: "星火 4.0 Ultra".into(),
            doc_url: "https://www.xfyun.cn/doc/spark".into(),
        },
        // ===== Aggregators =====
        ProviderPreset {
            id: "groq".into(), name: "Groq".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://api.groq.com/openai/v1".into(),
            env_key: "GROQ_API_KEY".into(), default_model: "llama-3.3-70b-versatile".into(),
            models: vec!["llama-3.3-70b-versatile".into(), "llama-3.1-8b-instant".into(), "mixtral-8x7b-32768".into()],
            color: "#f55036".into(), description: "超快推理，Llama/Mixtral".into(),
            doc_url: "https://console.groq.com/docs".into(),
        },
        ProviderPreset {
            id: "openrouter".into(), name: "OpenRouter".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://openrouter.ai/api/v1".into(),
            env_key: "OPENROUTER_API_KEY".into(), default_model: "anthropic/claude-sonnet-4".into(),
            models: vec!["anthropic/claude-sonnet-4".into(), "openai/gpt-4o".into(), "google/gemini-2.5-pro".into()],
            color: "#6366f1".into(), description: "多模型聚合，统一 API".into(),
            doc_url: "https://openrouter.ai/docs".into(),
        },
        ProviderPreset {
            id: "together".into(), name: "Together AI".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://api.together.xyz/v1".into(),
            env_key: "TOGETHER_API_KEY".into(), default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            models: vec!["meta-llama/Llama-3.3-70B-Instruct-Turbo".into(), "deepseek-ai/DeepSeek-V3".into()],
            color: "#0066ff".into(), description: "开源模型托管".into(),
            doc_url: "https://docs.together.ai".into(),
        },
        ProviderPreset {
            id: "fireworks".into(), name: "Fireworks AI".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://api.fireworks.ai/inference/v1".into(),
            env_key: "FIREWORKS_API_KEY".into(), default_model: "accounts/fireworks/models/llama-v3p3-70b-instruct".into(),
            models: vec!["accounts/fireworks/models/llama-v3p3-70b-instruct".into()],
            color: "#ff6b35".into(), description: "高性能推理".into(),
            doc_url: "https://docs.fireworks.ai".into(),
        },
        ProviderPreset {
            id: "perplexity".into(), name: "Perplexity".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://api.perplexity.ai".into(),
            env_key: "PERPLEXITY_API_KEY".into(), default_model: "sonar-pro".into(),
            models: vec!["sonar-pro".into(), "sonar".into(), "sonar-reasoning-pro".into()],
            color: "#20808d".into(), description: "联网搜索增强".into(),
            doc_url: "https://docs.perplexity.ai".into(),
        },
        ProviderPreset {
            id: "siliconflow".into(), name: "SiliconFlow (硅基流动)".into(), category: "aggregator".into(),
            api_format: "openai".into(), base_url: "https://api.siliconflow.cn/v1".into(),
            env_key: "SILICONFLOW_API_KEY".into(), default_model: "deepseek-ai/DeepSeek-V3".into(),
            models: vec!["deepseek-ai/DeepSeek-V3".into(), "Pro/deepseek-ai/DeepSeek-R1".into()],
            color: "#ff6b6b".into(), description: "国产模型聚合".into(),
            doc_url: "https://docs.siliconflow.cn".into(),
        },
        // ===== Cloud =====
        ProviderPreset {
            id: "azure_openai".into(), name: "Azure OpenAI".into(), category: "cloud".into(),
            api_format: "openai".into(), base_url: "https://{resource}.openai.azure.com/openai/deployments/{deployment}".into(),
            env_key: "AZURE_OPENAI_API_KEY".into(), default_model: "gpt-4o".into(),
            models: vec!["gpt-4o".into(), "gpt-4o-mini".into()],
            color: "#0078d4".into(), description: "Azure 托管的 OpenAI".into(),
            doc_url: "https://learn.microsoft.com/azure/ai-services/openai".into(),
        },
        ProviderPreset {
            id: "bedrock".into(), name: "AWS Bedrock".into(), category: "cloud".into(),
            api_format: "anthropic".into(), base_url: "https://bedrock-runtime.{region}.amazonaws.com".into(),
            env_key: "AWS_ACCESS_KEY_ID".into(), default_model: "anthropic.claude-sonnet-4-6".into(),
            models: vec!["anthropic.claude-sonnet-4-6".into(), "amazon.nova-pro-v1:0".into()],
            color: "#ff9900".into(), description: "AWS 托管多模型".into(),
            doc_url: "https://docs.aws.amazon.com/bedrock".into(),
        },
        ProviderPreset {
            id: "vertex".into(), name: "Google Vertex AI".into(), category: "cloud".into(),
            api_format: "openai".into(), base_url: "https://{region}-aiplatform.googleapis.com/v1".into(),
            env_key: "GOOGLE_APPLICATION_CREDENTIALS".into(), default_model: "gemini-2.5-pro".into(),
            models: vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into()],
            color: "#34a853".into(), description: "Google Cloud AI 平台".into(),
            doc_url: "https://cloud.google.com/vertex-ai/docs".into(),
        },
    ]
}

pub fn save_custom_provider(preset_id: &str, api_key: &str) -> Result<String, String> {
    let presets = get_all_presets();
    let preset = presets.iter().find(|p| p.id == preset_id)
        .ok_or(format!("未找到 Provider: {}", preset_id))?;

    // Store key securely via Keychain (macOS) or obfuscated file
    crate::keystore::store_key(preset_id, api_key)?;

    // Set env var for current session
    crate::keystore::registry_set(&preset.env_key, api_key);

    // Save provider registry (without the key!) so we know which providers are configured
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
        .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
        .join("ai-hub");
    std::fs::create_dir_all(&data_dir).ok();

    let config_path = data_dir.join("custom_providers.json");
    let mut providers: Vec<serde_json::Value> = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    // Only store provider ID + env_key, NOT the actual API key
    if !providers.iter().any(|p| p["id"].as_str() == Some(preset_id)) {
        providers.push(serde_json::json!({
            "id": preset_id,
            "env_key": preset.env_key,
        }));
    }

    let content = serde_json::to_string_pretty(&providers).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, content).map_err(|e| e.to_string())?;

    Ok(format!("{} 已添加", preset.name))
}

pub fn load_custom_providers() {
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
        .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
        .join("ai-hub");
    let config_path = data_dir.join("custom_providers.json");

    if !config_path.exists() { return; }

    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(providers) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            for p in &providers {
                let preset_id = match p["id"].as_str() { Some(id) => id, None => continue };
                let env_key = match p["env_key"].as_str() { Some(k) => k, None => continue };

                // Try loading from secure keystore first
                if let Some(api_key) = crate::keystore::get_key(preset_id) {
                    if !api_key.is_empty() {
                        crate::keystore::registry_set(env_key, &api_key);
                        continue;
                    }
                }

                // Fallback: migrate old plaintext key to keystore
                if let Some(old_key) = p["api_key"].as_str() {
                    if !old_key.is_empty() {
                        crate::keystore::registry_set(env_key, old_key);
                        // Migrate to keystore
                        if crate::keystore::store_key(preset_id, old_key).is_ok() {
                            println!("[Migration] Migrated API key for {} to secure storage", preset_id);
                        }
                    }
                }
            }

            // After migration, rewrite config without plaintext keys
            let cleaned: Vec<serde_json::Value> = providers.iter().map(|p| {
                serde_json::json!({
                    "id": p["id"],
                    "env_key": p["env_key"],
                })
            }).collect();
            if let Ok(c) = serde_json::to_string_pretty(&cleaned) {
                std::fs::write(&config_path, c).ok();
            }
        }
    }
}

pub fn remove_custom_provider(preset_id: &str) -> Result<String, String> {
    let presets = get_all_presets();
    if let Some(p) = presets.iter().find(|p| p.id == preset_id) {
        crate::keystore::registry_remove(&p.env_key);
    }

    // Remove from secure keystore
    crate::keystore::delete_key(preset_id).ok();

    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
        .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
        .join("ai-hub");
    let config_path = data_dir.join("custom_providers.json");

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(mut providers) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                providers.retain(|p| p["id"].as_str() != Some(preset_id));
                let content = serde_json::to_string_pretty(&providers).map_err(|e| e.to_string())?;
                std::fs::write(&config_path, content).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(format!("已移除 {}", preset_id))
}
