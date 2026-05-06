use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    SingleUser,
    MultiUser,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    /// true once the first-run wizard has been completed
    pub setup_done: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            mode: AppMode::SingleUser,
            setup_done: false,
        }
    }
}

fn config_path(app: &AppHandle) -> PathBuf {
    let mut p = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&p).ok();
    p.push("app_config.json");
    p
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let path = config_path(app);
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_path(app);
    let data = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

pub fn delete_config(app: &AppHandle) -> Result<(), String> {
    let path = config_path(app);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
