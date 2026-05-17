use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpointConfig {
    pub enabled: bool,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfigFile {
    pub mode: String,
    #[serde(default)]
    pub bearer_token: String,
    pub endpoints: HashMap<String, ApiEndpointConfig>,
}

impl Default for ApiConfigFile {
    fn default() -> Self {
        let mut endpoints = HashMap::new();
        let default_methods = [
            ("auth_register", "POST"),
            ("auth_login", "POST"),
            ("auth_validate", "GET"),
            ("auth_logout", "POST"),
            ("session_start", "POST"),
            ("session_stop", "POST"),
            ("session_active", "GET"),
            ("tracking_status", "POST"),
            ("time_logs_sync", "POST"),
            ("time_logs_get", "GET"),
            ("screenshots_upload", "POST"),
            ("screenshots_get", "GET"),
            ("urls_sync", "POST"),
            ("urls_get", "GET"),
            ("activity_sync", "POST"),
            ("activity_get", "GET"),
            ("input_stats_get", "GET"),
            ("dashboard_today", "GET"),
            ("dashboard_range", "GET"),
            ("admin_users_list", "GET"),
            ("admin_users_create", "POST"),
            ("admin_stats", "GET"),
            ("admin_user_screenshots", "GET"),
            ("admin_user_time_logs", "GET"),
            ("admin_user_activity", "GET"),
            ("admin_user_urls", "GET"),
            ("admin_user_input_stats", "GET"),
            ("app_categories_get", "GET"),
            ("app_categories_update", "PUT"),
            ("config_get", "GET"),
            ("config_save", "POST"),
            ("settings_get", "GET"),
            ("settings_update", "POST"),
            ("backup_export", "GET"),
            ("backup_import", "POST"),
            ("update_check", "GET"),
            ("pomodoro_start", "POST"),
            ("pomodoro_skip", "POST"),
            ("pomodoro_stop", "POST"),
            ("pomodoro_status", "GET"),
            ("autostart_set", "POST"),
            ("autostart_get", "GET"),
            ("reset_app", "POST"),
        ];
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        for (name, method) in &default_methods {
            endpoints.insert(
                name.to_string(),
                ApiEndpointConfig {
                    enabled: false,
                    method: method.to_string(),
                    url: String::new(),
                    headers: headers.clone(),
                },
            );
        }
        ApiConfigFile {
            mode: "offline".to_string(),
            bearer_token: String::new(),
            endpoints,
        }
    }
}

pub fn get_config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Failed to get app data dir")
}

pub fn get_config_path(app: &AppHandle) -> PathBuf {
    get_config_dir(app).join("api-config.json")
}

pub fn load(app: &AppHandle) -> Result<ApiConfigFile, String> {
    let path = get_config_path(app);
    if !path.exists() {
        return Ok(ApiConfigFile::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read api-config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse api-config: {}", e))
}

pub fn save(app: &AppHandle, config: &ApiConfigFile) -> Result<(), String> {
    let dir = get_config_dir(app);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    let path = get_config_path(app);
    let content = serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize api-config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write api-config: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn cmd_get_api_config(app_handle: tauri::AppHandle) -> Result<ApiConfigFile, String> {
    load(&app_handle)
}

#[tauri::command]
pub fn cmd_save_api_config(app_handle: tauri::AppHandle, config: ApiConfigFile) -> Result<(), String> {
    save(&app_handle, &config)
}
