use active_win_pos_rs::get_active_window;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;
use uuid::Uuid;

use crate::db::get_db_path;

#[derive(Clone)]
struct ActiveWindowInfo {
    app_name: String,
    window_title: String,
    start_time: chrono::DateTime<Utc>,
}

pub fn start_tracking(app: AppHandle) {
    let current_window: Arc<Mutex<Option<ActiveWindowInfo>>> = Arc::new(Mutex::new(None));

    std::thread::spawn(move || {
        let db_path = get_db_path(&app);

        loop {
            std::thread::sleep(Duration::from_secs(1));

            match get_active_window() {
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
                        // If there was a previous window, save it to DB
                        if let Some(cw) = current.take() {
                            let duration = (now - cw.start_time).num_seconds();
                            if duration > 0 {
                                // Save to DB
                                if let Ok(conn) = Connection::open(&db_path) {
                                    let id = Uuid::new_v4().to_string();
                                    let user_id = "default_user"; // For MVP

                                    let _ = conn.execute(
                                        "INSERT INTO time_logs (id, user_id, app_name, window_title, start_time, end_time, duration, created_at, updated_at)
                                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                        (
                                            &id,
                                            &user_id,
                                            &cw.app_name,
                                            &cw.window_title,
                                            &cw.start_time.to_rfc3339(),
                                            &now.to_rfc3339(),
                                            &duration,
                                            &now.to_rfc3339(),
                                            &now.to_rfc3339(),
                                        ),
                                    );
                                }
                            }
                        }

                        // Set new active window
                        *current = Some(ActiveWindowInfo {
                            app_name: window.app_name,
                            window_title: window.title,
                            start_time: now,
                        });
                    }
                }
                Err(_) => {
                    // Could not get active window, maybe locked or sleeping
                }
            }
        }
    });
}
