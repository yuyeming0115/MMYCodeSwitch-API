use anyhow::Result;
use chrono::Local;
use serde_json::Value;
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
