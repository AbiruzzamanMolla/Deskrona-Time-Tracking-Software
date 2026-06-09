use active_win_pos_rs::get_active_window;
use chrono::{DateTime, Local, Utc};
use device_query::{DeviceQuery, DeviceState};
use rusqlite::Connection;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use regex::Regex;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::db::get_db_path;
use xcap::Monitor;

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

// Tracking states: 0 = stopped, 1 = running, 2 = paused
pub static TRACKING_STATE: AtomicU8 = AtomicU8::new(1); // 1: running, 2: paused, 0: stopped

pub static TRACKING_START_TIME: Mutex<Option<DateTime<Local>>> = Mutex::new(None);
pub static TRACKING_PAUSED_TIME: Mutex<Option<i64>> = Mutex::new(None);

pub fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}


pub fn set_tracking_start_time(time: Option<DateTime<Local>>) {
    if let Ok(mut guard) = TRACKING_START_TIME.lock() {
        *guard = time;
    }
}

pub fn add_paused_time(seconds: i64) {
    if let Ok(mut guard) = TRACKING_PAUSED_TIME.lock() {
        if let Some(current) = *guard {
            *guard = Some(current + seconds);
        } else {
            *guard = Some(seconds);
        }
    }
}

pub fn reset_paused_time() {
    if let Ok(mut guard) = TRACKING_PAUSED_TIME.lock() {
        *guard = None;
    }
}

pub static ACTIVE_USER_ID: Mutex<Option<String>> = Mutex::new(None);
pub static CURRENT_WINDOW: Mutex<Option<ActiveWindowInfo>> = Mutex::new(None);

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

pub fn get_current_window_duration() -> i64 {
    if get_tracking_status() != "running" {
        return 0;
    }
    if let Ok(guard) = CURRENT_WINDOW.lock() {
        if let Some(cw) = &*guard {
            return (Local::now().naive_local() - cw.start_time.naive_local()).num_seconds();
        }
    }
    0
}

pub fn get_tracking_status() -> &'static str {
    match TRACKING_STATE.load(Ordering::SeqCst) {
        0 => "stopped",
        1 => "running",
        2 => "paused",
        _ => "unknown",
    }
}

pub static TRACKING_PAUSE_START: Mutex<Option<DateTime<Local>>> = Mutex::new(None);

pub fn set_tracking_status(status: &str) {
    let val = match status {
        "stopped" => 0,
        "running" => 1,
        "paused" => 2,
        _ => return,
    };

    let current_status = get_tracking_status();

    if val == 1 && current_status != "running" {
        if current_status == "paused" {
            // Resuming: calculate how long we were paused
            if let Ok(mut pause_guard) = TRACKING_PAUSE_START.lock() {
                if let Some(pause_start) = pause_guard.take() {
                    let paused_for = (Local::now() - pause_start).num_seconds();
                    add_paused_time(paused_for);
                }
            }
        } else {
            // Starting fresh
            set_tracking_start_time(Some(Local::now()));
            reset_paused_time();
        }
    } else if val == 2 && current_status == "running" {
        // Pausing: record the start of the pause
        if let Ok(mut pause_guard) = TRACKING_PAUSE_START.lock() {
            *pause_guard = Some(Local::now());
        }
    } else if val == 0 {
        set_tracking_start_time(None);
        reset_paused_time();
        if let Ok(mut pause_guard) = TRACKING_PAUSE_START.lock() {
            *pause_guard = None;
        }
    }

    TRACKING_STATE.store(val, Ordering::SeqCst);
}

#[derive(Clone)]
pub struct ActiveWindowInfo {
    pub app_name: String,
    pub window_title: String,
    pub start_time: DateTime<Local>,
}

/// Known browser process names
/// Known browser process names
const BROWSERS: &[&str] = &[
    "chrome", "firefox", "msedge", "brave", "opera", "vivaldi", "safari",
    "chromium", "arc", "Google Chrome", "Mozilla Firefox", "Microsoft Edge",
    "Brave Browser", "Vivaldi", "Opera Browser", "Browser", "Waterfox", "Pale Moon",
];

fn extract_domain_from_title(title: &str) -> Option<String> {
    // Improved regex to better capture domains and subdomains
    let re = Regex::new(r"(?i)\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z0-9][a-z0-9-]{0,61}[a-z0-9]\b").ok()?;
    re.find(title).map(|m| m.as_str().to_string())
}

#[cfg(target_os = "windows")]
fn get_browser_url_windows(hwnd_str: &str) -> Option<String> {
    // active-win-pos-rs returns values like "HWND(9700584)" on Windows.
    // PowerShell's IntPtr cast needs the numeric handle only.
    let hwnd_numeric = hwnd_str
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    if hwnd_numeric.is_empty() {
        return None;
    }

    // Enhanced PowerShell script with more strategies and multi-language support
    let script = format!(r#"
        Add-Type -AssemblyName UIAutomationClient
        try {{
            $hwnd = [IntPtr]{}
            $el = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
            if (!$el) {{ return }}

            # Strategy 1: Find by Name "Address and search bar" (Localized names)
            $names = @(
                "Address and search bar", "Address bar", "URL bar", "Adresse- og søgefelt",
                "barra de direcciones", "Barre d'adresse", "Indirizzo e barra di ricerca",
                "Adres ve arama çubuğu", "アドレスと検索バー", "地址和搜索栏"
            )
            foreach ($n in $names) {{
                $nameCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $n)
                $target = $el.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $nameCond)
                if ($target) {{
                    try {{
                        $pattern = $target.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
                        $v = $pattern.Current.Value
                        if ($v -and ($v -like "http*" -or $v -like "*.*")) {{ $v; return }}
                    }} catch {{}}
                }}
            }}

            # Strategy 2: Search for ANY Edit control that contains a URL-like string
            $editCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)
            $edits = $el.FindAll([System.Windows.Automation.TreeScope]::Descendants, $editCond)
            foreach ($e in $edits) {{
                try {{
                    $v = ""
                    # Try ValuePattern first
                    try {{
                        $p = $e.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
                        $v = $p.Current.Value
                    }} catch {{}}

                    # Fallback to Name if Value is empty (some browsers use Name or HelpText)
                    if (!$v) {{ $v = $e.Current.Name }}

                    if ($v -and ($v -like "http*" -or ($v -like "*.*" -and -not $v.Contains(" ")))) {{
                        $v; return
                    }}
                }} catch {{}}
            }}
        }} catch {{}}
    "#, hwnd_numeric);

    let output = std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW - no console flash
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }
    None
}

fn extract_url_from_title(app_name: &str, title: &str, window_id: &str) -> Option<String> {
    let is_browser = BROWSERS
        .iter()
        .any(|b| app_name.to_lowercase().contains(&b.to_lowercase()) || title.to_lowercase().contains("browser"));

    if !is_browser {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(url) = get_browser_url_windows(window_id) {
            return Some(url);
        }
    }

    // Fallback 1: If title looks like a full URL
    if title.to_lowercase().starts_with("http") || (title.contains(".") && !title.contains(" ")) {
        return Some(title.to_string());
    }

    // Fallback 2: Extract domain from title
    if let Some(domain) = extract_domain_from_title(title) {
        return Some(domain);
    }

    // Fallback 3: Use the title as is but mark it
    let clean_title = title
        .replace(" - Google Chrome", "")
        .replace(" - Microsoft Edge", "")
        .replace(" - Brave", "")
        .replace(" - Mozilla Firefox", "")
        .replace(" - Opera", "")
        .replace(" - Vivaldi", "")
        .trim()
        .to_string();

    Some(clean_title)
}

pub fn start_tracking(app: AppHandle) {
    let idle_logged: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let auto_paused_by_idle: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    let idle_threshold_base = Duration::from_secs(crate::db::get_idle_threshold_secs(&app));

    // --- Screenshot Thread ---
    let app_clone = app.clone();
    let is_wayland = is_wayland();
    if is_wayland {
        eprintln!("Deskrona: Wayland detected, X11 screen capture unavailable. Screenshots disabled.");
    }
    std::thread::spawn(move || {
    let mut last_screenshot_time: Option<Instant> = None; // None = trigger immediately on first loop

        loop {
            std::thread::sleep(Duration::from_secs(5));
            if is_wayland {
                continue; // X11 capture unsupported on Wayland
            }
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
                if let Ok(settings) = crate::db::get_settings(&app_clone) {
                    if !settings.is_screenshot_enabled {
                        last_screenshot_time = Some(Instant::now()); // Reset timer anyway
                        continue;
                    }
                }
                last_screenshot_time = Some(Instant::now());

                let dir = crate::db::get_screenshot_dir(&app_clone);
                let now_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();

                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Ok(monitors) = Monitor::all() {
                        let mut min_x = i32::MAX;
                        let mut min_y = i32::MAX;
                        let mut max_x = i32::MIN;
                        let mut max_y = i32::MIN;

                        for monitor in &monitors {
                            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (monitor.x(), monitor.y(), monitor.width(), monitor.height()) {
                                let x = x;
                                let y = y;
                                let w = w as i32;
                                let h = h as i32;
                                if x < min_x { min_x = x; }
                                if y < min_y { min_y = y; }
                                if x + w > max_x { max_x = x + w; }
                                if y + h > max_y { max_y = y + h; }
                            }
                        }

                        let (width, height) = ((max_x - min_x) as u32, (max_y - min_y) as u32);
                        if width > 0 && height > 0 {
                            let mut combined_image = image::RgbaImage::new(width, height);
                            for monitor in monitors {
                                if let (Ok(x), Ok(y), Ok(img)) = (monitor.x(), monitor.y(), monitor.capture_image()) {
                                    let offset_x = (x - min_x) as i64;
                                    let offset_y = (y - min_y) as i64;
                                    image::imageops::overlay(&mut combined_image, &img, offset_x, offset_y);
                                }
                            }
                            let filename = format!("screenshot_{}.png", now_str);
                            let mut path = std::path::PathBuf::from(&dir);
                            path.push(&filename);
                            if let Ok(()) = combined_image.save(&path) {
                                let _ = crate::db::insert_screenshot(&app_clone, &get_active_user_id(), &path.to_string_lossy());
                            }
                        }
                    }
                }));
            }
        }
    });

    std::thread::spawn(move || {
        let db_path = get_db_path(&app);
        let device_state = DeviceState::new();
        let mut last_mouse_pos = device_state.get_mouse().coords;
        let mut last_keys: Vec<device_query::Keycode> = device_state.get_keys();
        let mut last_mouse_input_time = Instant::now();
        let mut last_keyboard_input_time = Instant::now();
        let mut idle_threshold = idle_threshold_base;
        let mut monitor_mouse = true;
        let mut monitor_keyboard = true;
        let mut threshold_reload_counter: u32 = 0;
        let mut window_flush_counter: u32 = 0;
        let mut active_window_failures: u32 = 0;
        if let Ok(s) = crate::db::get_settings(&app) {
            monitor_mouse = s.idle_monitor_mouse;
            monitor_keyboard = s.idle_monitor_keyboard;
        }

        loop {
            std::thread::sleep(Duration::from_secs(1));

            threshold_reload_counter += 1;
            window_flush_counter += 1;
            if threshold_reload_counter >= 60 {
                threshold_reload_counter = 0;
                idle_threshold = Duration::from_secs(crate::db::get_idle_threshold_secs(&app));
                if let Ok(s) = crate::db::get_settings(&app) {
                    monitor_mouse = s.idle_monitor_mouse;
                    monitor_keyboard = s.idle_monitor_keyboard;
                }
            }

            // Periodic flush to DB every 60s to persist time even without window changes
            if window_flush_counter >= 60 {
                window_flush_counter = 0;
                if let Ok(mut current) = CURRENT_WINDOW.lock() {
                    if let Some(cw) = current.take() {
                        let now = Local::now();
                        let duration = (now.naive_local() - cw.start_time.naive_local()).num_seconds();
                        if duration > 0 {
                            if let Ok(conn) = Connection::open(&db_path) {
                                let task_id = crate::db::get_active_task_id();
                                let _ = conn.execute(
                                    "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at, status, task_id)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                                        "active",
                                        task_id.as_deref(),
                                    ),
                                );
                            }
                        }
                        *current = Some(ActiveWindowInfo {
                            app_name: cw.app_name.clone(),
                            window_title: cw.window_title.clone(),
                            start_time: now,
                        });
                    }
                }
            }

            // Check tracking state
            let state = TRACKING_STATE.load(Ordering::SeqCst);
            if state == 0 || (state == 2 && !*auto_paused_by_idle.lock().unwrap()) {
                // Stopped or Paused: flush current window if any
                if let Ok(mut current) = CURRENT_WINDOW.lock() {
                    if let Some(cw) = current.take() {
                        let now = Local::now();
                        let duration = (now.naive_local() - cw.start_time.naive_local()).num_seconds();
                        if duration > 0 {
                            if let Ok(conn) = Connection::open(&db_path) {
                                let status = if state == 2 { "paused" } else { "active" };
                                let task_id = crate::db::get_active_task_id();
                                let _ = conn.execute(
                                    "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at, status, task_id)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                                        task_id.as_deref(),
                                    ),
                                );
                            }
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

            if mouse_moved {
                last_mouse_input_time = Instant::now();
            }
            if keys_changed {
                last_keyboard_input_time = Instant::now();
            }

            let monitored_activity = (monitor_mouse && mouse_moved) || (monitor_keyboard && keys_changed);

            if monitored_activity {
                let was_idle = {
                    let mut idle = idle_logged.lock().unwrap();
                    let was = *idle;
                    *idle = false;
                    was
                };
                if was_idle {
                    if let Ok(conn) = Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'idle_end', ?3, ?3, NULL, 'active')",
                            (Uuid::new_v4().to_string(), get_active_user_id(), Local::now().to_rfc3339()),
                        );
                    }
                }
                let mut auto_idle_pause = auto_paused_by_idle.lock().unwrap();
                if *auto_idle_pause && TRACKING_STATE.load(Ordering::SeqCst) == 2 {
                    set_tracking_status("running");
                    *auto_idle_pause = false;
                }
            }

            if mouse_moved || keys_changed {
                // Log keyboard/mouse input events separately
                if let Ok(conn) = Connection::open(&db_path) {
                    let uid = get_active_user_id();
                    if keys_changed {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'keyboard', ?3, ?3, NULL, 'active')",
                            (Uuid::new_v4().to_string(), &uid, Local::now().to_rfc3339()),
                        );
                    }
                    if mouse_moved {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'mouse', ?3, ?3, NULL, 'active')",
                            (Uuid::new_v4().to_string(), &uid, Local::now().to_rfc3339()),
                        );
                    }
                }
            }
            last_mouse_pos = current_mouse;
            last_keys = current_keys;

            let should_idle = if monitor_mouse && monitor_keyboard {
                last_mouse_input_time.elapsed() >= idle_threshold
                    && last_keyboard_input_time.elapsed() >= idle_threshold
            } else if monitor_mouse {
                last_mouse_input_time.elapsed() >= idle_threshold
            } else if monitor_keyboard {
                last_keyboard_input_time.elapsed() >= idle_threshold
            } else {
                false
            };

            if should_idle && TRACKING_STATE.load(Ordering::SeqCst) == 1 {
                let mut idle = idle_logged.lock().unwrap();
                if !*idle {
                    *idle = true;
                    if let Ok(conn) = Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'idle_start', ?3, ?3, NULL, 'idle')",
                            (Uuid::new_v4().to_string(), get_active_user_id(), Local::now().to_rfc3339()),
                        );
                    }
                    set_tracking_status("paused");
                    *auto_paused_by_idle.lock().unwrap() = true;
                }
                continue;
            }

            // --- Active Window Tracking ---
            let active_window_result = if state == 2 {
                Ok(active_win_pos_rs::ActiveWindow {
                    title: "Break Time".to_string(),
                    process_path: std::path::PathBuf::new(),
                    app_name: "Deskrona".to_string(),
                    window_id: String::new(),
                    process_id: 0,
                    position: active_win_pos_rs::WindowPosition { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                })
            } else {
                get_active_window()
            };

            match active_window_result {
                Ok(window) => {
                    // println!("Tracking Loop - Active: {} ({})", window.title, window.app_name);
                    if let Ok(mut current) = CURRENT_WINDOW.lock() {
                        let is_different = match &*current {
                            Some(cw) => {
                                cw.app_name != window.app_name || cw.window_title != window.title
                            }
                            None => true,
                        };

                        let now = Local::now();
                        if is_different {
                            if let Some(cw) = current.take() {
                                let duration = (now.naive_local() - cw.start_time.naive_local()).num_seconds();
                                if duration > 0 {
                                    if let Ok(conn) = Connection::open(&db_path) {
                                        let status = if cw.window_title == "Break Time" { "paused" } else { "active" };
                                        let task_id = crate::db::get_active_task_id();
                                        let _ = conn.execute(
                                            "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at, status, task_id)
                                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                                                task_id.as_deref(),
                                            ),
                                        );

                                    }
                                }
                            }

                            let new_info = ActiveWindowInfo {
                                app_name: window.app_name.clone(),
                                window_title: window.title.clone(),
                                start_time: now,
                            };

                            // Auto-detect task from app rules
                            if let Ok(settings) = crate::db::get_settings(&app) {
                                if settings.task_auto_detect_enabled {
                                    if let Some(matched_task_id) = crate::db::match_task_by_app(&app, &new_info.app_name, &new_info.window_title) {
                                        let current_task = crate::db::get_active_task_id();
                                        if current_task.as_deref() != Some(&matched_task_id) {
                                            crate::db::set_active_task_id(Some(matched_task_id.clone()));
                                            let _ = app.emit("active-task-changed", Some(matched_task_id));
                                        }
                                    }
                                }
                            }

                            if let Some(url_context) = extract_url_from_title(&new_info.app_name, &new_info.window_title, &window.window_id) {
                                if let Ok(conn) = Connection::open(&db_path) {
                                    let _ = conn.execute(
                                        "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, ?3, ?4, ?4, NULL, 'active')",
                                        (
                                            Uuid::new_v4().to_string(),
                                            get_active_user_id(),
                                            format!("url:{}", url_context),
                                            now.to_rfc3339(),
                                        ),
                                    );
                                }
                            }

                            *current = Some(new_info);
                        }
                    }
                }
                Err(_) => {
                    active_window_failures += 1;
                    // Create fallback window on repeated failures (Linux/Wayland)
                    if active_window_failures >= 3 {
                        if let Ok(mut current) = CURRENT_WINDOW.lock() {
                            if current.is_none() {
                                *current = Some(ActiveWindowInfo {
                                    app_name: "Desktop".to_string(),
                                    window_title: "Active".to_string(),
                                    start_time: Local::now(),
                                });
                                active_window_failures = 0;
                            }
                        }
                    }
                }
            }
        }
    });
}
