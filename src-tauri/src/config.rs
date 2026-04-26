use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use crate::crypto;
use md5;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(rename = "iconFallback")]
    pub icon_fallback: String,
    #[serde(rename = "type")]
    pub provider_type: String, // "api" | "login"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub config_dir: String,
    pub active_provider_id: Option<String>,
}

/// 已激活的项目绑定记录（多项目模式核心数据结构）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveProject {
    pub id: String,
    pub name: String,
    #[serde(alias = "projectPath")]
    pub project_path: String,
    #[serde(alias = "providerId")]
    pub provider_id: String,
    #[serde(alias = "providerName")]
    pub provider_name: String,
    #[serde(alias = "createdAt")]
    pub created_at: String,
    #[serde(alias = "updatedAt")]
    pub updated_at: String,
    /// 项目专属配置目录路径（新增）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "configDir")]
    pub config_dir: Option<String>,
}

/// 会话归档记录（切换供应商时记录）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionArchive {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub switched_at: String,
    pub config_snapshot: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub instances: Vec<Instance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "defaultConfigDir")]
    pub default_config_dir: Option<String>,
    #[serde(default)]
    #[serde(rename = "activeProjects")]
    pub active_projects: Vec<ActiveProject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "backupExportPath")]
    pub backup_export_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let default_claude = home.join(".claude");
        Self {
            language: "zh".to_string(),
            instances: vec![Instance {
                id: "default".to_string(),
                name: "默认实例".to_string(),
                config_dir: default_claude.to_string_lossy().to_string(),
                active_provider_id: None,
            }],
            default_config_dir: None,
            active_projects: vec![],
            backup_export_path: None,
        }
    }
}

pub fn mmycs_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".mmycs")
}

pub fn ensure_dirs() -> Result<()> {
    let base = mmycs_dir();
    for sub in &["providers", "instances", "backups", "logs", "projects", "templates", "skills", "provider_templates"] {
        std::fs::create_dir_all(base.join(sub))?;
    }
    Ok(())
}

pub fn load_app_config() -> Result<AppConfig> {
    let path = mmycs_dir().join("config.json");
    if !path.exists() {
        let cfg = AppConfig::default();
        save_app_config(&cfg)?;
        return Ok(cfg);
    }
    let s = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&s)?)
}

pub fn save_app_config(cfg: &AppConfig) -> Result<()> {
    let path = mmycs_dir().join("config.json");
    std::fs::write(path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

fn key_path() -> PathBuf {
    mmycs_dir().join(".key")
}

pub fn get_or_create_key() -> Result<String> {
    let p = key_path();
    if p.exists() {
        Ok(std::fs::read_to_string(&p)?.trim().to_string())
    } else {
        let key = crypto::generate_key();
        std::fs::write(&p, &key)?;
        Ok(key)
    }
}

pub fn load_providers() -> Result<Vec<Provider>> {
    let dir = mmycs_dir().join("providers");
    let mut providers = vec![];
    if !dir.exists() { return Ok(providers); }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            let s = std::fs::read_to_string(entry.path())?;
            if let Ok(p) = serde_json::from_str::<Provider>(&s) {
                providers.push(p);
            }
        }
    }
    providers.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(providers)
}

pub fn save_provider(provider: &Provider) -> Result<()> {
    let path = mmycs_dir().join("providers").join(format!("{}.json", provider.id));
    std::fs::write(path, serde_json::to_string_pretty(provider)?)?;
    Ok(())
}

pub fn delete_provider(id: &str) -> Result<()> {
    let path = mmycs_dir().join("providers").join(format!("{}.json", id));
    if path.exists() { std::fs::remove_file(path)?; }
    Ok(())
}

// ── ActiveProject 辅助函数 ───────────────────────────────────────────────

/// 规范化项目路径（统一使用 / 分隔，去除尾部斜杠）
pub fn normalize_project_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

/// 根据路径查找已激活的项目
pub fn find_active_project_by_path(cfg: &AppConfig, path: &str) -> Option<usize> {
    let norm = normalize_project_path(path);
    cfg.active_projects
        .iter()
        .position(|p| normalize_project_path(&p.project_path) == norm)
}

// ── 项目专属配置目录相关函数 ───────────────────────────────────────────────

/// 生成项目路径的 MD5 哈希（作为目录标识）
pub fn get_project_hash(project_path: &str) -> String {
    let norm = normalize_project_path(project_path);
    let digest = md5::compute(norm.as_bytes());
    format!("{:x}", digest)
}

/// 获取项目专属配置目录路径
pub fn get_project_config_dir(project_path: &str) -> PathBuf {
    let hash = get_project_hash(project_path);
    mmycs_dir().join("projects").join(hash)
}

/// 确保项目专属配置目录存在（包括 sessions 子目录）
pub fn ensure_project_config_dir(project_path: &str) -> Result<PathBuf> {
    let dir = get_project_config_dir(project_path);
    std::fs::create_dir_all(dir.join("sessions"))?;
    Ok(dir)
}

/// 归档当前会话（切换供应商时记录历史）
pub fn archive_session(project_path: &str, provider: &Provider, config_snapshot: HashMap<String, String>) -> Result<()> {
    let sessions_dir = get_project_config_dir(project_path).join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let now = chrono::Utc::now();
    let archive = SessionArchive {
        id: format!("session_{}", now.timestamp_millis()),
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        switched_at: now.to_rfc3339(),
        config_snapshot,
    };

    // 文件名格式：{timestamp}_{provider_name}.json
    let ts = now.format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.json", ts, provider.name);
    let path = sessions_dir.join(filename);

    std::fs::write(path, serde_json::to_string_pretty(&archive)?)?;
    Ok(())
}

/// 获取项目的会话归档列表
pub fn get_project_sessions(project_path: &str) -> Result<Vec<SessionArchive>> {
    let sessions_dir = get_project_config_dir(project_path).join("sessions");
    if !sessions_dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = vec![];
    for entry in std::fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            let s = std::fs::read_to_string(entry.path())?;
            if let Ok(session) = serde_json::from_str::<SessionArchive>(&s) {
                sessions.push(session);
            }
        }
    }
    // 按时间倒序排列（最新的在前）
    sessions.sort_by(|a, b| b.switched_at.cmp(&a.switched_at));
    Ok(sessions)
}

/// 更新项目的绑定记录（写入 binding.json）
pub fn update_project_binding(project_path: &str, provider_id: &str, provider_name: &str) -> Result<()> {
    let dir = get_project_config_dir(project_path);
    std::fs::create_dir_all(&dir)?;

    let binding = serde_json::json!({
        "provider_id": provider_id,
        "provider_name": provider_name,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    std::fs::write(dir.join("binding.json"), serde_json::to_string_pretty(&binding)?)?;
    Ok(())
}

// ── 供应商模板（Provider Templates）──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderTemplateUrl {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "protocolHint")]
    pub protocol_hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderTemplate {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "builtinIcon")]
    pub builtin_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iconFallback")]
    pub icon_fallback: Option<String>,
    #[serde(rename = "baseUrls")]
    pub base_urls: Vec<ProviderTemplateUrl>,
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "keyPlaceholder")]
    pub key_placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "helpUrl")]
    pub help_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    /// 是否为内置模板
    #[serde(default)]
    pub builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 获取供应商模板目录
fn provider_templates_dir() -> PathBuf {
    mmycs_dir().join("provider_templates")
}

/// 加载用户自定义供应商模板
pub fn load_provider_templates() -> Result<Vec<ProviderTemplate>> {
    let dir = provider_templates_dir();
    let mut templates = vec![];
    if !dir.exists() { return Ok(templates); }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            let s = std::fs::read_to_string(entry.path())?;
            if let Ok(t) = serde_json::from_str::<ProviderTemplate>(&s) {
                templates.push(t);
            }
        }
    }
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

/// 保存供应商模板
pub fn save_provider_template(template: &ProviderTemplate) -> Result<()> {
    let dir = provider_templates_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", template.id));
    std::fs::write(path, serde_json::to_string_pretty(template)?)?;
    Ok(())
}

/// 删除供应商模板
pub fn delete_provider_template(id: &str) -> Result<()> {
    let path = provider_templates_dir().join(format!("{}.json", id));
    if path.exists() { std::fs::remove_file(path)?; }
    Ok(())
}
