export interface ApiEndpointConfig {
  enabled: boolean;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  url: string;
  headers: Record<string, string>;
}

export type EndpointKey =
  | 'auth_register' | 'auth_login' | 'auth_validate' | 'auth_logout'
  | 'session_start' | 'session_stop' | 'session_active'
  | 'tracking_status'
  | 'time_logs_sync' | 'time_logs_get'
  | 'screenshots_upload' | 'screenshots_get'
  | 'urls_sync' | 'urls_get'
  | 'activity_sync' | 'activity_get'
  | 'input_stats_get'
  | 'dashboard_today' | 'dashboard_range'
  | 'admin_users_list' | 'admin_users_create'
  | 'admin_stats'
  | 'admin_user_screenshots' | 'admin_user_time_logs'
  | 'admin_user_activity' | 'admin_user_urls' | 'admin_user_input_stats'
  | 'app_categories_get' | 'app_categories_update'
  | 'config_get' | 'config_save'
  | 'settings_get' | 'settings_update'
  | 'backup_export' | 'backup_import'
  | 'update_check'
  | 'pomodoro_start' | 'pomodoro_skip' | 'pomodoro_stop' | 'pomodoro_status'
  | 'autostart_set' | 'autostart_get'
  | 'calendar_month'
  | 'reset_app';

export interface ApiConfigFile {
  mode: 'offline' | 'online';
  bearer_token: string;
  endpoints: Record<EndpointKey, ApiEndpointConfig>;
}

export interface ApiEndpointInfo {
  key: EndpointKey;
  label: string;
  group: string;
  method: string;
  requestBody: string;
  responseBody: string;
}

export const ENDPOINT_GROUPS: { label: string; key: string; endpoints: ApiEndpointInfo[] }[] = [
  // ── Auth ────────────────────────────────────────────────────────
  {
    label: 'Auth', key: 'auth',
    endpoints: [
      {
        key: 'auth_register', label: 'Register Company + Admin', group: 'Auth', method: 'POST',
        requestBody: '{\n  "company_name": string,        // Company/organization name\n  "admin_username": string,       // Admin login username\n  "admin_display_name": string,   // Admin display name\n  "admin_password": string        // Password (min 6 chars)\n}',
        responseBody: '{\n  "token": string,     // JWT or session token\n  "user": {\n    "id": string, "company_id": string,\n    "username": string, "display_name": string,\n    "role": string, "created_at": string\n  }\n}',
      },
      {
        key: 'auth_login', label: 'Login', group: 'Auth', method: 'POST',
        requestBody: '{\n  "username": string,  // User login name\n  "password": string   // User password\n}',
        responseBody: '{\n  "token": string,     // JWT or session token\n  "user": {\n    "id": string, "company_id": string,\n    "username": string, "display_name": string,\n    "role": string, "created_at": string\n  }\n}',
      },
      {
        key: 'auth_validate', label: 'Validate Session', group: 'Auth', method: 'GET',
        requestBody: 'Headers:\n  Authorization: Bearer {token}',
        responseBody: '{\n  "valid": boolean,\n  "user": {\n    "id": string, "company_id": string,\n    "username": string, "display_name": string,\n    "role": string\n  }\n}',
      },
      {
        key: 'auth_logout', label: 'Logout', group: 'Auth', method: 'POST',
        requestBody: 'Headers:\n  Authorization: Bearer {token}\nBody: { "token": string }',
        responseBody: '{ "success": true }',
      },
    ],
  },
  // ── Sessions ─────────────────────────────────────────────────────
  {
    label: 'Sessions', key: 'sessions',
    endpoints: [
      {
        key: 'session_start', label: 'Start Session', group: 'Sessions', method: 'POST',
        requestBody: '{}  // No body needed, server generates session',
        responseBody: '{\n  "session": {\n    "id": string (UUID),\n    "start_time": string (ISO datetime),\n    "status": string\n  }\n}',
      },
      {
        key: 'session_stop', label: 'Stop Session', group: 'Sessions', method: 'POST',
        requestBody: '{\n  "session_id": string  // UUID of the session to stop\n}',
        responseBody: '{\n  "session": {\n    "id": string, "start_time": string,\n    "end_time": string (ISO datetime),\n    "duration": number (seconds)\n  }\n}',
      },
      {
        key: 'session_active', label: 'Get Active Session', group: 'Sessions', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "session": {\n    "id": string, "start_time": string,\n    "status": string\n  } | null  // null if no active session\n}',
      },
    ],
  },
  // ── Tracking ─────────────────────────────────────────────────────
  {
    label: 'Tracking', key: 'tracking',
    endpoints: [
      {
        key: 'tracking_status', label: 'Set/Get Tracking Status', group: 'Tracking', method: 'POST',
        requestBody: '{\n  "status": "running" | "paused" | "stopped"\n}',
        responseBody: '{\n  "status": "running" | "paused" | "stopped",\n  "elapsed": number (seconds since tracking started)\n}',
      },
    ],
  },
  // ── Time Logs ────────────────────────────────────────────────────
  {
    label: 'Time Logs', key: 'time_logs',
    endpoints: [
      {
        key: 'time_logs_sync', label: 'Sync Time Logs', group: 'Time Logs', method: 'POST',
        requestBody: '{\n  "logs": [{\n    "app_name": string,       // Application/process name\n    "window_title": string,    // Active window title\n    "category": string,        // "productive" | "neutral" | "unproductive"\n    "start_time": string,      // ISO datetime\n    "end_time": string,        // ISO datetime\n    "duration": number         // Seconds\n  }]\n}',
        responseBody: '{ "synced": number  // Count of logs accepted }',
      },
      {
        key: 'time_logs_get', label: 'Get Time Logs', group: 'Time Logs', method: 'GET',
        requestBody: 'Query params:\n  ?start=string (ISO date)    // Filter from date\n  &end=string (ISO date)      // Filter to date\n  &page=number                // Page number (1-based)\n  &limit=number               // Items per page (max 500)',
        responseBody: '{\n  "logs": [{\n    "id": string, "app_name": string,\n    "window_title": string, "category": string,\n    "start_time": string, "end_time": string,\n    "duration": number\n  }],\n  "total": number,\n  "page": number,\n  "limit": number\n}',
      },
    ],
  },
  // ── Screenshots ──────────────────────────────────────────────────
  {
    label: 'Screenshots', key: 'screenshots',
    endpoints: [
      {
        key: 'screenshots_upload', label: 'Upload Screenshot', group: 'Screenshots', method: 'POST',
        requestBody: 'FormData fields:\n  file: binary (PNG image)         // Screenshot file\n  screenshot_id: string (UUID)     // Unique screenshot identifier\n  user_id: string                  // User who owns the screenshot\n  captured_at: string (ISO datetime)  // When screenshot was taken',
        responseBody: '{ "url": string  // Accessible URL/path to the screenshot }',
      },
      {
        key: 'screenshots_get', label: 'Get Screenshots', group: 'Screenshots', method: 'GET',
        requestBody: 'Query params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "screenshots": [{\n    "id": string, "file_path": string,\n    "captured_at": string (ISO datetime)\n  }],\n  "total": number\n}',
      },
    ],
  },
  // ── URLs ─────────────────────────────────────────────────────────
  {
    label: 'URLs', key: 'urls',
    endpoints: [
      {
        key: 'urls_sync', label: 'Sync URL Entries', group: 'URLs', method: 'POST',
        requestBody: '{\n  "urls": [{\n    "url": string,          // Full URL (e.g. https://example.com/page)\n    "title": string,         // Browser tab/page title\n    "timestamp": string       // ISO datetime of visit\n  }]\n}',
        responseBody: '{ "synced": number  // Count of URLs accepted }',
      },
      {
        key: 'urls_get', label: 'Get URL Entries', group: 'URLs', method: 'GET',
        requestBody: 'Query params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "urls": [{\n    "id": string, "url": string,\n    "title": string, "timestamp": string\n  }],\n  "total": number\n}',
      },
    ],
  },
  // ── Activity ─────────────────────────────────────────────────────
  {
    label: 'Activity', key: 'activity',
    endpoints: [
      {
        key: 'activity_sync', label: 'Sync Activity Events', group: 'Activity', method: 'POST',
        requestBody: '{\n  "events": [{\n    "type": string,          // Event type: "keyboard" | "mouse" | "focus"\n    "app": string,           // Application name\n    "window_title": string,  // Active window title\n    "timestamp": string,     // ISO datetime\n    "data": {}               // Optional event payload\n  }]\n}',
        responseBody: '{ "synced": number  // Count of events accepted }',
      },
      {
        key: 'activity_get', label: 'Get Activity Events', group: 'Activity', method: 'GET',
        requestBody: 'Query params:\n  ?user_id=string\n  &start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "events": [{\n    "id": string, "type": string,\n    "app": string, "window_title": string,\n    "timestamp": string, "data": {}\n  }],\n  "total": number\n}',
      },
    ],
  },
  // ── Input Stats ──────────────────────────────────────────────────
  {
    label: 'Input Stats', key: 'input_stats',
    endpoints: [
      {
        key: 'input_stats_get', label: 'Get Input Stats', group: 'Input Stats', method: 'GET',
        requestBody: 'Query params:\n  ?user_id=string\n  &start=string (ISO date)\n  &end=string (ISO date)',
        responseBody: '{\n  "keyboard": number,  // Keyboard key press count\n  "mouse": number,     // Mouse click/move count\n  "idle": number       // Idle seconds\n}',
      },
    ],
  },
  // ── Dashboard ────────────────────────────────────────────────────
  {
    label: 'Dashboard', key: 'dashboard',
    endpoints: [
      {
        key: 'dashboard_today', label: 'Get Today Dashboard', group: 'Dashboard', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "total_active_seconds": number,\n  "total_idle_seconds": number,\n  "session_seconds": number,\n  "app_stats": [{\n    "app_name": string,\n    "total_seconds": number,\n    "session_count": number,\n    "category": string\n  }],\n  "recent_urls": [{\n    "url": string, "title": string, "timestamp": string\n  }],\n  "keyboard_count": number,\n  "mouse_count": number,\n  "productivity_score": number\n}',
      },
      {
        key: 'dashboard_range', label: 'Get Dashboard by Range', group: 'Dashboard', method: 'GET',
        requestBody: 'Query params:\n  ?start=string (ISO date)\n  &end=string (ISO date)',
        responseBody: '{\n  "days": [{\n    "date": string (ISO date),\n    "total_time": number,\n    "apps": [{\n      "app_name": string,\n      "total_seconds": number\n    }]\n  }]\n}',
      },
      {
        key: 'calendar_month', label: 'Get Calendar Month', group: 'Dashboard', method: 'GET',
        requestBody: 'Query params:\n  ?from=string (ISO date, first day of month)\n  &to=string (ISO date, last day of month)',
        responseBody: '{\n  "days": [{\n    "date": string (ISO date),\n    "total_seconds": number,\n    "app_count": number,\n    "has_screenshots": boolean\n  }]\n}',
      },
    ],
  },
  // ── Admin — Users ────────────────────────────────────────────────
  {
    label: 'Admin — Users', key: 'admin_users',
    endpoints: [
      {
        key: 'admin_users_list', label: 'List Company Users', group: 'Admin — Users', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "users": [{\n    "id": string, "company_id": string,\n    "username": string, "display_name": string,\n    "role": "admin" | "employee",\n    "created_at": string (ISO datetime)\n  }]\n}',
      },
      {
        key: 'admin_users_create', label: 'Create User', group: 'Admin — Users', method: 'POST',
        requestBody: '{\n  "username": string,       // Login username\n  "display_name": string,    // Display name\n  "password": string,        // Password (min 6 chars)\n  "role": "admin" | "employee"\n}',
        responseBody: '{\n  "user": {\n    "id": string, "company_id": string,\n    "username": string, "display_name": string,\n    "role": string, "created_at": string\n  }\n}',
      },
    ],
  },
  // ── Admin — Stats ────────────────────────────────────────────────
  {
    label: 'Admin — Stats', key: 'admin_stats',
    endpoints: [
      {
        key: 'admin_stats', label: 'Get Admin Stats', group: 'Admin — Stats', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "stats": [{\n    "user_id": string,\n    "display_name": string,\n    "username": string,\n    "total_active_seconds": number,\n    "session_count": number,\n    "keyboard_count": number,\n    "mouse_count": number\n  }]\n}',
      },
    ],
  },
  // ── Admin — Drill-down ───────────────────────────────────────────
  {
    label: 'Admin — Drill-down', key: 'admin_drill',
    endpoints: [
      {
        key: 'admin_user_screenshots', label: 'Get User Screenshots', group: 'Admin — Drill-down', method: 'GET',
        requestBody: 'URL template: replace {userId} with user UUID\nQuery params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "screenshots": [{\n    "id": string, "user_id": string,\n    "file_path": string, "captured_at": string\n  }],\n  "total": number\n}',
      },
      {
        key: 'admin_user_time_logs', label: 'Get User Time Logs', group: 'Admin — Drill-down', method: 'GET',
        requestBody: 'URL template: replace {userId} with user UUID\nQuery params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "logs": [{\n    "id": string, "user_id": string,\n    "app_name": string, "window_title": string,\n    "start_time": string, "end_time": string,\n    "duration": number\n  }],\n  "total": number\n}',
      },
      {
        key: 'admin_user_activity', label: 'Get User Activity', group: 'Admin — Drill-down', method: 'GET',
        requestBody: 'URL template: replace {userId} with user UUID\nQuery params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "events": [{\n    "id": string, "user_id": string,\n    "type": string, "app": string,\n    "timestamp": string\n  }],\n  "total": number\n}',
      },
      {
        key: 'admin_user_urls', label: 'Get User URLs', group: 'Admin — Drill-down', method: 'GET',
        requestBody: 'URL template: replace {userId} with user UUID\nQuery params:\n  ?start=string (ISO date)\n  &end=string (ISO date)\n  &page=number\n  &limit=number',
        responseBody: '{\n  "urls": [{\n    "id": string, "user_id": string,\n    "url": string, "title": string,\n    "timestamp": string\n  }],\n  "total": number\n}',
      },
      {
        key: 'admin_user_input_stats', label: 'Get User Input Stats', group: 'Admin — Drill-down', method: 'GET',
        requestBody: 'URL template: replace {userId} with user UUID\nQuery params:\n  ?start=string (ISO date)\n  &end=string (ISO date)',
        responseBody: '{\n  "keyboard": number,\n  "mouse": number,\n  "idle": number\n}',
      },
    ],
  },
  // ── App Categories ───────────────────────────────────────────────
  {
    label: 'App Categories', key: 'categories',
    endpoints: [
      {
        key: 'app_categories_get', label: 'Get App Categories', group: 'App Categories', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "categories": [{\n    "app_name": string,   // Application executable name\n    "category": string    // "productive" | "neutral" | "unproductive"\n  }]\n}',
      },
      {
        key: 'app_categories_update', label: 'Update App Category', group: 'App Categories', method: 'PUT',
        requestBody: '{\n  "app_name": string,   // Application executable name\n  "category": string    // "productive" | "neutral" | "unproductive"\n}',
        responseBody: '{ "success": true }',
      },
    ],
  },
  // ── App Config ───────────────────────────────────────────────────
  {
    label: 'App Config', key: 'config',
    endpoints: [
      {
        key: 'config_get', label: 'Get App Config', group: 'App Config', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "mode": "single_user" | "multi_user",\n  "setup_done": boolean\n}',
      },
      {
        key: 'config_save', label: 'Save App Config', group: 'App Config', method: 'POST',
        requestBody: '{\n  "mode": "single_user" | "multi_user",\n  "setup_done": boolean\n}',
        responseBody: '{ "success": true }',
      },
    ],
  },
  // ── Settings ─────────────────────────────────────────────────────
  {
    label: 'Settings', key: 'settings',
    endpoints: [
      {
        key: 'settings_get', label: 'Get Settings', group: 'Settings', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "settings": {\n    "language": string, "theme": string,\n    "screenshot_interval": number, "screenshot_location": string,\n    "backup_frequency": string, "backup_location": string,\n    "idle_threshold": number,\n    "pomodoro_focus_minutes": number, ...\n  }\n}',
      },
      {
        key: 'settings_update', label: 'Update Settings', group: 'Settings', method: 'POST',
        requestBody: '{\n  "settings": {\n    "language": "en" | "bn",\n    "theme": "light" | "dark" | "system",\n    "screenshot_interval": number (minutes),\n    "screenshot_location": string (directory path),\n    "idle_threshold": number (minutes),\n    "is_screenshot_enabled": boolean,\n    "backup_frequency": "never" | "daily" | "weekly",\n    "backup_location": string,\n    "pomodoro_focus_minutes": number,\n    "pomodoro_short_break_minutes": number,\n    "pomodoro_long_break_minutes": number,\n    "pomodoro_sessions_before_long": number,\n    "pomodoro_auto_start": boolean,\n    "pomodoro_sound_enabled": boolean\n  }\n}',
        responseBody: '{ "success": true }',
      },
    ],
  },
  // ── Backup & Updates ────────────────────────────────────────────
  {
    label: 'Backup & Updates', key: 'backup',
    endpoints: [
      {
        key: 'backup_export', label: 'Export Backup', group: 'Backup & Updates', method: 'GET',
        requestBody: '',
        responseBody: 'Binary ZIP file containing:\n  - tracker.db (SQLite database)\n  - screenshots/ (PNG image files)\n  - api-config.json (endpoint configuration)',
      },
      {
        key: 'backup_import', label: 'Import Backup', group: 'Backup & Updates', method: 'POST',
        requestBody: 'FormData field:\n  file: binary (ZIP archive)\n    Expected ZIP contents:\n    - tracker.db (required)\n    - screenshots/ (optional)\n    - api-config.json (optional)',
        responseBody: '{ "success": true }',
      },
      {
        key: 'update_check', label: 'Check for Updates', group: 'Backup & Updates', method: 'GET',
        requestBody: '',
        responseBody: '[\n  {\n    "name": string   // Git tag name, e.g. "v0.0.9"\n  }\n]',
      },
    ],
  },
  // ── Pomodoro ─────────────────────────────────────────────────────
  {
    label: 'Pomodoro', key: 'pomodoro',
    endpoints: [
      {
        key: 'pomodoro_start', label: 'Start Pomodoro Focus', group: 'Pomodoro', method: 'POST',
        requestBody: '{}  // Server starts focus phase using default or user settings',
        responseBody: '{\n  "phase": "focus",\n  "remaining_secs": number (e.g. 1500 for 25min)\n}',
      },
      {
        key: 'pomodoro_skip', label: 'Skip Pomodoro Phase', group: 'Pomodoro', method: 'POST',
        requestBody: '{}  // Server advances to next phase',
        responseBody: '{\n  "phase": "short_break" | "long_break" | "focus",\n  "remaining_secs": number\n}',
      },
      {
        key: 'pomodoro_stop', label: 'Stop/Reset Pomodoro', group: 'Pomodoro', method: 'POST',
        requestBody: '{}  // Server resets pomodoro to idle',
        responseBody: '{ "phase": "idle", "remaining_secs": 0 }',
      },
      {
        key: 'pomodoro_status', label: 'Get Pomodoro Status', group: 'Pomodoro', method: 'GET',
        requestBody: '',
        responseBody: '{\n  "phase": "idle" | "focus" | "short_break" | "long_break",\n  "remaining_secs": number,\n  "count_today": number  // Focus sessions completed today\n}',
      },
    ],
  },
  // ── Autostart ────────────────────────────────────────────────────
  {
    label: 'Autostart', key: 'autostart',
    endpoints: [
      {
        key: 'autostart_set', label: 'Set Autostart', group: 'Autostart', method: 'POST',
        requestBody: '{\n  "enabled": boolean  // true = launch on boot, false = disable\n}',
        responseBody: '{ "success": true }',
      },
      {
        key: 'autostart_get', label: 'Get Autostart Status', group: 'Autostart', method: 'GET',
        requestBody: '',
        responseBody: '{ "enabled": boolean }',
      },
    ],
  },
  // ── Reset ────────────────────────────────────────────────────────
  {
    label: 'Reset', key: 'reset',
    endpoints: [
      {
        key: 'reset_app', label: 'Factory Reset App', group: 'Reset', method: 'POST',
        requestBody: '{}  // Server deletes all user data and resets to factory state',
        responseBody: '{ "success": true }',
      },
    ],
  },
];
