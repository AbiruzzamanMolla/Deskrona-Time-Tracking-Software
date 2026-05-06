use active_win_pos_rs::get_active_window;
use chrono::Utc;
use device_query::{DeviceQuery, DeviceState};
use rusqlite::Connection;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use uuid::Uuid;

use crate::db::get_db_path;
use screenshots::Screen;

// Tracking states: 0 = stopped, 1 = running, 2 = paused
pub static TRACKING_STATE: AtomicU8 = AtomicU8::new(1); // Start as running by default

pub static ACTIVE_USER_ID: Mutex<Option<String>> = Mutex::new(None);

pub fn set_active_user_id(user_id: Option<String>) {
    if let Ok(mut guard) = ACTIVE_USER_ID.lock() {
        *guard = user_id;
    }
}

pub fn get_active_user_id() -> String {
    if let Ok(guard) = ACTIVE_USER_ID.lock() {
        if let Some(id) = &*guard {
            return id.clone();
        }
    }
    "default_user".to_string()
}

pub fn get_tracking_status() -> &'static str {
    match TRACKING_STATE.load(Ordering::SeqCst) {
        0 => "stopped",
        1 => "running",
        2 => "paused",
        _ => "unknown",
    }
}

pub fn set_tracking_status(status: &str) {
    let val = match status {
        "stopped" => 0,
        "running" => 1,
        "paused" => 2,
        _ => return,
    };
    TRACKING_STATE.store(val, Ordering::SeqCst);
}

#[derive(Clone)]
struct ActiveWindowInfo {
    app_name: String,
    window_title: String,
    start_time: chrono::DateTime<Utc>,
}

/// Known browser process names
const BROWSERS: &[&str] = &[
    "chrome", "firefox", "msedge", "brave", "opera", "vivaldi", "safari",
    "chromium", "arc", "Google Chrome", "Mozilla Firefox", "Microsoft Edge",
];

fn extract_url_from_title(app_name: &str, title: &str) -> Option<String> {
    let is_browser = BROWSERS
        .iter()
        .any(|b| app_name.to_lowercase().contains(&b.to_lowercase()));
    if !is_browser {
        return None;
    }
    Some(title.to_string())
}

pub fn start_tracking(app: AppHandle) {
    let current_window: Arc<Mutex<Option<ActiveWindowInfo>>> = Arc::new(Mutex::new(None));
    let last_input_time: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    let idle_logged: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    let idle_threshold = Duration::from_secs(300);

    // --- Screenshot Thread ---
    let app_clone = app.clone();
    std::thread::spawn(move || {
    let mut last_screenshot_time: Option<Instant> = None; // None = trigger immediately on first loop

        loop {
            std::thread::sleep(Duration::from_secs(5));
            let state = TRACKING_STATE.load(Ordering::SeqCst);
            if state != 1 {
                continue; // Only take screenshots when running
            }

            let interval_secs = crate::db::get_screenshot_interval_secs(&app_clone);
            let should_capture = match last_screenshot_time {
                None => true,
                Some(t) => t.elapsed().as_secs() >= interval_secs,
            };
            if should_capture {
                last_screenshot_time = Some(Instant::now());
                
                let dir = crate::db::get_screenshot_dir(&app_clone);
                let now_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                
                if let Ok(screens) = Screen::all() {
                    for (i, screen) in screens.into_iter().enumerate() {
                        if let Ok(image) = screen.capture() {
                            let filename = format!("screenshot_{}_screen{}.png", now_str, i);
                            let mut path = std::path::PathBuf::from(&dir);
                            path.push(&filename);
                            
                            if let Ok(_) = image.save(&path) {
                                let _ = crate::db::insert_screenshot(&app_clone, &path.to_string_lossy());
                            }
                        }
                    }
                }
            }
        }
    });

    std::thread::spawn(move || {
        let db_path = get_db_path(&app);
        let device_state = DeviceState::new();
        let mut last_mouse_pos = device_state.get_mouse().coords;
        let mut last_keys: Vec<device_query::Keycode> = device_state.get_keys();

        loop {
            std::thread::sleep(Duration::from_secs(1));

            // Check tracking state
            let state = TRACKING_STATE.load(Ordering::SeqCst);
            if state == 0 || state == 2 {
                // Stopped or Paused: flush current window if any
                let mut current = current_window.lock().unwrap();
                if let Some(cw) = current.take() {
                    let now = Utc::now();
                    let duration = (now - cw.start_time).num_seconds();
                    if duration > 0 {
                        if let Ok(conn) = Connection::open(&db_path) {
                            let status = if state == 2 { "paused" } else { "active" };
                            let _ = conn.execute(
                                "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at, status)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                (
                                    Uuid::new_v4().to_string(),
                                    get_active_user_id(),
                                    &cw.app_name,
                                    &cw.window_title,
                                    &cw.start_time.to_rfc3339(),
                                    &now.to_rfc3339(),
                                    &duration,
                                    &now.to_rfc3339(),
                                    &now.to_rfc3339(),
                                    status,
                                ),
                            );
                        }
                    }
                }
                continue;
            }

            // --- Idle Detection ---
            let current_mouse = device_state.get_mouse().coords;
            let current_keys = device_state.get_keys();

            let mouse_moved = current_mouse != last_mouse_pos;
            let keys_changed = current_keys != last_keys;

            if mouse_moved || keys_changed {
                *last_input_time.lock().unwrap() = Instant::now();
                let was_idle = {
                    let mut idle = idle_logged.lock().unwrap();
                    let was = *idle;
                    *idle = false;
                    was
                };
                if was_idle {
                    if let Ok(conn) = Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, 'idle_end', ?2, ?2, NULL, 'active')",
                            (Uuid::new_v4().to_string(), Utc::now().to_rfc3339()),
                        );
                    }
                }
            }
            last_mouse_pos = current_mouse;
            last_keys = current_keys;

            let elapsed = last_input_time.lock().unwrap().elapsed();
            if elapsed >= idle_threshold {
                let mut idle = idle_logged.lock().unwrap();
                if !*idle {
                    *idle = true;
                    if let Ok(conn) = Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, 'idle_start', ?2, ?2, NULL, 'idle')",
                            (Uuid::new_v4().to_string(), Utc::now().to_rfc3339()),
                        );
                    }
                }
                continue;
            }

            // --- Active Window Tracking ---
            let active_window_result = if state == 2 {
                Ok(active_win_pos_rs::ActiveWindow {
                    title: "Break Time".to_string(),
                    process_path: std::path::PathBuf::new(),
                    app_name: "Time Guardian".to_string(),
                    window_id: String::new(),
                    process_id: 0,
                    position: active_win_pos_rs::WindowPosition { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                })
            } else {
                get_active_window()
            };

            match active_window_result {
                Ok(window) => {
                    let mut current = current_window.lock().unwrap();
                    let now = Utc::now();

                    let is_different = match &*current {
                        Some(cw) => {
                            cw.app_name != window.app_name || cw.window_title != window.title
                        }
                        None => true,
                    };

                    if is_different {
                        if let Some(cw) = current.take() {
                            let duration = (now - cw.start_time).num_seconds();
                            if duration > 0 {
                                if let Ok(conn) = Connection::open(&db_path) {
                                    let status = if cw.window_title == "Break Time" { "paused" } else { "active" };
                                    let _ = conn.execute(
                                        "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at, status)
                                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                        (
                                            Uuid::new_v4().to_string(),
                                            get_active_user_id(),
                                            &cw.app_name,
                                            &cw.window_title,
                                            &cw.start_time.to_rfc3339(),
                                            &now.to_rfc3339(),
                                            &duration,
                                            &now.to_rfc3339(),
                                            &now.to_rfc3339(),
                                            status,
                                        ),
                                    );

                                    if status == "active" {
                                        if let Some(url_context) =
                                            extract_url_from_title(&cw.app_name, &cw.window_title)
                                        {
                                            let _ = conn.execute(
                                                "INSERT INTO activity_events (id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, ?3, ?3, NULL, 'active')",
                                                (
                                                    Uuid::new_v4().to_string(),
                                                    format!("url:{}", url_context),
                                                    now.to_rfc3339(),
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        *current = Some(ActiveWindowInfo {
                            app_name: window.app_name,
                            window_title: window.title,
                            start_time: now,
                        });
                    }
                }
                Err(_) => {}
            }
        }
    });
}
