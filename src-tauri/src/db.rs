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
            synced_at TEXT
        );
        ",
    )?;

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
