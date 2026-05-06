use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use uuid::Uuid;

use crate::db::get_db_path;

// ─── Structs ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub company_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResult {
    pub token: String,
    pub user: AuthUser,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterCompanyPayload {
    pub company_name: String,
    pub admin_username: String,
    pub admin_display_name: String,
    pub admin_password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserPayload {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: String,
}

// ─── Schema Init ──────────────────────────────────────────────────

pub fn init_auth_schema(app: &AppHandle) -> Result<()> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path)?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS companies (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS auth_users (
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            username TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'employee',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (company_id) REFERENCES companies(id)
        );

        CREATE TABLE IF NOT EXISTS auth_sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            company_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES auth_users(id)
        );
    ")?;
    Ok(())
}

// ─── Password Hashing ─────────────────────────────────────────────

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = PasswordHash::new(hash).ok();
    parsed
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

// ─── Session Token ────────────────────────────────────────────────

fn generate_token() -> String {
    Uuid::new_v4().to_string() + "-" + &Uuid::new_v4().to_string()
}

// ─── Commands ─────────────────────────────────────────────────────

/// Register a new company and its first admin user.
pub fn register_company(app: &AppHandle, payload: RegisterCompanyPayload) -> Result<LoginResult, String> {
    init_auth_schema(app).map_err(|e| e.to_string())?;
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let company_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO companies (id, name, created_at) VALUES (?1, ?2, ?3)",
        params![company_id, payload.company_name, now],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Company name already taken".to_string()
        } else {
            format!("Failed to create company: {}", e)
        }
    })?;

    let password_hash = hash_password(&payload.admin_password)?;
    let user_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO auth_users (id, company_id, username, display_name, password_hash, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'admin', ?6, ?7)",
        params![user_id, company_id, payload.admin_username, payload.admin_display_name, password_hash, now, now],
    )
    .map_err(|e| format!("Username already taken: {}", e))?;

    let token = generate_token();
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    conn.execute(
        "INSERT INTO auth_sessions (token, user_id, company_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![token, user_id, company_id, now, expires_at],
    )
    .map_err(|e| e.to_string())?;

    let result = LoginResult {
        token: token.clone(),
        user: AuthUser {
            id: user_id,
            company_id,
            username: payload.admin_username,
            display_name: payload.admin_display_name,
            role: "admin".to_string(),
            created_at: now,
        },
    };

    // Update background tracking user ID
    crate::tracking::set_active_user_id(Some(result.user.id.clone()));

    Ok(result)
}

/// Log in an existing user.
pub fn login(app: &AppHandle, payload: LoginPayload) -> Result<LoginResult, String> {
    init_auth_schema(app).map_err(|e| e.to_string())?;
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let row: Option<(String, String, String, String, String, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, company_id, username, display_name, password_hash, role FROM auth_users WHERE username = ?1",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_row(params![payload.username], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
            ))
        })
        .ok()
    };

    let (user_id, company_id, username, display_name, password_hash, role) =
        row.ok_or_else(|| "Invalid username or password".to_string())?;

    if !verify_password(&payload.password, &password_hash) {
        return Err("Invalid username or password".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let token = generate_token();
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    conn.execute(
        "INSERT INTO auth_sessions (token, user_id, company_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![token, user_id, company_id, now, expires_at],
    )
    .map_err(|e| e.to_string())?;

    let result = LoginResult {
        token,
        user: AuthUser {
            id: user_id,
            company_id,
            username,
            display_name,
            role,
            created_at: now,
        },
    };

    // Update background tracking user ID
    crate::tracking::set_active_user_id(Some(result.user.id.clone()));

    Ok(result)
}

/// Validate a session token and return the current user.
pub fn validate_session(app: &AppHandle, token: &str) -> Result<AuthUser, String> {
    init_auth_schema(app).map_err(|e| e.to_string())?;
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let row: Option<(String, String, String, String, String, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT u.id, u.company_id, u.username, u.display_name, u.role, u.created_at
                 FROM auth_sessions s JOIN auth_users u ON s.user_id = u.id
                 WHERE s.token = ?1 AND s.expires_at > ?2",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_row(params![token, now], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
            ))
        })
        .ok()
    };

    let (id, company_id, username, display_name, role, created_at) =
        row.ok_or_else(|| "Session expired or invalid".to_string())?;

    let user = AuthUser { id, company_id, username, display_name, role, created_at };

    // Update background tracking user ID
    crate::tracking::set_active_user_id(Some(user.id.clone()));

    Ok(user)
}

/// Invalidate (logout) a session token.
pub fn logout(app: &AppHandle, token: &str) -> Result<(), String> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM auth_sessions WHERE token = ?1", params![token])
        .map_err(|e| e.to_string())?;

    // Reset background tracking user ID
    crate::tracking::set_active_user_id(None);

    Ok(())
}

/// Admin: list all users in the same company.
pub fn get_company_users(app: &AppHandle, company_id: &str) -> Result<Vec<AuthUser>, String> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, company_id, username, display_name, role, created_at FROM auth_users WHERE company_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![company_id], |r| {
            Ok(AuthUser {
                id: r.get(0)?,
                company_id: r.get(1)?,
                username: r.get(2)?,
                display_name: r.get(3)?,
                role: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Admin: create a new employee under the same company.
pub fn create_user(app: &AppHandle, company_id: &str, payload: CreateUserPayload) -> Result<AuthUser, String> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&payload.password)?;
    let role = if payload.role == "admin" { "admin" } else { "employee" };

    conn.execute(
        "INSERT INTO auth_users (id, company_id, username, display_name, password_hash, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![user_id, company_id, payload.username, payload.display_name, password_hash, role, now, now],
    )
    .map_err(|e| format!("Failed to create user: {}", e))?;

    Ok(AuthUser {
        id: user_id,
        company_id: company_id.to_string(),
        username: payload.username,
        display_name: payload.display_name,
        role: role.to_string(),
        created_at: now,
    })
}

/// Admin: aggregated productivity stats per user (today).
#[derive(Serialize, Deserialize, Debug)]
pub struct UserProductivityStat {
    pub user_id: String,
    pub display_name: String,
    pub username: String,
    pub total_active_seconds: i64,
    pub session_count: i64,
    pub keyboard_count: i64,
    pub mouse_count: i64,
}

pub fn get_admin_stats(app: &AppHandle, company_id: &str) -> Result<Vec<UserProductivityStat>, String> {
    let db_path = get_db_path(app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Join auth_users with time_logs by user_id for the company
    let mut stmt = conn
        .prepare(
            "SELECT u.id, u.display_name, u.username,
                    COALESCE(SUM(t.duration), 0) as active_secs,
                    COUNT(DISTINCT s.id) as sess_count,
                    (SELECT COUNT(*) FROM activity_events WHERE user_id = u.id AND type = 'keyboard' AND date(timestamp) = ?1) as kb_count,
                    (SELECT COUNT(*) FROM activity_events WHERE user_id = u.id AND type = 'mouse' AND date(timestamp) = ?1) as ms_count
             FROM auth_users u
             LEFT JOIN time_logs t ON t.user_id = u.id AND date(t.start_time) = ?1
             LEFT JOIN sessions s ON s.user_id = u.id AND date(s.start_time) = ?1
             WHERE u.company_id = ?2
             GROUP BY u.id
             ORDER BY active_secs DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![today, company_id], |r| {
            Ok(UserProductivityStat {
                user_id: r.get(0)?,
                display_name: r.get(1)?,
                username: r.get(2)?,
                total_active_seconds: r.get(3)?,
                session_count: r.get(4)?,
                keyboard_count: r.get(5)?,
                mouse_count: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
