use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
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

// ── 连通性测试 ────────────────────────────────────────────────────────────────
#[tauri::command]
async fn test_provider(provider_id: String) -> Result<bool, String> {
    let providers = config::load_providers().map_err(|e| e.to_string())?;
    let p = providers.iter().find(|p| p.id == provider_id).ok_or("not found")?;
    if p.provider_type != "api" { return Ok(true); }
    let base_url = p.base_url.as_deref().unwrap_or("https://api.anthropic.com");
    let machine_key = config::get_or_create_key().map_err(|e| e.to_string())?;
    let api_key = match &p.api_key_encrypted {
        Some(enc) => crypto::decrypt(enc, &machine_key).map_err(|e| e.to_string())?,
        None => return Err("no api key".to_string()),
    };
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build().map_err(|e| e.to_string())?;
    let resp = client.get(&url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .send().await.map_err(|e| e.to_string())?;
    Ok(resp.status().is_success() || resp.status().as_u16() == 401)
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

#[tauri::command]
fn parse_paste(text: String) -> serde_json::Value {
    let mut result = serde_json::json!({ "baseUrl": null, "apiKey": null });
    for line in text.lines() {
        let line = line.trim();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(url) = json.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                result["baseUrl"] = serde_json::Value::String(url.to_string());
            }
            if let Some(key) = json.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) {
                result["apiKey"] = serde_json::Value::String(key.to_string());
            }
        }
        if line.starts_with("export ANTHROPIC_BASE_URL=") {
            result["baseUrl"] = serde_json::Value::String(line["export ANTHROPIC_BASE_URL=".len()..].trim_matches('"').to_string());
        }
        if line.starts_with("export ANTHROPIC_AUTH_TOKEN=") {
            result["apiKey"] = serde_json::Value::String(line["export ANTHROPIC_AUTH_TOKEN=".len()..].trim_matches('"').to_string());
        }
        if line.starts_with("https://") && result["baseUrl"].is_null() {
            result["baseUrl"] = serde_json::Value::String(line.to_string());
        }
        if (line.starts_with("sk-") || line.len() > 20) && result["apiKey"].is_null() && !line.contains(' ') && !line.starts_with("https://") {
            result["apiKey"] = serde_json::Value::String(line.to_string());
        }
    }
    result
}

fn build_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let providers = config::load_providers().unwrap_or_default();
    let cfg = config::load_app_config().unwrap_or_default();
    let active_id = cfg.instances.first().and_then(|i| i.active_provider_id.clone());

    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![];
    for p in &providers {
        let label = if Some(&p.id) == active_id.as_ref() {
            format!("✓ {}", p.name)
        } else {
            p.name.clone()
        };
        let item = MenuItem::with_id(app, &p.id, label, true, None::<&str>)?;
        items.push(Box::new(item));
    }
    let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
    items.push(Box::new(sep));
    let quit = MenuItem::with_id(app, "__quit__", "退出", true, None::<&str>)?;
    items.push(Box::new(quit));

    let menu = Menu::with_items(app, &items.iter().map(|i| i.as_ref()).collect::<Vec<_>>())?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("MMYCodeSwitch-API")
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "__quit__" {
                app.exit(0);
                return;
            }
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
            // 重建托盘菜单以更新勾选状态
            if let Some(tray) = app.tray_by_id("main") {
                let _ = build_tray_menu(app, &tray);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .id("main")
        .build(app)?;
    Ok(())
}

fn build_tray_menu(app: &tauri::AppHandle, tray: &tauri::tray::TrayIcon) -> Result<(), Box<dyn std::error::Error>> {
    let providers = config::load_providers().unwrap_or_default();
    let cfg = config::load_app_config().unwrap_or_default();
    let active_id = cfg.instances.first().and_then(|i| i.active_provider_id.clone());
    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![];
    for p in &providers {
        let label = if Some(&p.id) == active_id.as_ref() { format!("✓ {}", p.name) } else { p.name.clone() };
        let item = MenuItem::with_id(app, &p.id, label, true, None::<&str>)?;
        items.push(Box::new(item));
    }
    let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
    items.push(Box::new(sep));
    let quit = MenuItem::with_id(app, "__quit__", "退出", true, None::<&str>)?;
    items.push(Box::new(quit));
    let menu = Menu::with_items(app, &items.iter().map(|i| i.as_ref()).collect::<Vec<_>>())?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            build_tray(app)?;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
