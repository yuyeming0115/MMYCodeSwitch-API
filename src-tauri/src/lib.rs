use tauri::{
    menu::{MenuItem, MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager,
};

use chrono::Timelike;

mod config;
mod crypto;
mod inject;

use config::{AppConfig, ActiveProject, Provider, SessionArchive, ProviderTemplate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: u8,
    pub providers: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct DetectedInstance {
    pub name: String,
    pub config_dir: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProviderInput {
    pub id: Option<String>,
    pub name: String,
    pub icon_fallback: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub api_key_plain: Option<String>,
    pub models: Option<HashMap<String, String>>,
    pub notes: Option<String>,
    pub icon_path: Option<String>,
}

#[tauri::command]
fn init_app() -> Result<(), String> {
    config::ensure_dirs().map_err(|e| e.to_string())?;
    config::get_or_create_key().map_err(|e| e.to_string())?;
    // 执行 v1 → v2 数据结构迁移
    match config::migrate_v1_to_v2() {
        Ok(true) => eprintln!("[MMYCS] ✓ 已完成 v1 → v2 数据结构迁移（供应商分目录）"),
        Ok(false) => eprintln!("[MMYCS] 数据已是 v2 格式，无需迁移"),
        Err(e) => eprintln!("[MMYCS] ✗ v1 → v2 迁移失败: {}", e),
    }
    Ok(())
}

#[tauri::command]
fn get_app_config() -> Result<AppConfig, String> {
    config::load_app_config().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_app_config(cfg: AppConfig) -> Result<(), String> {
    config::save_app_config(&cfg).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_providers() -> Result<Vec<Provider>, String> {
    config::load_providers().map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_provider(input: ProviderInput) -> Result<Provider, String> {
    let key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = input.id.unwrap_or_else(|| format!("provider_{}", chrono::Utc::now().timestamp_millis()));
    let existing = config::load_providers().unwrap_or_default();
    let old = existing.iter().find(|p| p.id == id);
    let max_order = existing.iter().map(|p| p.order).max().unwrap_or(0);
    let api_key_encrypted = match &input.api_key_plain {
        Some(k) if !k.is_empty() => Some(crypto::encrypt(k, &key).map_err(|e| e.to_string())?),
        _ => old.and_then(|p| p.api_key_encrypted.clone()),
    };
    let provider = Provider {
        id: id.clone(),
        name: input.name,
        icon_fallback: input.icon_fallback,
        provider_type: input.provider_type,
        base_url: input.base_url,
        api_key_encrypted,
        models: input.models,
        notes: input.notes,
        icon_path: input.icon_path.or_else(|| old.and_then(|p| p.icon_path.clone())),
        order: old.map(|p| p.order).unwrap_or(max_order + 1),
        created_at: old.map(|p| p.created_at.clone()).unwrap_or_else(|| now.clone()),
        updated_at: now,
    };
    config::save_provider(&provider).map_err(|e| e.to_string())?;
    Ok(provider)
}

#[tauri::command]
fn delete_provider(id: String) -> Result<(), String> {
    config::delete_provider(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_providers(ordered_ids: Vec<String>) -> Result<(), String> {
    config::reorder_providers(&ordered_ids).map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_projects(ordered_ids: Vec<String>) -> Result<(), String> {
    config::reorder_projects(&ordered_ids).map_err(|e| e.to_string())
}

#[tauri::command]
fn switch_provider(config_dir: String, provider_id: String) -> Result<(), String> {
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let provider = providers.iter().find(|p| p.id == provider_id)
        .ok_or("Provider not found")?;
    let api_key_plain = if provider.provider_type == "api" {
        if let Some(enc) = &provider.api_key_encrypted {
            let key = config::get_or_create_key().map_err(|e| e.to_string())?;
            Some(crypto::decrypt(enc, &key).map_err(|e| e.to_string())?)
        } else { None }
    } else { None };
    inject::inject(&config_dir, provider, api_key_plain.as_deref())
        .map_err(|e| e.to_string())?;
    let mut cfg = config::load_app_config().map_err(|e| e.to_string())?;
    for inst in &mut cfg.instances {
        if inst.config_dir == config_dir {
            inst.active_provider_id = Some(provider_id.clone());
        }
    }
    config::save_app_config(&cfg).map_err(|e| e.to_string())?;
    Ok(())
}

// ── 多项目模式：注入 API 到指定项目目录 ───────────────────────────────
#[derive(Serialize)]
pub struct InjectToProjectResult {
    pub project: ActiveProject,
    pub was_existing: bool,
    /// 项目专属配置目录路径（新增）
    pub config_dir: String,
}

#[tauri::command]
fn inject_to_project(project_path: String, provider_id: String) -> Result<InjectToProjectResult, String> {
    // 1. 规范化路径
    let norm_path = config::normalize_project_path(&project_path);

    // 2. 加载 Provider 并解密 Key
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let provider = providers.iter().find(|p| p.id == provider_id)
        .ok_or("Provider not found")?;
    let api_key_plain = if provider.provider_type == "api" {
        if let Some(enc) = &provider.api_key_encrypted {
            let key = config::get_or_create_key().map_err(|e| e.to_string())?;
            Some(crypto::decrypt(enc, &key).map_err(|e| e.to_string())?)
        } else { None }
    } else { None };

    // 3. 注入到项目专属配置目录（方案C）
    let config_dir = inject::inject_to_project_dir(&norm_path, provider, api_key_plain.as_deref())
        .map_err(|e| e.to_string())?;

    // 4. 更新 active_projects 记录
    let mut cfg = config::load_app_config().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let project_name = std::path::Path::new(&norm_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知项目")
        .to_string();

    let existing_idx = config::find_active_project_by_path(&cfg, &norm_path);
    let was_existing = existing_idx.is_some();

    let project = if let Some(idx) = existing_idx {
        // 更新已有记录
        cfg.active_projects[idx].provider_id = provider_id.clone();
        cfg.active_projects[idx].provider_name = provider.name.clone();
        cfg.active_projects[idx].updated_at = now.clone();
        cfg.active_projects[idx].config_dir = Some(config_dir.clone());
        cfg.active_projects[idx].clone()
    } else {
        // 新增记录
        let max_order = cfg.active_projects.iter().map(|p| p.order).max().unwrap_or(0);
        let provider_dir = config::provider_name_to_dir(&provider.name);
        let new_proj = ActiveProject {
            id: format!("proj_{}", chrono::Utc::now().timestamp_millis()),
            name: project_name,
            project_path: norm_path.clone(),
            provider_id: provider_id.clone(),
            provider_name: provider.name.clone(),
            created_at: now.clone(),
            updated_at: now,
            config_dir: Some(config_dir.clone()),
            order: max_order + 1,
            provider_dir,
        };
        cfg.active_projects.push(new_proj.clone());
        new_proj
    };

    config::save_app_config(&cfg).map_err(|e| e.to_string())?;

    Ok(InjectToProjectResult { project, was_existing, config_dir })
}

// ── 获取已激活项目列表 ────────────────────────────────────────────────
#[tauri::command]
fn get_active_projects() -> Result<Vec<ActiveProject>, String> {
    let cfg = config::load_app_config().map_err(|e| e.to_string())?;
    let mut projects = cfg.active_projects;
    projects.sort_by(|a, b| a.order.cmp(&b.order));
    Ok(projects)
}

// ── 移除已激活项目（从列表删除 + 清理 CLAUDE.md 标记块） ───────────────
#[tauri::command]
fn remove_active_project(id: String) -> Result<(), String> {
    let mut cfg = config::load_app_config().map_err(|e| e.to_string())?;
    // 找到要移除的项目，获取 project_path 用于清理 CLAUDE.md
    let project_path = cfg.active_projects.iter()
        .find(|p| p.id == id)
        .map(|p| p.project_path.clone());

    cfg.active_projects.retain(|p| p.id != id);
    config::save_app_config(&cfg).map_err(|e| e.to_string())?;

    // 清理项目中的 CLAUDE.md 标记块 + settings.local.json（静默失败，不阻塞主流程）
    if let Some(path) = project_path {
        if let Err(e) = crate::inject::clean_project_settings(&path) {
            eprintln!("[MMYCS] settings 清理警告（项目 {}）: {}", id, e);
        }
        if let Err(e) = crate::inject::remove_claude_md_block(&path) {
            eprintln!("[MMYCS] CLAUDE.md 清理警告（项目 {}）: {}", id, e);
        }
    }

    Ok(())
}

// ── 独立清理项目的 CLAUDE.md 标记块（供前端手动调用） ──────────────────
#[tauri::command]
fn clean_claude_md_block(project_path: String) -> Result<bool, String> {
    crate::inject::remove_claude_md_block(&project_path).map_err(|e| e.to_string())
}
#[tauri::command]
fn get_project_config_dir(project_path: String) -> Result<String, String> {
    let norm_path = config::normalize_project_path(&project_path);
    // 从 active_projects 中查找 provider 信息
    let cfg = config::load_app_config().map_err(|e| e.to_string())?;
    if let Some(proj) = cfg.active_projects.iter().find(|p| config::normalize_project_path(&p.project_path) == norm_path) {
        let dir = config::get_project_config_dir_for_project(proj);
        Ok(dir.to_string_lossy().to_string())
    } else {
        // 回退：使用 provider_name 生成路径
        let providers = config::load_providers().map_err(|e| e.to_string())?;
        let provider_name = providers.first().map(|p| p.name.clone()).unwrap_or_else(|| "unknown".to_string());
        let dir = config::get_project_config_dir(&norm_path, &provider_name);
        Ok(dir.to_string_lossy().to_string())
    }
}

// ── 获取项目的会话归档列表 ────────────────────────────────────────────────
#[tauri::command]
fn get_project_sessions(project_path: String) -> Result<Vec<SessionArchive>, String> {
    let norm_path = config::normalize_project_path(&project_path);
    // 从 active_projects 中查找 provider 信息
    let cfg = config::load_app_config().map_err(|e| e.to_string())?;
    if let Some(proj) = cfg.active_projects.iter().find(|p| config::normalize_project_path(&p.project_path) == norm_path) {
        config::get_project_sessions(&norm_path, &proj.provider_name).map_err(|e| e.to_string())
    } else {
        // 回退：遍历所有供应商查找
        Ok(vec![])
    }
}

// ── 图标上传 ──────────────────────────────────────────────────────────────────
#[tauri::command]
fn save_provider_icon(provider_id: String, data_base64: String, ext: String) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bytes = STANDARD.decode(&data_base64).map_err(|e| e.to_string())?;
    let icons_dir = config::mmycs_dir().join("icons");
    std::fs::create_dir_all(&icons_dir).map_err(|e| e.to_string())?;
    let filename = format!("{}.{}", provider_id, ext);
    let path = icons_dir.join(&filename);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// ── 导出备份（新格式：二进制文件，默认无密码，可选密码保护）──────────────────────
/// 文件格式：
/// [5 bytes]  Magic: "MMYCS"
/// [1 byte]   Version: 0x02
/// [32 bytes] Machine key hash (SHA-256 of machine_key)
/// [1 byte]   Flag: 0x00 = 无密码, 0x01 = 有密码
/// [若有密码: 32 bytes password verify hash]
/// [剩余]     AES-256-GCM 加密的 providers JSON

const BACKUP_MAGIC: &[u8] = b"MMYCS";
const BACKUP_VERSION: u8 = 0x02;
const FLAG_NO_PASSWORD: u8 = 0x00;
const FLAG_HAS_PASSWORD: u8 = 0x01;

/// 计算机器密钥的 SHA-256 hash（用于识别同机导入）
fn machine_key_hash(machine_key: &str) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(machine_key.as_bytes());
    hasher.finalize().into()
}

/// 从密码派生密钥（用于跨机器加密）
fn derive_key_from_password(password: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Sha256, Digest};
    // 简单的 PBKDF 替代：多次 SHA-256
    let mut hash = password.as_bytes().to_vec();
    for _ in 0..10000 {
        let mut hasher = Sha256::new();
        hasher.update(&hash);
        hash = hasher.finalize().to_vec();
    }
    STANDARD.encode(&hash[..32])
}

#[tauri::command]
fn export_backup(password: String) -> Result<Vec<u8>, String> {
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;

    // 构建导出数据
    let mut exported: Vec<serde_json::Value> = vec![];
    for p in &providers {
        let v = serde_json::to_value(p).map_err(|e| e.to_string())?;
        // API key 已经用 machine_key 加密，直接保留
        exported.push(v);
    }
    let providers_json = serde_json::to_string(&exported).map_err(|e| e.to_string())?;

    // 根据是否有密码选择加密方式
    let (flag, encrypted_data, pwd_for_hash) = if password.is_empty() {
        // 无密码：用机器密钥加密（同机可直接导入）
        (FLAG_NO_PASSWORD, crypto::encrypt(&providers_json, &machine_key).map_err(|e| e.to_string())?, None)
    } else {
        // 有密码：用密码派生密钥加密
        let backup_key = derive_key_from_password(&password);
        (FLAG_HAS_PASSWORD, crypto::encrypt(&providers_json, &backup_key).map_err(|e| e.to_string())?, Some(password))
    };

    // 构建二进制文件
    let mut output: Vec<u8> = vec![];

    // 1. Magic
    output.extend_from_slice(BACKUP_MAGIC);

    // 2. Version
    output.push(BACKUP_VERSION);

    // 3. Machine key hash
    output.extend_from_slice(&machine_key_hash(&machine_key));

    // 4. Flag
    output.push(flag);

    // 5. 若有密码，写入密码验证 hash
    if flag == FLAG_HAS_PASSWORD {
        let pwd_hash = derive_key_from_password(&pwd_for_hash.unwrap());
        output.extend_from_slice(&machine_key_hash(&pwd_hash));
    }

    // 6. 加密数据（base64 编码）
    output.extend_from_slice(encrypted_data.as_bytes());

    Ok(output)
}

/// 导出结果（包含文件路径）
#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
    pub filename: String,
}

/// 快速导出 - 导出到指定路径或默认 backups 目录，文件名带日期后缀
#[tauri::command]
fn export_backup_quick(password: String, custom_path: Option<String>) -> Result<ExportResult, String> {
    let data = export_backup(password)?;

    // 确定导出目录
    let backups_dir = if let Some(path) = custom_path {
        if path.is_empty() {
            config::mmycs_dir().join("backups")
        } else {
            std::path::PathBuf::from(&path)
        }
    } else {
        config::mmycs_dir().join("backups")
    };

    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    // 生成带日期后缀的文件名
    let now = chrono::Local::now();
    let filename = format!("mmycs_backup_{}.mmycs", now.format("%Y%m%d_%H%M%S"));
    let filepath = backups_dir.join(&filename);

    // 写入文件
    std::fs::write(&filepath, &data).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path: filepath.to_string_lossy().to_string(),
        filename,
    })
}

/// 导入结果
#[derive(Serialize)]
pub struct ImportResult {
    pub count: usize,
    pub same_machine: bool,
    pub need_password: bool,
}

/// 检测备份文件信息（导入前预检查）
#[derive(Serialize)]
pub struct BackupInfo {
    pub version: u8,
    pub same_machine: bool,
    pub has_password: bool,
    pub is_full_backup: bool,  // v3 及以上版本为完整备份（可能含插件文件）
}

#[tauri::command]
fn check_backup_file(data: Vec<u8>) -> Result<BackupInfo, String> {
    if data.len() < 39 {
        return Err("文件格式无效".to_string());
    }

    // 检查 Magic
    if &data[0..5] != BACKUP_MAGIC {
        return Err("不是 MMYCS 备份文件".to_string());
    }

    // 检查版本
    let version = data[5];
    if version != BACKUP_VERSION {
        return Err(format!("不支持的版本: {}", version));
    }

    // 读取机器密钥 hash
    let stored_hash: [u8; 32] = data[6..38].try_into().map_err(|_| "hash 读取失败")?;

    // 检查是否同机
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let current_hash = machine_key_hash(&machine_key);
    let same_machine = stored_hash == current_hash;

    // 检查是否有密码
    let flag = data[38];
    let has_password = flag == FLAG_HAS_PASSWORD;

    Ok(BackupInfo {
        version,
        same_machine,
        has_password,
        is_full_backup: version >= BACKUP_VERSION_V3,
    })
}

#[tauri::command]
fn import_backup(data: Vec<u8>, password: String) -> Result<ImportResult, String> {
    if data.len() < 39 {
        return Err("文件格式无效".to_string());
    }

    // 检查 Magic
    if &data[0..5] != BACKUP_MAGIC {
        return Err("不是 MMYCS 备份文件".to_string());
    }

    // 检查版本
    let version = data[5];
    if version != BACKUP_VERSION {
        return Err(format!("不支持的版本: {}", version));
    }

    // 读取机器密钥 hash
    let stored_hash: [u8; 32] = data[6..38].try_into().map_err(|_| "hash 读取失败")?;

    // 检查是否同机
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let current_hash = machine_key_hash(&machine_key);
    let same_machine = stored_hash == current_hash;

    // 读取 flag
    let flag = data[38];
    let has_password = flag == FLAG_HAS_PASSWORD;

    // 计算加密数据起始位置
    let data_start = if has_password { 39 + 32 } else { 39 };

    if data.len() < data_start {
        return Err("文件数据不完整".to_string());
    }

    // 提取加密数据
    let encrypted_data = String::from_utf8_lossy(&data[data_start..]).to_string();

    // 解密
    let providers_json = if has_password {
        // 有密码：需要用户提供密码
        if password.is_empty() {
            return Err("需要输入备份密码".to_string());
        }
        let backup_key = derive_key_from_password(&password);
        crypto::decrypt(&encrypted_data, &backup_key)
            .map_err(|_| "密码错误或文件损坏".to_string())?
    } else {
        // 无密码：根据是否同机选择解密方式
        if same_machine {
            // 同机：用机器密钥解密
            crypto::decrypt(&encrypted_data, &machine_key)
                .map_err(|e| format!("解密失败: {}", e))?
        } else {
            // 跨机器但无密码：无法导入
            return Err("此备份文件未设置密码保护，仅能在原机器上导入。\n如需跨机器迁移，请在导出时设置密码。".to_string());
        }
    };

    // 解析并导入 providers
    let providers: Vec<Provider> = serde_json::from_str(&providers_json)
        .map_err(|e| format!("解析失败: {}", e))?;

    let mut count = 0usize;
    for p in providers {
        // 保存 provider（API key 已加密，直接保存）
        config::save_provider(&p).map_err(|e| e.to_string())?;
        count += 1;
    }

    Ok(ImportResult {
        count,
        same_machine,
        need_password: has_password || (!same_machine && !has_password),
    })
}

// ── 旧版兼容：保留原有 JSON 格式导入（向后兼容）──────────────────────────────
#[tauri::command]
fn import_providers_legacy(json: String, password: String) -> Result<usize, String> {
    let bundle: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let backup_key = derive_key_from_password(&password);
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let providers = bundle["providers"].as_array().ok_or("invalid bundle")?;
    let mut count = 0usize;
    for v in providers {
        let mut p: Provider = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
        if let Some(enc) = p.api_key_encrypted.as_deref() {
            let plain = crypto::decrypt(enc, &backup_key).map_err(|_| "密码错误或文件损坏")?;
            p.api_key_encrypted = Some(crypto::encrypt(&plain, &machine_key).map_err(|e| e.to_string())?);
        }
        config::save_provider(&p).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

// ── 连通性测试（智能适配不同协议） ────────────────────────────────────────
// ── 获取模型列表（通过后端代理，避免浏览器 CORS 限制） ────────────────
#[derive(Serialize)]
pub struct FetchModelsResult {
    pub models: Vec<String>,
}

#[tauri::command]
async fn fetch_models(base_url: String, api_key: String) -> Result<FetchModelsResult, String> {
    let url_lower = base_url.to_lowercase();
    let models_url = if url_lower.contains("/apps/anthropic") || url_lower.contains("anthropic.com") {
        // Anthropic 兼容端点
        format!("{}/v1/models", base_url.trim_end_matches('/'))
    } else {
        // OpenAI 兼容端点
        let trimmed = base_url.trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            format!("{}/models", trimmed)
        } else {
            format!("{}/v1/models", trimmed)
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(&models_url);

    // 判断认证方式
    if url_lower.contains("dashscope") || url_lower.contains("minimax") || url_lower.contains("moonshot") || url_lower.contains("volces") || url_lower.contains("hunyuan") {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    } else if url_lower.contains("anthropic") {
        req = req.header("x-api-key", &api_key)
               .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req.send().await.map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, &err_text[..err_text.len().min(200)]));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    let ids: Vec<String> = if let Some(arr) = data.get("data").and_then(|d| d.as_array()) {
        arr.iter().filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from)).collect()
    } else if let Some(arr) = data.get("models").and_then(|d| d.as_array()) {
        arr.iter().filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from)).collect()
    } else if let Some(arr) = data.as_array() {
        arr.iter().filter_map(|m| m.get("id").or_else(|| m.get("name")).and_then(|v| v.as_str()).map(String::from)).collect()
    } else {
        return Err("未识别的响应格式".to_string());
    };

    Ok(FetchModelsResult { models: ids })
}

#[tauri::command]
async fn test_provider(provider_id: String) -> Result<bool, String> {
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let p = providers.iter().find(|p| p.id == provider_id).ok_or("not found")?;
    if p.provider_type != "api" { return Ok(true); }
    let base_url = p.base_url.as_deref().unwrap_or("https://api.anthropic.com").trim_end_matches('/');
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let api_key = match &p.api_key_encrypted {
        Some(enc) => crypto::decrypt(enc, &machine_key).map_err(|e| e.to_string())?,
        None => return Err("no api key".to_string()),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| e.to_string())?;

    // 根据端点特征选择测试策略
    let url_lower = base_url.to_lowercase();
    let (test_url, use_bearer) = if url_lower.contains("/apps/anthropic") || url_lower.ends_with("anthropic.com") || !url_lower.contains("dashscope") {
        // Anthropic 兼容：尝试 GET /v1/models（标准 Anthropic 端点）
        (format!("{}/v1/models", base_url), false)
    } else if url_lower.contains("/v1") && !url_lower.ends_with("/v1") {
        // 已包含版本路径（如 ...com/v1）：直接用 /models
        (format!("{}/models", base_url), true)
    } else {
        // OpenAI 兼容（默认）：追加 /v1/models，使用 Bearer 认证
        (format!("{}/v1/models", base_url), true)
    };

    let mut req = client.get(&test_url);
    if use_bearer {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    } else {
        req = req.header("x-api-key", &api_key)
               .header("anthropic-version", "2023-06-01");
    }

    match req.send().await {
        Ok(resp) => Ok(resp.status().is_success() || resp.status().as_u16() == 401),
        Err(e) => Err(format!("请求失败: {}", e)),
    }
}

#[tauri::command]
fn save_window_state(x: i32, y: i32, width: u32, height: u32, is_dark: Option<bool>, compact_mode: Option<bool>) -> Result<(), String> {
    let path = config::mmycs_dir().join("window_state.json");
    let mut state = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if let Some(d) = is_dark { state["isDark"] = serde_json::json!(d); }
    if let Some(c) = compact_mode { state["compactMode"] = serde_json::json!(c); }
    state["x"] = serde_json::json!(x);
    state["y"] = serde_json::json!(y);
    state["width"] = serde_json::json!(width);
    state["height"] = serde_json::json!(height);
    std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_window_state() -> Option<serde_json::Value> {
    let path = config::mmycs_dir().join("window_state.json");
    if !path.exists() { return None; }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
}

#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── CLAUDE.md 模板管理 ───────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    /// 是否为内置模板
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Serialize, Deserialize)]
pub struct TemplateBinding {
    pub project_path: String,
    pub template_name: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct TemplateBindingsFile {
    pub bindings: Vec<TemplateBinding>,
}

fn templates_dir() -> std::path::PathBuf {
    config::mmycs_dir().join("templates")
}

fn template_bindings_path() -> std::path::PathBuf {
    config::mmycs_dir().join("template_bindings.json")
}

/// 内置规则模板（硬编码，随应用更新）
fn get_builtin_rule_templates() -> Vec<Template> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Template {
            name: "通用开发规范".to_string(),
            content: "# 通用开发规范\n\n## 编码原则\n- **最小化修改**：每次改动只解决当前问题，不顺手重构无关代码\n- **只读现有代码**：修改前先完整理解上下文，不引入新依赖\n- **渐进式修改**：大功能分步实现，每步都可独立验证\n- **保持向后兼容**：除非明确要求，不破坏已有 API 或接口\n\n## 质量要求\n- 修改后运行 lint/build/test，确保通过再提交\n- 新增代码必须包含必要的注释，解释 \"为什么\" 而非 \"做什么\"\n- 错误处理要具体，不使用空的 catch / unwrap\n- 日志要包含上下文信息，方便排查问题\n\n## 工作流程\n- 先给出方案再写代码（对于复杂变更）\n- 代码审查时说明修改意图和影响范围\n- 不确定时先问，不要猜测".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            builtin: true,
        },
        Template {
            name: "前端项目规范".to_string(),
            content: "# 前端项目规范\n\n## 技术栈\n- Vue 3 + TypeScript + Vite\n- Naive UI 组件库\n- Pinia 状态管理\n- Vue Router 路由管理\n\n## 编码规范\n- 组件文件使用 PascalCase 命名（`ProviderGrid.vue`）\n- 组合式 API 优先（`<script setup>`）\n- Props/Emits 使用 TypeScript 类型定义\n- 所有组件必须有 scoped 样式，避免全局污染\n\n## 样式规范\n- 使用 scoped 样式 + CSS 变量\n- 深色模式使用 `body.dark .class` 选择器\n- 全局滚动条样式放在 `App.vue` 的非 scoped 块中\n\n## API 请求\n- 使用统一的 request 封装，处理 token 刷新和错误\n- API 错误统一展示 toast 提示\n- 接口类型定义放在 `types/` 目录下\n\n## 国际化\n- 所有用户可见文本必须通过 `useI18n` 获取\n- i18n key 使用 snake_case 命名".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            builtin: true,
        },
        Template {
            name: "后端/API 规范".to_string(),
            content: "# 后端/API 规范\n\n## API 设计\n- 使用 RESTful 风格：`GET /resources`, `POST /resources`, `PUT /resources/{id}`\n- 所有接口必须返回统一的 JSON 格式：`{ success: bool, data?: any, error?: string }`\n- 使用有意义的 HTTP 状态码（200, 201, 400, 404, 500）\n- 分页参数：`page`, `pageSize`；返回：`{ items, total, page, pageSize }`\n\n## 错误处理\n- 使用 Result/Option 模式，不使用 unwrap\n- 业务错误返回明确的错误码和消息\n- 敏感信息（密码、token）绝不记录到日志\n- 所有外部输入必须验证和消毒\n\n## 数据库\n- 使用迁移管理 schema 变更，不可手动修改数据库\n- 查询使用参数化语句，防止 SQL 注入\n- 索引设计考虑查询模式，避免全表扫描\n- 软删除优先于物理删除（`deleted_at` 字段）\n\n## 安全\n- API 必须使用认证（JWT/Bearer Token）\n- 敏感操作需要额外授权检查\n- 定期轮换密钥和 token".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            builtin: true,
        },
        Template {
            name: "Git 提交规范".to_string(),
            content: "# Git 提交规范\n\n## Conventional Commits 格式\n\n```\n<type>(<scope>): <description>\n\n[optional body]\n\n[optional footer(s)]\n```\n\n## Type 类型\n- `feat`: 新功能\n- `fix`: Bug 修复\n- `docs`: 文档更新\n- `style`: 代码格式（不影响功能）\n- `refactor`: 重构（不新增功能、不修复 Bug）\n- `test`: 测试相关\n- `chore`: 构建/工具链相关\n- `perf`: 性能优化\n\n## 示例\n- `feat(auth): 添加 OAuth2 登录支持`\n- `fix(api): 修复分页参数越界问题`\n- `refactor(store): 重构 Pinia store 模块拆分`\n\n## 提交前检查\n- 运行 lint（`npm run lint` 或 `cargo clippy`）\n- 运行测试（`npm test` 或 `cargo test`）\n- 确保不提交敏感文件（`.env`, `credentials.json`）\n- 提交信息用中文书写（团队约定）\n- 单次提交只包含一个逻辑变更".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            builtin: true,
        },
    ]
}

// 内置模板名称列表（用于删除保护）
const BUILTIN_TEMPLATE_NAMES: &[&str] = &["通用开发规范", "前端项目规范", "后端/API 规范", "Git 提交规范"];

#[tauri::command]
fn get_templates() -> Result<Vec<Template>, String> {
    // 1. 内置模板在前
    let mut all = get_builtin_rule_templates();
    // 2. 用户自定义模板
    let dir = templates_dir();
    if dir.exists() {
        for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                let name = entry.path().file_stem().and_then(|n| n.to_str()).unwrap_or("").to_string();
                let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                // 跳过已内置的同名模板（用户已采用）
                if !BUILTIN_TEMPLATE_NAMES.contains(&name.as_str()) {
                    all.push(Template {
                        name,
                        content,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        builtin: false,
                    });
                }
            }
        }
    }
    // builtin 在前，同类型按名称排序
    all.sort_by(|a, b| {
        a.builtin.cmp(&b.builtin).reverse().then_with(|| a.name.cmp(&b.name))
    });
    Ok(all)
}

#[tauri::command]
fn save_template(name: String, content: String) -> Result<(), String> {
    let dir = templates_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.md", name));
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 采用内置模板：将内置模板内容复制到用户模板目录
#[tauri::command]
fn adopt_template(name: String) -> Result<(), String> {
    let builtin = get_builtin_rule_templates();
    let tpl = builtin.iter().find(|t| t.name == name).ok_or("内置模板不存在")?;
    let dir = templates_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.md", tpl.name));
    std::fs::write(&path, &tpl.content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_template(name: String) -> Result<(), String> {
    // 禁止删除内置模板
    if BUILTIN_TEMPLATE_NAMES.contains(&name.as_str()) {
        return Err("内置模板不可删除".to_string());
    }
    let path = templates_dir().join(format!("{}.md", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    // 同时删除相关绑定
    let bindings_path = template_bindings_path();
    if bindings_path.exists() {
        let mut bindings: TemplateBindingsFile = std::fs::read_to_string(&bindings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(TemplateBindingsFile { bindings: vec![] });
        bindings.bindings.retain(|b| b.template_name != name);
        std::fs::write(&bindings_path, serde_json::to_string_pretty(&bindings).unwrap_or_default())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_template_bindings() -> Result<Vec<TemplateBinding>, String> {
    let path = template_bindings_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let bindings: TemplateBindingsFile = serde_json::from_str(&content)
        .map_err(|_| "解析失败".to_string())?;
    Ok(bindings.bindings)
}

#[tauri::command]
fn bind_template(project_path: String, template_name: String) -> Result<(), String> {
    let path = template_bindings_path();
    let mut bindings: TemplateBindingsFile = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(TemplateBindingsFile { bindings: vec![] })
    } else {
        TemplateBindingsFile { bindings: vec![] }
    };
    // 更新或添加绑定
    let norm_path = config::normalize_project_path(&project_path);
    let now = chrono::Utc::now().to_rfc3339();
    let existing = bindings.bindings.iter_mut().find(|b| b.project_path == norm_path);
    if let Some(b) = existing {
        b.template_name = template_name;
        b.updated_at = now;
    } else {
        bindings.bindings.push(TemplateBinding {
            project_path: norm_path,
            template_name,
            updated_at: now,
        });
    }
    std::fs::write(&path, serde_json::to_string_pretty(&bindings).unwrap_or_default())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn unbind_template(project_path: String) -> Result<(), String> {
    let path = template_bindings_path();
    if !path.exists() { return Ok(()); }
    let mut bindings: TemplateBindingsFile = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(TemplateBindingsFile { bindings: vec![] });
    let norm_path = config::normalize_project_path(&project_path);
    bindings.bindings.retain(|b| b.project_path != norm_path);
    std::fs::write(&path, serde_json::to_string_pretty(&bindings).unwrap_or_default())
        .map_err(|e| e.to_string())?;

    // 清理模板注入的 .claude/CLAUDE.md
    let claude_md = std::path::Path::new(&norm_path).join(".claude").join("CLAUDE.md");
    if claude_md.exists() {
        let _ = std::fs::remove_file(&claude_md);
    }

    Ok(())
}

#[tauri::command]
fn inject_claude_md(project_path: String, template_name: String) -> Result<String, String> {
    // 读取模板内容
    let template_path = templates_dir().join(format!("{}.md", template_name));
    if !template_path.exists() {
        return Err("模板不存在".to_string());
    }
    let content = std::fs::read_to_string(&template_path).map_err(|e| e.to_string())?;

    // 写入项目的 .claude/CLAUDE.md
    let norm_path = config::normalize_project_path(&project_path);
    let claude_dir = std::path::Path::new(&norm_path).join(".claude");
    std::fs::create_dir_all(&claude_dir).map_err(|e| e.to_string())?;
    let claude_md_path = claude_dir.join("CLAUDE.md");
    std::fs::write(&claude_md_path, &content).map_err(|e| e.to_string())?;

    // 同时更新绑定
    bind_template(project_path, template_name)?;

    Ok(claude_md_path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_project_template(project_path: String) -> Result<Option<String>, String> {
    let bindings = get_template_bindings()?;
    let norm_path = config::normalize_project_path(&project_path);
    let binding = bindings.iter().find(|b| b.project_path == norm_path);
    Ok(binding.map(|b| b.template_name.clone()))
}

// ── 插件管理 ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub marketplace: String,
    pub enabled: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MarketplaceSource {
    pub id: String,
    #[serde(rename = "type")]
    pub source_type: String,  // "github" | "npm" | "local"
    pub repo: Option<String>,
    pub url: Option<String>,
}

// ── Token 使用统计类型 ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UsageStats {
    pub sessions: usize,
    pub messages: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub active_days: usize,
    pub current_streak: usize,
    pub longest_streak: usize,
    pub peak_hour: usize,
    pub favorite_model: String,
    pub daily_data: Vec<DailyUsage>,
    pub model_data: Vec<ModelUsage>,
}

#[derive(Serialize)]
pub struct DailyUsage {
    pub date: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub messages: usize,
}

#[derive(Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub percentage: f64,
}

/// 获取 Claude Code 配置目录（默认 ~/.claude）
fn claude_config_dir() -> std::path::PathBuf {
    let cfg = config::load_app_config().ok();
    if let Some(c) = cfg {
        if let Some(inst) = c.instances.first() {
            return std::path::PathBuf::from(&inst.config_dir);
        }
    }
    dirs::home_dir().unwrap_or_default().join(".claude")
}

/// 读取 Claude Code settings.json
fn read_claude_settings() -> Result<serde_json::Value, String> {
    let path = claude_config_dir().join("settings.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// 保存 Claude Code settings.json
fn save_claude_settings(settings: &serde_json::Value) -> Result<(), String> {
    let path = claude_config_dir().join("settings.json");
    std::fs::write(&path, serde_json::to_string_pretty(settings).unwrap_or_default())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(s)
}

/// 递归收集目录下所有文件（排除 node_modules 和 .git）
fn collect_dir_files(base: &std::path::PathBuf, current: &std::path::PathBuf, files: &mut serde_json::Map<String, serde_json::Value>) -> std::io::Result<()> {
    let entries = std::fs::read_dir(current)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // 排除 node_modules 和 .git
        if relative.contains("node_modules") || relative.contains(".git") { continue; }
        if path.is_dir() {
            collect_dir_files(base, &path, files)?;
        } else {
            let content = std::fs::read(&path)?;
            files.insert(relative, serde_json::Value::String(base64_encode(&content)));
        }
    }
    Ok(())
}

/// 收集 plugins/cache 目录下所有插件的文件内容（Base64 编码）
/// 返回 { "marketplace/plugin": { "path/to/file": "<base64>" } }
fn collect_plugin_files(plugins_cache_dir: &std::path::PathBuf) -> Result<serde_json::Value, String> {
    if !plugins_cache_dir.exists() {
        return Ok(serde_json::json!({}));
    }
    let mut archive = serde_json::Map::new();
    for marketplace_entry in std::fs::read_dir(plugins_cache_dir).map_err(|e| e.to_string())? {
        let marketplace_dir = marketplace_entry.map_err(|e| e.to_string())?.path();
        if !marketplace_dir.is_dir() { continue; }
        let marketplace_name = marketplace_dir.file_name().unwrap().to_string_lossy().to_string();
        for plugin_entry in std::fs::read_dir(&marketplace_dir).map_err(|e| e.to_string())? {
            let plugin_dir = plugin_entry.map_err(|e| e.to_string())?.path();
            if !plugin_dir.is_dir() { continue; }
            let plugin_name = plugin_dir.file_name().unwrap().to_string_lossy().to_string();
            let plugin_key = format!("{}/{}", marketplace_name, plugin_name);
            let mut files = serde_json::Map::new();
            collect_dir_files(&plugin_dir, &plugin_dir, &mut files).map_err(|e| e.to_string())?;
            archive.insert(plugin_key, serde_json::Value::Object(files));
        }
    }
    Ok(serde_json::Value::Object(archive))
}

#[tauri::command]
fn get_plugins() -> Result<Vec<PluginInfo>, String> {
    let settings = read_claude_settings()?;
    let plugins_dir = claude_config_dir().join("plugins").join("cache");

    // 解析 enabledPlugins
    let enabled_plugins: HashMap<String, bool> = settings
        .get("enabledPlugins")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_bool().unwrap_or(false))).collect())
        .unwrap_or_default();

    // 扫描 plugins/cache 目录获取已下载插件
    let mut plugins: Vec<PluginInfo> = vec![];

    if plugins_dir.exists() {
        // 遍历 marketplace 目录（如 claude-hud, claude-plugins-official）
        for marketplace_entry in std::fs::read_dir(&plugins_dir).map_err(|e| e.to_string())? {
            let marketplace_entry = marketplace_entry.map_err(|e| e.to_string())?;
            let marketplace_name = marketplace_entry.file_name().to_string_lossy().to_string();

            // 遍历该 marketplace 下的插件目录
            for plugin_entry in std::fs::read_dir(marketplace_entry.path()).map_err(|e| e.to_string())? {
                let plugin_entry = plugin_entry.map_err(|e| e.to_string())?;
                let plugin_dir_name = plugin_entry.file_name().to_string_lossy().to_string();

                // 插件 ID 格式：plugin_name@marketplace
                let plugin_id = format!("{}@{}", plugin_dir_name, marketplace_name);

                // 尝试读取 package.json 或 manifest 获取版本
                let version = plugin_entry.path().join("package.json")
                    .exists()
                    .then(|| {
                        std::fs::read_to_string(plugin_entry.path().join("package.json"))
                            .ok()
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                            .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from))
                    })
                    .flatten();

                plugins.push(PluginInfo {
                    id: plugin_id.clone(),
                    name: plugin_dir_name.clone(),
                    marketplace: marketplace_name.clone(),
                    enabled: enabled_plugins.get(&plugin_id).copied().unwrap_or(false),
                    version,
                    path: Some(plugin_entry.path().to_string_lossy().to_string()),
                });
            }
        }
    }

    // 添加 enabled 但未下载的插件（只存在于 settings.json）
    for (id, enabled) in &enabled_plugins {
        if !plugins.iter().any(|p| &p.id == id) {
            // 解析 ID：plugin_name@marketplace
            let parts: Vec<&str> = id.split('@').collect();
            let (name, marketplace) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (id.clone(), "unknown".to_string())
            };
            plugins.push(PluginInfo {
                id: id.clone(),
                name,
                marketplace,
                enabled: *enabled,
                version: None,
                path: None,
            });
        }
    }

    // 按 marketplace + name 排序
    plugins.sort_by(|a, b| a.marketplace.cmp(&b.marketplace).then_with(|| a.name.cmp(&b.name)));

    Ok(plugins)
}

#[tauri::command]
fn toggle_plugin(id: String, enabled: bool) -> Result<(), String> {
    let mut settings = read_claude_settings()?;

    // 确保 enabledPlugins 存在
    if settings.get("enabledPlugins").is_none() {
        settings["enabledPlugins"] = serde_json::json!({});
    }

    // 更新状态
    settings["enabledPlugins"][&id] = serde_json::json!(enabled);

    save_claude_settings(&settings)?;
    Ok(())
}

#[tauri::command]
fn get_marketplaces() -> Result<Vec<MarketplaceSource>, String> {
    let settings = read_claude_settings()?;

    let marketplaces: Vec<MarketplaceSource> = settings
        .get("extraKnownMarketplaces")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter().map(|(id, val)| {
                let source = val.get("source");
                MarketplaceSource {
                    id: id.clone(),
                    source_type: source
                        .and_then(|s| s.get("source").and_then(|t| t.as_str()))
                        .unwrap_or("unknown")
                        .to_string(),
                    repo: source
                        .and_then(|s| s.get("repo").and_then(|r| r.as_str()))
                        .map(String::from),
                    url: None,
                }
            }).collect()
        })
        .unwrap_or_default();

    Ok(marketplaces)
}

#[tauri::command]
fn add_marketplace(id: String, repo: String) -> Result<(), String> {
    let mut settings = read_claude_settings()?;

    // 确保 extraKnownMarketplaces 存在
    if settings.get("extraKnownMarketplaces").is_none() {
        settings["extraKnownMarketplaces"] = serde_json::json!({});
    }

    settings["extraKnownMarketplaces"][&id] = serde_json::json!({
        "source": {
            "source": "github",
            "repo": repo
        }
    });

    save_claude_settings(&settings)?;
    Ok(())
}

#[tauri::command]
fn remove_marketplace(id: String) -> Result<(), String> {
    let mut settings = read_claude_settings()?;

    if let Some(obj) = settings.get("extraKnownMarketplaces").and_then(|v| v.as_object()) {
        let mut marketplaces = obj.clone();
        marketplaces.remove(&id);
        settings["extraKnownMarketplaces"] = serde_json::Value::Object(marketplaces);
    }

    save_claude_settings(&settings)?;
    Ok(())
}

// ── Token 使用统计 ───────────────────────────────────────────────────────────────

/// 解析 JSONL 中的一行，如果是 assistant 消息且包含 usage 信息则提取
fn parse_usage_line(line: &str) -> Option<(String, u64, u64, String)> {
    // (model, input_tokens, output_tokens, date_str)
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    // 只统计 assistant 消息（顶层 type）
    if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
        return None;
    }
    // 只统计 CLI 入口
    let entrypoint = v.get("entrypoint").and_then(|e| e.as_str()).unwrap_or("");
    if entrypoint != "cli" {
        return None;
    }
    // model 和 usage 在 message 对象内
    let msg = v.get("message")?;
    let model = msg.get("model")?.as_str()?.to_string();
    let usage = msg.get("usage")?;
    let input_tokens = usage.get("input_tokens")?.as_u64()?;
    let output_tokens = usage.get("output_tokens")?.as_u64()?;
    // 提取日期（从 timestamp）
    let date_str = v.get("timestamp")
        .and_then(|t| t.as_str())
        .map(|t| t.split('T').next().unwrap_or(""))
        .unwrap_or("")
        .to_string();
    Some((model, input_tokens, output_tokens, date_str))
}

/// 计算连续天数
fn calc_streaks(dates: &[String]) -> (usize, usize) {
    if dates.is_empty() {
        return (0, 0);
    }
    let mut sorted: Vec<String> = dates.iter().cloned().collect();
    sorted.sort();
    sorted.dedup();

    // 转换为日期
    let today = chrono::Utc::now().date_naive();
    let date_list: Vec<chrono::NaiveDate> = sorted
        .iter()
        .filter_map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();

    if date_list.is_empty() {
        return (0, 0);
    }

    // 最长连续
    let mut longest = 1;
    let mut current = 1;
    for i in 1..date_list.len() {
        if date_list[i] - date_list[i - 1] == chrono::Duration::days(1) {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 1;
        }
    }

    // 当前连续（从今天往前数）
    let mut streak = 0;
    let mut check_date = today;
    let date_set: std::collections::HashSet<chrono::NaiveDate> = date_list.into_iter().collect();
    while date_set.contains(&check_date) {
        streak += 1;
        check_date -= chrono::Duration::days(1);
    }

    (streak, longest)
}

#[tauri::command]
fn get_usage_stats(period: String) -> Result<UsageStats, String> {
    use std::collections::HashMap;

    let config_dir = claude_config_dir();
    let projects_dir = config_dir.join("projects");
    if !projects_dir.exists() {
        return Ok(UsageStats {
            sessions: 0, messages: 0, input_tokens: 0, output_tokens: 0,
            active_days: 0, current_streak: 0, longest_streak: 0,
            peak_hour: 0, favorite_model: String::new(),
            daily_data: vec![], model_data: vec![],
        });
    }

    // 计算时间范围
    let now = chrono::Utc::now();
    let cutoff = match period.as_str() {
        "7d" => now - chrono::Duration::days(7),
        "30d" => now - chrono::Duration::days(30),
        _ => chrono::DateTime::<chrono::Utc>::MIN_UTC, // "all"
    };

    // 聚合数据结构
    let mut sessions: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut messages: usize = 0;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut hour_counts: HashMap<usize, usize> = HashMap::new();
    let mut model_input: HashMap<String, u64> = HashMap::new();
    let mut model_output: HashMap<String, u64> = HashMap::new();
    let mut daily_input: HashMap<String, u64> = HashMap::new();
    let mut daily_output: HashMap<String, u64> = HashMap::new();
    let mut daily_messages: HashMap<String, usize> = HashMap::new();
    let mut all_dates: Vec<String> = Vec::new();

    // 遍历所有项目目录
    for project_entry in std::fs::read_dir(&projects_dir).map_err(|e| e.to_string())? {
        let project_dir = project_entry.map_err(|e| e.to_string())?.path();
        if !project_dir.is_dir() { continue; }

        // 遍历项目下的 JSONL 文件（包括子目录中的）
        for jsonl_entry in walk_jsonl_files(&project_dir) {
            let content = std::fs::read_to_string(&jsonl_entry).map_err(|e| e.to_string())?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }

                // 提取 sessionId（用于去重）
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(sid) = v.get("sessionId").and_then(|s| s.as_str()) {
                        sessions.insert(sid.to_string());
                    }
                    // 提取 timestamp 用于时间过滤
                    if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                            if dt < cutoff { continue; }
                        }
                    }
                }

                // 解析 usage
                if let Some((model, input, output, date)) = parse_usage_line(line) {
                    messages += 1;
                    total_input += input;
                    total_output += output;

                    // 提取小时
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                                let hour = dt.hour() as usize;
                                *hour_counts.entry(hour).or_insert(0) += 1;
                            }
                        }
                    }

                    // 按日期聚合
                    if !date.is_empty() {
                        *daily_input.entry(date.clone()).or_insert(0) += input;
                        *daily_output.entry(date.clone()).or_insert(0) += output;
                        *daily_messages.entry(date.clone()).or_insert(0) += 1;
                        all_dates.push(date.clone());
                    }

                    // 按模型聚合
                    *model_input.entry(model.clone()).or_insert(0) += input;
                    *model_output.entry(model.clone()).or_insert(0) += output;
                }
            }
        }
    }

    // 计算活跃天数
    all_dates.sort();
    all_dates.dedup();
    let active_days = all_dates.len();

    // 计算连续天数
    let (current_streak, longest_streak) = calc_streaks(&all_dates);

    // 计算最活跃小时
    let peak_hour = hour_counts.iter()
        .max_by_key(|(_, &count)| count)
        .map(|(&hour, _)| hour)
        .unwrap_or(0);

    // 计算最常用模型（按 token 总量）
    let mut model_totals: HashMap<String, u64> = HashMap::new();
    for (model, &input) in &model_input {
        *model_totals.entry(model.clone()).or_insert(0) += input;
        *model_totals.entry(model.clone()).or_insert(0) += model_output.get(model).copied().unwrap_or(0);
    }
    let favorite_model = model_totals.iter()
        .max_by_key(|(_, &total)| total)
        .map(|(model, _)| model.clone())
        .unwrap_or_default();

    // 构建每日数据
    let mut daily_data: Vec<DailyUsage> = daily_input.iter().map(|(date, &input)| {
        DailyUsage {
            date: date.clone(),
            input_tokens: input,
            output_tokens: daily_output.get(date).copied().unwrap_or(0),
            messages: daily_messages.get(date).copied().unwrap_or(0),
        }
    }).collect();
    daily_data.sort_by(|a, b| a.date.cmp(&b.date));

    // 构建模型数据
    let total_tokens = total_input + total_output;
    let mut model_data: Vec<ModelUsage> = model_input.iter().map(|(model, &input)| {
        let output = model_output.get(model).copied().unwrap_or(0);
        let pct = if total_tokens > 0 {
            ((input + output) as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };
        ModelUsage {
            model: model.clone(),
            input_tokens: input,
            output_tokens: output,
            percentage: (pct * 10.0).round() / 10.0, // 保留一位小数
        }
    }).collect();
    model_data.sort_by(|a, b| b.input_tokens.cmp(&a.input_tokens)); // 按输入量降序

    Ok(UsageStats {
        sessions: sessions.len(),
        messages,
        input_tokens: total_input,
        output_tokens: total_output,
        active_days,
        current_streak,
        longest_streak,
        peak_hour,
        favorite_model,
        daily_data,
        model_data,
    })
}

/// 递归查找目录下所有 JSONL 文件
fn walk_jsonl_files(dir: &std::path::PathBuf) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walk_jsonl_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                results.push(path);
            }
        }
    }
    results
}

// ── Skill 模板管理 ───────────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone)]
pub struct Skill {
    pub name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

fn skills_dir() -> std::path::PathBuf {
    config::mmycs_dir().join("skills")
}

#[tauri::command]
fn get_skills() -> Result<Vec<Skill>, String> {
    let dir = skills_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        return Ok(vec![]);
    }
    let mut skills = vec![];
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
            let name = entry.path().file_stem().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            skills.push(Skill {
                name,
                content,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

#[tauri::command]
fn save_skill(name: String, content: String) -> Result<(), String> {
    let dir = skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.md", name));
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_skill(name: String) -> Result<(), String> {
    let path = skills_dir().join(format!("{}.md", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── 供应商模板管理 ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct ProviderTemplateInput {
    pub id: Option<String>,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub builtin_icon: Option<String>,
    pub icon_fallback: Option<String>,
    pub base_urls: Vec<ProviderTemplateUrlInput>,
    pub models: Vec<String>,
    pub key_placeholder: Option<String>,
    pub help_url: Option<String>,
    pub badge: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProviderTemplateUrlInput {
    pub label: String,
    pub value: String,
    pub hint: Option<String>,
    pub protocol_hint: Option<String>,
}

/// 内置供应商模板（硬编码，随应用更新）
fn get_builtin_provider_templates() -> Vec<ProviderTemplate> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        // 1. 阿里云百炼（聚合平台，支持多供应商模型）
        ProviderTemplate {
            id: "builtin_dashscope".to_string(),
            name: "阿里云百炼".to_string(),
            icon: Some("☁️".to_string()),
            color: Some("#FF6A00".to_string()),
            description: Some("DashScope · 多模型聚合".to_string()),
            builtin_icon: Some("icons/dashscope.svg".to_string()),
            icon_fallback: Some("百炼".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "Anthropic 兼容".to_string(),
                    value: "https://coding.dashscope.aliyuncs.com/apps/anthropic".to_string(),
                    hint: Some("Claude Code".to_string()),
                    protocol_hint: Some("适用于 Claude Code、Cursor 等 Anthropic 协议工具".to_string()),
                },
                config::ProviderTemplateUrl {
                    label: "OpenAI 兼容".to_string(),
                    value: "https://coding.dashscope.aliyuncs.com/v1".to_string(),
                    hint: Some("通用兼容".to_string()),
                    protocol_hint: Some("适用于 OpenAI SDK 兼容的客户端".to_string()),
                },
            ],
            models: vec![
                "qwen3-max-2026-01-23".to_string(),
                "qwen3.6-plus".to_string(),
                "qwen3.5-plus".to_string(),
                "qwen3-coder-next".to_string(),
                "qwen3-coder-plus".to_string(),
                "glm-5".to_string(),
                "glm-4.7".to_string(),
                "kimi-k2.5".to_string(),
                "MiniMax-M2.5".to_string(),
            ],
            key_placeholder: Some("sk-xxxxxxxx".to_string()),
            help_url: Some("https://help.aliyun.com/zh/model-studio/developer-reference/model-qwen".to_string()),
            badge: Some("推荐".to_string()),
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 2. DeepSeek（最新 V4）
        ProviderTemplate {
            id: "builtin_deepseek".to_string(),
            name: "DeepSeek".to_string(),
            icon: Some("🔮".to_string()),
            color: Some("#6366F1".to_string()),
            description: Some("DeepSeek V4 · 推理增强".to_string()),
            builtin_icon: Some("icons/deepseek.svg".to_string()),
            icon_fallback: Some("DS".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "API".to_string(),
                    value: "https://api.deepseek.com".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "deepseek-chat".to_string(),
                "deepseek-coder".to_string(),
                "deepseek-reasoner".to_string(),
            ],
            key_placeholder: Some("sk-xxxxx".to_string()),
            help_url: Some("https://platform.deepseek.com/api-docs/models".to_string()),
            badge: Some("NEW".to_string()),
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 3. 智谱 GLM
        ProviderTemplate {
            id: "builtin_zhipu".to_string(),
            name: "智谱 Zhipu".to_string(),
            icon: Some("🅉".to_string()),
            color: Some("#4D6BFE".to_string()),
            description: Some("GLM 系列 · 国产领先".to_string()),
            builtin_icon: Some("icons/zhipu.svg".to_string()),
            icon_fallback: Some("智谱".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "OpenAI 兼容".to_string(),
                    value: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "glm-4-plus".to_string(),
                "glm-4-0520".to_string(),
                "glm-4-air".to_string(),
                "glm-4-airx".to_string(),
                "glm-4-flash".to_string(),
                "glm-4-long".to_string(),
            ],
            key_placeholder: Some("xxxx.xxxx".to_string()),
            help_url: Some("https://open.bigmodel.cn/dev/api#models".to_string()),
            badge: None,
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 4. Kimi 月之暗面
        ProviderTemplate {
            id: "builtin_kimi".to_string(),
            name: "Kimi (月之暗面)".to_string(),
            icon: Some("🅺".to_string()),
            color: Some("#7C3AED".to_string()),
            description: Some("Kimi · 长文本专家".to_string()),
            builtin_icon: Some("icons/kimi.svg".to_string()),
            icon_fallback: Some("K".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "API".to_string(),
                    value: "https://api.moonshot.cn/v1".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "moonshot-v1-8k".to_string(),
                "moonshot-v1-32k".to_string(),
                "moonshot-v1-128k".to_string(),
            ],
            key_placeholder: Some("sk-xxxxx".to_string()),
            help_url: Some("https://platform.moonshot.cn/docs/models".to_string()),
            badge: None,
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 5. MiniMax
        ProviderTemplate {
            id: "builtin_minimax".to_string(),
            name: "MiniMax".to_string(),
            icon: Some("〰️".to_string()),
            color: Some("#FF4D4F".to_string()),
            description: Some("MiniMax · 海螺AI".to_string()),
            builtin_icon: Some("icons/minimax.svg".to_string()),
            icon_fallback: Some("MM".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "API".to_string(),
                    value: "https://api.minimax.chat/v1".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "abab6.5-chat".to_string(),
                "abab6.5s-chat".to_string(),
                "abab5.5-chat".to_string(),
            ],
            key_placeholder: Some("eyJxxxxxx".to_string()),
            help_url: Some("https://platform.minimaxi.com/document/guides/models".to_string()),
            badge: None,
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 6. 火山引擎 豆包
        ProviderTemplate {
            id: "builtin_huoshan".to_string(),
            name: "火山引擎".to_string(),
            icon: Some("🔥".to_string()),
            color: Some("#F25919".to_string()),
            description: Some("豆包 Doubao · 字节跳动".to_string()),
            builtin_icon: Some("icons/huoshan.svg".to_string()),
            icon_fallback: Some("火".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "API".to_string(),
                    value: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "doubao-pro-32k".to_string(),
                "doubao-pro-128k".to_string(),
                "doubao-lite-32k".to_string(),
                "doubao-lite-128k".to_string(),
            ],
            key_placeholder: Some("xxxxx".to_string()),
            help_url: Some("https://www.volcengine.com/docs/82379/1299475".to_string()),
            badge: None,
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        // 7. 腾讯混元
        ProviderTemplate {
            id: "builtin_tencent".to_string(),
            name: "腾讯云".to_string(),
            icon: Some("☁️".to_string()),
            color: Some("#0066FF".to_string()),
            description: Some("混元 Hunyuan · 腾讯".to_string()),
            builtin_icon: Some("icons/tencent.svg".to_string()),
            icon_fallback: Some("腾".to_string()),
            base_urls: vec![
                config::ProviderTemplateUrl {
                    label: "OpenAI 兼容".to_string(),
                    value: "https://api.hunyuan.cloud.tencent.com/v1".to_string(),
                    hint: None,
                    protocol_hint: None,
                },
            ],
            models: vec![
                "hunyuan-lite".to_string(),
                "hunyuan-standard".to_string(),
                "hunyuan-pro".to_string(),
                "hunyuan-turbo".to_string(),
            ],
            key_placeholder: Some("sk-xxxxx".to_string()),
            help_url: Some("https://cloud.tencent.com/document/product/1729/97732".to_string()),
            badge: None,
            builtin: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
    ]
}

#[tauri::command]
fn get_provider_templates() -> Result<Vec<ProviderTemplate>, String> {
    // 1. 获取内置模板
    let builtin = get_builtin_provider_templates();
    // 2. 获取用户自定义模板
    let custom = config::load_provider_templates().map_err(|e| e.to_string())?;
    // 3. 合并缓存模型（如果有）
    let mut all = builtin;
    all.extend(custom);

    // 尝试加载缓存的模型列表
    for tpl in &mut all {
        let cached = config::load_cached_models(&tpl.id).unwrap_or_default();
        if !cached.is_empty() {
            // 合并缓存模型和内置模型（缓存优先）
            let mut merged = cached;
            for m in &tpl.models {
                if !merged.contains(m) {
                    merged.push(m.clone());
                }
            }
            tpl.models = merged;
        }
    }

    Ok(all)
}

/// 刷新模板模型列表（调用供应商 API 实时获取）
#[derive(Serialize)]
pub struct RefreshModelsResult {
    pub models: Vec<String>,
    pub cached_at: String,
}

#[tauri::command]
async fn refresh_template_models(template_id: String, base_url: String, api_key: String) -> Result<RefreshModelsResult, String> {
    // 使用已有的 fetch_models 逻辑获取模型列表
    let result = fetch_models(base_url, api_key).await?;

    // 保存到缓存
    config::save_cached_models(&template_id, &result.models).map_err(|e| e.to_string())?;
    let cached_at = chrono::Utc::now().to_rfc3339();

    Ok(RefreshModelsResult {
        models: result.models,
        cached_at,
    })
}

/// 获取模板缓存的模型列表
#[tauri::command]
fn get_cached_models(template_id: String) -> Result<Vec<String>, String> {
    config::load_cached_models(&template_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_provider_template(input: ProviderTemplateInput) -> Result<ProviderTemplate, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = input.id.unwrap_or_else(|| format!("custom_{}", chrono::Utc::now().timestamp_millis()));
    let template = ProviderTemplate {
        id: id.clone(),
        name: input.name,
        icon: input.icon,
        color: input.color,
        description: input.description,
        builtin_icon: input.builtin_icon,
        icon_fallback: input.icon_fallback,
        base_urls: input.base_urls.into_iter().map(|u| config::ProviderTemplateUrl {
            label: u.label,
            value: u.value,
            hint: u.hint,
            protocol_hint: u.protocol_hint,
        }).collect(),
        models: input.models,
        key_placeholder: input.key_placeholder,
        help_url: input.help_url,
        badge: input.badge,
        builtin: false,
        created_at: now.clone(),
        updated_at: now,
    };
    config::save_provider_template(&template).map_err(|e| e.to_string())?;
    Ok(template)
}

#[tauri::command]
fn delete_provider_template(id: String) -> Result<(), String> {
    // 禁止删除内置模板
    if id.starts_with("builtin_") {
        return Err("无法删除内置模板".to_string());
    }
    config::delete_provider_template(&id).map_err(|e| e.to_string())?;
    Ok(())
}

// ── 完整备份导出/导入（v3.5）──────────────────────────────────────────────────────
const BACKUP_VERSION_V3: u8 = 0x03;  // v3 支持完整备份

#[derive(Serialize)]
pub struct FullBackupResult {
    pub path: String,
    pub filename: String,
    pub included: Vec<String>,  // 包含的内容类型
}

/// 内部备份函数（用于自动备份，无需 command 序列化开销）
fn export_full_backup_internal(password: String, include_templates: bool, include_skills: bool, include_plugins: bool, custom_path: Option<String>) -> Result<FullBackupResult, String> {
    // 1. 导出 providers（加密）
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let providers_json = serde_json::to_string(&providers).map_err(|e| e.to_string())?;

    let (flag, providers_encrypted) = if password.is_empty() {
        (FLAG_NO_PASSWORD, crypto::encrypt(&providers_json, &machine_key).map_err(|e| e.to_string())?)
    } else {
        let backup_key = derive_key_from_password(&password);
        (FLAG_HAS_PASSWORD, crypto::encrypt(&providers_json, &backup_key).map_err(|e| e.to_string())?)
    };

    // 2. 收集模板
    let templates: Vec<Template> = if include_templates {
        get_templates().unwrap_or_default()
    } else {
        vec![]
    };

    // 3. 收集 skills
    let skills: Vec<Skill> = if include_skills {
        get_skills().unwrap_or_default()
    } else {
        vec![]
    };

    // 4. 收集插件配置 + 插件文件
    let (enabled_plugins, extra_marketplaces, plugins_archive) = if include_plugins {
        let settings = read_claude_settings()?;
        let plugins = settings.get("enabledPlugins").cloned().unwrap_or(serde_json::json!({}));
        let marketplaces = settings.get("extraKnownMarketplaces").cloned().unwrap_or(serde_json::json!({}));
        let plugins_cache_dir = claude_config_dir().join("plugins").join("cache");
        let archive = collect_plugin_files(&plugins_cache_dir)?;
        (plugins, marketplaces, archive)
    } else {
        (serde_json::json!({}), serde_json::json!({}), serde_json::json!({}))
    };

    // 5. 收集应用配置
    let app_config = config::load_app_config().map_err(|e| e.to_string())?;
    let bindings = get_template_bindings().unwrap_or_default();

    // 6. 构建完整备份 JSON
    let full_backup = serde_json::json!({
        "providers_encrypted": providers_encrypted,
        "templates": templates.iter().map(|t| serde_json::json!({
            "name": t.name,
            "content": t.content
        })).collect::<Vec<_>>(),
        "skills": skills.iter().map(|s| serde_json::json!({
            "name": s.name,
            "content": s.content
        })).collect::<Vec<_>>(),
        "app_config": {
            "language": app_config.language,
            "backupExportPath": app_config.backup_export_path,
            "defaultConfigDir": app_config.default_config_dir,
        },
        "template_bindings": bindings,
        "enabledPlugins": enabled_plugins,
        "extraKnownMarketplaces": extra_marketplaces,
        "plugins_archive": plugins_archive,
    });
    let backup_json = serde_json::to_string(&full_backup).map_err(|e| e.to_string())?;

    // 加密整个备份
    let backup_encrypted = if password.is_empty() {
        crypto::encrypt(&backup_json, &machine_key).map_err(|e| e.to_string())?
    } else {
        let backup_key = derive_key_from_password(&password);
        crypto::encrypt(&backup_json, &backup_key).map_err(|e| e.to_string())?
    };

    // 7. 构建二进制文件
    let mut output: Vec<u8> = vec![];
    output.extend_from_slice(BACKUP_MAGIC);
    output.push(BACKUP_VERSION_V3);
    output.extend_from_slice(&machine_key_hash(&machine_key));
    output.push(flag);
    if flag == FLAG_HAS_PASSWORD {
        let pwd_hash = derive_key_from_password(&password);
        output.extend_from_slice(&machine_key_hash(&pwd_hash));
    }
    output.extend_from_slice(backup_encrypted.as_bytes());

    // 8. 写入文件
    let backups_dir = if let Some(path) = custom_path {
        if path.is_empty() {
            config::mmycs_dir().join("backups")
        } else {
            std::path::PathBuf::from(&path)
        }
    } else {
        config::mmycs_dir().join("backups")
    };
    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;
    let now = chrono::Local::now();
    let filename = format!("mmycs_full_backup_{}.mmycs", now.format("%Y%m%d_%H%M%S"));
    let filepath = backups_dir.join(&filename);
    std::fs::write(&filepath, &output).map_err(|e| e.to_string())?;

    // 9. 自动轮转备份文件（保持最多7个）
    let _ = rotate_backup_files();

    let mut included: Vec<String> = vec!["providers".to_string()];
    if include_templates { included.push("templates".to_string()); }
    if include_skills { included.push("skills".to_string()); }
    if include_plugins { included.push("plugins".to_string()); }

    Ok(FullBackupResult {
        path: filepath.to_string_lossy().to_string(),
        filename,
        included,
    })
}

#[tauri::command]
fn export_full_backup(password: String, include_templates: bool, include_skills: bool, include_plugins: bool, custom_path: Option<String>) -> Result<FullBackupResult, String> {
    export_full_backup_internal(password, include_templates, include_skills, include_plugins, custom_path)
}

// ── 备份文件管理 ───────────────────────────────────────────────────────────

const MAX_BACKUP_FILES: usize = 7;

#[derive(Serialize, Clone)]
pub struct BackupFile {
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
}

fn backups_dir() -> std::path::PathBuf {
    config::mmycs_dir().join("backups")
}

#[tauri::command]
fn get_backup_files() -> Result<Vec<BackupFile>, String> {
    let dir = backups_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut files: Vec<BackupFile> = vec![];
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("mmycs") {
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            // 从文件名提取创建时间：mmycs_full_backup_YYYYMMDD_HHMMSS.mmycs
            let created_at = extract_backup_time(&filename);
            files.push(BackupFile {
                filename,
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                created_at,
            });
        }
    }

    // 按创建时间降序排序（最新的在前）
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(files)
}

fn extract_backup_time(filename: &str) -> String {
    // 尝试从文件名解析时间：mmycs_full_backup_20260511_143052.mmycs
    if let Some(time_part) = filename.strip_prefix("mmycs_full_backup_").and_then(|s| s.strip_suffix(".mmycs")) {
        // YYYYMMDD_HHMMSS 格式
        if time_part.len() == 15 {
            let date = &time_part[0..8];
            let time = &time_part[9..15];
            if let (Ok(y), Ok(m), Ok(d)) = (date[0..4].parse::<u32>(), date[4..6].parse::<u32>(), date[6..8].parse::<u32>()) {
                if let (Ok(h), Ok(min), Ok(s)) = (time[0..2].parse::<u32>(), time[2..4].parse::<u32>(), time[4..6].parse::<u32>()) {
                    return format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, min, s);
                }
            }
        }
    }
    // 兼容旧格式：mmycs_backup_YYYYMMDD_HHMMSS.mmycs
    if let Some(time_part) = filename.strip_prefix("mmycs_backup_").and_then(|s| s.strip_suffix(".mmycs")) {
        if time_part.len() == 15 {
            let date = &time_part[0..8];
            let time = &time_part[9..15];
            if let (Ok(y), Ok(m), Ok(d)) = (date[0..4].parse::<u32>(), date[4..6].parse::<u32>(), date[6..8].parse::<u32>()) {
                if let (Ok(h), Ok(min), Ok(s)) = (time[0..2].parse::<u32>(), time[2..4].parse::<u32>(), time[4..6].parse::<u32>()) {
                    return format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, min, s);
                }
            }
        }
    }
    "未知时间".to_string()
}

fn rotate_backup_files() -> Result<usize, String> {
    let dir = backups_dir();
    if !dir.exists() {
        return Ok(0);
    }

    // 获取所有备份文件，按时间排序
    let mut files: Vec<std::path::PathBuf> = vec![];
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("mmycs") {
            files.push(path);
        }
    }

    // 按文件名排序（文件名包含时间戳，新文件名更大）
    files.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        b_name.cmp(a_name) // 降序，最新的在前
    });

    // 删除超出限制的旧文件
    let mut deleted = 0;
    while files.len() > MAX_BACKUP_FILES {
        let old_file = files.pop().unwrap(); // 取最旧的
        if let Err(e) = std::fs::remove_file(&old_file) {
            eprintln!("删除旧备份失败: {}", e);
        } else {
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[tauri::command]
fn delete_backup_file(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct FullImportResult {
    pub providers_count: usize,
    pub templates_count: usize,
    pub skills_count: usize,
    pub plugins_count: usize,
    pub same_machine: bool,
    pub need_password: bool,
}

#[tauri::command]
fn check_full_backup_file(data: Vec<u8>) -> Result<BackupInfo, String> {
    check_backup_file(data)  // 使用相同的检查逻辑
}

#[tauri::command]
fn import_full_backup(data: Vec<u8>, password: String, import_templates: bool, import_skills: bool, import_plugins: bool) -> Result<FullImportResult, String> {
    if data.len() < 39 {
        return Err("文件格式无效".to_string());
    }

    // 检查 Magic
    if &data[0..5] != BACKUP_MAGIC {
        return Err("不是 MMYCS 备份文件".to_string());
    }

    // 检查版本
    let version = data[5];
    if version != BACKUP_VERSION && version != BACKUP_VERSION_V3 {
        return Err(format!("不支持的版本: {}", version));
    }

    // 读取机器密钥 hash
    let stored_hash: [u8; 32] = data[6..38].try_into().map_err(|_| "hash 读取失败")?;

    // 检查是否同机
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let current_hash = machine_key_hash(&machine_key);
    let same_machine = stored_hash == current_hash;

    // 读取 flag
    let flag = data[38];
    let has_password = flag == FLAG_HAS_PASSWORD;

    // 计算加密数据起始位置
    let data_start = if has_password { 39 + 32 } else { 39 };
    if data.len() < data_start {
        return Err("文件数据不完整".to_string());
    }

    // 提取加密数据
    let encrypted_data = String::from_utf8_lossy(&data[data_start..]).to_string();

    // 解密
    let backup_json = if has_password {
        if password.is_empty() {
            return Err("需要输入备份密码".to_string());
        }
        let backup_key = derive_key_from_password(&password);
        crypto::decrypt(&encrypted_data, &backup_key)
            .map_err(|_| "密码错误或文件损坏".to_string())?
    } else {
        if same_machine {
            crypto::decrypt(&encrypted_data, &machine_key)
                .map_err(|e| format!("解密失败: {}", e))?
        } else {
            return Err("此备份文件未设置密码保护，仅能在原机器上导入。".to_string());
        }
    };

    // 解析完整备份
    let backup: serde_json::Value = serde_json::from_str(&backup_json)
        .map_err(|e| format!("解析失败: {}", e))?;

    // 导入 providers
    let providers_count = if let Some(providers_enc) = backup.get("providers_encrypted").and_then(|v| v.as_str()) {
        // 先解密 providers
        let providers_json = if has_password {
            let backup_key = derive_key_from_password(&password);
            crypto::decrypt(providers_enc, &backup_key).map_err(|_| "providers 解密失败".to_string())?
        } else {
            crypto::decrypt(providers_enc, &machine_key).map_err(|e| format!("providers 解密失败: {}", e))?
        };
        let providers: Vec<Provider> = serde_json::from_str(&providers_json).map_err(|e| e.to_string())?;
        let count = providers.len();
        for p in &providers {
            config::save_provider(p).map_err(|e| e.to_string())?;
        }
        count
    } else {
        0
    };

    // 导入 templates
    let templates_count = if import_templates {
        if let Some(templates_arr) = backup.get("templates").and_then(|v| v.as_array()) {
            for t in templates_arr {
                if let (Some(name), Some(content)) = (t.get("name").and_then(|n| n.as_str()), t.get("content").and_then(|c| c.as_str())) {
                    save_template(name.to_string(), content.to_string()).ok();
                }
            }
            templates_arr.len()
        } else {
            0
        }
    } else {
        0
    };

    // 导入 skills
    let skills_count = if import_skills {
        if let Some(skills_arr) = backup.get("skills").and_then(|v| v.as_array()) {
            for s in skills_arr {
                if let (Some(name), Some(content)) = (s.get("name").and_then(|n| n.as_str()), s.get("content").and_then(|c| c.as_str())) {
                    save_skill(name.to_string(), content.to_string()).ok();
                }
            }
            skills_arr.len()
        } else {
            0
        }
    } else {
        0
    };

    // 导入应用配置
    if let Some(cfg) = backup.get("app_config") {
        let mut app_cfg = config::load_app_config().map_err(|e| e.to_string())?;
        if let Some(lang) = cfg.get("language").and_then(|l| l.as_str()) {
            app_cfg.language = lang.to_string();
        }
        if let Some(path) = cfg.get("backupExportPath").and_then(|p| p.as_str()) {
            app_cfg.backup_export_path = Some(path.to_string());
        }
        if let Some(dir) = cfg.get("defaultConfigDir").and_then(|d| d.as_str()) {
            app_cfg.default_config_dir = Some(dir.to_string());
        }
        config::save_app_config(&app_cfg).map_err(|e| e.to_string())?;
    }

    // 导入模板绑定
    if let Some(bindings_arr) = backup.get("template_bindings").and_then(|v| v.as_array()) {
        let bindings_path = template_bindings_path();
        let mut bindings: TemplateBindingsFile = if bindings_path.exists() {
            std::fs::read_to_string(&bindings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(TemplateBindingsFile { bindings: vec![] })
        } else {
            TemplateBindingsFile { bindings: vec![] }
        };
        for b in bindings_arr {
            if let (Some(path), Some(name), Some(time)) = (
                b.get("project_path").and_then(|p| p.as_str()),
                b.get("template_name").and_then(|n| n.as_str()),
                b.get("updated_at").and_then(|t| t.as_str())
            ) {
                // 不重复添加
                if !bindings.bindings.iter().any(|existing| existing.project_path == path) {
                    bindings.bindings.push(TemplateBinding {
                        project_path: path.to_string(),
                        template_name: name.to_string(),
                        updated_at: time.to_string(),
                    });
                }
            }
        }
        std::fs::write(&bindings_path, serde_json::to_string_pretty(&bindings).unwrap_or_default())
            .map_err(|e| e.to_string())?;
    }

    // 导入插件配置
    let plugins_count = if import_plugins {
        let mut count = 0;
        let mut settings = read_claude_settings()?;

        // 导入 enabledPlugins
        if let Some(enabled_plugins) = backup.get("enabledPlugins") {
            if let Some(obj) = enabled_plugins.as_object() {
                if !obj.is_empty() {
                    count += obj.len();
                    // 合并：保留现有的，添加新的
                    let existing = settings.get("enabledPlugins")
                        .and_then(|v| v.as_object())
                        .cloned()
                        .unwrap_or_default();
                    let mut merged: serde_json::Map<String, serde_json::Value> = existing;
                    for (k, v) in obj {
                        merged.insert(k.clone(), v.clone());
                    }
                    settings["enabledPlugins"] = serde_json::Value::Object(merged);
                }
            }
        }

        // 导入 extraKnownMarketplaces
        if let Some(extra_marketplaces) = backup.get("extraKnownMarketplaces") {
            if let Some(obj) = extra_marketplaces.as_object() {
                if !obj.is_empty() {
                    // 合并：保留现有的，添加新的
                    let existing = settings.get("extraKnownMarketplaces")
                        .and_then(|v| v.as_object())
                        .cloned()
                        .unwrap_or_default();
                    let mut merged: serde_json::Map<String, serde_json::Value> = existing;
                    for (k, v) in obj {
                        merged.insert(k.clone(), v.clone());
                    }
                    settings["extraKnownMarketplaces"] = serde_json::Value::Object(merged);
                }
            }
        }

        // 还原插件文件（从 plugins_archive）
        if let Some(archive) = backup.get("plugins_archive") {
            if let Some(obj) = archive.as_object() {
                let target_dir = claude_config_dir().join("plugins").join("cache");
                std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
                for (plugin_key, file_map) in obj {
                    if let Some(files) = file_map.as_object() {
                        for (relative_path, content_b64) in files {
                            if let Some(b64) = content_b64.as_str() {
                                let decoded = base64_decode(b64).map_err(|e| e.to_string())?;
                                let target_path = target_dir.join(plugin_key).join(relative_path);
                                std::fs::create_dir_all(target_path.parent().unwrap()).map_err(|e| e.to_string())?;
                                // 文件已存在则跳过
                                if !target_path.exists() {
                                    std::fs::write(&target_path, &decoded).map_err(|e| e.to_string())?;
                                }
                            }
                        }
                    }
                }
            }
        }

        if count > 0 {
            save_claude_settings(&settings)?;
        }
        count
    } else {
        0
    };

    Ok(FullImportResult {
        providers_count,
        templates_count,
        skills_count,
        plugins_count,
        same_machine,
        need_password: has_password,
    })
}

// ── 检测哪些项目目录有正在运行的 Claude Code CLI ──────────────────────────────
#[derive(Serialize)]
pub struct ActiveClaudeProcess {
    pub project_path: String,
    pub pid: u32,
}

#[tauri::command]
fn check_active_claude_processes(project_paths: Vec<String>) -> Vec<ActiveClaudeProcess> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let mut result: Vec<ActiveClaudeProcess> = vec![];

        // 使用 PowerShell 获取所有包含 "claude" 的进程及其命令行
        let ps_script = "Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*claude*' } | Select-Object ProcessId, CommandLine";
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", ps_script])
            .output()
            .ok();

        if let Some(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // 解析输出，找出包含项目路径的进程
            for proj_path in &project_paths {
                let norm_proj = proj_path.replace("\\", "/");
                let proj_lower = proj_path.to_lowercase();
                let norm_lower = norm_proj.to_lowercase();

                for line in stdout.lines() {
                    let line_lower = line.to_lowercase();
                    if line_lower.contains("claude") &&
                       (line_lower.contains(&proj_lower) || line_lower.contains(&norm_lower)) {
                        // 尝试提取 PID（第一列数字）
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(first) = parts.first() {
                            if let Ok(pid) = first.parse::<u32>() {
                                if !result.iter().any(|r| r.project_path == *proj_path) {
                                    result.push(ActiveClaudeProcess {
                                        project_path: proj_path.clone(),
                                        pid,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let mut result: Vec<ActiveClaudeProcess> = vec![];

        // macOS/Linux: 使用 ps 命令检测
        let output = Command::new("ps")
            .args(["aux"])
            .output()
            .ok();

        if let Some(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);

            for line in stdout.lines() {
                if line.contains("claude") {
                    for proj_path in &project_paths {
                        let norm_proj = proj_path.replace("\\", "/");
                        if line.contains(proj_path) || line.contains(&norm_proj) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            let pid: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                            if pid > 0 && !result.iter().any(|r| r.project_path == *proj_path) {
                                result.push(ActiveClaudeProcess {
                                    project_path: proj_path.clone(),
                                    pid,
                                });
                            }
                            break;
                        }
                    }
                }
            }
        }

        result
    }
}

// ── 自动检测 Claude Code 实例 ─────────────────────────────────────────────────
#[tauri::command]
fn detect_instances() -> Vec<DetectedInstance> {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates: &[(&str, &str)] = &[
        ("默认 (~/.claude)", ".claude"),
        ("Cursor (~/.cursor)", ".cursor"),
    ];
    let mut result = vec![];
    for (label, sub) in candidates {
        let path = home.join(sub);
        if path.join("settings.json").exists() || path.is_dir() {
            result.push(DetectedInstance {
                name: label.to_string(),
                config_dir: path.to_string_lossy().to_string(),
            });
        }
    }
    // Windows: %APPDATA%\Claude
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = std::path::PathBuf::from(&appdata).join("Claude");
        if p.is_dir() {
            result.push(DetectedInstance { name: "Claude (AppData)".to_string(), config_dir: p.to_string_lossy().to_string() });
        }
    }
    result
}

// ── 启动 Claude Code CLI 终端 ─────────────────────────────────────────────
#[tauri::command]
fn launch_terminal(workdir: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        // Windows: 用 PowerShell 启动 claude CLI，优先尝试 wt (Windows Terminal)
        use std::os::windows::process::CommandExt;
        let work_path = std::path::Path::new(&workdir);
        if !work_path.exists() {
            return Err(format!("目录不存在: {}", workdir));
        }

        // 用 where 命令检测是否有 wt（不依赖外部 crate）
        let has_wt = Command::new("where")
            .arg("wt")
            .creation_flags(0x08000000)  // CREATE_NO_WINDOW — 静默检测
            .output()
            .ok()
            .and_then(|o| if o.status.success() { Some(()) } else { None })
            .is_some();

        let mut cmd;
        if has_wt {
            cmd = Command::new("wt");
            cmd.arg("-d").arg(&workdir)
               .arg("powershell").arg("-NoExit").arg("-Command").arg("claude");
        } else {
            cmd = Command::new("powershell");
            cmd.arg("-NoExit").arg("-Command").arg("claude");
        }

        cmd.current_dir(work_path);
        const CREATE_NEW_CONSOLE: u32 = 0x00000010;
        cmd.creation_flags(CREATE_NEW_CONSOLE);
        cmd.spawn().map_err(|e| format!("启动终端失败: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
    #[cfg(target_os = "macos")]
    {
        // macOS：使用 AppleScript 让 Terminal 在工作目录中执行 claude 命令
        // 注意：open -a Terminal --args claude 只能打开终端但不会执行命令
        let script = format!(
            "tell application \"Terminal\" to do script \"cd {} && claude\"",
            workdir.replace('"', "\\\"")  // 转义路径中的引号
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }
        #[cfg(target_os = "linux")]
        {
            Command::new("x-terminal-emulator")
                .or_else(|_| Command::new("gnome-terminal"))
                .or_else(|_| Command::new("konsole"))
                .map_err(|e| format!("未找到终端模拟器: {}", e))?
                .args(["--", "-e", "claude"])
                .current_dir(&workdir)
                .spawn()
                .map_err(|e| format!("启动终端失败: {}", e))?;
        }
        Ok(())
    }
}

/// 从任意嵌套 JSON 中递归查找目标字段（支持 env 包裹、多层嵌套等中转站格式）
fn deep_find(json: &serde_json::Value, targets: &[&str]) -> Option<(String, String)> {
    // 1. 当前层直接匹配
    let mut found_url = None;
    let mut found_key = None;
    for &target in targets {
        if let Some(v) = json.get(target).and_then(|v| v.as_str()) {
            if target.contains("URL") || target.contains("url") || target.contains("BASE") {
                if found_url.is_none() { found_url = Some(v.to_string()); }
            } else if target.contains("KEY") || target.contains("key") || target.contains("TOKEN") || target.contains("token") {
                if found_key.is_none() { found_key = Some(v.to_string()); }
            }
        }
    }
    if found_url.is_some() || found_key.is_some() {
        return Some((found_url.unwrap_or_default(), found_key.unwrap_or_default()));
    }

    // 2. 递归子节点（Object 的每个 value / Array 的每个元素）
    match json {
        serde_json::Value::Object(map) => {
            for (_k, v) in map {
                if let Some(result) = deep_find(v, targets) { return Some(result); }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(result) = deep_find(item, targets) { return Some(result); }
            }
        }
        _ => {}
    }
    None
}

/// 已知的模型名称字段——覆盖 Anthropic / OpenAI 常见命名
const KNOWN_MODEL_FIELDS: &[&str] = &[
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_REASONING_MODEL",
    "OPENAI_MODEL",
    "DEFAULT_MODEL",
    "MODEL",
];
const KNOWN_URL_FIELDS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_BASE",
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE",
    "API_BASE_URL",
    "BASE_URL",
    "baseUrl",
    "base_url",
    "apiBase",
    "endpoint",
    "ENDPOINT",
];

const KNOWN_KEY_FIELDS: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "OPENAI_AUTH_TOKEN",
    "API_KEY",
    "apiKey",
    "api_key",
    "AUTH_TOKEN",
    "authToken",
    "auth_token",
    "token",
    "TOKEN",
    "ACCESS_TOKEN",
    "access_token",
];

/// 从 JSON 中递归收集所有模型名称字段
fn collect_models_from_json(json: &serde_json::Value) -> Vec<String> {
    let mut models = Vec::new();
    match json {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if KNOWN_MODEL_FIELDS.contains(&k.as_str()) {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() { models.push(s.to_string()); }
                    }
                }
                models.extend(collect_models_from_json(v));
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                models.extend(collect_models_from_json(item));
            }
        }
        _ => {}
    }
    models
}

#[tauri::command]
fn parse_paste(text: String) -> serde_json::Value {
    let mut result = serde_json::json!({ "baseUrl": null, "apiKey": null, "models": [] });
    let mut models: Vec<String> = Vec::new();

    // ── 策略1: 整体作为 JSON 递归深挖（支持嵌套/中转站格式）──
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        let all_targets: Vec<&str> = KNOWN_URL_FIELDS.iter().chain(KNOWN_KEY_FIELDS.iter()).copied().collect();
        if let Some((url, key)) = deep_find(&json, &all_targets) {
            if !url.is_empty() { result["baseUrl"] = serde_json::Value::String(url); }
            if !key.is_empty() { result["apiKey"] = serde_json::Value::String(key); }
        }
        models = collect_models_from_json(&json);
    }

    // ── 策略2: 逐行解析（兼容 export 语句、裸值、扁平 JSON）──
    for line in text.lines() {
        let line = line.trim();
        if result["baseUrl"].is_null() || result["apiKey"].is_null() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                let all_targets: Vec<&str> = KNOWN_URL_FIELDS.iter().chain(KNOWN_KEY_FIELDS.iter()).copied().collect();
                if let Some((url, key)) = deep_find(&json, &all_targets) {
                    if !url.is_empty() && result["baseUrl"].is_null() { result["baseUrl"] = serde_json::Value::String(url); }
                    if !key.is_empty() && result["apiKey"].is_null() { result["apiKey"] = serde_json::Value::String(key); }
                    continue;
                }
            }

            if line.starts_with("export ") && result["baseUrl"].is_null() {
                for &field in KNOWN_URL_FIELDS {
                    let prefix = format!("export {}=", field);
                    if let Some(rest) = line.strip_prefix(&prefix) {
                        result["baseUrl"] = serde_json::Value::String(rest.trim_matches('"').trim_matches('\'').to_string());
                        break;
                    }
                }
            }
            if line.starts_with("export ") && result["apiKey"].is_null() {
                for &field in KNOWN_KEY_FIELDS {
                    let prefix = format!("export {}=", field);
                    if let Some(rest) = line.strip_prefix(&prefix) {
                        result["apiKey"] = serde_json::Value::String(rest.trim_matches('"').trim_matches('\'').to_string());
                        break;
                    }
                }
            }

            if (line.starts_with("https://") || line.starts_with("http://")) && result["baseUrl"].is_null() {
                result["baseUrl"] = serde_json::Value::String(line.to_string());
            }

            if (line.starts_with("sk-") || line.len() > 20)
                && result["apiKey"].is_null()
                && !line.contains(' ')
                && !(line.starts_with("https://") || line.starts_with("http://"))
                && !line.starts_with("export ")
                && !line.starts_with("{") {
                result["apiKey"] = serde_json::Value::String(line.to_string());
            }
        }
    }

    models.sort();
    models.dedup();
    result["models"] = serde_json::Value::Array(models.into_iter().map(serde_json::Value::String).collect());
    result
}

fn setup_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let providers = config::load_providers().unwrap_or_default();
    let cfg = config::load_app_config().unwrap_or_default();
    let active_id = cfg.instances.first().and_then(|i| i.active_provider_id.clone());

    // ── "启动平台" 子菜单
    let mut platform_builder = SubmenuBuilder::new(app, "启动平台");
    for p in &providers {
        let label = if Some(&p.id) == active_id.as_ref() {
            format!("✓ {}", p.name)
        } else {
            p.name.clone()
        };
        platform_builder = platform_builder.text(&p.id, label);
    }
    if providers.is_empty() {
        platform_builder = platform_builder.text("__empty__", "(无平台)");
    }
    let platform_sub = platform_builder.build()?;

    // ── 顶层菜单
    let show_win = MenuItem::with_id(app, "__show__", "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "__quit__", "退出", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&show_win)
        .item(&platform_sub)
        .item(&quit)
        .build()?;

    // 获取配置文件创建的托盘图标并设置菜单
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "__show__" => {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.unminimize();
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                "__quit__" => {
                    app.exit(0);
                }
                "__empty__" => {}
                _ => {
                    // 点击了某个 provider → 切换
                    let provider_id = id.to_string();
                    let cfg = config::load_app_config().unwrap_or_default();
                    if let Some(inst) = cfg.instances.first() {
                        let config_dir = inst.config_dir.clone();
                        let _ = (|| -> Result<(), String> {
                            let providers = config::load_providers().map_err(|e| e.to_string())?;
                            let provider = providers.iter().find(|p| p.id == provider_id).ok_or("not found")?;
                            let api_key_plain = if provider.provider_type == "api" {
                                if let Some(enc) = &provider.api_key_encrypted {
                                    let key = config::get_or_create_key().map_err(|e| e.to_string())?;
                                    Some(crypto::decrypt(enc, &key).map_err(|e| e.to_string())?)
                                } else { None }
                            } else { None };
                            inject::inject(&config_dir, provider, api_key_plain.as_deref()).map_err(|e| e.to_string())?;
                            let mut cfg2 = config::load_app_config().map_err(|e| e.to_string())?;
                            for inst in &mut cfg2.instances {
                                if inst.config_dir == config_dir {
                                    inst.active_provider_id = Some(provider_id.clone());
                                }
                            }
                            config::save_app_config(&cfg2).map_err(|e| e.to_string())
                        })();
                    }
                    // 刷新托盘菜单
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = rebuild_tray_menu(app, &tray);
                    }
                }
            }
        });
        tray.on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        });
    }
    Ok(())
}

/// 重建托盘菜单（切换 provider 后刷新勾选标记）
fn rebuild_tray_menu(app: &tauri::AppHandle, tray: &tauri::tray::TrayIcon) -> Result<(), Box<dyn std::error::Error>> {
    let providers = config::load_providers().unwrap_or_default();
    let cfg = config::load_app_config().unwrap_or_default();
    let active_id = cfg.instances.first().and_then(|i| i.active_provider_id.clone());

    let mut platform_builder = SubmenuBuilder::new(app, "启动平台");
    for p in &providers {
        let label = if Some(&p.id) == active_id.as_ref() { format!("✓ {}", p.name) } else { p.name.clone() };
        platform_builder = platform_builder.text(&p.id, label);
    }
    if providers.is_empty() {
        platform_builder = platform_builder.text("__empty__", "(无平台)");
    }
    let platform_sub = platform_builder.build()?;

    let show_win = MenuItem::with_id(app, "__show__", "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "__quit__", "退出", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&show_win)
        .item(&platform_sub)
        .item(&quit)
        .build()?;

    tray.set_menu(Some(menu))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            setup_tray(&app.handle())?;

            // ── 恢复上次窗口尺寸和位置 ──
            if let Some(win) = app.get_webview_window("main") {
                let state_path = config::mmycs_dir().join("window_state.json");
                if state_path.exists() {
                    if let Ok(s) = std::fs::read_to_string(&state_path) {
                        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&s) {
                            let _ = win.set_size(tauri::LogicalSize::new(
                                state["width"].as_u64().unwrap_or(510) as u32,
                                state["height"].as_u64().unwrap_or(620) as u32,
                            ));
                            // 只在有合理坐标时才恢复位置（避免屏幕外）
                            if let (Some(x), Some(y)) = (state["x"].as_i64(), state["y"].as_i64()) {
                                if x >= -100 && y >= -100 {
                                    let _ = win.set_position(tauri::LogicalPosition::new(x as i32, y as i32));
                                }
                            }
                        }
                    }
                }

                // 监听窗口关闭事件：阻止默认关闭 → 自动备份 → 保存状态 → 隐藏到托盘
                let win_clone = win.clone();
                let path_clone = state_path.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // ★ 关键：阻止默认关闭行为
                        api.prevent_close();

                        // 1. 自动备份（静默执行，不含插件文件以减小体积）
                        let _ = export_full_backup_internal(
                            "".to_string(),
                            true,   // include_templates
                            true,   // include_skills
                            false,  // include_plugins
                            None,   // custom_path
                        ).map_err(|e| eprintln!("auto backup failed: {}", e));

                        // 2. 保存窗口状态
                        if let Ok(pos) = win_clone.outer_position() {
                            if let Ok(size) = win_clone.outer_size() {
                                let mut state = if path_clone.exists() {
                                    std::fs::read_to_string(&path_clone).ok()
                                        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                        .unwrap_or(serde_json::json!({}))
                                } else { serde_json::json!({}) };
                                state["x"] = serde_json::json!(pos.x);
                                state["y"] = serde_json::json!(pos.y);
                                state["width"] = serde_json::json!(size.width);
                                state["height"] = serde_json::json!(size.height);
                                let _ = std::fs::write(&path_clone, serde_json::to_string_pretty(&state).unwrap_or_default());
                            }
                        }
                        // 3. 隐藏到托盘
                        let _ = win_clone.hide().map_err(|e| eprintln!("hide failed: {}", e));
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_app,
            get_app_config,
            save_app_config,
            get_providers,
            upsert_provider,
            delete_provider,
            reorder_providers,
            reorder_projects,
            switch_provider,
            inject_to_project,
            get_active_projects,
            remove_active_project,
            get_project_config_dir,
            get_project_sessions,
            launch_terminal,
            parse_paste,
            export_backup,
            export_backup_quick,
            import_backup,
            check_backup_file,
            import_providers_legacy,
            export_full_backup,
            import_full_backup,
            check_full_backup_file,
            // Backup file management
            get_backup_files,
            delete_backup_file,
            fetch_models,
            test_provider,
            detect_instances,
            save_provider_icon,
            save_window_state,
            get_window_state,
            hide_to_tray,
            check_active_claude_processes,
            clean_claude_md_block,
            // Template management
            get_templates,
            save_template,
            delete_template,
            adopt_template,
            get_template_bindings,
            bind_template,
            unbind_template,
            inject_claude_md,
            get_project_template,
            // Skill management
            get_skills,
            save_skill,
            delete_skill,
            // Provider templates
            get_provider_templates,
            save_provider_template,
            delete_provider_template,
            refresh_template_models,
            get_cached_models,
        // Plugin management
            get_plugins,
            toggle_plugin,
            get_marketplaces,
            add_marketplace,
            remove_marketplace,
            get_usage_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
