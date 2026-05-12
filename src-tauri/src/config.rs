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
    #[serde(default)]
    pub order: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub config_dir: String,
    #[serde(alias = "activeProviderId")]
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
    /// 项目专属配置目录路径
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "configDir")]
    pub config_dir: Option<String>,
    /// 排序索引（拖拽排序用）
    #[serde(default)]
    pub order: u32,
    /// 供应商目录名（v2 新增，用于定位 providers/<dir>/ 下的数据）
    #[serde(default)]
    #[serde(alias = "providerDir")]
    pub provider_dir: String,
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
    /// 数据结构版本（当前为 2，v1 = 扁平结构，v2 = 供应商分目录）
    #[serde(default = "default_config_version")]
    pub version: u8,
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

fn default_config_version() -> u8 { 2 }

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let default_claude = home.join(".claude");
        Self {
            version: 2,
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
    for sub in &["providers", "instances", "backups", "logs", "templates", "skills", "provider_templates"] {
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

    // 优先从供应商子目录读取（v2 结构：providers/<dir>/provider.json）
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let provider_path = path.join("provider.json");
            if provider_path.exists() {
                if let Ok(s) = std::fs::read_to_string(&provider_path) {
                    if let Ok(p) = serde_json::from_str::<Provider>(&s) {
                        providers.push(p);
                    }
                }
            }
        }
    }

    // 回退兼容：读取旧格式平铺文件（providers/{id}.json）
    if providers.is_empty() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let s = std::fs::read_to_string(entry.path())?;
                if let Ok(p) = serde_json::from_str::<Provider>(&s) {
                    providers.push(p);
                }
            }
        }
    }

    providers.sort_by(|a, b| {
        a.order.cmp(&b.order).then_with(|| a.created_at.cmp(&b.created_at))
    });
    Ok(providers)
}

pub fn save_provider(provider: &Provider) -> Result<()> {
    // 优先使用 provider_dir 作为子目录（v2 结构）
    let provider_dir = if !provider.id.starts_with("provider_") {
        // ID 本身就是规范化目录名（如 foxcode、how88）
        provider.id.clone()
    } else {
        // 旧格式 ID → 生成目录名
        provider_name_to_dir(&provider.name)
    };
    let dir = mmycs_dir().join("providers").join(&provider_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("provider.json");
    std::fs::write(path, serde_json::to_string_pretty(provider)?)?;
    Ok(())
}

pub fn delete_provider(id: &str) -> Result<()> {
    // 优先尝试删除 v2 目录
    let providers = load_providers()?;
    if let Some(p) = providers.iter().find(|p| p.id == id) {
        let provider_dir = provider_name_to_dir(&p.name);
        let dir = mmycs_dir().join("providers").join(&provider_dir);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            return Ok(());
        }
    }
    // 回退：旧格式文件
    let path = mmycs_dir().join("providers").join(format!("{}.json", id));
    if path.exists() { std::fs::remove_file(path)?; }
    Ok(())
}

/// 重排供应商顺序：接收排序后的 ID 列表，依次写入 order 字段
pub fn reorder_providers(ordered_ids: &[String]) -> Result<()> {
    let mut providers = load_providers()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        if let Some(p) = providers.iter_mut().find(|p| p.id == *id) {
            p.order = idx as u32;
            save_provider(p)?;
        }
    }
    Ok(())
}

/// 重排项目顺序：接收排序后的 ID 列表，依次写入 order 字段
pub fn reorder_projects(ordered_ids: &[String]) -> Result<()> {
    let mut cfg = load_app_config()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        if let Some(p) = cfg.active_projects.iter_mut().find(|p| p.id == *id) {
            p.order = idx as u32;
        }
    }
    save_app_config(&cfg)?;
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

// ── 供应商目录命名辅助函数 ───────────────────────────────────────────────

/// 将供应商名转换为安全的目录名
/// 规则：全小写、中文转拼音风格、特殊字符替换为连字符
pub fn provider_name_to_dir(name: &str) -> String {
    let mut result = String::new();
    for c in name.chars() {
        match c {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' | '-' => result.push(c),
            ' ' | '.' | '(' | ')' | '（' | '）' => result.push('-'),
            c if c.is_ascii() => {} // 跳过其他 ASCII 特殊字符
            c => result.push(c),     // 保留中文等字符
        }
    }
    let result = result.to_lowercase();
    // 去除连续连字符
    let result = result
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    result.trim_matches('-').to_string()
}

/// 根据 ActiveProject 获取供应商目录名
pub fn get_provider_dir_for_project(project: &ActiveProject) -> String {
    if !project.provider_dir.is_empty() {
        return project.provider_dir.clone();
    }
    // 回退：从 provider_name 生成
    provider_name_to_dir(&project.provider_name)
}

// ── 项目专属配置目录相关函数 ───────────────────────────────────────────────

/// 生成项目路径的 MD5 哈希（作为目录标识）
pub fn get_project_hash(project_path: &str) -> String {
    let norm = normalize_project_path(project_path);
    let digest = md5::compute(norm.as_bytes());
    format!("{:x}", digest)
}

/// 获取供应商目录（从 provider_name 生成）
fn get_provider_dir(provider_name: &str) -> String {
    provider_name_to_dir(provider_name)
}

/// 获取项目专属配置目录路径
/// v1 结构: ~/.mmycs/projects/{md5_hash}/
/// v2 结构: ~/.mmycs/providers/{provider_dir}/projects/{project_name}/
pub fn get_project_config_dir(project_path: &str, provider_name: &str) -> PathBuf {
    let provider_dir = get_provider_dir(provider_name);
    let norm = normalize_project_path(project_path);
    let project_name = std::path::Path::new(&norm)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let v2_dir = mmycs_dir().join("providers").join(&provider_dir).join("projects").join(project_name);
    if v2_dir.exists() {
        v2_dir
    } else {
        // 回退到 v1 哈希目录
        let hash = get_project_hash(project_path);
        mmycs_dir().join("projects").join(hash)
    }
}

/// 根据 ActiveProject 获取配置目录（优先使用 provider_dir 字段）
pub fn get_project_config_dir_for_project(project: &ActiveProject) -> PathBuf {
    let provider_dir = get_provider_dir_for_project(project);
    let norm = normalize_project_path(&project.project_path);
    let project_name = std::path::Path::new(&norm)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    mmycs_dir().join("providers").join(&provider_dir).join("projects").join(project_name)
}

/// 确保项目专属配置目录存在（包括 sessions 子目录）
pub fn ensure_project_config_dir(project_path: &str, provider_name: &str) -> Result<PathBuf> {
    let dir = get_project_config_dir(project_path, provider_name);
    std::fs::create_dir_all(dir.join("sessions"))?;
    Ok(dir)
}

/// 确保项目专属配置目录存在（使用 ActiveProject）
pub fn ensure_project_config_dir_for_project(project: &ActiveProject) -> Result<PathBuf> {
    let dir = get_project_config_dir_for_project(project);
    std::fs::create_dir_all(dir.join("sessions"))?;
    Ok(dir)
}

/// 归档当前会话（切换供应商时记录历史）
pub fn archive_session(project_path: &str, provider: &Provider, config_snapshot: HashMap<String, String>) -> Result<()> {
    let sessions_dir = get_project_config_dir(project_path, &provider.name).join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let now = chrono::Utc::now();
    let archive = SessionArchive {
        id: format!("session_{}", now.timestamp_millis()),
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        switched_at: now.to_rfc3339(),
        config_snapshot,
    };

    let ts = now.format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.json", ts, provider.name);
    let path = sessions_dir.join(filename);

    std::fs::write(path, serde_json::to_string_pretty(&archive)?)?;
    Ok(())
}

/// 获取项目的会话归档列表
pub fn get_project_sessions(project_path: &str, provider_name: &str) -> Result<Vec<SessionArchive>> {
    let sessions_dir = get_project_config_dir(project_path, provider_name).join("sessions");
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
    sessions.sort_by(|a, b| b.switched_at.cmp(&a.switched_at));
    Ok(sessions)
}

/// 更新项目的绑定记录（写入 binding.json）
pub fn update_project_binding(project_path: &str, provider_id: &str, provider_name: &str) -> Result<()> {
    let dir = get_project_config_dir(project_path, provider_name);
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

// ── 模型缓存（用于实时刷新后保存）──────────────────────────────────────────────

/// 获取模型缓存目录
fn models_cache_dir() -> PathBuf {
    mmycs_dir().join("models_cache")
}

/// 保存模板模型缓存
pub fn save_cached_models(template_id: &str, models: &[String]) -> Result<()> {
    let dir = models_cache_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", template_id));
    let cache = serde_json::json!({
        "models": models,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });
    std::fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

/// 加载模板模型缓存
pub fn load_cached_models(template_id: &str) -> Result<Vec<String>> {
    let path = models_cache_dir().join(format!("{}.json", template_id));
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let cache: serde_json::Value = serde_json::from_str(&content)?;
    if let Some(arr) = cache.get("models").and_then(|m| m.as_array()) {
        Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
    } else {
        Ok(vec![])
    }
}

/// 获取缓存更新时间
pub fn get_cache_updated_at(template_id: &str) -> Option<String> {
    let path = models_cache_dir().join(format!("{}.json", template_id));
    if !path.exists() {
        return None;
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cache) = serde_json::from_str::<serde_json::Value>(&content) {
            return cache.get("updated_at").and_then(|t| t.as_str()).map(String::from);
        }
    }
    None
}

// ── 数据结构迁移（v1 → v2：供应商分目录）──────────────────────────────────────

/// 检查并执行 v1 → v2 迁移
/// v1: providers/{id}.json + projects/{md5_hash}/
/// v2: providers/{dir}/provider.json + providers/{dir}/projects/{name}/
pub fn migrate_v1_to_v2() -> Result<bool> {
    let mut cfg = load_app_config()?;
    if cfg.version >= 2 {
        return Ok(false); // 已是最新版本，无需迁移
    }

    // 1. 为现有供应商创建目录并移动 provider JSON
    let providers = load_providers()?;
    let mut name_count: HashMap<String, u32> = HashMap::new();
    let mut provider_map: HashMap<String, String> = HashMap::new(); // provider_id -> provider_dir

    for provider in &providers {
        let mut dir_name = provider_name_to_dir(&provider.name);
        // 处理重名
        let count = name_count.entry(dir_name.clone()).or_insert(0);
        if *count > 0 {
            dir_name = format!("{}-{}", dir_name, *count);
        }
        *count += 1;

        let provider_dir = mmycs_dir().join("providers").join(&dir_name);
        std::fs::create_dir_all(&provider_dir)?;

        // 移动/复制 provider JSON
        let old_path = mmycs_dir().join("providers").join(format!("{}.json", provider.id));
        let new_path = provider_dir.join("provider.json");
        if old_path.exists() && !new_path.exists() {
            std::fs::copy(&old_path, &new_path)?;
        }

        provider_map.insert(provider.id.clone(), dir_name);
    }

    // 2. 扫描旧 projects/ 目录，读取 binding.json 确定供应商
    let old_projects_dir = mmycs_dir().join("projects");
    if old_projects_dir.exists() {
        for entry in std::fs::read_dir(&old_projects_dir)? {
            let entry = entry?;
            if !entry.path().is_dir() { continue; }
            let binding_path = entry.path().join("binding.json");
            if !binding_path.exists() { continue; }

            let content = std::fs::read_to_string(&binding_path)?;
            let binding: serde_json::Value = serde_json::from_str(&content)?;
            let provider_id = binding.get("provider_id").and_then(|v| v.as_str()).unwrap_or("");
            let provider_name = binding.get("provider_name").and_then(|v| v.as_str()).unwrap_or("");

            let target_dir_name = provider_map.get(provider_id)
                .cloned()
                .unwrap_or_else(|| provider_name_to_dir(provider_name));

            let project_name = entry.file_name();
            let target_dir = mmycs_dir().join("providers").join(&target_dir_name).join("projects").join(&project_name);
            if !target_dir.exists() {
                std::fs::rename(entry.path(), &target_dir)?;
            }
        }
    }

    // 3. 更新 config.json 中 active_projects 的 provider_dir 字段
    for project in &mut cfg.active_projects {
        if let Some(dir_name) = provider_map.get(&project.provider_id) {
            project.provider_dir = dir_name.clone();
        } else {
            project.provider_dir = provider_name_to_dir(&project.provider_name);
        }
        // 更新 config_dir 为新路径
        let norm = normalize_project_path(&project.project_path);
        let project_name = std::path::Path::new(&norm)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        project.config_dir = Some(
            mmycs_dir()
                .join("providers")
                .join(&project.provider_dir)
                .join("projects")
                .join(project_name)
                .to_string_lossy()
                .to_string()
        );
    }

    // 4. 设置版本号并保存
    cfg.version = 2;
    save_app_config(&cfg)?;

    // 5. 清理旧 providers 平铺文件
    let providers_dir = mmycs_dir().join("providers");
    for entry in std::fs::read_dir(&providers_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    // 6. 清理旧 projects 目录
    let _ = std::fs::remove_dir_all(old_projects_dir);

    Ok(true)
}
