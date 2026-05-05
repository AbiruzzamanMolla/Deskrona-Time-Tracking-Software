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
            type TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            created_at TEXT NOT NULL,
            synced_at TEXT,
            activity_status TEXT NOT NULL DEFAULT 'active'
        );
        ",
    )?;

    // Migrations for existing databases
    let _ = conn.execute("ALTER TABLE time_logs ADD COLUMN status TEXT NOT NULL DEFAULT 'active'", []);
    let _ = conn.execute("ALTER TABLE activity_events ADD COLUMN activity_status TEXT NOT NULL DEFAULT 'active'", []);

    // Insert default settings if none exists
    conn.execute(
        "INSERT INTO settings (id, user_id, language, theme, auto_start_on_boot, screenshot_interval, screenshot_location, backup_frequency, backup_location, created_at, updated_at)
         SELECT ?1, ?2, 'en', 'system', 0, 10, '', 'never', '', ?3, ?3
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
}

pub fn get_settings(app: &AppHandle) -> Result<Settings> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT language, theme, auto_start_on_boot, screenshot_interval, screenshot_location, backup_frequency, backup_location FROM settings WHERE user_id = 'default_user' LIMIT 1")?;
    let settings = stmt.query_row([], |row| {
        Ok(Settings {
            language: row.get(0)?,
            theme: row.get(1)?,
            auto_start_on_boot: row.get(2)?,
            screenshot_interval: row.get(3)?,
            screenshot_location: row.get(4)?,
            backup_frequency: row.get(5)?,
            backup_location: row.get(6)?,
        })
    })?;
    Ok(settings)
}

pub fn update_settings(app: &AppHandle, settings: Settings) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    conn.execute(
        "UPDATE settings SET 
            language = ?1, 
            theme = ?2, 
            auto_start_on_boot = ?3,
            screenshot_interval = ?4,
            screenshot_location = ?5,
            backup_frequency = ?6,
            backup_location = ?7,
            updated_at = ?8 
        WHERE user_id = 'default_user'",
        params![
            settings.language,
            settings.theme,
            settings.auto_start_on_boot,
            settings.screenshot_interval,
            settings.screenshot_location,
            settings.backup_frequency,
            settings.backup_location,
            chrono::Utc::now().to_rfc3339()
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
    conn.execute(
        "INSERT INTO sessions (id, user_id, start_time, created_at, updated_at) VALUES (?1, 'default_user', ?2, ?2, ?2)",
        params![id, now],
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
    let mut stmt = conn.prepare(
        "SELECT id, start_time, end_time FROM sessions WHERE user_id = 'default_user' AND end_time IS NULL ORDER BY start_time DESC LIMIT 1"
    )?;
    let result = stmt.query_row([], |row| {
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardData {
    pub total_active_seconds: i64,
    pub total_idle_seconds: i64,
    pub session_seconds: i64,
    pub app_stats: Vec<AppUsageStat>,
    pub recent_urls: Vec<UrlEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UrlEntry {
    pub url: String,
    pub timestamp: String,
}

pub fn get_dashboard_data(app: &AppHandle) -> Result<DashboardData> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Total active seconds today
    let total_active: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration), 0) FROM time_logs WHERE user_id = 'default_user' AND date(start_time) = ?1",
        params![today],
        |row| row.get(0),
    )?;

    // Idle time: count pairs of idle_start/idle_end events today
    let idle_events: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT type, timestamp FROM activity_events WHERE (type = 'idle_start' OR type = 'idle_end') AND date(timestamp) = ?1 ORDER BY timestamp ASC"
        )?;
        let rows = stmt.query_map(params![today], |row| {
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
        ), 0) FROM sessions WHERE user_id = 'default_user' AND date(start_time) = ?1",
        params![today],
        |row| row.get(0),
    )?;

    // App usage stats today
    let app_stats: Vec<AppUsageStat> = {
        let mut stmt = conn.prepare(
            "SELECT app_name, SUM(duration) as total_secs, COUNT(*) as cnt FROM time_logs WHERE user_id = 'default_user' AND date(start_time) = ?1 GROUP BY app_name ORDER BY total_secs DESC LIMIT 20"
        )?;
        let rows = stmt.query_map(params![today], |row| {
            Ok(AppUsageStat {
                app_name: row.get(0)?,
                total_seconds: row.get(1)?,
                session_count: row.get(2)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Recent URL visits today
    let recent_urls: Vec<UrlEntry> = {
        let mut stmt = conn.prepare(
            "SELECT type, timestamp FROM activity_events WHERE type LIKE 'url:%' AND date(timestamp) = ?1 ORDER BY timestamp DESC LIMIT 30"
        )?;
        let rows = stmt.query_map(params![today], |row| {
            let raw: String = row.get(0)?;
            Ok(UrlEntry {
                url: raw.strip_prefix("url:").unwrap_or(&raw).to_string(),
                timestamp: row.get(1)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    Ok(DashboardData {
        total_active_seconds: total_active,
        total_idle_seconds: total_idle,
        session_seconds,
        app_stats,
        recent_urls,
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

pub fn get_time_logs_range(app: &AppHandle, from: &str, to: &str) -> Result<Vec<TimeLogEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, app_name, window_title, start_time, COALESCE(end_time,''), COALESCE(duration,0), COALESCE(status,'active') FROM time_logs WHERE user_id='default_user' AND date(start_time) >= ?1 AND date(start_time) <= ?2 ORDER BY start_time DESC LIMIT 500"
    )?;
    let rows = stmt.query_map(params![from, to], |row| {
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

pub fn get_urls_range(app: &AppHandle, from: &str, to: &str) -> Result<Vec<UrlEntryFull>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, type, timestamp, COALESCE(activity_status,'active') FROM activity_events WHERE type LIKE 'url:%' AND date(timestamp) >= ?1 AND date(timestamp) <= ?2 ORDER BY timestamp DESC LIMIT 500"
    )?;
    let rows = stmt.query_map(params![from, to], |row| {
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

pub fn get_screenshots_range(app: &AppHandle, from: &str, to: &str) -> Result<Vec<ScreenshotEntry>> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, file_path, captured_at FROM screenshots WHERE user_id='default_user' AND date(captured_at) >= ?1 AND date(captured_at) <= ?2 ORDER BY captured_at DESC LIMIT 200"
    )?;
    let rows = stmt.query_map(params![from, to], |row| {
        Ok(ScreenshotEntry {
            id: row.get(0)?,
            file_path: row.get(1)?,
            captured_at: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── Insert Screenshot Record ────────────────────────────────────

pub fn insert_screenshot(app: &AppHandle, file_path: &str) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO screenshots (id, user_id, file_path, captured_at, created_at) VALUES (?1, 'default_user', ?2, ?3, ?3)",
        params![uuid::Uuid::new_v4().to_string(), file_path, now],
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

