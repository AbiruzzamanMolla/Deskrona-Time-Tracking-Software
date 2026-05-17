# Deskrona API Reference — Server-Side Implementation Guide

The app lets user set **full URL** for each endpoint (not base + path).  
Server only needs to serve what the app sends.  All authenticated endpoints expect `Authorization: Bearer {token}` header.

---

## Auth

### POST — Register Company + Admin
**URL:** user-configured  
**Headers:** `Content-Type: application/json`  
**Body:**
```json
{
  "company_name": "string",
  "admin_username": "string",
  "admin_display_name": "string",
  "admin_password": "string"
}
```
**Response:**
```json
{
  "token": "string",
  "user": {
    "id": "string",
    "company_id": "string",
    "username": "string",
    "display_name": "string",
    "role": "string",
    "created_at": "string (ISO datetime)"
  }
}
```

---

### POST — Login
**URL:** user-configured  
**Headers:** `Content-Type: application/json`  
**Body:**
```json
{
  "username": "string",
  "password": "string"
}
```
**Response:**
```json
{
  "token": "string",
  "user": {
    "id": "string",
    "company_id": "string",
    "username": "string",
    "display_name": "string",
    "role": "string",
    "created_at": "string (ISO datetime)"
  }
}
```

---

### GET — Validate Session
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** none  
**Response:**
```json
{
  "valid": true,
  "user": {
    "id": "string",
    "company_id": "string",
    "username": "string",
    "display_name": "string",
    "role": "string"
  }
}
```

---

### POST — Logout
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "token": "string"
}
```
**Response:**
```json
{ "success": true }
```

---

## Sessions

### POST — Start Session
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:** `{}`  
**Response:**
```json
{
  "session": {
    "id": "string (UUID)",
    "start_time": "string (ISO datetime)",
    "status": "string"
  }
}
```

---

### POST — Stop Session
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "session_id": "string (UUID)"
}
```
**Response:**
```json
{
  "session": {
    "id": "string",
    "start_time": "string",
    "end_time": "string (ISO datetime)",
    "duration": 3600
  }
}
```

---

### GET — Get Active Session
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "session": {
    "id": "string",
    "start_time": "string (ISO datetime)",
    "status": "string"
  }
}
```
Returns `{ "session": null }` if none active.

---

## Tracking

### POST — Set/Get Tracking Status
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "status": "running | paused | stopped"
}
```
**Response:**
```json
{
  "status": "running | paused | stopped",
  "elapsed": 0
}
```
`elapsed` = seconds since tracking started.

---

## Time Logs

### POST — Sync Time Logs
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "logs": [
    {
      "app_name": "string",
      "window_title": "string",
      "category": "productive | neutral | unproductive",
      "start_time": "string (ISO datetime)",
      "end_time": "string (ISO datetime)",
      "duration": 300
    }
  ]
}
```
**Response:**
```json
{
  "synced": 50
}
```

---

### GET — Get Time Logs
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "logs": [
    {
      "id": "string",
      "app_name": "string",
      "window_title": "string",
      "category": "string",
      "start_time": "string",
      "end_time": "string",
      "duration": 300
    }
  ],
  "total": 500,
  "page": 1,
  "limit": 100
}
```

---

## Screenshots

### POST — Upload Screenshot
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}` (Content-Type set automatically for FormData)  
**Body:** FormData with fields:
| Field | Type | Description |
|---|---|---|
| `file` | binary (PNG) | Screenshot image |
| `screenshot_id` | string (UUID) | Unique screenshot ID |
| `user_id` | string | Owning user ID |
| `captured_at` | string (ISO datetime) | Timestamp of capture |

**Response:**
```json
{
  "url": "string (accessible URL or path)"
}
```

---

### GET — Get Screenshots
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "screenshots": [
    {
      "id": "string",
      "file_path": "string",
      "captured_at": "string (ISO datetime)"
    }
  ],
  "total": 200
}
```

---

## URLs

### POST — Sync URL Entries
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "urls": [
    {
      "url": "https://example.com/page",
      "title": "Page Title",
      "timestamp": "string (ISO datetime)"
    }
  ]
}
```
**Response:**
```json
{ "synced": 25 }
```

---

### GET — Get URL Entries
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "urls": [
    {
      "id": "string",
      "url": "string",
      "title": "string",
      "timestamp": "string (ISO datetime)"
    }
  ],
  "total": 300
}
```

---

## Activity

### POST — Sync Activity Events
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "events": [
    {
      "type": "keyboard | mouse | focus",
      "app": "chrome.exe",
      "window_title": "string",
      "timestamp": "string (ISO datetime)",
      "data": {}
    }
  ]
}
```
**Response:**
```json
{ "synced": 100 }
```

---

### GET — Get Activity Events
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "events": [
    {
      "id": "string",
      "type": "string",
      "app": "string",
      "window_title": "string",
      "timestamp": "string",
      "data": {}
    }
  ],
  "total": 500
}
```

---

## Input Stats

### GET — Get Input Stats
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate`  
**Response:**
```json
{
  "keyboard": 1420,
  "mouse": 880,
  "idle": 360
}
```

---

## Dashboard

### GET — Get Today Dashboard
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "total_active_seconds": 28800,
  "total_idle_seconds": 1200,
  "session_seconds": 28800,
  "app_stats": [
    {
      "app_name": "code.exe",
      "total_seconds": 14400,
      "session_count": 3,
      "category": "productive"
    }
  ],
  "recent_urls": [
    {
      "url": "https://example.com",
      "title": "Example",
      "timestamp": "string (ISO datetime)"
    }
  ],
  "keyboard_count": 5000,
  "mouse_count": 2000,
  "productivity_score": 85
}
```

---

### GET — Get Dashboard by Range
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate`  
**Response:**
```json
{
  "days": [
    {
      "date": "2025-01-15",
      "total_time": 28800,
      "apps": [
        {
          "app_name": "code.exe",
          "total_seconds": 14400
        }
      ]
    }
  ]
}
```

---

## Admin — Users

### GET — List Company Users
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "users": [
    {
      "id": "string",
      "company_id": "string",
      "username": "string",
      "display_name": "string",
      "role": "admin | employee",
      "created_at": "string (ISO datetime)"
    }
  ]
}
```
Server determines company from auth token.

---

### POST — Create User
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "username": "string",
  "display_name": "string",
  "password": "string (min 6 chars)",
  "role": "admin | employee"
}
```
**Response:**
```json
{
  "user": {
    "id": "string",
    "company_id": "string",
    "username": "string",
    "display_name": "string",
    "role": "string",
    "created_at": "string"
  }
}
```

---

## Admin — Stats

### GET — Get Admin Stats
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "stats": [
    {
      "user_id": "string",
      "display_name": "string",
      "username": "string",
      "total_active_seconds": 28800,
      "session_count": 5,
      "keyboard_count": 3000,
      "mouse_count": 1500
    }
  ]
}
```

---

## Admin — Drill-down

All drill-down endpoints have `{userId}` placeholder in URL, e.g.  
`https://server.com/admin/users/{userId}/screenshots`

### GET — Get User Screenshots
**URL:** user-configured (with `{userId}` replaced by the target user ID)  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "screenshots": [
    {
      "id": "string",
      "user_id": "string",
      "file_path": "string",
      "captured_at": "string (ISO datetime)"
    }
  ],
  "total": 50
}
```

---

### GET — Get User Time Logs
**URL:** user-configured (with `{userId}` replaced)  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "logs": [
    {
      "id": "string",
      "user_id": "string",
      "app_name": "string",
      "window_title": "string",
      "start_time": "string",
      "end_time": "string",
      "duration": 300
    }
  ],
  "total": 200
}
```

---

### GET — Get User Activity
**URL:** user-configured (with `{userId}` replaced)  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "events": [
    {
      "id": "string",
      "user_id": "string",
      "type": "string",
      "app": "string",
      "timestamp": "string"
    }
  ],
  "total": 500
}
```

---

### GET — Get User URLs
**URL:** user-configured (with `{userId}` replaced)  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate&limit=100&offset=0`  
**Response:**
```json
{
  "urls": [
    {
      "id": "string",
      "user_id": "string",
      "url": "string",
      "title": "string",
      "timestamp": "string"
    }
  ],
  "total": 300
}
```

---

### GET — Get User Input Stats
**URL:** user-configured (with `{userId}` replaced)  
**Headers:** `Authorization: Bearer {token}`  
**Query params:** `?from=ISOdate&to=ISOdate`  
**Response:**
```json
{
  "keyboard": 1420,
  "mouse": 880,
  "idle": 360
}
```

---

## App Categories

### GET — Get App Categories
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "categories": [
    {
      "app_name": "code.exe",
      "category": "productive | neutral | unproductive"
    }
  ]
}
```

---

### PUT — Update App Category
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "app_name": "code.exe",
  "category": "productive | neutral | unproductive"
}
```
**Response:**
```json
{ "success": true }
```

---

## App Config

### GET — Get App Config
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "config": {
    "mode": "single_user | multi_user",
    "setup_done": true
  }
}
```
Note: Response is wrapped in `{ "config": {...} }`.

---

### POST — Save App Config
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "config": {
    "mode": "single_user | multi_user",
    "setup_done": true
  }
}
```
**Response:**
```json
{ "success": true }
```

---

## Settings

### GET — Get Settings
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "settings": {
    "language": "en | bn",
    "theme": "light | dark | system",
    "screenshot_interval": 5,
    "screenshot_location": "/path/to/screenshots",
    "backup_frequency": "never | daily | weekly",
    "backup_location": "/path/to/backups",
    "idle_threshold": 5,
    "idle_monitor_mouse": true,
    "idle_monitor_keyboard": true,
    "is_screenshot_enabled": true,
    "pomodoro_focus_minutes": 25,
    "pomodoro_short_break_minutes": 5,
    "pomodoro_long_break_minutes": 15,
    "pomodoro_sessions_before_long": 4,
    "pomodoro_auto_start": false,
    "pomodoro_sound_enabled": true
  }
}
```

---

### POST — Update Settings
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "settings": {
    "language": "en",
    "theme": "dark",
    "screenshot_interval": 5,
    "screenshot_location": "C:/screenshots",
    "idle_threshold": 5,
    "idle_monitor_mouse": true,
    "idle_monitor_keyboard": true,
    "is_screenshot_enabled": true,
    "backup_frequency": "daily",
    "backup_location": "C:/backups",
    "pomodoro_focus_minutes": 25,
    "pomodoro_short_break_minutes": 5,
    "pomodoro_long_break_minutes": 15,
    "pomodoro_sessions_before_long": 4,
    "pomodoro_auto_start": false,
    "pomodoro_sound_enabled": true
  }
}
```
**Response:**
```json
{ "success": true }
```

---

## Backup & Updates

### GET — Export Backup
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:** Binary ZIP file containing:
- `tracker.db` — SQLite database
- `screenshots/` — PNG images
- `api-config.json` — endpoint configuration

### POST — Import Backup
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}` (Content-Type auto-set for FormData)  
**Body:** FormData with field:
| Field | Type | Description |
|---|---|---|
| `file` | binary (ZIP) | Backup archive |
Expected ZIP contents:
- `tracker.db` (required)
- `screenshots/` (optional)
- `api-config.json` (optional)

**Response:**
```json
{ "success": true }
```

---

### GET — Check for Updates
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
[
  {
    "name": "v0.0.9"
  }
]
```

---

## Pomodoro

### POST — Start Pomodoro Focus
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:** `{}`  
**Response:**
```json
{
  "phase": "focus",
  "remaining_secs": 1500
}
```

---

### POST — Skip Pomodoro Phase
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:** `{}`  
**Response:**
```json
{
  "phase": "short_break | long_break | focus",
  "remaining_secs": 300
}
```

---

### POST — Stop/Reset Pomodoro
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:** `{}`  
**Response:**
```json
{
  "phase": "idle",
  "remaining_secs": 0
}
```

---

### GET — Get Pomodoro Status
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{
  "phase": "idle | focus | short_break | long_break",
  "remaining_secs": 1500,
  "count_today": 3
}
```

---

## Autostart

### POST — Set Autostart
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:**
```json
{
  "enabled": true
}
```
**Response:**
```json
{ "success": true }
```

---

### GET — Get Autostart Status
**URL:** user-configured  
**Headers:** `Authorization: Bearer {token}`  
**Response:**
```json
{ "enabled": true }
```

---

## Reset

### POST — Factory Reset App
**URL:** user-configured  
**Headers:** `Content-Type: application/json`, `Authorization: Bearer {token}`  
**Body:** `{}`  
**Response:**
```json
{ "success": true }
```

---

## Implementation Notes

### Authentication
All endpoints except `auth_register` and `auth_login` expect `Authorization: Bearer {token}` header.  The token is set manually by the user in settings (login/register endpoints are optional helpers).

### Query Parameters
All GET endpoints with date filtering use these param names (as sent by the app):
- `from`, `to` — ISO date/datetime strings
- `limit`, `offset` — pagination (not `page`)

### URL Templates
Admin drill-down endpoints support `{userId}` placeholders in the URL, replaced at runtime:
```
https://server.com/admin/users/{userId}/screenshots?from=...&to=...
```

### File Uploads
Screenshot and backup import use `multipart/form-data` (FormData), not JSON.
Exact field names (`file`, `screenshot_id`, `user_id`, `captured_at`) are critical.

### Offline-First
The app writes locally first and syncs in background.  Sync endpoints (`time_logs_sync`, `urls_sync`, `activity_sync`) receive batches of records.  Retry with exponential backoff (×5) for failed requests.

### Backup Format
Export returns raw ZIP bytes.  Import expects ZIP with at minimum `tracker.db`.  The app-side runs this through Tauri dialogs (save/open file), not browser downloads.

---

## Quick-fill Default Paths

When user sets a **Server URL** during first-run wizard, app auto-fills all endpoint URLs with these paths (user can override any in settings):

| Endpoint Key | Default Path |
|---|---|
| `auth_register` | `/api/auth/register` |
| `auth_login` | `/api/auth/login` |
| `auth_validate` | `/api/auth/validate` |
| `auth_logout` | `/api/auth/logout` |
| `session_start` | `/api/sessions/start` |
| `session_stop` | `/api/sessions/stop` |
| `session_active` | `/api/sessions/active` |
| `tracking_status` | `/api/tracking/status` |
| `time_logs_sync` | `/api/time-logs/sync` |
| `time_logs_get` | `/api/time-logs` |
| `screenshots_upload` | `/api/screenshots/upload` |
| `screenshots_get` | `/api/screenshots` |
| `urls_sync` | `/api/urls/sync` |
| `urls_get` | `/api/urls` |
| `activity_sync` | `/api/activity/sync` |
| `activity_get` | `/api/activity` |
| `input_stats_get` | `/api/input-stats` |
| `dashboard_today` | `/api/dashboard/today` |
| `dashboard_range` | `/api/dashboard/range` |
| `admin_users_list` | `/api/admin/users` |
| `admin_users_create` | `/api/admin/users` |
| `admin_stats` | `/api/admin/stats` |
| `admin_user_screenshots` | `/api/admin/users/{userId}/screenshots` |
| `admin_user_time_logs` | `/api/admin/users/{userId}/time-logs` |
| `admin_user_activity` | `/api/admin/users/{userId}/activity` |
| `admin_user_urls` | `/api/admin/users/{userId}/urls` |
| `admin_user_input_stats` | `/api/admin/users/{userId}/input-stats` |
| `app_categories_get` | `/api/categories` |
| `app_categories_update` | `/api/categories` |
| `config_get` | `/api/config` |
| `config_save` | `/api/config` |
| `settings_get` | `/api/settings` |
| `settings_update` | `/api/settings` |
| `backup_export` | `/api/backup/export` |
| `backup_import` | `/api/backup/import` |
| `update_check` | `/api/updates` |
| `pomodoro_start` | `/api/pomodoro/start` |
| `pomodoro_skip` | `/api/pomodoro/skip` |
| `pomodoro_stop` | `/api/pomodoro/stop` |
| `pomodoro_status` | `/api/pomodoro` |
| `autostart_set` | `/api/autostart` |
| `autostart_get` | `/api/autostart` |
| `reset_app` | `/api/reset` |

Your server can use these paths or user can set custom URLs per endpoint.
