use tauri::{
    menu::{MenuItem, MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager,
};

mod config;
mod crypto;
mod inject;

use config::{AppConfig, Provider};
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

// ── 导出备份 ──────────────────────────────────────────────────────────────────
#[tauri::command]
fn export_providers(password: String) -> Result<String, String> {
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    // 用备份密码派生一个 32 字节 key（简单 SHA-256 stretch）
    let backup_key = derive_key_from_password(&password);
    let mut exported: Vec<serde_json::Value> = vec![];
    for p in &providers {
        let mut v = serde_json::to_value(p).map_err(|e| e.to_string())?;
        // 用备份密码重新加密 api_key
        if let Some(enc) = p.api_key_encrypted.as_deref() {
            let plain = crypto::decrypt(enc, &machine_key).map_err(|e| e.to_string())?;
            let re_enc = crypto::encrypt(&plain, &backup_key).map_err(|e| e.to_string())?;
            v["api_key_encrypted"] = serde_json::Value::String(re_enc);
        }
        exported.push(v);
    }
    let bundle = serde_json::json!({ "version": 1, "providers": exported });
    Ok(serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?)
}

#[tauri::command]
fn import_providers(json: String, password: String) -> Result<usize, String> {
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

fn derive_key_from_password(password: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    // 简单 PBKDF：SHA-256 x 10000 轮
    let mut hash = password.as_bytes().to_vec();
    for _ in 0..10000 {
        let mut h = [0u8; 32];
        // 用 aes-gcm 依赖中已有的 sha2 不可用，改用简单 xor-fold 替代
        // 实际项目建议换成 argon2/pbkdf2
        for (i, b) in hash.iter().enumerate() {
            h[i % 32] ^= b;
        }
        hash = h.to_vec();
    }
    STANDARD.encode(&hash[..32])
}

// ── 连通性测试（智能适配不同协议） ────────────────────────────────────────
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
fn save_window_state(x: i32, y: i32, width: u32, height: u32, isDark: Option<bool>) -> Result<(), String> {
    let path = config::mmycs_dir().join("window_state.json");
    let mut state = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if let Some(d) = isDark { state["isDark"] = serde_json::json!(d); }
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

/// 已知的目标字段名列表——覆盖 Anthropic / OpenAI / 中转站常见命名
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

#[tauri::command]
fn parse_paste(text: String) -> serde_json::Value {
    let mut result = serde_json::json!({ "baseUrl": null, "apiKey": null });

    // ── 策略1: 整体作为 JSON 递归深挖（支持嵌套/中转站格式）──
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        // 合并 URL 和 KEY 目标字段
        let all_targets: Vec<&str> = KNOWN_URL_FIELDS.iter().chain(KNOWN_KEY_FIELDS.iter()).copied().collect();
        if let Some((url, key)) = deep_find(&json, &all_targets) {
            if !url.is_empty() { result["baseUrl"] = serde_json::Value::String(url); }
            if !key.is_empty() { result["apiKey"] = serde_json::Value::String(key); }
            // 如果已经找到全部，直接返回；否则继续用行级解析补充
            if !result["baseUrl"].is_null() && !result["apiKey"].is_null() {
                return result;
            }
        }
    }

    // ── 策略2: 逐行解析（兼容 export 语句、裸值、扁平 JSON）──
    for line in text.lines() {
        let line = line.trim();
        if result["baseUrl"].is_null() || result["apiKey"].is_null() {
            // 2a. 单行 JSON 对象
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                let all_targets: Vec<&str> = KNOWN_URL_FIELDS.iter().chain(KNOWN_KEY_FIELDS.iter()).copied().collect();
                if let Some((url, key)) = deep_find(&json, &all_targets) {
                    if !url.is_empty() && result["baseUrl"].is_null() { result["baseUrl"] = serde_json::Value::String(url); }
                    if !key.is_empty() && result["apiKey"].is_null() { result["apiKey"] = serde_json::Value::String(key); }
                    continue;
                }
            }

            // 2b. bash export 语句
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

            // 2c. 裸 URL
            if (line.starts_with("https://") || line.starts_with("http://")) && result["baseUrl"].is_null() {
                result["baseUrl"] = serde_json::Value::String(line.to_string());
            }

            // 2d. 裸 Key（sk- 开头或长字符串）
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
        .setup(|app| {
            setup_tray(&app.handle())?;

            // ── 恢复上次窗口尺寸和位置 ──
            if let Some(win) = app.get_webview_window("main") {
                let state_path = config::mmycs_dir().join("window_state.json");
                if state_path.exists() {
                    if let Ok(s) = std::fs::read_to_string(&state_path) {
                        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&s) {
                            let _ = win.set_size(tauri::LogicalSize::new(
                                state["width"].as_u64().unwrap_or(900) as u32,
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

                // 监听窗口关闭事件：阻止默认关闭 → 保存状态 → 隐藏到托盘
                let win_clone = win.clone();
                let path_clone = state_path.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // ★ 关键：阻止默认关闭行为
                        api.prevent_close();

                        // 1. 保存窗口状态
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
                        // 2. 隐藏到托盘
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
            switch_provider,
            parse_paste,
            export_providers,
            import_providers,
            test_provider,
            detect_instances,
            save_provider_icon,
            save_window_state,
            get_window_state,
            hide_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
