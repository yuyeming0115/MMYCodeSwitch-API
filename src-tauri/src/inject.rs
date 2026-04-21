use anyhow::Result;
use chrono::Local;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use crate::config::Provider;

// 需要清除的 API 相关字段（login 模式或切换 provider 时全部移除）
const ENV_KEYS: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    // 分层模型字段 — 切换 provider 时必须全部清除，避免残留
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_REASONING_MODEL",
];

pub fn inject(config_dir: &str, provider: &Provider, api_key_plain: Option<&str>) -> Result<()> {
    let settings_path = Path::new(config_dir).join("settings.json");
    backup(&settings_path)?;

    let mut root: Value = if settings_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?
    } else {
        serde_json::json!({})
    };

    let env = root.as_object_mut().unwrap();
    let env_map = env.entry("env").or_insert(serde_json::json!({}));
    let env_obj = env_map.as_object_mut().unwrap();

    // ── 先清除所有 API 相关字段（避免残留旧的分层模型值）────
    if provider.provider_type == "login" {
        // login 模式：全部移除
        for key in ENV_KEYS {
            env_obj.remove(*key);
        }
    } else {
        // API 模式：先清除旧字段，再写入新值
        for key in ENV_KEYS {
            env_obj.remove(*key);
        }

        if let Some(key) = api_key_plain {
            env_obj.insert("ANTHROPIC_AUTH_TOKEN".to_string(), Value::String(key.to_string()));
        }
        if let Some(url) = &provider.base_url {
            env_obj.insert("ANTHROPIC_BASE_URL".to_string(), Value::String(url.clone()));
        }
        if let Some(models) = &provider.models {
            let pairs = [
                ("default", "ANTHROPIC_MODEL"),
                ("haiku", "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
                ("sonnet", "ANTHROPIC_DEFAULT_SONNET_MODEL"),
                ("opus", "ANTHROPIC_DEFAULT_OPUS_MODEL"),
                ("reasoning", "ANTHROPIC_REASONING_MODEL"),
            ];
            for (model_key, env_key) in &pairs {
                if let Some(v) = models.get(*model_key) {
                    if !v.is_empty() {
                        env_obj.insert(env_key.to_string(), Value::String(v.clone()));
                    }
                }
            }
        }
    }

    std::fs::create_dir_all(config_dir)?;
    std::fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;
    log_switch(config_dir, &provider.name)?;
    Ok(())
}

fn backup(settings_path: &Path) -> Result<()> {
    if !settings_path.exists() { return Ok(()); }
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".mmycs").join("backups");
    std::fs::create_dir_all(&backup_dir)?;
    let dest = backup_dir.join(format!("settings_{}.json", ts));
    std::fs::copy(settings_path, dest)?;
    Ok(())
}

fn log_switch(config_dir: &str, provider_name: &str) -> Result<()> {
    let log_path = dirs::home_dir().unwrap_or_default().join(".mmycs").join("logs").join("switch.log");
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] {} -> {}\n", ts, config_dir, provider_name);
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(log_path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

// ── 项目专属目录注入（方案C）───────────────────────────────────────────────

/// 注入 API 配置到项目专属配置目录
/// 返回项目专属目录路径
pub fn inject_to_project_dir(project_path: &str, provider: &Provider, api_key_plain: Option<&str>) -> Result<String> {
    // 1. 确保项目专属目录存在
    let config_dir = crate::config::ensure_project_config_dir(project_path)?;
    let settings_path = config_dir.join("settings.json");

    // 2. 备份现有 settings.json（如果有）
    if settings_path.exists() {
        backup(&settings_path)?;
    }

    // 3. 构建/更新 settings.json
    let mut root: Value = if settings_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?
    } else {
        serde_json::json!({})
    };

    {
        let root_obj = root.as_object_mut().unwrap();
        let env_map = root_obj.entry("env").or_insert(serde_json::json!({}));
        let env = env_map.as_object_mut().unwrap();

        // 先清除所有 API 相关字段
        for key in ENV_KEYS {
            env.remove(*key);
        }

        // 写入新配置
        if provider.provider_type == "api" {
            if let Some(key) = api_key_plain {
                env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), Value::String(key.to_string()));
            }
            if let Some(url) = &provider.base_url {
                env.insert("ANTHROPIC_BASE_URL".to_string(), Value::String(url.clone()));
            }
            if let Some(models) = &provider.models {
                let pairs = [
                    ("default", "ANTHROPIC_MODEL"),
                    ("haiku", "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
                    ("sonnet", "ANTHROPIC_DEFAULT_SONNET_MODEL"),
                    ("opus", "ANTHROPIC_DEFAULT_OPUS_MODEL"),
                    ("reasoning", "ANTHROPIC_REASONING_MODEL"),
                ];
                for (model_key, env_key) in &pairs {
                    if let Some(v) = models.get(*model_key) {
                        if !v.is_empty() {
                            env.insert(env_key.to_string(), Value::String(v.clone()));
                        }
                    }
                }
            }
        }
    }

    // 4. 写入 settings.json
    std::fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;

    // 5. 更新绑定记录
    crate::config::update_project_binding(project_path, &provider.id, &provider.name)?;

    // 6. 归档会话（记录配置快照）- 从 root 中重新读取 env
    let config_snapshot: HashMap<String, String> = root.get("env")
        .and_then(|e| e.as_object())
        .map(|env| env.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect())
        .unwrap_or_default();
    crate::config::archive_session(project_path, provider, config_snapshot)?;

    // 7. 记录切换日志
    log_switch(&config_dir.to_string_lossy(), &provider.name)?;

    Ok(config_dir.to_string_lossy().to_string())
}
