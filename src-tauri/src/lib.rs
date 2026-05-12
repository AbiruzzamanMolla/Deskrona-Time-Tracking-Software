mod db;
mod tracking;
mod backup;
mod config;
mod auth;

use tauri_plugin_autostart::ManagerExt;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use device_query::{DeviceQuery, DeviceState};

static OVERLAY_POS: std::sync::Mutex<Option<tauri::PhysicalPosition<i32>>> = std::sync::Mutex::new(None);

static TRAY_HANDLE: std::sync::OnceLock<tauri::tray::TrayIcon> = std::sync::OnceLock::new();

#[tauri::command]
fn show_overlay_window(app: tauri::AppHandle, x: i32, y: i32, always_on_top: bool, click_through: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
        let _ = window.set_always_on_top(always_on_top);
        let _ = window.set_ignore_cursor_events(click_through);
        let _ = window.show();
        let _ = window.set_focus();
        if let Ok(mut pos) = OVERLAY_POS.lock() {
            *pos = Some(tauri::PhysicalPosition { x, y });
        }
        Ok(())
    } else {
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn hide_overlay_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
        Ok(())
    } else {
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn update_overlay_time(app: tauri::AppHandle, time: String, status: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.eval(&format!(
            "if(window.updateOverlayTime) window.updateOverlayTime('{}', '{}');",
            time, status
        ));
        Ok(())
    } else {
        Err("Overlay window not found".to_string())
    }
}

#[tauri::command]
fn start_drag_overlay(window: tauri::WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
fn move_overlay_window(window: tauri::WebviewWindow, x: i32, y: i32) -> Result<(), String> {
    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    if let Ok(mut pos) = OVERLAY_POS.lock() {
        *pos = Some(tauri::PhysicalPosition { x, y });
    }
    Ok(())
}

#[tauri::command]
fn end_drag_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    if let Ok(pos) = window.outer_position() {
        let _ = db::update_settings_overlay_position(&app, pos.x, pos.y);
    }
    Ok(())
}

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
fn cmd_set_tracking(app_handle: tauri::AppHandle, status: String) -> Result<(), String> {
    tracking::set_tracking_status(&status);
    let _ = app_handle.emit("tracking-status-changed", &status);
    Ok(())
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let status = tracking::get_tracking_status();
    let total_seconds = db::get_today_active_seconds(app).unwrap_or(0);
    let time_str = tracking::format_duration(total_seconds);

    let status_text = format!("Status: {} ({})", status, time_str);
    let status_item = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
    let dashboard_i = MenuItem::with_id(app, "dashboard", "Show Dashboard", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let start_i = MenuItem::with_id(app, "start", "Start Tracking", true, None::<&str>)?;
    let pause_i = MenuItem::with_id(app, "pause", "Pause Tracking", true, None::<&str>)?;
    let stop_i = MenuItem::with_id(app, "stop", "Stop Tracking", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(app, &[&status_item, &dashboard_i, &sep1, &start_i, &pause_i, &stop_i, &sep2, &quit_i])
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
    let total_seconds = db::get_today_active_seconds(&app_handle).unwrap_or(0);
    let elapsed = tracking::format_duration(total_seconds);
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

#[tauri::command]
fn cmd_get_app_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    config::load_config(&app_handle)
}

#[tauri::command]
fn cmd_save_app_config(app_handle: tauri::AppHandle, cfg: config::AppConfig) -> Result<(), String> {
    config::save_config(&app_handle, &cfg)
}

#[tauri::command]
fn cmd_reset_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    config::delete_config(&app_handle)?;
    auth::init_auth_schema(&app_handle).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_register_company(
    app_handle: tauri::AppHandle,
    payload: auth::RegisterCompanyPayload,
) -> Result<auth::LoginResult, String> {
    auth::register_company(&app_handle, payload)
}

#[tauri::command]
fn cmd_login(
    app_handle: tauri::AppHandle,
    payload: auth::LoginPayload,
) -> Result<auth::LoginResult, String> {
    auth::login(&app_handle, payload)
}

#[tauri::command]
fn cmd_validate_session(
    app_handle: tauri::AppHandle,
    token: String,
) -> Result<auth::AuthUser, String> {
    auth::validate_session(&app_handle, &token)
}

#[tauri::command]
fn cmd_logout(app_handle: tauri::AppHandle, token: String) -> Result<(), String> {
    auth::logout(&app_handle, &token)
}

#[tauri::command]
fn cmd_get_company_users(
    app_handle: tauri::AppHandle,
    company_id: String,
) -> Result<Vec<auth::AuthUser>, String> {
    auth::get_company_users(&app_handle, &company_id)
}

#[tauri::command]
fn cmd_create_user(
    app_handle: tauri::AppHandle,
    company_id: String,
    payload: auth::CreateUserPayload,
) -> Result<auth::AuthUser, String> {
    auth::create_user(&app_handle, &company_id, payload)
}

#[tauri::command]
fn cmd_get_admin_stats(
    app_handle: tauri::AppHandle,
    company_id: String,
) -> Result<Vec<auth::UserProductivityStat>, String> {
    auth::get_admin_stats(&app_handle, &company_id)
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--flag"])))
        .setup(|app| {
            db::init_db(app.handle()).expect("Failed to initialize database");
            auth::init_auth_schema(app.handle()).expect("Failed to initialize auth schema");

            if let Ok(settings) = db::get_settings(app.handle()) {
                let manager = app.handle().autolaunch();
                if settings.auto_start_on_boot {
                    let _ = manager.enable();
                } else {
                    let _ = manager.disable();
                }
            }

            tracking::start_tracking(app.handle().clone());
            backup::start_backup_scheduler(app.handle().clone());

            let menu = build_tray_menu(app.handle())?;

            let main_window = app.get_webview_window("main").unwrap();
            let window_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            let status_tooltip = format!("Deskrona - Status: {}", tracking::get_tracking_status());
            
            let mut tray_builder = TrayIconBuilder::new()
                .tooltip(&status_tooltip)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "dashboard" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
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
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let tray = tray_builder.build(app)?;
            let _ = TRAY_HANDLE.set(tray);

            let app_handle_bg = app.handle().clone();
            // Dedicated cursor polling thread for overlay click-through
            let app_handle_cursor = app.handle().clone();
            std::thread::spawn(move || {
                let device_state = DeviceState::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if let Some(window) = app_handle_cursor.get_webview_window("overlay") {
                        if let Ok(settings) = db::get_settings(&app_handle_cursor) {
                            if settings.overlay_enabled && settings.overlay_click_through {
                                let status = tracking::get_tracking_status();
                                if status != "stopped" {
                                    if let Ok(pos_guard) = OVERLAY_POS.lock() {
                                        if let Some(pos) = *pos_guard {
                                            let mouse = device_state.get_mouse();
                                            let wx = pos.x;
                                            let wy = pos.y;
                                            // Overlay is 220w x 48h
                                            // Interactive zones: left drag strip (0-14px) + right button (220-40 to 220px)
                                            let drag_left = wx;
                                            let drag_right = wx + 14;
                                            let btn_left = wx + 220 - 40;
                                            let btn_right = wx + 220;
                                            let top = wy;
                                            let bottom = wy + 48;
                                            let on_drag = mouse.coords.0 >= drag_left && mouse.coords.0 <= drag_right
                                                && mouse.coords.1 >= top && mouse.coords.1 <= bottom;
                                            let on_btn = mouse.coords.0 >= btn_left && mouse.coords.0 <= btn_right
                                                && mouse.coords.1 >= top && mouse.coords.1 <= bottom;
                                            let over_interactive = on_drag || on_btn;
                                            let _ = window.set_ignore_cursor_events(!over_interactive);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });

            std::thread::spawn(move || {
                let mut last_shown = false;
                let mut last_status = String::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let status = tracking::get_tracking_status();
                    let total_seconds = db::get_today_active_seconds(&app_handle_bg).unwrap_or(0);
                    let time_str = tracking::format_duration(total_seconds);

                    let tooltip = match status {
                        "running" => format!("Deskrona - Running Today: {}", time_str),
                        "paused" => format!("Deskrona - Paused (Today: {})", time_str),
                        _ => format!("Deskrona - Stopped (Today: {})", time_str),
                    };
                    
                    if let Some(tray) = TRAY_HANDLE.get() {
                        let _ = tray.set_tooltip(Some(&tooltip));
                        // Only rebuild menu when status changes to avoid console flash
                        if status != last_status {
                            if let Ok(new_menu) = build_tray_menu(&app_handle_bg) {
                                let _ = tray.set_menu(Some(new_menu));
                            }
                            last_status = status.to_string();
                        }
                    }

                    if let Ok(settings) = db::get_settings(&app_handle_bg) {
                        let should_show_overlay = settings.overlay_enabled && status != "stopped";
                        if should_show_overlay {
                            if let Some(window) = app_handle_bg.get_webview_window("overlay") {
                                let _ = window.set_always_on_top(settings.overlay_always_on_top);

                                // Click-through is handled by the dedicated cursor thread
                                // but ensure non-click-through mode still works
                                if !settings.overlay_click_through {
                                    let _ = window.set_ignore_cursor_events(false);
                                }
                                
                                #[derive(Clone, Serialize)]
                                struct OverlayUpdate {
                                    time: String,
                                    status: String,
                                }
                                let _ = window.emit("update-overlay", OverlayUpdate {
                                    time: time_str,
                                    status: status.to_string(),
                                });

                                if !last_shown {
                                    let _ = window.show();
                                    last_shown = true;
                                    // Store position for cursor thread
                                    if let Ok(mut pos_guard) = OVERLAY_POS.lock() {
                                        *pos_guard = Some(tauri::PhysicalPosition {
                                            x: settings.overlay_position_x,
                                            y: settings.overlay_position_y,
                                        });
                                    }
                                }
                            } else {
                                // Create overlay window
                                if let Ok(ref win) = tauri::WebviewWindowBuilder::new(
                                    &app_handle_bg,
                                    "overlay",
                                    tauri::WebviewUrl::App("overlay.html".into())
                                )
                                .title("Deskrona Overlay")
                                .decorations(false)
                                .transparent(true)
                                .always_on_top(settings.overlay_always_on_top)
                                .resizable(false)
                                .skip_taskbar(true)
                                .inner_size(220.0, 48.0)
                                .position(settings.overlay_position_x as f64, settings.overlay_position_y as f64)
                                .build()
                                {
                                    win.on_window_event(move |event| {
                                        if let tauri::WindowEvent::Moved(position) = event {
                                            if let Ok(mut pos_guard) = OVERLAY_POS.lock() {
                                                *pos_guard = Some(*position);
                                            }
                                        }
                                    });
                                }
                                // Store position for cursor thread
                                if let Ok(mut pos_guard) = OVERLAY_POS.lock() {
                                    *pos_guard = Some(tauri::PhysicalPosition {
                                        x: settings.overlay_position_x,
                                        y: settings.overlay_position_y,
                                    });
                                }
                                last_shown = true;
                            }
                        } else {
                            if let Some(window) = app_handle_bg.get_webview_window("overlay") {
                                let is_visible = window.is_visible().unwrap_or(false);
                                if is_visible {
                                    let _ = window.hide();
                                }
                            }
                            last_shown = false;
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
            start_drag_overlay,
            move_overlay_window,
            end_drag_overlay,
            cmd_get_tray_info,
            cmd_get_time_logs_range,
            cmd_get_urls_range,
            cmd_get_screenshots_range,
            cmd_get_screenshot_dir,
            cmd_update_app_category,
            backup::cmd_export_db,
            backup::cmd_import_db,
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
            cmd_get_user_screenshots,
            cmd_get_user_time_logs,
            cmd_get_user_activity,
            cmd_get_user_input_stats,
            cmd_get_user_urls,
            cmd_get_all_app_categories,
            show_overlay_window,
            hide_overlay_window,
            update_overlay_time,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
