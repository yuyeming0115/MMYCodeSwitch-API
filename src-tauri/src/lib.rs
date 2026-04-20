mod config;
mod crypto;
mod inject;

use config::{AppConfig, Provider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            init_app,
            get_app_config,
            save_app_config,
            get_providers,
            upsert_provider,
            delete_provider,
            switch_provider,
            parse_paste,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
