mod db;
mod tracking;
mod backup;

use tauri_plugin_autostart::ManagerExt;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_settings(app_handle: tauri::AppHandle) -> Result<db::Settings, String> {
    db::get_settings(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_settings(app_handle: tauri::AppHandle, settings: db::Settings) -> Result<(), String> {
    db::update_settings(&app_handle, settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app_handle: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app_handle.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_autostart(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let manager = app_handle.autolaunch();
    manager.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_start_session(app_handle: tauri::AppHandle) -> Result<db::Session, String> {
    db::start_session(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_stop_session(app_handle: tauri::AppHandle, session_id: String) -> Result<db::Session, String> {
    db::stop_session(&app_handle, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_active_session(app_handle: tauri::AppHandle) -> Result<Option<db::Session>, String> {
    db::get_active_session(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_dashboard_data(app_handle: tauri::AppHandle) -> Result<db::DashboardData, String> {
    db::get_dashboard_data(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_set_tracking(status: String) -> Result<(), String> {
    tracking::set_tracking_status(&status);
    Ok(())
}

#[tauri::command]
fn cmd_get_tracking() -> Result<String, String> {
    Ok(tracking::get_tracking_status().to_string())
}

#[tauri::command]
fn cmd_get_time_logs_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::TimeLogEntry>, String> {
    db::get_time_logs_range(&app_handle, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_urls_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::UrlEntryFull>, String> {
    db::get_urls_range(&app_handle, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_screenshots_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::ScreenshotEntry>, String> {
    db::get_screenshots_range(&app_handle, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_screenshot_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    Ok(db::get_screenshot_dir(&app_handle))
}

#[tauri::command]
fn cmd_update_app_category(app_handle: tauri::AppHandle, app_name: String, category: String) -> Result<(), String> {
    db::update_app_category(&app_handle, &app_name, &category).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            db::init_db(app.handle()).expect("Failed to initialize database");

            // Sync autostart state from settings
            if let Ok(settings) = db::get_settings(app.handle()) {
                let manager = app.handle().autolaunch();
                if settings.auto_start_on_boot {
                    let _ = manager.enable();
                } else {
                    let _ = manager.disable();
                }
            }

            // Start background tracking
            tracking::start_tracking(app.handle().clone());

            // Start backup scheduler
            backup::start_backup_scheduler(app.handle().clone());

            let start_i = MenuItem::with_id(app, "start", "Start Tracking", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause", "Pause Tracking", true, None::<&str>)?;
            let stop_i = MenuItem::with_id(app, "stop", "Stop Tracking", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&start_i, &pause_i, &stop_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "start" => {
                        crate::tracking::set_tracking_status("running");
                        let _ = app.emit("tracking-status-changed", "running");
                    }
                    "pause" => {
                        crate::tracking::set_tracking_status("paused");
                        let _ = app.emit("tracking-status-changed", "paused");
                    }
                    "stop" => {
                        crate::tracking::set_tracking_status("stopped");
                        let _ = app.emit("tracking-status-changed", "stopped");
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_settings,
            update_settings,
            set_autostart,
            get_autostart,
            cmd_start_session,
            cmd_stop_session,
            cmd_get_active_session,
            cmd_get_dashboard_data,
            cmd_set_tracking,
            cmd_get_tracking,
            cmd_get_time_logs_range,
            cmd_get_urls_range,
            cmd_get_screenshots_range,
            cmd_get_screenshot_dir,
            cmd_update_app_category,
            backup::cmd_export_db,
            backup::cmd_import_db
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
