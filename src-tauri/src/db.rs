use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

pub fn get_db_path(app: &AppHandle) -> PathBuf {
    let mut path = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&path).expect("Failed to create app data dir");
    path.push("app.db");
    path
}

pub fn init_db(app: &AppHandle) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path)?;

    // Create tables
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            language TEXT NOT NULL DEFAULT 'en',
            theme TEXT NOT NULL DEFAULT 'system',
            auto_start_on_boot BOOLEAN NOT NULL DEFAULT 0,
            screenshot_interval INTEGER NOT NULL DEFAULT 10,
            screenshot_location TEXT NOT NULL DEFAULT '',
            backup_frequency TEXT NOT NULL DEFAULT 'never',
            backup_location TEXT NOT NULL DEFAULT '',
            idle_threshold INTEGER NOT NULL DEFAULT 5,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );


        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS time_logs (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            app_name TEXT NOT NULL,
            window_title TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            duration INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS app_usage (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            app_name TEXT NOT NULL,
            total_time INTEGER NOT NULL,
            date TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS screenshots (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            captured_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS activity_events (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT 'default_user',
            type TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            created_at TEXT NOT NULL,
            synced_at TEXT,
            activity_status TEXT NOT NULL DEFAULT 'active'
        );

        CREATE TABLE IF NOT EXISTS app_categories (
            app_name TEXT PRIMARY KEY,
            category TEXT NOT NULL DEFAULT 'neutral'
        );
        ",
    )?;

    // Migrations for existing databases
    let _ = conn.execute("ALTER TABLE time_logs ADD COLUMN status TEXT NOT NULL DEFAULT 'active'", []);
    let _ = conn.execute("ALTER TABLE activity_events ADD COLUMN activity_status TEXT NOT NULL DEFAULT 'active'", []);
    let _ = conn.execute("ALTER TABLE activity_events ADD COLUMN user_id TEXT NOT NULL DEFAULT 'default_user'", []);
    let _ = conn.execute("CREATE TABLE IF NOT EXISTS app_categories (app_name TEXT PRIMARY KEY, category TEXT NOT NULL DEFAULT 'neutral')", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN idle_threshold INTEGER NOT NULL DEFAULT 5", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN is_screenshot_enabled INTEGER NOT NULL DEFAULT 1", []);

    // Insert default settings if none exists
    conn.execute(
        "INSERT INTO settings (id, user_id, language, theme, auto_start_on_boot, screenshot_interval, screenshot_location, backup_frequency, backup_location, idle_threshold, is_screenshot_enabled, created_at, updated_at)
         SELECT ?1, ?2, 'en', 'system', 0, 10, '', 'never', '', 5, 1, ?3, ?3
         WHERE NOT EXISTS (SELECT 1 FROM settings WHERE user_id = ?2)",
        (
            uuid::Uuid::new_v4().to_string(),
            "default_user",
            chrono::Utc::now().to_rfc3339(),
        ),
    )?;

    Ok(())
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub auto_start_on_boot: bool,
    pub screenshot_interval: i32,
    pub screenshot_location: String,
    pub backup_frequency: String,
    pub backup_location: String,
    pub idle_threshold: i32,
    pub is_screenshot_enabled: bool,
}

pub fn get_settings(app: &AppHandle) -> Result<Settings> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT language, theme, auto_start_on_boot, screenshot_interval, screenshot_location, backup_frequency, backup_location, COALESCE(idle_threshold, 5), COALESCE(is_screenshot_enabled, 1) FROM settings WHERE user_id = 'default_user' LIMIT 1")?;
    let settings = stmt.query_row([], |row| {
        Ok(Settings {
            language: row.get(0)?,
            theme: row.get(1)?,
            auto_start_on_boot: row.get(2)?,
            screenshot_interval: row.get(3)?,
            screenshot_location: row.get(4)?,
            backup_frequency: row.get(5)?,
            backup_location: row.get(6)?,
            idle_threshold: row.get(7)?,
            is_screenshot_enabled: row.get::<_, i32>(8)? != 0,
        })
    })?;
    Ok(settings)
}

pub fn update_settings(app: &AppHandle, settings: Settings) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE settings SET 
            language = ?1, 
            theme = ?2, 
            auto_start_on_boot = ?3,
            screenshot_interval = ?4,
            screenshot_location = ?5,
            backup_frequency = ?6,
            backup_location = ?7,
            idle_threshold = ?8,
            is_screenshot_enabled = ?9,
            updated_at = ?10 
        WHERE user_id = 'default_user'",
        params![
            settings.language,
            settings.theme,
            settings.auto_start_on_boot as i32,
            settings.screenshot_interval,
            settings.screenshot_location,
            settings.backup_frequency,
            settings.backup_location,
            settings.idle_threshold,
            settings.is_screenshot_enabled as i32,
            now
        ],
    )?;
    Ok(())
}

// ─── Session Management ───────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration: Option<i64>,
}

pub fn start_session(app: &AppHandle) -> Result<Session> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let user_id = crate::tracking::get_active_user_id();
    conn.execute(
        "INSERT INTO sessions (id, user_id, start_time, created_at, updated_at) VALUES (?1, ?2, ?3, ?3, ?3)",
        params![id, user_id, now],
    )?;
    Ok(Session {
        id,
        start_time: now,
        end_time: None,
        duration: None,
    })
}

pub fn stop_session(app: &AppHandle, session_id: &str) -> Result<Session> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE sessions SET end_time = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, session_id],
    )?;
    let mut stmt = conn.prepare("SELECT id, start_time, end_time FROM sessions WHERE id = ?1")?;
    let session = stmt.query_row(params![session_id], |row| {
        let start: String = row.get(0)?;
        let end: Option<String> = row.get(2)?;
        Ok(Session {
            id: row.get(0)?,
            start_time: row.get(1)?,
            end_time: end.clone(),
            duration: if let (Ok(s), Some(e)) = (
                chrono::DateTime::parse_from_rfc3339(&start),
                end.as_ref().and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok()),
            ) {
                Some((e - s).num_seconds())
            } else {
                None
            },
        })
    })?;
    Ok(session)
}

pub fn get_active_session(app: &AppHandle) -> Result<Option<Session>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let user_id = crate::tracking::get_active_user_id();
    let mut stmt = conn.prepare(
        "SELECT id, start_time, end_time FROM sessions WHERE user_id = ?1 AND end_time IS NULL ORDER BY start_time DESC LIMIT 1"
    )?;
    let result = stmt.query_row(params![user_id], |row| {
        Ok(Session {
            id: row.get(0)?,
            start_time: row.get(1)?,
            end_time: row.get(2)?,
            duration: None,
        })
    });
    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ─── Dashboard Data Queries ───────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct AppUsageStat {
    pub app_name: String,
    pub total_seconds: i64,
    pub session_count: i64,
    pub category: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppCategoryEntry {
    pub app_name: String,
    pub category: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardData {
    pub total_active_seconds: i64,
    pub total_idle_seconds: i64,
    pub session_seconds: i64,
    pub app_stats: Vec<AppUsageStat>,
    pub recent_urls: Vec<UrlEntry>,
    pub keyboard_count: i64,
    pub mouse_count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UrlEntry {
    pub url: String,
    pub timestamp: String,
}

pub fn get_dashboard_data(app: &AppHandle) -> Result<DashboardData> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let user_id = crate::tracking::get_active_user_id();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Total active seconds today
    let total_active: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration), 0) FROM time_logs WHERE user_id = ?1 AND date(start_time) = ?2",
        params![user_id, today],
        |row| row.get(0),
    )?;

    // Idle time: count pairs of idle_start/idle_end events today
    let idle_events: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT type, timestamp FROM activity_events WHERE user_id = ?1 AND (type = 'idle_start' OR type = 'idle_end') AND date(timestamp) = ?2 ORDER BY timestamp ASC"
        )?;
        let rows = stmt.query_map(params![user_id, today], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut total_idle: i64 = 0;
    let mut idle_start_time: Option<chrono::DateTime<chrono::FixedOffset>> = None;
    for (event_type, ts) in &idle_events {
        if event_type == "idle_start" {
            idle_start_time = chrono::DateTime::parse_from_rfc3339(ts).ok();
        } else if event_type == "idle_end" {
            if let (Some(start), Ok(end)) = (idle_start_time.take(), chrono::DateTime::parse_from_rfc3339(ts)) {
                total_idle += (end - start).num_seconds();
            }
        }
    }

    // Session time today
    let session_seconds: i64 = conn.query_row(
        "SELECT COALESCE(SUM(
            CAST((julianday(COALESCE(end_time, datetime('now'))) - julianday(start_time)) * 86400 AS INTEGER)
        ), 0) FROM sessions WHERE user_id = ?1 AND date(start_time) = ?2",
        params![user_id, today],
        |row| row.get(0),
    )?;

    // App usage stats today
    let app_stats: Vec<AppUsageStat> = {
        let mut stmt = conn.prepare(
            "SELECT t.app_name, SUM(t.duration) as total_secs, COUNT(*) as cnt, COALESCE(c.category, 'neutral') as cat 
             FROM time_logs t 
             LEFT JOIN app_categories c ON t.app_name = c.app_name 
             WHERE t.user_id = ?1 AND date(t.start_time) = ?2 
             GROUP BY t.app_name 
             ORDER BY total_secs DESC LIMIT 100"
        )?;
        let rows = stmt.query_map(params![user_id, today], |row| {
            Ok(AppUsageStat {
                app_name: row.get(0)?,
                total_seconds: row.get(1)?,
                session_count: row.get(2)?,
                category: row.get(3)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Recent URL visits today
    let recent_urls: Vec<UrlEntry> = {
        let mut stmt = conn.prepare(
            "SELECT type, timestamp FROM activity_events WHERE user_id = ?1 AND type LIKE 'url:%' AND date(timestamp) = ?2 ORDER BY timestamp DESC LIMIT 30"
        )?;
        let rows = stmt.query_map(params![user_id, today], |row| {
            let raw: String = row.get(0)?;
            Ok(UrlEntry {
                url: raw.strip_prefix("url:").unwrap_or(&raw).to_string(),
                timestamp: row.get(1)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Keyboard/Mouse activity counts
    let keyboard_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_events WHERE user_id = ?1 AND type = 'keyboard' AND date(timestamp) = ?2",
        params![user_id, today], |r| r.get(0))?;
    let mouse_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_events WHERE user_id = ?1 AND type = 'mouse' AND date(timestamp) = ?2",
        params![user_id, today], |r| r.get(0))?;

    Ok(DashboardData {
        total_active_seconds: total_active,
        total_idle_seconds: total_idle,
        session_seconds,
        app_stats,
        recent_urls,
        keyboard_count,
        mouse_count,
    })
}

// ─── Date-Range Query: Time Logs (Trackings) ─────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct TimeLogEntry {
    pub id: String,
    pub app_name: String,
    pub window_title: String,
    pub start_time: String,
    pub end_time: String,
    pub duration: i64,
    pub status: String,
}

pub fn get_time_logs_range(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<TimeLogEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, app_name, window_title, start_time, COALESCE(end_time,''), COALESCE(duration,0), COALESCE(status,'active') FROM time_logs WHERE user_id=?1 AND date(start_time) >= ?2 AND date(start_time) <= ?3 ORDER BY start_time DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        Ok(TimeLogEntry {
            id: row.get(0)?,
            app_name: row.get(1)?,
            window_title: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            duration: row.get(5)?,
            status: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Date-Range Query: Browser URLs ──────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct UrlEntryFull {
    pub id: String,
    pub url: String,
    pub timestamp: String,
    pub activity_status: String,
}

pub fn get_urls_range(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<UrlEntryFull>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, type, timestamp, COALESCE(activity_status,'active') FROM activity_events WHERE user_id=?1 AND type LIKE 'url:%' AND date(timestamp) >= ?2 AND date(timestamp) <= ?3 ORDER BY timestamp DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        let raw: String = row.get(1)?;
        Ok(UrlEntryFull {
            id: row.get(0)?,
            url: raw.strip_prefix("url:").unwrap_or(&raw).to_string(),
            timestamp: row.get(2)?,
            activity_status: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Date-Range Query: Screenshots ───────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct ScreenshotEntry {
    pub id: String,
    pub file_path: String,
    pub captured_at: String,
}

pub fn get_screenshots_range(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<ScreenshotEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, file_path, captured_at FROM screenshots WHERE user_id=?1 AND date(captured_at) >= ?2 AND date(captured_at) <= ?3 ORDER BY captured_at DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        Ok(ScreenshotEntry {
            id: row.get(0)?,
            file_path: row.get(1)?,
            captured_at: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Insert Screenshot Record ────────────────────────────────────

pub fn insert_screenshot(app: &AppHandle, user_id: &str, file_path: &str) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO screenshots (id, user_id, file_path, captured_at, created_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![uuid::Uuid::new_v4().to_string(), user_id, file_path, now],
    )?;
    Ok(())
}

// ─── Get screenshot location from settings ───────────────────────

pub fn get_screenshot_dir(app: &AppHandle) -> String {
    if let Ok(settings) = get_settings(app) {
        if !settings.screenshot_location.is_empty() {
            return settings.screenshot_location;
        }
    }
    // Default: app data dir / screenshots
    let mut path = app.path().app_data_dir().expect("Failed to get app data dir");
    path.push("screenshots");
    std::fs::create_dir_all(&path).ok();
    path.to_string_lossy().to_string()
}

pub fn get_screenshot_interval_secs(app: &AppHandle) -> u64 {
    if let Ok(settings) = get_settings(app) {
        return (settings.screenshot_interval as u64) * 60;
    }
    600 // default 10 min
}

pub fn get_idle_threshold_secs(app: &AppHandle) -> u64 {
    if let Ok(settings) = get_settings(app) {
        return (settings.idle_threshold as u64) * 60;
    }
    300 // default 5 min
}

// ─── Admin: Per-User Queries ──────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct AdminScreenshotEntry {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub file_path: String,
    pub captured_at: String,
}

pub fn get_user_screenshots(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<AdminScreenshotEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT CAST(s.id AS TEXT), s.user_id, COALESCE(u.display_name, s.user_id), s.file_path, s.captured_at
         FROM screenshots s
         LEFT JOIN auth_users u ON s.user_id = u.id
         WHERE s.user_id = ?1 AND date(s.captured_at) >= ?2 AND date(s.captured_at) <= ?3
         ORDER BY s.captured_at DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        Ok(AdminScreenshotEntry {
            id: row.get(0)?,
            user_id: row.get(1)?,
            display_name: row.get(2)?,
            file_path: row.get(3)?,
            captured_at: row.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdminTimeLogEntry {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub app_name: String,
    pub window_title: String,
    pub start_time: String,
    pub end_time: String,
    pub duration: i64,
    pub status: String,
}

pub fn get_user_time_logs(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<AdminTimeLogEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT CAST(t.id AS TEXT), t.user_id, COALESCE(u.display_name, t.user_id), t.app_name, t.window_title,
                t.start_time, COALESCE(t.end_time,''), COALESCE(t.duration,0), COALESCE(t.status,'active')
         FROM time_logs t
         LEFT JOIN auth_users u ON t.user_id = u.id
         WHERE t.user_id = ?1 AND date(t.start_time) >= ?2 AND date(t.start_time) <= ?3
         ORDER BY t.start_time DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        Ok(AdminTimeLogEntry {
            id: row.get(0)?,
            user_id: row.get(1)?,
            display_name: row.get(2)?,
            app_name: row.get(3)?,
            window_title: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            duration: row.get(7)?,
            status: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdminActivityEntry {
    pub id: String,
    pub event_type: String,
    pub timestamp: String,
    pub activity_status: String,
}

pub fn get_user_activity(app: &AppHandle, user_id: &str, from: &str, to: &str, limit: u32, offset: u32) -> Result<Vec<AdminActivityEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT CAST(id AS TEXT), type, timestamp, COALESCE(activity_status,'active')
         FROM activity_events
         WHERE user_id = ?1 AND date(timestamp) >= ?2 AND date(timestamp) <= ?3
         ORDER BY timestamp DESC LIMIT ?4 OFFSET ?5"
    )?;
    let rows = stmt.query_map(params![user_id, from, to, limit, offset], |row| {
        Ok(AdminActivityEntry {
            id: row.get(0)?,
            event_type: row.get(1)?,
            timestamp: row.get(2)?,
            activity_status: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInputStats {
    pub keyboard_count: i64,
    pub mouse_count: i64,
    pub idle_start_count: i64,
}

pub fn get_user_input_stats(app: &AppHandle, user_id: &str, from: &str, to: &str) -> Result<UserInputStats> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let keyboard_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_events WHERE user_id = ?1 AND type = 'keyboard' AND date(timestamp) >= ?2 AND date(timestamp) <= ?3",
        params![user_id, from, to], |r| r.get(0))?;
    let mouse_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_events WHERE user_id = ?1 AND type = 'mouse' AND date(timestamp) >= ?2 AND date(timestamp) <= ?3",
        params![user_id, from, to], |r| r.get(0))?;
    let idle_start_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_events WHERE user_id = ?1 AND type = 'idle_start' AND date(timestamp) >= ?2 AND date(timestamp) <= ?3",
        params![user_id, from, to], |r| r.get(0))?;
    Ok(UserInputStats { keyboard_count, mouse_count, idle_start_count })
}

pub fn update_app_category(app: &AppHandle, app_name: &str, category: &str) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    conn.execute(
        "INSERT INTO app_categories (app_name, category) VALUES (?1, ?2)
         ON CONFLICT(app_name) DO UPDATE SET category = excluded.category",
        params![app_name, category],
    )?;
    Ok(())
}

pub fn get_all_app_categories(app: &AppHandle) -> Result<Vec<AppCategoryEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.app_name, COALESCE(c.category, 'neutral') 
         FROM time_logs t 
         LEFT JOIN app_categories c ON t.app_name = c.app_name 
         ORDER BY t.app_name ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AppCategoryEntry {
            app_name: row.get(0)?,
            category: row.get(1)?,
        })
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

