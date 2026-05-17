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
                    let _ = fs::copy(&db_path, &backup_file);
                    // Also backup api-config.json
                    let api_config_path = crate::api_config::get_config_path(&app);
                    if api_config_path.exists() {
                        let api_backup_file = backup_dir.join(format!("api_config_{}.json", date_str));
                        let _ = fs::copy(&api_config_path, api_backup_file);
                    }
                }
            }
        }
    });
}

#[tauri::command]
pub fn cmd_export_db(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let screenshot_dir = crate::db::get_screenshot_dir(&app_handle);
    
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
        
    // Add DB
    zip.start_file("tracker.db", options).map_err(|e| e.to_string())?;
    let mut db_file = fs::File::open(&db_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut db_file, &mut zip).map_err(|e| e.to_string())?;
    
    // Add screenshots
    if Path::new(&screenshot_dir).exists() {
        zip.add_directory("screenshots", options).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(&screenshot_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(name) = entry_path.file_name() {
                    let zip_path = format!("screenshots/{}", name.to_string_lossy());
                    zip.start_file(zip_path, options).map_err(|e| e.to_string())?;
                    let mut f = fs::File::open(&entry_path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    
    // Add api-config.json
    let api_config_path = crate::api_config::get_config_path(&app_handle);
    if api_config_path.exists() {
        zip.start_file("api-config.json", options).map_err(|e| e.to_string())?;
        let mut config_file = fs::File::open(&api_config_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut config_file, &mut zip).map_err(|e| e.to_string())?;
    }
    
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cmd_import_db(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let screenshot_dir = crate::db::get_screenshot_dir(&app_handle);
    
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match file.enclosed_name() {
            Some(p) => p.to_owned(),
            None => continue,
        };
        
        if file.name().ends_with('/') {
            continue;
        }
        
        if file.name() == "tracker.db" {
            let mut out_file = fs::File::create(&db_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
        } else if file.name().starts_with("screenshots/") {
            let filename = outpath.file_name().unwrap();
            let dest_path = Path::new(&screenshot_dir).join(filename);
            if !Path::new(&screenshot_dir).exists() {
                fs::create_dir_all(&screenshot_dir).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
        } else if file.name() == "api-config.json" {
            let api_config_path = crate::api_config::get_config_path(&app_handle);
            if let Some(parent) = api_config_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&api_config_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
        }
    }
    
    Ok(())
}
