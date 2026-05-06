use active_win_pos_rs::get_active_window;
use chrono::{DateTime, Local, Utc};
use device_query::{DeviceQuery, DeviceState};
use rusqlite::Connection;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use uuid::Uuid;
use regex::Regex;

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
    start_time: DateTime<Local>,
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
    "#, hwnd_str);

    let output = std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
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
    let current_window: Arc<Mutex<Option<ActiveWindowInfo>>> = Arc::new(Mutex::new(None));
    let last_input_time: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    let idle_logged: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    let idle_threshold_base = Duration::from_secs(crate::db::get_idle_threshold_secs(&app));

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
                if let Ok(settings) = crate::db::get_settings(&app_clone) {
                    if !settings.is_screenshot_enabled {
                        last_screenshot_time = Some(Instant::now()); // Reset timer anyway
                        continue;
                    }
                }
                last_screenshot_time = Some(Instant::now());
                
                let dir = crate::db::get_screenshot_dir(&app_clone);
                let now_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                
                if let Ok(screens) = Screen::all() {
                    let mut min_x = 0;
                    let mut min_y = 0;
                    let mut max_x = 0;
                    let mut max_y = 0;
                    for screen in &screens {
                        let info = screen.display_info;
                        if info.x < min_x { min_x = info.x; }
                        if info.y < min_y { min_y = info.y; }
                        if info.x + info.width as i32 > max_x { max_x = info.x + info.width as i32; }
                        if info.y + info.height as i32 > max_y { max_y = info.y + info.height as i32; }
                    }
                    
                    let width = (max_x - min_x) as u32;
                    let height = (max_y - min_y) as u32;
                    if width > 0 && height > 0 {
                        let mut combined_image = image::RgbaImage::new(width, height);
                        for screen in screens {
                            if let Ok(img) = screen.capture() {
                                let info = screen.display_info;
                                let offset_x = (info.x - min_x) as i64;
                                let offset_y = (info.y - min_y) as i64;
                                
                                let w = img.width();
                                let h = img.height();
                                let raw = img.into_raw();
                                if let Some(converted_img) = image::RgbaImage::from_raw(w, h, raw) {
                                    image::imageops::overlay(&mut combined_image, &converted_img, offset_x, offset_y);
                                }
                            }
                        }
                        let filename = format!("screenshot_{}.png", now_str);
                        let mut path = std::path::PathBuf::from(&dir);
                        path.push(&filename);
                        if let Ok(_) = combined_image.save(&path) {
                            let _ = crate::db::insert_screenshot(&app_clone, &get_active_user_id(), &path.to_string_lossy());
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
        // Reload threshold every N seconds to respect settings changes
        let mut idle_threshold = idle_threshold_base;
        let mut threshold_reload_counter: u32 = 0;

        loop {
            std::thread::sleep(Duration::from_secs(1));

            // Reload idle threshold from settings every 60 seconds
            threshold_reload_counter += 1;
            if threshold_reload_counter >= 60 {
                threshold_reload_counter = 0;
                idle_threshold = Duration::from_secs(crate::db::get_idle_threshold_secs(&app));
            }

            // Check tracking state
            let state = TRACKING_STATE.load(Ordering::SeqCst);
            if state == 0 || state == 2 {
                // Stopped or Paused: flush current window if any
                let mut current = current_window.lock().unwrap();
                if let Some(cw) = current.take() {
                    let now = Local::now();
                    let duration = (now.naive_local() - cw.start_time.naive_local()).num_seconds();
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
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'idle_end', ?3, ?3, NULL, 'active')",
                            (Uuid::new_v4().to_string(), get_active_user_id(), Local::now().to_rfc3339()),
                        );
                    }
                }
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

            let elapsed = last_input_time.lock().unwrap().elapsed();
            if elapsed >= idle_threshold {
                let mut idle = idle_logged.lock().unwrap();
                if !*idle {
                    *idle = true;
                    if let Ok(conn) = Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT INTO activity_events (id, user_id, type, timestamp, created_at, synced_at, activity_status) VALUES (?1, ?2, 'idle_start', ?3, ?3, NULL, 'idle')",
                            (Uuid::new_v4().to_string(), get_active_user_id(), Local::now().to_rfc3339()),
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

                        let new_info = ActiveWindowInfo {
                            app_name: window.app_name.clone(),
                            window_title: window.title.clone(),
                            start_time: now,
                        };

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
                Err(_) => {}
            }
        }
    });
}
