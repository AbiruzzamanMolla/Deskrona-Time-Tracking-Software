use crate::db::{get_db_path, get_settings};
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::AppHandle;

pub fn start_backup_scheduler(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_backup_check = Utc::now() - chrono::Duration::hours(24);
        loop {
            std::thread::sleep(Duration::from_secs(3600)); // Check every hour
            
            if let Ok(settings) = get_settings(&app) {
                if settings.backup_frequency == "never" || settings.backup_location.is_empty() {
                    continue;
                }
                
                let should_backup = match settings.backup_frequency.as_str() {
                    "daily" => (Utc::now() - last_backup_check).num_hours() >= 24,
                    "weekly" => (Utc::now() - last_backup_check).num_days() >= 7,
                    _ => false,
                };
                
                if should_backup {
                    last_backup_check = Utc::now();
                    let db_path = get_db_path(&app);
                    let backup_dir = Path::new(&settings.backup_location);
                    if !backup_dir.exists() {
                        let _ = fs::create_dir_all(&backup_dir);
                    }
                    let date_str = Utc::now().format("%Y%m%d").to_string();
                    let backup_file = backup_dir.join(format!("time_guardian_backup_{}.db", date_str));
                    let _ = fs::copy(&db_path, backup_file);
                }
            }
        }
    });
}

#[tauri::command]
pub fn cmd_export_db(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    fs::copy(&db_path, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cmd_import_db(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    fs::copy(&path, &db_path).map_err(|e| e.to_string())?;
    Ok(())
}
