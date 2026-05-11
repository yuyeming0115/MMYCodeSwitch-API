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
    let settings_path = Path::new(config_dir).join("settings.local.json");
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
    // 1. 确保项目专属目录存在（用于归档历史记录）
    let _mmycs_config_dir = crate::config::ensure_project_config_dir(project_path)?;

    // 2. ★ 关键：写入到项目目录下的 .claude/settings.local.json（Claude Code CLI 会读取这个）
    let project_claude_dir = Path::new(project_path).join(".claude");
    std::fs::create_dir_all(&project_claude_dir)?;
    let settings_path = project_claude_dir.join("settings.local.json");

    // 3. 备份现有 settings.local.json（如果有）
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

    // 4. 写入 settings.local.json
    std::fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;

    // 5. 更新绑定记录
    crate::config::update_project_binding(project_path, &provider.id, &provider.name)?;

    // 6. 归档会话（记录配置快照，Token 字段脱敏）- 从 root 中重新读取 env
    let config_snapshot: HashMap<String, String> = root.get("env")
        .and_then(|e| e.as_object())
        .map(|env| env.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| {
                let val = if k == "ANTHROPIC_AUTH_TOKEN" {
                    format!("{}***", &s[..s.len().min(8)])
                } else {
                    s.to_string()
                };
                (k.clone(), val)
            })).collect())
        .unwrap_or_default();
    crate::config::archive_session(project_path, provider, config_snapshot)?;

    // 7. 记录切换日志
    log_switch(&project_claude_dir.to_string_lossy(), &provider.name)?;

    // 8. ★ 注入/更新项目 CLAUDE.md（标记块方式，不影响用户已有内容）
    if let Err(e) = inject_claude_md(project_path, &provider.name, provider.models.as_ref()) {
        // CLAUDE.md 注入失败不阻塞主流程（settings.json 已写入成功）
        eprintln!("[MMYCS] CLAUDE.md 注入警告: {}", e);
    }

    // 返回项目目录下的 .claude 路径（这才是实际生效的配置目录）
    Ok(project_claude_dir.to_string_lossy().to_string())
}

// ── CLAUDE.md 标记块注入/清理 ────────────────────────────────────────────

/// CLAUDE.md 注入标记（用于定位和替换我们的内容）
const CLAUDE_MD_MARKER_START: &str = "<!-- MMY-INJECT:START -->";
const CLAUDE_MD_MARKER_END: &str = "<!-- MMY-INJECT:END -->";

/// 构建模型身份提示词内容
fn build_claude_md_content(provider_name: &str, models: Option<&HashMap<String, String>>) -> String {
    let model_display = models
        .and_then(|m| m.get("default"))
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or("未知模型");

    format!(
        r#"## 模型信息（由 MMYCodeSwitch-API 自动管理）

> 当前使用的模型为 **{model}**，通过 **{provider}** 提供服务。
> 此段内容由 `<!-- MMY-INJECT:START -->` 和 `<!-- MMY-INJECT:END -->` 包裹，
> 切换供应商或解绑项目时会自动更新，请勿手动编辑此区域。
"#,
        model = model_display,
        provider = provider_name,
    )
}

/// 向项目目录注入/更新 CLAUDE.md（安全标记块方式）
/// 
/// 策略：
/// - 文件不存在 → 新建文件并写入标记块
/// - 文件存在但无标记块 → 在末尾追加标记块
/// - 文件存在且有标记块 → 只替换标记块内部内容（保留用户原有内容）
pub fn inject_claude_md(
    project_path: &str,
    provider_name: &str,
    models: Option<&HashMap<String, String>>,
) -> Result<()> {
    let claude_md_path = Path::new(project_path).join("CLAUDE.md");
    let new_content = build_claude_md_content(provider_name, models);
    let marker_block = format!(
        "{}\n{}\n{}",
        CLAUDE_MD_MARKER_START,
        new_content,
        CLAUDE_MD_MARKER_END,
    );

    if !claude_md_path.exists() {
        // 场景 A：文件不存在 → 直接新建
        std::fs::write(&claude_md_path, marker_block)?;
        return Ok(());
    }

    // 场景 B/C：文件已存在
    let existing = std::fs::read_to_string(&claude_md_path)?;

    if existing.contains(CLAUDE_MD_MARKER_START) && existing.contains(CLAUDE_MD_MARKER_END) {
        // 场景 C：有旧标记块 → 替换标记块内部内容
        let updated = replace_marker_block(&existing, &marker_block);
        std::fs::write(&claude_md_path, updated)?;
    } else {
        // 场景 B：无标记块 → 追加到末尾
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&claude_md_path)?;

        // 如果原文件末尾没有空行，先补一个换行
        let needs_separator = !existing.ends_with('\n');
        if needs_separator {
            writeln!(f)?;
        }
        writeln!(f)?;
        f.write_all(marker_block.as_bytes())?;
        writeln!(f)?;
    }

    Ok(())
}

/// 仅移除 CLAUDE.md 中的 MMY 标记块（保留用户所有其他内容）
/// 如果整个文件只有我们的标记块，则删除文件
pub fn remove_claude_md_block(project_path: &str) -> Result<bool> {
    let claude_md_path = Path::new(project_path).join("CLAUDE.md");

    if !claude_md_path.exists() {
        return Ok(false); // 文件不存在，无需操作
    }

    let content = std::fs::read_to_string(&claude_md_path)?;

    if !content.contains(CLAUDE_MD_MARKER_START) || !content.contains(CLAUDE_MD_MARKER_END) {
        return Ok(false); // 无标记块，不动文件
    }

    let cleaned = remove_marker_block_from_content(&content);

    // 检查清除后是否只剩空白
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        // 整个文件都是我们的内容 → 删除文件
        std::fs::remove_file(&claude_md_path)?;
        return Ok(true);
    }

    // 保留剩余内容
    std::fs::write(&claude_md_path, cleaned.trim_end())?;
    Ok(true)
}

/// 替换标记块及其内容（保留标记前后的所有内容）
fn replace_marker_block(content: &str, new_marker_block: &str) -> String {
    let start_idx = content.find(CLAUDE_MD_MARKER_START).unwrap();
    let end_idx = content.rfind(CLAUDE_MD_MARKER_END).unwrap() + CLAUDE_MD_MARKER_END.len();

    format!(
        "{}\n{}",
        &content[..start_idx].trim_end(),
        new_marker_block.trim(),
    ) + &content[end_idx..]
}

/// 移除标记块及其内容（保留标记前后的所有内容）
fn remove_marker_block_from_content(content: &str) -> String {
    let start_idx = content.find(CLAUDE_MD_MARKER_START).unwrap();
    let end_idx = content.rfind(CLAUDE_MD_MARKER_END).unwrap() + CLAUDE_MD_MARKER_END.len();

    let before = content[..start_idx].trim_end().to_string();
    let after = content[end_idx..].trim_start().to_string();

    match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (true, false) => after,
        (false, true) => before,
        (false, false) => format!("{}\n\n{}", before, after),
    }
}

/// 清理项目级 settings.local.json 中的 API Key 字段
pub fn clean_project_settings(project_path: &str) -> Result<()> {
    let settings_path = Path::new(project_path)
        .join(".claude")
        .join("settings.local.json");
    if !settings_path.exists() { return Ok(()); }

    let mut root: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?;
    if let Some(env) = root.get_mut("env").and_then(|e| e.as_object_mut()) {
        for key in ENV_KEYS { env.remove(*key); }
        if env.is_empty() {
            root.as_object_mut().unwrap().remove("env");
        }
    }
    if root.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        std::fs::remove_file(&settings_path)?;
    } else {
        std::fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;
    }
    Ok(())
}
