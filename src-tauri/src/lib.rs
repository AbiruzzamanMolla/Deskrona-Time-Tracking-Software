mod db;
mod tracking;
mod backup;
mod config;
mod auth;

use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use tauri_plugin_autostart::ManagerExt;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIcon},
    Emitter, Manager,
};

static TRAY_HANDLE: std::sync::OnceLock<tauri::tray::TrayIcon> = std::sync::OnceLock::new();
static UPDATE_TOOLTIP: AtomicBool = AtomicBool::new(true);

#[tauri::command]
fn show_overlay_window(app: tauri::AppHandle, x: i32, y: i32, always_on_top: bool, click_through: bool) -> Result<(), String> {
    println!("show_overlay_window called: x={}, y={}, always_on_top={}, click_through={}", x, y, always_on_top, click_through);
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
        let _ = window.set_always_on_top(always_on_top);
        let _ = window.set_ignore_cursor_events(click_through);
        let _ = window.show();
        let _ = window.set_focus();
        println!("Overlay window shown successfully");
        Ok(())
    } else {
        println!("Overlay window not found");
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn hide_overlay_window(app: tauri::AppHandle) -> Result<(), String> {
    println!("hide_overlay_window called");
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
        println!("Overlay window hidden successfully");
        Ok(())
    } else {
        println!("Overlay window not found for hiding");
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn update_overlay_time(app: tauri::AppHandle, time: String, status: String) -> Result<(), String> {
    println!("update_overlay_time called: time={}, status={}", time, status);
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.eval(&format!(
            "if(window.updateOverlayTime) window.updateOverlayTime('{}', '{}');",
            time, status
        ));
        println!("Overlay time updated successfully");
        Ok(())
    } else {
        println!("Overlay window not found for time update");
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn move_overlay_window(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
        Ok(())
    } else {
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn start_drag_overlay(app: tauri::AppHandle) -> Result<(), String> {
    println!("Start drag overlay");
    Ok(())
}

#[tauri::command]
fn end_drag_overlay(app: tauri::AppHandle) -> Result<(), String> {
    println!("End drag overlay");
    Ok(())
}



// ─── Existing Commands ────────────────────────────────────────────

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

#[derive(Serialize)]
pub struct TrayInfo {
    pub status: String,
    pub elapsed: String,
}

#[tauri::command]
fn cmd_get_tray_info(app_handle: tauri::AppHandle) -> Result<TrayInfo, String> {
    let status = tracking::get_tracking_status().to_string();
    let elapsed = if status == "running" {
        if let Ok(Some(session)) = db::get_active_session(&app_handle) {
            let start = chrono::DateTime::parse_from_rfc3339(&session.start_time)
                .map_err(|e| e.to_string())?;
            let now = chrono::Local::now();
            let duration = now.signed_duration_since(start);
            let secs = duration.num_seconds();
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            "00:00:00".to_string()
        }
    } else {
        "00:00:00".to_string()
    };
    Ok(TrayInfo { status, elapsed })
}

#[tauri::command]
fn cmd_get_time_logs_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::TimeLogEntry>, String> {
    let uid = tracking::get_active_user_id();
    db::get_time_logs_range(&app_handle, &uid, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_urls_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::UrlEntryFull>, String> {
    let uid = tracking::get_active_user_id();
    db::get_urls_range(&app_handle, &uid, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_screenshots_range(app_handle: tauri::AppHandle, from: String, to: String, limit: u32, offset: u32) -> Result<Vec<db::ScreenshotEntry>, String> {
    let uid = tracking::get_active_user_id();
    db::get_screenshots_range(&app_handle, &uid, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_screenshot_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    Ok(db::get_screenshot_dir(&app_handle))
}

#[tauri::command]
fn cmd_update_app_category(app_handle: tauri::AppHandle, app_name: String, category: String) -> Result<(), String> {
    db::update_app_category(&app_handle, &app_name, &category).map_err(|e| e.to_string())
}

// ─── Phase 8: App Config Commands ────────────────────────────────

/// Returns the persisted app config (mode + setup_done flag).
#[tauri::command]
fn cmd_get_app_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    config::load_config(&app_handle)
}

/// Persists the app config. Called when the wizard completes.
#[tauri::command]
fn cmd_save_app_config(app_handle: tauri::AppHandle, cfg: config::AppConfig) -> Result<(), String> {
    config::save_config(&app_handle, &cfg)
}

/// Reset app: delete config file + wipe auth tables. Frontend will re-run wizard.
#[tauri::command]
fn cmd_reset_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    // 1. Delete config
    config::delete_config(&app_handle)?;
    // 2. Reset auth schema (drops + recreates tables)
    auth::init_auth_schema(&app_handle).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Phase 8: Auth Commands ───────────────────────────────────────

/// First-time: register company + admin user.
#[tauri::command]
fn cmd_register_company(
    app_handle: tauri::AppHandle,
    payload: auth::RegisterCompanyPayload,
) -> Result<auth::LoginResult, String> {
    auth::register_company(&app_handle, payload)
}

/// Login with username + password.
#[tauri::command]
fn cmd_login(
    app_handle: tauri::AppHandle,
    payload: auth::LoginPayload,
) -> Result<auth::LoginResult, String> {
    auth::login(&app_handle, payload)
}

/// Validate session token, returns current user.
#[tauri::command]
fn cmd_validate_session(
    app_handle: tauri::AppHandle,
    token: String,
) -> Result<auth::AuthUser, String> {
    auth::validate_session(&app_handle, &token)
}

/// Logout / invalidate token.
#[tauri::command]
fn cmd_logout(app_handle: tauri::AppHandle, token: String) -> Result<(), String> {
    auth::logout(&app_handle, &token)
}

/// Admin: list all users in company.
#[tauri::command]
fn cmd_get_company_users(
    app_handle: tauri::AppHandle,
    company_id: String,
) -> Result<Vec<auth::AuthUser>, String> {
    auth::get_company_users(&app_handle, &company_id)
}

/// Admin: create a new user under this company.
#[tauri::command]
fn cmd_create_user(
    app_handle: tauri::AppHandle,
    company_id: String,
    payload: auth::CreateUserPayload,
) -> Result<auth::AuthUser, String> {
    auth::create_user(&app_handle, &company_id, payload)
}

/// Admin: aggregated productivity stats.
#[tauri::command]
fn cmd_get_admin_stats(
    app_handle: tauri::AppHandle,
    company_id: String,
) -> Result<Vec<auth::UserProductivityStat>, String> {
    auth::get_admin_stats(&app_handle, &company_id)
}

// ─── Admin: Per-User Data Commands ──────────────────────────────────

#[tauri::command]
fn cmd_get_user_screenshots(
    app_handle: tauri::AppHandle,
    user_id: String,
    from: String,
    to: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<db::AdminScreenshotEntry>, String> {
    db::get_user_screenshots(&app_handle, &user_id, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_user_time_logs(
    app_handle: tauri::AppHandle,
    user_id: String,
    from: String,
    to: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<db::AdminTimeLogEntry>, String> {
    db::get_user_time_logs(&app_handle, &user_id, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_user_activity(
    app_handle: tauri::AppHandle,
    user_id: String,
    from: String,
    to: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<db::AdminActivityEntry>, String> {
    db::get_user_activity(&app_handle, &user_id, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_user_urls(
    app_handle: tauri::AppHandle,
    user_id: String,
    from: String,
    to: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<db::UrlEntryFull>, String> {
    db::get_urls_range(&app_handle, &user_id, &from, &to, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_user_input_stats(
    app_handle: tauri::AppHandle,
    user_id: String,
    from: String,
    to: String,
) -> Result<db::UserInputStats, String> {
    db::get_user_input_stats(&app_handle, &user_id, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_all_app_categories(app_handle: tauri::AppHandle) -> Result<Vec<db::AppCategoryEntry>, String> {
    db::get_all_app_categories(&app_handle).map_err(|e| e.to_string())
}

// ─── Tauri Setup ──────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            db::init_db(app.handle()).expect("Failed to initialize database");

            // Initialize auth schema (idempotent)
            auth::init_auth_schema(app.handle()).expect("Failed to initialize auth schema");

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

            let app_handle = app.handle().clone();

            fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
                let status = tracking::get_tracking_status();
                let elapsed = if status == "running" {
                    if let Ok(Some(session)) = db::get_active_session(app) {
                        if let Ok(start) = chrono::DateTime::parse_from_rfc3339(&session.start_time) {
                            let now = chrono::Local::now();
                            let duration = now.signed_duration_since(start);
                            let secs = duration.num_seconds();
                            let hours = secs / 3600;
                            let mins = (secs % 3600) / 60;
                            let secs = secs % 60;
                            format!("{:02}:{:02}:{:02}", hours, mins, secs)
                        } else { "00:00:00".to_string() }
                    } else { "00:00:00".to_string() }
                } else { "00:00:00".to_string() };

                let status_text = format!("Status: {} ({})", status, elapsed);
                let status_item = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
                let start_i = MenuItem::with_id(app, "start", "Start Tracking", true, None::<&str>)?;
                let pause_i = MenuItem::with_id(app, "pause", "Pause Tracking", true, None::<&str>)?;
                let stop_i = MenuItem::with_id(app, "stop", "Stop Tracking", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

                Menu::with_items(app, &[&status_item, &start_i, &pause_i, &stop_i, &quit_i])
            }

            let menu = build_tray_menu(&app_handle)?;

            // Hide to tray instead of closing when user clicks X
            let main_window = app.get_webview_window("main").unwrap();
            let window_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            let status_tooltip = format!("Deskrona - Status: {}", tracking::get_tracking_status());
            println!("Creating tray icon with default icon...");

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(&status_tooltip)
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
            println!("Tray icon created successfully!");

            let _ = TRAY_HANDLE.set(tray);

            // Background thread to update tray tooltip with elapsed time
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let status = tracking::get_tracking_status();
                    if status == "running" || status == "paused" {
                        let time = tracking::get_tracking_formatted_time();
                        let tooltip = if status == "running" {
                            format!("Deskrona - Running: {}", time)
                        } else {
                            format!("Deskrona - Paused: {}", time)
                        };
                        if let Some(tray) = TRAY_HANDLE.get() {
                            let _ = tray.set_tooltip(Some(&tooltip));
                        }
                    }
                }
            });

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
            cmd_get_tray_info,
            cmd_get_time_logs_range,
            cmd_get_urls_range,
            cmd_get_screenshots_range,
            cmd_get_screenshot_dir,
            cmd_update_app_category,
            backup::cmd_export_db,
            backup::cmd_import_db,
            // Phase 8
            cmd_get_app_config,
            cmd_save_app_config,
            cmd_reset_app,
            cmd_register_company,
            cmd_login,
            cmd_validate_session,
            cmd_logout,
            cmd_get_company_users,
            cmd_create_user,
            cmd_get_admin_stats,
            // Admin per-user
            cmd_get_user_screenshots,
            cmd_get_user_time_logs,
            cmd_get_user_activity,
            cmd_get_user_input_stats,
            cmd_get_user_urls,
cmd_get_all_app_categories,
            show_overlay_window,
            hide_overlay_window,
            update_overlay_time,
            move_overlay_window,
            start_drag_overlay,
            end_drag_overlay,
        ])
        .setup(|app| {
            // Check if overlay window exists and create hidden if needed
            let overlay = app.get_webview_window("overlay");
            if overlay.is_none() {
                if let Ok(window) = tauri::WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    tauri::WebviewUrl::App("overlay.html".into()),
                )
                .title("Deskrona Timer")
                .inner_size(200.0, 60.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .visible(false)
                .skip_taskbar(true)
                .resizable(false)
                .build()
                {
                    println!("Overlay window created successfully");
                    let _ = window.hide();
                } else {
                    println!("Failed to create overlay window");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
