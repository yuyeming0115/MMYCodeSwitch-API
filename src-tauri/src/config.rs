use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use crate::crypto;

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
        }
    }
}

pub fn mmycs_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".mmycs")
}

pub fn ensure_dirs() -> Result<()> {
    let base = mmycs_dir();
    for sub in &["providers", "instances", "backups", "logs"] {
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
