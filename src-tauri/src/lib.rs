mod db;
mod tracking;
mod backup;
mod config;
mod auth;
mod api_config;
mod break_reminder;

use tauri_plugin_autostart::ManagerExt;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use device_query::{DeviceQuery, DeviceState};

static OVERLAY_POS: std::sync::Mutex<Option<tauri::PhysicalPosition<i32>>> = std::sync::Mutex::new(None);

static TRAY_HANDLE: std::sync::OnceLock<tauri::tray::TrayIcon> = std::sync::OnceLock::new();

// Pomodoro state (in-memory, no DB)
#[derive(Clone, Serialize)]
struct PomodoroState {
    phase: String,       // idle, focus, short_break, long_break
    remaining_secs: i32,
    count_today: i32,
    total_today: i32,    // focus count today
}

static POMODORO_PHASE: std::sync::Mutex<&'static str> = std::sync::Mutex::new("idle");
static POMODORO_REMAINING: std::sync::Mutex<i32> = std::sync::Mutex::new(0);
static POMODORO_FOCUS_TODAY: std::sync::Mutex<i32> = std::sync::Mutex::new(0);

fn get_pomodoro_state() -> PomodoroState {
    let phase = POMODORO_PHASE.lock().unwrap();
    let remaining = *POMODORO_REMAINING.lock().unwrap();
    let count = *POMODORO_FOCUS_TODAY.lock().unwrap();
    PomodoroState { phase: phase.to_string(), remaining_secs: remaining, count_today: count, total_today: count }
}

fn reset_pomodoro() {
    *POMODORO_PHASE.lock().unwrap() = "idle";
    *POMODORO_REMAINING.lock().unwrap() = 0;
}

fn set_pomodoro_phase(phase: &'static str, remaining: i32) {
    *POMODORO_PHASE.lock().unwrap() = phase;
    *POMODORO_REMAINING.lock().unwrap() = remaining;
}

#[tauri::command]
fn cmd_pomodoro_start(app: tauri::AppHandle) -> Result<PomodoroState, String> {
    let settings = db::get_settings(&app).map_err(|e| e.to_string())?;
    set_pomodoro_phase("focus", settings.pomodoro_focus_minutes * 60);
    Ok(get_pomodoro_state())
}

#[tauri::command]
fn cmd_pomodoro_skip(app: tauri::AppHandle) -> Result<PomodoroState, String> {
    let current = POMODORO_PHASE.lock().unwrap().clone();
    if current == "idle" {
        return Ok(get_pomodoro_state());
    }
    let settings = db::get_settings(&app).map_err(|e| e.to_string())?;
    if current == "focus" || current == "short_break" || current == "long_break" {
        set_pomodoro_phase("focus", settings.pomodoro_focus_minutes * 60);
    }
    Ok(get_pomodoro_state())
}

#[tauri::command]
fn cmd_pomodoro_stop() -> Result<PomodoroState, String> {
    reset_pomodoro();
    Ok(get_pomodoro_state())
}

#[tauri::command]
fn cmd_pomodoro_status() -> Result<PomodoroState, String> {
    Ok(get_pomodoro_state())
}

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
fn cmd_get_filtered_dashboard_data(app_handle: tauri::AppHandle, from: String, to: String) -> Result<db::DashboardData, String> {
    db::get_filtered_dashboard_data(&app_handle, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_calendar_month(app_handle: tauri::AppHandle, from: String, to: String) -> Result<Vec<db::CalendarDayEntry>, String> {
    db::get_calendar_month(&app_handle, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_set_tracking(app_handle: tauri::AppHandle, status: String) -> Result<(), String> {
    tracking::set_tracking_status(&status);
    
    // Auto start/stop active task in the database based on tracking status updates from anywhere (frontend or tray menu)
    if status == "running" {
        if let Some(task_id) = db::get_active_task_id() {
            // Failsafe: trigger event to keep frontend UI fully synchronized
            let _ = app_handle.emit("active-task-changed", Some(task_id));
        }
    } else if status == "stopped" {
        // Halt active task logging in DB but DO NOT broadcast active-task-changed event,
        // so that the frontend's local activeTaskId/activeTaskName is preserved for resuming.
        let _ = db::set_active_task_id(None);
    }
    
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
    // Break reminder menu items in a submenu
    let break_status = break_reminder::get_break_state(app);
    let break_text = format!("Break Cycle ({})", break_status.state);
    
    let break_pause_30 = MenuItem::with_id(app, "break_pause_30m", "Pause Breaks 30 min", true, None::<&str>)?;
    let break_pause_1h = MenuItem::with_id(app, "break_pause_1h", "Pause Breaks 1 hour", true, None::<&str>)?;
    let break_pause_2h = MenuItem::with_id(app, "break_pause_2h", "Pause Breaks 2 hours", true, None::<&str>)?;
    let break_pause_5h = MenuItem::with_id(app, "break_pause_5h", "Pause Breaks 5 hours", true, None::<&str>)?;
    let break_resume = MenuItem::with_id(app, "break_resume", "Resume Breaks", true, None::<&str>)?;
    let break_reset = MenuItem::with_id(app, "break_reset", "Reset Break Cycle", true, None::<&str>)?;

    let break_submenu = Submenu::with_items(
        app,
        &break_text,
        true,
        &[&break_pause_30, &break_pause_1h, &break_pause_2h, &break_pause_5h, &break_resume, &break_reset],
    )?;
    
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(app, &[
        &status_item, &dashboard_i, &sep1,
        &start_i, &pause_i, &stop_i, &sep2,
        &break_submenu, &sep3,
        &quit_i,
    ])
}

#[tauri::command]
fn cmd_get_tracking() -> Result<String, String> {
    Ok(tracking::get_tracking_status().to_string())
}

#[derive(Serialize)]
pub struct TrayInfo {
    pub status: String,
    pub elapsed: String,
    pub active_task_name: Option<String>,
}

#[tauri::command]
fn cmd_get_tray_info(app_handle: tauri::AppHandle) -> Result<TrayInfo, String> {
    let status = tracking::get_tracking_status().to_string();
    let total_seconds = db::get_today_active_seconds(&app_handle).unwrap_or(0);
    let elapsed = tracking::format_duration(total_seconds);
    let active_task_name = if let Some(task_id) = db::get_active_task_id() {
        let tasks = db::list_tasks(&app_handle, None).unwrap_or_default();
        tasks.into_iter().find(|t| t.id == task_id).map(|t| t.name)
    } else {
        None
    };
    Ok(TrayInfo { status, elapsed, active_task_name })
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

// ─── Projects & Tasks Commands ───────────────────────────────────

#[tauri::command]
fn cmd_create_project(app_handle: tauri::AppHandle, name: String, color: String) -> Result<db::Project, String> {
    db::create_project(&app_handle, &name, &color).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_list_projects(app_handle: tauri::AppHandle) -> Result<Vec<db::Project>, String> {
    db::list_projects(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_update_project(app_handle: tauri::AppHandle, id: String, name: Option<String>, color: Option<String>, archived: Option<bool>) -> Result<(), String> {
    db::update_project(&app_handle, &id, name.as_deref(), color.as_deref(), archived).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_project(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    db::delete_project(&app_handle, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_create_task(app_handle: tauri::AppHandle, project_id: String, name: String) -> Result<db::Task, String> {
    db::create_task(&app_handle, &project_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_list_tasks(app_handle: tauri::AppHandle, project_id: Option<String>) -> Result<Vec<db::Task>, String> {
    db::list_tasks(&app_handle, project_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_update_task(app_handle: tauri::AppHandle, id: String, name: Option<String>, status: Option<String>) -> Result<(), String> {
    db::update_task(&app_handle, &id, name.as_deref(), status.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_task(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    db::delete_task(&app_handle, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_set_active_task(app_handle: tauri::AppHandle, task_id: Option<String>) -> Result<(), String> {
    db::set_active_task_id(task_id.clone());
    let _ = app_handle.emit("active-task-changed", task_id);
    Ok(())
}

#[tauri::command]
fn cmd_get_active_task(app_handle: tauri::AppHandle) -> Result<Option<db::Task>, String> {
    if let Some(task_id) = db::get_active_task_id() {
        let tasks = db::list_tasks(&app_handle, None).map_err(|e| e.to_string())?;
        Ok(tasks.into_iter().find(|t| t.id == task_id))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn cmd_create_task_rule(app_handle: tauri::AppHandle, task_id: String, app_name: String, window_pattern: Option<String>) -> Result<db::TaskRule, String> {
    db::create_task_rule(&app_handle, &task_id, &app_name, window_pattern.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_list_task_rules(app_handle: tauri::AppHandle, task_id: Option<String>) -> Result<Vec<db::TaskRule>, String> {
    db::list_task_rules(&app_handle, task_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_task_rule(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    db::delete_task_rule(&app_handle, &id).map_err(|e| e.to_string())
}

// ─── Reports & Timesheet Commands ────────────────────────────────

#[tauri::command]
fn cmd_get_project_time_summary(app_handle: tauri::AppHandle, from: String, to: String) -> Result<Vec<db::ProjectTimeSummary>, String> {
    db::get_project_time_summary(&app_handle, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_task_time_summary(app_handle: tauri::AppHandle, project_id: Option<String>, from: String, to: String) -> Result<Vec<db::TaskTimeSummary>, String> {
    db::get_task_time_summary(&app_handle, project_id.as_deref(), &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_timesheet_data(app_handle: tauri::AppHandle, from: String, to: String) -> Result<Vec<db::TimesheetCell>, String> {
    db::get_timesheet_data(&app_handle, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_daily_task_summary(app_handle: tauri::AppHandle, date: String) -> Result<Vec<db::DailyTaskEntry>, String> {
    db::get_daily_task_summary(&app_handle, &date).map_err(|e| e.to_string())
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
            break_reminder::start_break_reminder(app.handle().clone());

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
                    "break_pause_30m" => {
                        let _ = crate::break_reminder::pause_break_reminder(app, 30);
                    }
                    "break_pause_1h" => {
                        let _ = crate::break_reminder::pause_break_reminder(app, 60);
                    }
                    "break_pause_2h" => {
                        let _ = crate::break_reminder::pause_break_reminder(app, 120);
                    }
                    "break_pause_5h" => {
                        let _ = crate::break_reminder::pause_break_reminder(app, 300);
                    }
                    "break_resume" => {
                        let _ = crate::break_reminder::resume_break_reminder(app);
                    }
                    "break_reset" => {
                        let _ = crate::break_reminder::reset_break_cycle(app);
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
                                let url = if cfg!(dev) {
                                     match &app_handle_bg.config().build.dev_url {
                                         Some(dev_url) => {
                                             let target_url = dev_url.join("overlay.html").unwrap();
                                             tauri::WebviewUrl::External(target_url)
                                         }
                                         None => tauri::WebviewUrl::App("overlay.html".into()),
                                     }
                                 } else {
                                     tauri::WebviewUrl::App("overlay.html".into())
                                 };

                                if let Ok(ref win) = tauri::WebviewWindowBuilder::new(
                                    &app_handle_bg,
                                    "overlay",
                                    url
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
                                    let win_clone = win.clone();
                                    win.on_window_event(move |event| {
                                        match event {
                                            tauri::WindowEvent::CloseRequested { api, .. } => {
                                                api.prevent_close();
                                                let _ = win_clone.hide();
                                            }
                                            tauri::WindowEvent::Moved(position) => {
                                                if let Ok(mut pos_guard) = OVERLAY_POS.lock() {
                                                    *pos_guard = Some(*position);
                                                }
                                            }
                                            _ => {}
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

                    // Break overlay window management
                    {
                        let break_st = crate::break_reminder::get_break_state(&app_handle_bg);
                        let should_show_break = break_st.state == "on_break";
                        let break_settings = db::get_settings(&app_handle_bg).ok();
                        let _should_show_overlay = break_settings.as_ref().map(|s| s.overlay_enabled).unwrap_or(false) && status != "stopped";
                        
                        if should_show_break {
                            if let Ok(monitors) = app_handle_bg.available_monitors() {
                                for (i, monitor) in monitors.iter().enumerate() {
                                    let win_label = format!("break_overlay_{}", i);
                                    if let Some(window) = app_handle_bg.get_webview_window(&win_label) {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    } else {
                                        let fullscreen = break_settings.as_ref().map(|s| s.break_fullscreen).unwrap_or(true);
                                        
                                        // Get monitor geometry in logical pixels to handle DPI scaling correctly
                                        let pos = monitor.position();
                                        let size = monitor.size();
                                        let scale = monitor.scale_factor();
                                        let logical_x = pos.x as f64 / scale;
                                        let logical_y = pos.y as f64 / scale;
                                        let logical_w = size.width as f64 / scale;
                                        let logical_h = size.height as f64 / scale;

                                        let url = if cfg!(dev) {
                                             match &app_handle_bg.config().build.dev_url {
                                                 Some(dev_url) => {
                                                     let target_url = dev_url.join("break_overlay.html").unwrap();
                                                     tauri::WebviewUrl::External(target_url)
                                                 }
                                                 None => tauri::WebviewUrl::App("break_overlay.html".into()),
                                             }
                                         } else {
                                             tauri::WebviewUrl::App("break_overlay.html".into())
                                         };

                                        let mut builder = tauri::WebviewWindowBuilder::new(
                                            &app_handle_bg,
                                            &win_label,
                                            url,
                                        )
                                        .title("Break Time - Deskrona")
                                        .decorations(false)
                                        .always_on_top(true)
                                        .skip_taskbar(true);
                                        
                                        if fullscreen {
                                            builder = builder
                                                .position(logical_x, logical_y)
                                                .inner_size(logical_w, logical_h)
                                                .maximized(true);
                                        } else {
                                            let w = 800.0;
                                            let h = 600.0;
                                            let cx = logical_x + (logical_w - w) / 2.0;
                                            let cy = logical_y + (logical_h - h) / 2.0;
                                            builder = builder
                                                .position(cx, cy)
                                                .inner_size(w, h)
                                                .resizable(true);
                                        }
                                        
                                        if let Ok(win) = builder.build() {
                                            let _ = win.show();
                                            let _ = win.set_focus();
                                        }
                                    }
                                }
                            }
                        } else {
                            // Hide all break overlay windows if break ended
                            for (label, window) in app_handle_bg.webview_windows() {
                                if label.starts_with("break_overlay_") {
                                    let is_visible = window.is_visible().unwrap_or(false);
                                    if is_visible {
                                        let _ = window.hide();
                                    }
                                }
                            }
                        }
                    }

                    // Pomodoro tick
                    {
                        let phase = *POMODORO_PHASE.lock().unwrap();
                        if phase != "idle" {
                            let remaining = {
                                let mut r = POMODORO_REMAINING.lock().unwrap();
                                if *r > 0 { *r -= 1; }
                                *r
                            };
                            if remaining <= 0 {
                                let settings = db::get_settings(&app_handle_bg).ok();
                                match phase {
                                    "focus" => {
                                        let count = {
                                            let mut c = POMODORO_FOCUS_TODAY.lock().unwrap();
                                            *c += 1;
                                            *c
                                        };
                                        let max = settings.as_ref().map(|s| s.pomodoro_sessions_before_long).unwrap_or(4);
                                        if count >= max {
                                            let secs = settings.as_ref().map(|s| s.pomodoro_long_break_minutes * 60).unwrap_or(900);
                                            set_pomodoro_phase("long_break", secs);
                                        } else {
                                            let secs = settings.as_ref().map(|s| s.pomodoro_short_break_minutes * 60).unwrap_or(300);
                                            set_pomodoro_phase("short_break", secs);
                                        }
                                    },
                                    _ => {
                                        let secs = settings.as_ref().map(|s| s.pomodoro_focus_minutes * 60).unwrap_or(1500);
                                        set_pomodoro_phase("focus", secs);
                                    },
                                };
                                let _ = app_handle_bg.emit("pomodoro-phase-changed", get_pomodoro_state());
                            }
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
            cmd_get_filtered_dashboard_data,
            cmd_set_tracking,
            cmd_get_tracking,
            start_drag_overlay,
            move_overlay_window,
            end_drag_overlay,
            cmd_get_tray_info,
            cmd_get_time_logs_range,
            cmd_get_urls_range,
            cmd_get_screenshots_range,
            cmd_get_calendar_month,
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
            cmd_pomodoro_start,
            cmd_pomodoro_skip,
            cmd_pomodoro_stop,
            cmd_pomodoro_status,
            api_config::cmd_get_api_config,
            api_config::cmd_save_api_config,
            break_reminder::cmd_break_status,
            break_reminder::cmd_break_test_preview,
            break_reminder::cmd_break_postpone,
            break_reminder::cmd_break_skip,
            break_reminder::cmd_break_pause,
            break_reminder::cmd_break_resume,
            break_reminder::cmd_break_reset,
            cmd_create_project,
            cmd_list_projects,
            cmd_update_project,
            cmd_delete_project,
            cmd_create_task,
            cmd_list_tasks,
            cmd_update_task,
            cmd_delete_task,
            cmd_set_active_task,
            cmd_get_active_task,
            cmd_create_task_rule,
            cmd_list_task_rules,
            cmd_delete_task_rule,
            cmd_get_project_time_summary,
            cmd_get_task_time_summary,
            cmd_get_timesheet_data,
            cmd_get_daily_task_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
